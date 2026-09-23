#!/usr/bin/env python3
"""workflow-engine CLI — 工作流状态机的薄壳。

由 workflow-engine 及各节点（`.workflow/node/<节点>/NODE.md`）通过 `python3 .workflow/engine/cli.py <subcommand>` 调用，
把 state.json / feedback INDEX / issue 模板等机械动作收口到一处。
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))

from _lib import config as config_lib  # noqa: E402
from _lib import issue as issue_lib  # noqa: E402
from _lib import nodes  # noqa: E402
from _lib import state as state_lib  # noqa: E402
from _lib import telemetry as telemetry_lib  # noqa: E402
from _lib import upgrade as upgrade_lib  # noqa: E402
from _lib import wizard as wizard_lib  # noqa: E402

ISSUE_ID_RE = re.compile(r"^issue-\d+$")

CATEGORY_HELP = "沉淀归属三分类：design（设计）/code（编码）/test（测试）——决定问题沉淀写进哪个 *-rule.md"


def _die(msg: str):
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def _detect_cli_version(cmd: list[str]) -> str | None:
    """跑外部 CLI 版本探测命令，取其 stdout 首行；命令不存在/报错/超时都软失败返回 None，不阻断 init。

    WORKFLOW_SKIP_VERSION_DETECT=1 时直接跳过（自测用：避免依赖本机是否装了 lb/lbcli 全局包，
    也避免每次 init 都 fork 子进程拖慢测试）。"""
    if os.environ.get("WORKFLOW_SKIP_VERSION_DETECT") == "1":
        return None
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=5)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    return result.stdout.strip().splitlines()[0].strip() if result.stdout.strip() else None


def _require_lbcli_skill():
    """防呆：lbcli skill 是工作流强依赖（查 DB/日志/JSF/配置等内部系统都靠它）。
    全局 ~/.claude/skills/lbcli/ 未安装则拒绝 init，提示先装。
    skills 根可用 WORKFLOW_SKILLS_HOME 覆盖（测试/非标准 HOME 用）。"""
    skills_home = Path(os.environ.get("WORKFLOW_SKILLS_HOME") or Path.home())
    if not (skills_home / ".claude" / "skills" / "lbcli" / "SKILL.md").exists():
        _die("未检测到全局 lbcli skill（~/.claude/skills/lbcli/），工作流强依赖它操作内部研发系统。\n"
             "       请先运行 `lbcli setup` 安装全局 skill，装好后再跑 init。")


def _now() -> str:
    return datetime.now().strftime("%Y-%m-%d %H:%M:%S")


def _read_json_or_none(p: Path):
    """读 JSON 文件原文（供上报打包）；不存在/解析失败返回 None，绝不阻断主流程。"""
    try:
        with p.open() as f:
            return json.load(f)
    except (OSError, ValueError):
        return None


# ── 活跃任务清单（delivery/active-tasks.json）——供线上工作流平台读取本仓库当前有哪些活跃
#    （未归档）任务、最近建的是哪个。只反映活跃任务、非全量历史索引。
#    落 delivery/ 下而非 .workflow/：.workflow 每次 `lb init` 整目录覆盖会抹掉运行时文件，
#    delivery/ 只在首次建空目录、从不覆盖已有内容，故运行时产物安全。由 cli.py 单独维护，
#    模板不预置。tasks 数组按创建顺序排（append），latest_task 指最近一次 init 建的任务。
#    维护方式：init 登记，advance 推到 done 那一刻摘除（见 cmd_advance）。摘除不等 archive-move
#    ——那条命令要 AI 自己记得跑，漏跑就永久残留；而 done 是引擎自己算出来的，绑它才可靠。
#    单向即够：done 是终态（enter/set-env/reroute 都挡 done），不存在「归档后又活过来」。
def _tasks_index_path(target: Path) -> Path:
    return target / "delivery" / "active-tasks.json"


def _load_tasks_index(target: Path) -> dict:
    """读任务清单；不存在/坏 JSON/结构非法都当空清单，不阻断主流程。"""
    data = _read_json_or_none(_tasks_index_path(target))
    if not isinstance(data, dict) or not isinstance(data.get("tasks"), list):
        return {"tasks": [], "latest_task": None}
    return {"tasks": data["tasks"], "latest_task": data.get("latest_task")}


def _save_tasks_index(target: Path, idx: dict) -> None:
    """写任务清单；软失败——写不动绝不能拖垮 init/archive 主流程。

    但**失败要出声**（stderr 一行）：全静默会让「归档了却还挂在清单上」看起来完全正常
    （退出码 0、✓ 照打），线上平台读到脏数据也无从追溯。提示不阻断、不改退出码。
    """
    try:
        p = _tasks_index_path(target)
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(idx, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    except OSError as e:
        print(f"⚠️ 活跃任务清单 delivery/active-tasks.json 写入失败（{e}）——"
              f"清单可能与实际任务状态不一致，不影响本次流转。", file=sys.stderr)


def _norm_task(task: str) -> str:
    """任务名归一化：吃掉首尾空白与斜杠。

    tab 补全会带出 `delivery/foo/` 这种尾斜杠，而 Path 拼接会静默吞掉它——
    目录动作照常成功，清单里按字符串全等比对就匹配不上，摘除静默失效且退出码为 0。
    故凡拿 task 参数比对清单/打印的地方，先过这里。
    """
    return task.strip().strip("/")


def _tasks_index_add(target: Path, task: str, created_at: str) -> None:
    """新建任务：append 到 tasks 末尾（同名先去重防重复），latest_task 指向它。"""
    task = _norm_task(task)
    idx = _load_tasks_index(target)
    idx["tasks"] = [t for t in idx["tasks"] if t.get("task") != task]
    idx["tasks"].append({"task": task, "created_at": created_at})
    idx["latest_task"] = task
    _save_tasks_index(target, idx)


def _tasks_index_remove(target: Path, task: str) -> None:
    """摘出活跃清单；latest_task 回退指向剩余最后一个（空则 None）。幂等。

    两处调用：advance 推到 done（常态，不等 archive-move）、archive-move（兜底，
    主要服务 --force 直迁那条没走到 done 的路）。已不在清单时直接返回、不写盘——
    免得幂等空转触发写失败告警。
    """
    task = _norm_task(task)
    idx = _load_tasks_index(target)
    if not any(t.get("task") == task for t in idx["tasks"]):
        return
    idx["tasks"] = [t for t in idx["tasks"] if t.get("task") != task]
    idx["latest_task"] = idx["tasks"][-1]["task"] if idx["tasks"] else None
    _save_tasks_index(target, idx)


def _archive_move_pending_hint(target: Path, task: str) -> list:
    """done 但 delivery/<task>/ 还在原位 → 提示还差物理归档一步；已迁走返回空列表。

    补这条是因为 done 的三个出口（advance / next / run-loop）原本一句都没提 archive-move，
    next 还直接打印「任务已归档」——物理归档只写在归档节点文档里，AI 漏读或被 compact
    掉那段就断在这，任务目录永远留在 delivery/ 下。

    只提示不代劳：archive-move 前还有 schema 迁出根 docs/schema/、retro 落盘等真实工作
    （见归档节点文档），且它是不可逆收口，不能由引擎替人按下。
    """
    task = _norm_task(task)
    if not (target / "delivery" / task).is_dir():
        return []
    return [f"  ⚠️ 尚未物理归档：delivery/{task}/ 还在原位。归档节点收尾步骤（schema 迁出、retro）做完后，"
            f"跑 python3 .workflow/engine/cli.py archive-move {task} 迁入 delivery/archive/，"
            f"再按归档节点的提交纪律走一次兜底 commit。"]


def _report_state(target: Path, task: str) -> None:
    """流转收口：读当前 state.json，按其冻结的 telemetry 配置上报全量原文。

    仅在改状态命令 save_state 之后调用。守卫/spawn/失败静默全在 telemetry_lib.report 内，
    这里读 state 失败也吞掉——上报绝不能影响主流程。
    项目标识（light-app-name）由 telemetry 侧从 state.identity.git_remote 解析。
    """
    try:
        st = state_lib.load_state(target, task)
    except Exception:
        return
    telemetry_lib.report(st.get("telemetry"), {"event": "advance", "state": st})


def _next_phrase(pipeline, node: str, task: str, target_root=None) -> str:
    """「下一步」引导文本（含 task）。执行单元是 `/skill` 或文档节点别名 `/workflow-engine node <名>`，
    两者都是正经斜杠命令，task 统一作为命令参数拼在后面（`/… <task>`）。"""
    units = nodes.skills_for(pipeline, node)
    chain = nodes.skill_chain(pipeline, node, target_root)
    lead = "依次跑 " if len(units) > 1 else ""
    return f"{lead}{chain} {task}"


def _hook_chain(skills: list, target_root=None) -> str:
    """钩子执行单元列表的展示文本：按 node_ref 自动区分 `/skill` 与文档节点别名 `/workflow-engine node <名>`。"""
    return " → ".join(nodes.node_ref(s, target_root) for s in skills)


def _env_line(env: dict) -> str:
    """一行展示任务环境块（init 末尾 + enter 上下文包共用）。"""
    parts = [f"测试目标环境：{env['target']}（写物料/部署/回归须钉死此环境；读请求任何环境皆可）",
             f"范围={'+'.join(env['scope'])}"]
    if env.get("http_domain"):
        parts.append(f"HTTP域名={env['http_domain']}")
    if env.get("jsf_alias"):
        parts.append(f"JSF别名={env['jsf_alias']}")
    if env.get("eone_feature_env"):
        parts.append(f"eone={env['eone_feature_env']}")
    return "  ".join(parts)


def _resolve_target(target: str | None) -> Path:
    return Path(target).resolve() if target else Path.cwd().resolve()


INIT_SUBDIRS = ("prd", "schema", "test", "review", "feedback", "workspace")


def _has_real_content(d: Path) -> bool:
    """目录存在且含 .gitkeep 之外的真实文件。"""
    return d.is_dir() and any(p.name != ".gitkeep" for p in d.iterdir())


def _project_initialized(target: Path) -> bool:
    """项目知识底座是否已落地。

    判据：bootstrap-init.sh 的进度文件 .workflow/bootstrap-state.json 里，
    overview + 至少一个 module + 至少一个 tech 都 done。
    overview（project-overview.md）是「始终加载的压缩包」、知识底座的锚；
    再要求至少各落地一个 module-knowledge / tech-knowledge，才算后续节点能加载到真实上下文。
    该文件由 bootstrap-init.sh discover/generate 产出。
    """
    state = target / ".workflow" / "bootstrap-state.json"
    legacy = target / ".claude" / "bootstrap-state.json"
    # 迁移：旧位置 .claude/ → .workflow/（仅当旧存在、新不存在）
    if legacy.exists() and not state.exists():
        try:
            state.parent.mkdir(parents=True, exist_ok=True)
            legacy.rename(state)
        except OSError:
            state = legacy
    if not state.exists():
        return False
    try:
        data = json.loads(state.read_text(encoding="utf-8"))
    except (OSError, ValueError):
        return False

    def _done(node) -> bool:
        return isinstance(node, dict) and node.get("done") is True

    def _any_done(group) -> bool:
        return isinstance(group, dict) and any(_done(v) for v in group.values())

    return _done(data.get("overview")) and _any_done(data.get("modules")) and _any_done(data.get("tech"))


# ── 起步分支闸门 ──────────────────────────────────────────────────────
# master* / main* / release* 是集成分支，任务产物（delivery/、TRD、代码）不该落在上面。
# 前缀匹配（小写后 startswith），故 master-fix / release-1.2 / releases 一并命中。
# 只守「起任务」这一刻的两个入口（list-profiles = /workflow-start 正门、init），
# 不守 enter/advance（任务中途切分支是另一个问题），也不守 TS 侧 `lb init`（保护分支上刷模板照常可用）。
PROTECTED_BRANCH_PREFIXES = ("master", "main", "release")


def _current_branch(target: Path) -> str | None:
    """当前分支名；非 git 仓库 / 无 git / detached HEAD 都返回 None。

    用 `git branch --show-current`（Git 2.22+，专为脚本设计）：无 commit 的空仓库照样
    给出分支名，detached HEAD 给空串——两种边界一条命令覆盖，无需特判。
    别换成 `rev-parse --abbrev-ref HEAD`：它 detached 时返回字面量 "HEAD"，会被当成分支名。
    刻意不复用 telemetry._git —— 那条路径认 WORKFLOW_SKIP_TELEMETRY=1 返回 None，
    闸门借它等于自带一个环境变量旁路。
    """
    try:
        result = subprocess.run(
            ["git", "-C", str(target), "branch", "--show-current"],
            capture_output=True, text=True, timeout=5,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    out = result.stdout.strip()
    return out.splitlines()[0].strip() if out else None


def _gate_start_branch(target: Path, retry_hint: str) -> str | None:
    """保护分支拦截（命中即 die）。返回当前分支名，供调用方复用（如 init 比对任务名）。

    探测不到分支（非 git / 无 git / detached HEAD）→ 静默放行返回 None：
    本模块跑在用户机器上、难更新，环境探测只能软失败降级，测不出就不管，绝不误挡。
    """
    branch = _current_branch(target)
    if branch is None:
        return None
    if not branch.lower().startswith(PROTECTED_BRANCH_PREFIXES):
        return branch
    _die(
        f"当前分支 `{branch}` 是保护分支（{' / '.join(p + '*' for p in PROTECTED_BRANCH_PREFIXES)} 一律不允许起任务）。\n"
        f"       集成分支上不开任务：delivery/ 产物、TRD、代码都该落在特性分支。\n"
        f"       {retry_hint}：git checkout -b feature-<需求名>\n"
        f"       该闸门无旁路（不认 --force/-y），必须切分支。"
    )


def _gate_task_name(target: Path, task: str, branch: str | None):
    """init 前置：任务名唯一性 + 不与当前分支同名。多条违规合并成一条报错。

    唯一性判据用文件系统（delivery/<task>/state.json 活跃 + delivery/archive/<task>/ 归档），
    不信 delivery/active-tasks.json —— 那个索引本身是软失败维护的（写不动就 pass），
    当校验依据既会误挡也会漏挡。
    挡归档同名是为了早失败：不挡的话任务能一路跑到 workflow-archive，
    最后一步 archive-move 才报「目标已存在」，失败被推到成本最高的时刻。
    目录存在但无 state.json（如 prd-pre-review 先落了 prd/）不算重名，仍走半成品收编。
    """
    violations = []
    if branch is not None and task.strip().lower() == branch.strip().lower():
        violations.append("与当前分支名相同 —— 任务名描述需求语义，不是分支名的复制品。")
    if state_lib.state_path(target, task).exists():
        violations.append(
            f"delivery/{task}/ 已初始化（state.json 存在，同名活跃任务）—— "
            f"看它的状态跑 /workflow-engine status {task}。")
    if (target / "delivery" / "archive" / task).exists():
        violations.append(
            f"delivery/archive/{task}/ 已存在（同名归档任务）—— 任务名须全仓库唯一，"
            f"否则归档时 archive-move 撞车。")
    if not violations:
        return
    lines = "\n".join(f"       · {v}" for v in violations)
    _die(f"任务名 `{task}` 不可用（{len(violations)} 项）：\n{lines}\n       换个名字重跑 init。")


def _parse_scope(raw: str) -> list:
    items = [s.strip() for s in raw.split(",") if s.strip()]
    seen = []
    for s in items:
        if s not in state_lib.ENV_SCOPES:
            _die(f"非法 --scope 值 '{s}'（只能是 {' / '.join(state_lib.ENV_SCOPES)} 的逗号分隔子集）")
        if s not in seen:
            seen.append(s)
    if not seen:
        _die("--scope 不能为空")
    return seen


def _env_from_flags(args, has_dedicated: bool) -> dict:
    """非交互 flag 路径：组装 env，执行 has_dedicated 拦截。"""
    target = args.env
    if target is None:
        target = "test" if has_dedicated else "gamma"
    elif not has_dedicated and target in ("test", "local"):
        _die(f"项目无独立测试环境（.workflow/config.json 的 test_env.has_dedicated=false），"
             f"不能选 --env {target}；本地依赖测试环境同样起不来。需要测试环境请先改全局配置 has_dedicated=true 再来。")
    scope = _parse_scope(args.scope) if args.scope else ["http"]
    eone = args.eone if target == "test" else None
    return {
        "target": target,
        "scope": scope,
        "http_domain": args.http_domain if "http" in scope else None,
        "jsf_alias": args.jsf_alias if "jsf" in scope else None,
        "eone_feature_env": eone,
    }


def _gate_app_info(cfg: dict, skip_flag: bool) -> bool:
    """init 前置：按 app_info.policy 三态守应用信息是否已采
    （config.json 的 app_info.app + app_id 均非空才算已采）。

    返回 skipped（是否走了逃生开关），供 cmd_init 记进 state 留痕。
    - off / 已采(collected)：静默放行，返回 False。
    - required 未采：
        --skip-app-info → 逃生放行，打红字警告下游会退化，返回 True（留痕）。
        否则 loud fail：采集不是 init 能代劳的（要 AI 跑 lbcli 起浏览器），故阻断、指路，不静默。
    - suggested 未采：loud 提示不阻断，返回 False（建议档不留 skip 痕迹）。

    采集本身由 AI 临时执行（问 appName → lbcli xingyun app-info → 按 key 映射填 config.json.app_info，
    接口驼峰翻成 snake_case），完整流程与失败处理写在 config.json 的 _doc_app_info（失败时触发式加载，
    不占常驻文档）。此处只做门禁，不跑网络。
    """
    ai = cfg["app_info"]
    if ai["policy"] == "off" or ai["collected"]:
        return False

    if ai["policy"] == "required":
        if skip_flag:
            print("⚠️  已跳过应用信息采集（--skip-app-info）。下游节点（部署/排查/转测）拿不到应用信息，")
            print("    会退回逐次询问你。建议尽快补采：让 AI 跑 lbcli xingyun app-info 填 .workflow/config.json 的 app_info。")
            return True
        _die(
            "本项目 app_info.policy=required，但 .workflow/config.json 的 app_info.app 未填（应用信息未采集）。\n"
            "       应用信息是发布/排查/转测的准确来源，须先采一次（完整流程/失败处理见 config.json 的 _doc_app_info）：\n"
            "       ① 问用户应用名 appName；② 跑 `lbcli xingyun app-info --app <名> -f json`；\n"
            "       ③ 把结果按 key 映射填进 config.json 的 app_info 块（接口驼峰翻成 snake_case：appId→app_id 等），再 init。\n"
            "       登录不了/网不通等临时拿不到 → 加 --skip-app-info 逃生（留痕告警，下游退化为询问）；\n"
            "       本项目根本没有 JDOS 应用（纯库/工具）→ 把 config.json 的 app_info.policy 改成 off，永久免采。"
        )

    # suggested 未采：建议不强制。
    print("ℹ️  建议先采集应用信息（app_info.policy=suggested，当前未采）：让 AI 跑 lbcli xingyun app-info 填 config.json，")
    print("   发布/排查/转测会更省事。本次不阻断，继续 init。")
    return False


def _resolve_init_choices(args, target_root):
    """决定本任务的 (profile, env)。

    profile 与 env 解耦：--profile 只定编排模式，不短路环境收集。
    env 分支：任一 env flag → flag 路径（显式优先）；否则 tty → 向导；否则（非 tty）→ 见下。
    非 tty（AI 编排路径）无 env flag：需要环境的 profile 直接 loud fail（不静默默认，逼 AI 先收集），
    无端到端自动化测试的 profile（has_e2etest=false）例外——环境坐标无处消费，静默钉 local。
    显式 env flag 全程尊重（loud override）。
    profile：--profile 优先；否则 tty 向导选；否则（非 tty）loud fail 逼显式带 --profile
    （不静默默认——AI 编排下 profile 靠记忆传递，静默默认会悄悄落错模式）。
    """
    cfg = config_lib.load_project_config(target_root)
    has_dedicated = cfg["test_env"]["has_dedicated"]
    profiles = {k: v for k, v in cfg["profiles"].items() if not k.startswith("_")}
    profile_names = list(profiles)
    default_profile = cfg["default_profile"]

    # --profile 显式指定：允许禁用的 profile（bypass 展示层过滤），仅校验已定义。
    if args.profile is not None and args.profile not in profile_names:
        _die(f"未知 --profile {args.profile}（现有：{', '.join(profile_names)}）")

    # 向导候选只列 enabled 的 profile（禁用的用户拉不到，仅显式 --profile 可用）。
    enabled_names = [n for n in profile_names if nodes.is_enabled(profiles[n])]

    e2e_by_profile = {name: nodes.has_e2etest(prof) for name, prof in profiles.items()}

    env_flag_names = ("env", "scope", "http_domain", "jsf_alias", "eone")
    any_env_flag = any(getattr(args, n) is not None for n in env_flag_names)

    profile = args.profile
    # profile 护栏（对齐 env 的 loud-fail）：非 tty（AI 编排）下必须显式 --profile。
    # profile 靠 AI 记忆从 workflow-start 经探索期传来，compact 后易丢，静默默认会落错模式。
    if profile is None and not sys.stdin.isatty():
        _die(
            "非交互 init 未收到 --profile，不静默默认。\n"
            "       回到 workflow-start 用户选定的 profile，带上重跑：\n"
            f"       init {args.task} --profile <名> ..."
        )
    if any_env_flag:
        env = _env_from_flags(args, has_dedicated)
    elif sys.stdin.isatty():
        descs = {k: (v.get("desc") if isinstance(v, dict) else None) for k, v in profiles.items()}
        picked = wizard_lib.run_wizard(enabled_names, default_profile, has_dedicated, descs,
                                       preselected_profile=profile,
                                       e2etest_by_profile=e2e_by_profile)
        env = picked["env"]
        if profile is None:
            profile = picked["profile"]
    else:
        effective = profile or default_profile
        if not e2e_by_profile.get(effective, True):
            # 无端到端自动化测试的 profile：环境坐标无处消费，静默钉 local，不拦。
            env = state_lib.default_env()
            env["target"] = "local"
            env["eone_feature_env"] = None
        else:
            # 非 tty（AI 编排路径）下 profile 需要环境却没给任何 env flag：
            # 静默默认会给出用户从没确认过的环境（如 target=test），loud fail 逼先收集再带 flag 重跑。
            _die(
                f"profile「{effective}」含端到端自动化测试，需显式环境配置，但未收到任何 env flag。\n"
                f"       非交互执行 init 不得静默默认环境——请先向用户收集，再带 flag 重跑：\n"
                f"       init {args.task} --profile {effective} --env <test|gamma|local> "
                f"--scope <http[,jsf]> [--http-domain <d>] [--jsf-alias <a>] [--eone <e>]\n"
                f"       （env 收集契约见 .workflow/engine/INIT.md「AI 编排路径」）"
            )

    if profile is None:
        profile = default_profile
    return profile, env


def cmd_init(args):
    _require_lbcli_skill()
    state_lib.validate_task_name(args.task)
    if args.task == "archive":
        _die("archive 是归档区保留名（delivery/archive/），不能作任务名")
    target = _resolve_target(args.target)
    # 起步分支闸门（先于一切写操作）：保护分支上不许起任务。返回分支名供下面比对任务名。
    branch = _gate_start_branch(target, "切分支后重跑 init")
    # 任务名校验（唯一性 + 不与分支同名）：一次报全，AI 编排下每次 die 都是一轮往返。
    # 守卫认 state.json，不认目录：有 state.json 才是已初始化任务，硬拒；
    # 只有目录（如 prd-pre-review 先落了 prd/）= 半成品骨架，收编补齐而非拒绝。
    _gate_task_name(target, args.task, branch)
    task_dir = target / "delivery" / args.task
    adopting = task_dir.exists()
    pre_filled = [sub for sub in INIT_SUBDIRS if _has_real_content(task_dir / sub)] if adopting else []
    # 应用信息门禁（可能 loud fail）：先于建骨架，失败不留孤儿目录。
    app_info_skipped = _gate_app_info(config_lib.load_project_config(target), args.skip_app_info)
    # 先定 profile+env（可能 loud fail），再建骨架——失败不留孤儿目录。
    profile, env = _resolve_init_choices(args, target)
    for sub in INIT_SUBDIRS:
        (task_dir / sub).mkdir(parents=True, exist_ok=True)
    cfg = config_lib.load_project_config(target)
    resolved = nodes.resolve_profile_full(cfg["profiles"], profile, cfg["default_profile"])
    frozen = resolved["pipeline"]
    roles = nodes.resolve_roles(resolved["roles"], args.task)
    npm_lb_version = _detect_cli_version(["lb", "-V"])
    npm_lbcli_version = _detect_cli_version(["lbcli", "-V"])
    # 仓库态：本仓库上次 `lb init` 落地的模板版本，取自 config.json.lb_version（cfg 上面已 load）；缺省软失败为 None。
    repo_lb_version = cfg.get("lb_version") or None
    # 上报配置与身份：init 时一次性拉取/收集并冻进 state.json（后续流转只读、不刷新，均软失败不阻断）。
    init_ts = _now()  # 秒级时间戳，作服务端上报主键组件（giturl+erp+task+init_ts）
    telemetry = telemetry_lib.fetch_remote_config()
    identity = telemetry_lib.collect_identity(target)
    st = state_lib.default_state(args.task, _now(), frozen, env, roles, profile,
                                 npm_lb_version, npm_lbcli_version, repo_lb_version,
                                 telemetry=telemetry, identity=identity, init_ts=init_ts,
                                 start_node=resolved.get("start_node"))
    if app_info_skipped:
        st["app_info_skipped"] = True  # 逃生开关留痕：应用信息未采，下游读到此标记可提示退化
    # 上下文基线：init 在 start_node（如 explore，状态机外先跑）之后，故此值即「探索消耗」快照。
    # 软失败为 None → 不塞字段，不阻断 init。
    init_ctx = telemetry_lib.collect_context_used()
    if init_ctx["context_used"] is not None:
        st["init_context_used"] = init_ctx["context_used"]
        st["init_session_id"] = init_ctx["session_id"]
        if init_ctx["model"] is not None:
            st["init_model"] = init_ctx["model"]
        if init_ctx["turns"] is not None:
            st["init_turns"] = init_ctx["turns"]
        # 起步基线：会话「什么都还没干」时的上下文占用（system prompt + CLAUDE.md + 工具定义 +
        # skills 索引）。只在 init 落一次——它是会话属性而非节点属性，enter/advance 重复存无意义；
        # init_context_used 减它即起步链路（workflow-start → start_node 澄清）真实烧掉的量。
        if init_ctx["baseline_context_used"] is not None:
            st["baseline_context_used"] = init_ctx["baseline_context_used"]
    telemetry_lib.apply_token_snapshot(st, init_ctx.get("token_usage"))
    state_lib.save_state(target, args.task, st)
    # 任务清单登记：新任务 append 到 delivery/active-tasks.json、latest_task 指向它（供线上平台读取）。
    _tasks_index_add(target, args.task, init_ts)
    # init 上报：项目级两文件 + state 打一个包发一次（config 恒有，bootstrap-state 存在才带）。
    telemetry_lib.report(telemetry, {
        "event": "init",
        "config": _read_json_or_none(config_lib.config_path(target)),
        "bootstrap_state": _read_json_or_none(target / ".workflow" / "bootstrap-state.json"),
        "state": st,
    })
    issue_lib.write_index(target, args.task, st)
    seq = nodes.active_sequence(st["pipeline"])
    seed = seq[0]
    if adopting and pre_filled:
        print(f"✓ delivery/{args.task}/ 已存在（无 state.json），按半成品收编：保留 {', '.join(s + '/' for s in pre_filled)} 原有内容，补齐其余骨架")
    elif adopting:
        print(f"✓ delivery/{args.task}/ 已存在（无 state.json），已补齐骨架")
    else:
        print(f"✓ delivery/{args.task}/ 已就绪")
    print(f"✓ state.json 已初始化（current_step={seed}）")
    print(f"  profile: {profile}（编排模式，取自 .workflow/profile/；init 时冻结进 state.json）")
    if st.get("start_node"):
        print(f"  start_node: {st['start_node']}（该模式起步节点，状态机外正门；init 时快照进 state.json）")
    print(f"  {_env_line(env)}")
    print(f"  npm lb: {npm_lb_version or '未探测到'} / npm lbcli: {npm_lbcli_version or '未探测到'}（init 时机器全局工具版本快照）")
    print(f"  repo lb: {repo_lb_version or '未知'}（本仓库落地模板版本，取自 config.json）")
    print(f"  本任务节点序列：{' → '.join(seq)}")
    print(f"  转场闸门由各节点 exit_gate/entry_gate 声明（auto/confirm/confirm-locked）；")
    print()
    print(f"起步入口：把需求起点（plan/change/prd 位置或一句话描述）写入 delivery/{args.task}/context.md，"
          f"供切窗口/新窗口冷启动找回起点。")
    if roles:
        print(f"产物区域（role）：跑 /workflow-engine roles {args.task} 查本任务各 role 的落盘路径。")
    print(f"下一步：{_next_phrase(st['pipeline'], seed, args.task, target)}")
    # 首节点即无人区入口时补打一次 run-loop 强制确认：init 直接 set current_step 不走 advance，
    # cmd_advance 的 is_zone_entry 钩子够不着 seed，否则「第一个节点就是 autopilot」的 profile 会漏问。
    if nodes.is_zone_entry(st["pipeline"], seed):
        hint = nodes.run_loop_zone_entry_confirm(seed, st["pipeline"])
        if hint:
            print(hint.replace("<task>", args.task))
    print(f"查看状态：/workflow-engine status {args.task}")
    if not _project_initialized(target):
        print()
        print("⚠️  项目知识底座未初始化（需 .workflow/bootstrap-state.json 中 overview + 至少一个 module + 一个 tech 落地）。")
        print("    可能会一定程度影响需求理解与产物质量。")
        print("    知识底座初始化须在独立窗口做（对上下文占用极高，在本窗口做会严重影响当次需求交付）——")
        print("    本窗口继续当前任务即可，需要时另开窗口沉淀知识底座。")


def cmd_status(args):
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    cur = st["main"]["current_step"]
    hist = st["main"]["history"]
    fb = st["feedback"]
    seq = nodes.active_sequence(st["pipeline"])
    profile = st.get("profile")
    print(f"profile:   {profile or '（未知——init 前的老任务）'}")
    print(f"main:      {cur}  (history: {len(hist)} 个节点已完成 / 本任务共 {len(seq)} 节点)")
    print(f"feedback:  {len(fb['open'])} open / {len(fb['resolved'])} resolved")
    pg = st["main"].get("pending_gate")
    if pg:
        print(f"pending:   {pg['from']} → {pg['to']}（闸门 {pg['gate']}，待兑现）")


def cmd_roles(args):
    """打印本任务冻结的 role 清单（role → 解析后路径，<task> 已替换）。

    role = 产物区域的语义名；skill 正文凡指到某 role、不确定路径就查这里。
    """
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    roles = st.get("roles") or {}
    if not roles:
        print(f"本任务（profile 未声明 roles）无 role 清单；产物路径见各节点文档（.workflow/node/<节点>/NODE.md）正文。")
        return
    print(f"本任务产物区域（role → 路径）：")
    for role, path in roles.items():
        print(f"  {role}: {path}")


def cmd_profile(args):
    """打印本任务冻结的 profile 名（init 时选定，冻结进 state.json）。

    供节点判断血统（如名字含 openspec 的 profile 走 openspec skill）。老任务（init 前）无此字段 → 打印空。
    """
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    profile = st.get("profile")
    if not profile:
        print("（未知——本任务在 profile 落盘之前创建，state.json 无 profile 字段）")
        return
    print(profile)


def cmd_project_type(args):
    """打印项目类型（backend|frontend），供节点判前后端血统。

    项目级固定属性，读 .workflow/config.json 的 project_type（缺失退化 backend），与 task 无关。
    """
    target = _resolve_target(args.target)
    cfg = config_lib.load_project_config(target)
    print(cfg["project_type"])


def cmd_remote_config_files(args):
    """打印 JDOS 实例配置三态（unknown|true|false）；`--set` 写入后回显。无需 task。"""
    target = _resolve_target(args.target)
    set_value = getattr(args, "set_value", None)
    if set_value is not None:
        try:
            print(config_lib.write_remote_config_files(target, set_value))
        except ValueError as e:
            _die(str(e))
        return
    print(config_lib.read_remote_config_files(target))


def _maybe_auto_upgrade(target: Path, cfg: dict) -> bool:
    """检测新版并代跑升级；全程软失败。返回 True 表示需让 AI 重跑 /workflow-start（当前进程 cfg 已过时）。

    顺序（见 upgrade.maybe_auto_upgrade）：先第①步升 npm 包（升 lbcli 连带刷 setup），
    再第②步模板对齐——同一次里连做完。

    生效机制（据实设计、不打扰用户）：
      - skill 与 available-skills 列表由 harness 文件监听在当前会话自动生效，**无需重开窗口**。
      - Python（cli.py 等）每次执行现读盘，重跑即新。
      - 仅 CLAUDE.md 不能原地热重载，但补丁升级几乎不动它，故不为此打扰用户。
    因此：只有「刷了模板→profile 可能变，当前进程的 cfg 已旧」时，才让 AI 重跑一次 /workflow-start
    （重跑用的是新 cli.py + 已自动生效的新 skill）；其余情况当前 cfg 仍有效，直接继续列选单。

    结果分派（result 各键可组合出现）：
      - upgraded → 已升全局包（含 lbcli setup 已刷，自动生效）。
      - refresh_template → workflow 模板落后 → lb init 刷模板 → 让 AI 重跑、返回 True。
      - blocked_task → 有在途任务挡住刷模板（项目仍用旧模板保护 state）→ 继续列选单、返回 False。
      - higher_major → workflow 有更高 major → 打印提示（不自动升）。
    """
    try:
        result = upgrade_lib.maybe_auto_upgrade(target, cfg)
    except Exception:
        return False  # 任何异常都不阻断 list-profiles
    if not result:
        return False

    upgraded = bool(result.get("upgraded"))
    if upgraded:
        print("✅ 已升级 lightboat 套件（全局 npm 包，新 skill 已自动生效）。")
    if result.get("higher_major"):
        print(f"ℹ️  另有更高大版本 {result['higher_major']}（跨 major 需人工确认，未自动升）。")

    if result.get("blocked_task"):
        # 有在途任务：不刷 workflow 模板（不半路掉包旧 state）。项目仍用旧模板、当前 cfg 有效，
        # 已升的包（如 lbcli）skill 已自动生效 → 直接继续列选单，不打扰、不重跑。
        print(f"ℹ️  本仓库有进行中任务 `{result['blocked_task']}`，跑完归档后自动对齐模板，"
              f"或手动 `lb update`。")
        return False

    if result.get("refresh_template"):
        # 升包后 / 模板落后 都在此刷模板——lb init 是独立子进程，跑的是刚装好的新全局 lb。
        print("→ 正在刷新项目模板（lb init --yes）...")
        try:
            r = subprocess.run(["lb", "init", "--yes"], timeout=300)
            ok = r.returncode == 0
        except (OSError, subprocess.TimeoutExpired):
            ok = False
        if ok:
            # 模板/profile 已刷新，当前进程加载的 cfg 已过时 → 让 AI 重跑一次（新 cli.py + 新 skill）。
            print("✅ 升级与模板刷新已完成，新 skill 已自动生效。请重新运行 /workflow-start 基于最新模板列出模式。")
            return True
        # 刷模板失败：模板没换成，当前 cfg 仍是原样有效 → 继续列选单，别卡住。
        print("⚠️  模板刷新失败，可手动执行 `lb init --yes`。", file=sys.stderr)
        return False

    # 仅升了包（如 lbcli）、workflow 模板未落后不刷：新 skill 已自动生效、当前 cfg 有效 → 继续列选单。
    return False


def cmd_list_profiles(args):
    """打印项目级所有 profile 的编号列表，供 onboarding 直接转贴给用户选。

    纯读+打印，不落状态。列出：序号 / 名字 / desc 首句 / 起步节点。
    这一步机械动作交给 CLI，onboarding 只管转贴 + 底部顺手推荐一句。
    """
    target = _resolve_target(args.target)
    # 起步分支闸门：放在自动升级之前——要拦就一件事都不做，尤其别在保护分支的工作区
    # 悄悄刷一遍模板文件（那些改动会挂在该分支上等着被误提交），也免得先冒一屏升级日志
    # 再接一条「换分支」的矛盾输出。用户切完分支重跑 /workflow-start，升级在那一刻照跑。
    _gate_start_branch(target, "切分支后重跑 /workflow-start")
    cfg = config_lib.load_project_config(target)

    # 新任务正门：顺手检测新版并代跑升级（全程软失败，绝不阻断选单输出）。
    if _maybe_auto_upgrade(target, cfg):
        return  # 刷了模板→当前 cfg 已过时，让 AI 重跑 /workflow-start 基于新模板重列，不在旧进程里继续

    # 采集通道闸门：本命令是 workflow-start 第一步、每个新任务必经，探测挂在这里就等于
    # 「起新任务前验一次通道」，且 skill 一个字都不用改（AI 零认知负担）。
    # 通则一个字不打；不通则 _die 拦下——通道不通时采集是 100% 静默丢，与其让用户跑完
    # 整个任务才发现数据没了，不如在这儿花十秒修好。
    # 必须在打印选单**之前**：先打选单再报错的话，AI 会照着已经到手的选单往下走，
    # 把 stderr 的报错当噪音无视掉，闸门就形同虚设。
    # 刻意不包 try/except：默认阻断意味着探测自身崩了也该拦；收敛可预期异常是 ping 内部的责任。
    if not args.skip_ping:
        msg = telemetry_lib.ping(target)
        if msg:
            _die(msg)

    profiles = cfg["profiles"]
    default_profile = cfg["default_profile"]
    names = [k for k in profiles if not k.startswith("_") and nodes.is_enabled(profiles[k])]

    print("可用编排模式（项目级 .workflow/profile/）：")
    for i, name in enumerate(names, 1):
        prof = profiles[name]
        desc = (prof.get("desc") or "").strip()
        first = desc.split("。")[0].split("\n")[0].strip() if desc else "（无描述）"
        start = prof.get("start_node") or "（未声明起步节点）"
        default_mark = "（默认）" if name == default_profile else ""
        print(f"  {i}. {name}{default_mark} — {first}　[起步:{start}]")
    print("回复编号选择，选完直接进该模式的起步节点。")


def cmd_set_env(args):
    """更新任务级 env（target/scope/域名/别名/eone）；写 state，禁止 skill 手改。

    env 是「往哪测」的坐标、不改节点序列，故全程可改（除已归档）。
    部分更新——只动给了的字段；可空字段传空串清空（--http-domain ''）。
    """
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    if st["main"]["current_step"] == nodes.DONE:
        _die("任务已归档（current_step=done），不改 env")
    env = dict(st["env"])
    changed = []

    if args.env is not None:
        has_dedicated = config_lib.load_project_config(target)["test_env"]["has_dedicated"]
        if not has_dedicated and args.env in ("test", "local"):
            _die(f"项目无独立测试环境（test_env.has_dedicated=false），不能改成 --env {args.env}；"
                 f"本地依赖测试环境同样起不来。需要测试环境请先改全局配置 has_dedicated=true 再来。")
        env["target"] = args.env
        changed.append("target")
    if args.scope is not None:
        env["scope"] = _parse_scope(args.scope)
        changed.append("scope")
    for flag, key in (("http_domain", "http_domain"), ("jsf_alias", "jsf_alias"), ("eone", "eone_feature_env")):
        v = getattr(args, flag)
        if v is not None:
            env[key] = v or None  # 空串 → 清空
            changed.append(key)

    if not changed:
        _die("至少指定一个 --env/--scope/--http-domain/--jsf-alias/--eone")
    # target 离开 test 时，eone（特性环境）失去语义，连带清空
    if env["target"] != "test" and env.get("eone_feature_env"):
        env["eone_feature_env"] = None
        if "eone_feature_env" not in changed:
            changed.append("eone_feature_env(连带清)")

    st["env"] = env
    state_lib.save_state(target, args.task, st)  # save 内含 _validate_state，非法 target/scope 在此 loud fail
    _report_state(target, args.task)
    print(f"✓ env 已更新：{', '.join(changed)}")
    print(f"  {_env_line(st['env'])}")


def cmd_archive_move(args):
    """把 delivery/<task>/ 整体迁入 delivery/archive/<task>/（workflow-archive 收尾调用）。"""
    target = _resolve_target(args.target)
    # 归一化吃掉 tab 补全带出的尾斜杠：Path 拼接会静默吞掉它，迁移照常成功，
    # 但清单摘除按字符串全等比对就匹配不上（静默失效 + 退出码 0），打印也会多出个 //。
    task = _norm_task(args.task)
    st = state_lib.load_state(target, task)
    cur = st["main"]["current_step"]
    if not args.force and cur != nodes.DONE:
        _die(f"任务未归档完成（current_step={cur}，应为 done），不迁移。先走完归档节点（{nodes.node_ref('workflow-archive', target)}）到 done 再来。")
    src = target / "delivery" / task
    dst = target / "delivery" / "archive" / task
    if dst.exists():
        _die(f"目标已存在：delivery/archive/{task}/，不覆盖。请先手动处理")
    dst.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(src), str(dst))
    # 兜底摘清单：常态下 advance 推到 done 时已摘过（此处幂等空转），
    # 这里保留是为 --force 直迁（没走到 done、没触发过那次摘除）那条路。
    _tasks_index_remove(target, task)
    print(f"✓ delivery/{task}/ → delivery/archive/{task}/")


def cmd_next(args):
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    cur = st["main"]["current_step"]
    fb = st["feedback"]

    if cur == nodes.DONE:
        hint = _archive_move_pending_hint(target, args.task)
        print("任务已归档" if not hint else "任务主线已走完（current_step=done）")
        for line in hint:
            print(line)
        return
    if fb["open"]:
        ids = ", ".join(it["id"] for it in fb["open"])
        print(f"⚠️ 当前有 {len(fb['open'])} 个 open feedback，主线节点已锁定。")
        print(f"请先跑：/workflow-feedback-loop {args.task}")
        print(f"开放 issue：{ids}")
        return
    pg = st["main"].get("pending_gate")
    # 有非 auto 待兑现闸门时不打印「下一步：/X」——它会诱导跳过确认直接调用；
    # 只打印闸门指令（instruction 内已含「确认后再调 /X」）。auto 闸门与无闸门常态照常引导。
    gated = bool(pg) and pg.get("gate") != nodes.GATE_AUTO
    if not gated:
        print(f"下一步：{_next_phrase(st['pipeline'], cur, args.task, target)}")
        rem = nodes.subagent_reminder(cur, st["pipeline"]) or nodes.external_window_reminder(cur, st["pipeline"])
        if rem:
            print(rem)
        hint = nodes.run_loop_hint(cur, st["pipeline"])
        if hint:
            print(hint.replace("<task>", args.task))
    if pg:
        print(f"待兑现闸门：{pg['from']} → {pg['to']}（{pg['gate']}）")
        print(f"  → {pg['instruction']}")
        # 同 advance：沙箱下归档就在眼前 → 补提醒一次。判据取 pending_gate.to（此刻 cur 还是上游节点），
        # 正好圈住「advance 完、尚未 enter 归档」这段窗口——换窗口/被 compact 后靠 next 找回进度时不至于漏掉。
        sandbox_hint = nodes.sandbox_commit_reminder(pg["to"], st["pipeline"])
        if sandbox_hint:
            print(sandbox_hint)


def cmd_node(args):
    """文档节点别名：把「/workflow-engine node <名> <task>」还原成「读 .workflow/node/<名>/NODE.md」。
    转场提示里打的是这条干净命令（藏掉路径细节），被真正调用时在此吐出该读哪份 NODE.md。
    纯别名——与原「读 …/NODE.md」等价，不加载 state、不过门禁（门禁走 enter），任何窗口冷启动都能直接调。"""
    target = _resolve_target(args.target)
    name = args.step
    rel = f".workflow/node/{name}/NODE.md"
    if not (target / rel).is_file():
        _die(f"节点 {name!r} 无 {rel}（不是文档节点，或节点名写错）。主线节点见 status；"
             f"skill 形态节点直接按 /{name} 调。")
    print(f"读 {rel}（任务 {args.task}）")


def cmd_enter(args):
    """节点入口门禁：5 步判定序，放行才打印上下文包并清 pending_gate。"""
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    step = args.step
    fb_open = st["feedback"]["open"]

    # feedback-loop 条件反转：必须有 open issue 才有事可做
    if step == "workflow-feedback-loop":
        if not fb_open:
            _die(f"无 open feedback，feedback-loop 无事可做；纯沉淀（已闭环的「记一下」）直调 /workflow-pit-record")
        print(f"✓ enter workflow-feedback-loop（{len(fb_open)} 个 open issue）")
        for it in fb_open:
            print(f"  {it['id']}  from={it['from']}  category={it.get('category') or '未分诊'}")
        print(f"  详情：delivery/{args.task}/feedback/INDEX.md")
        return

    if step not in nodes._seq(st["pipeline"]):
        _die(f"未知节点 {step}（主线节点见 state.pipeline；feedback 分诊用 workflow-feedback-loop）")

    cur = st["main"]["current_step"]
    if cur == nodes.DONE:
        _die("任务已归档（current_step=done）")

    untriaged = [it for it in fb_open if not it.get("category")]
    if untriaged:
        ids = ", ".join(it["id"] for it in untriaged)
        _die(f"主线锁定：有未分诊 open issue（{ids}）。先跑 /workflow-feedback-loop {args.task}")

    if fb_open:
        targets = {it["reroute_target"] for it in fb_open if it.get("reroute_target")}
        if step not in targets:
            _die(f"主线锁定：open issue 已分诊、回流目标是 {'/'.join(sorted(targets)) or '（无回退，仅记录待闭环）'}，"
                 f"本节点不是目标。等回流修复闭环（issue-resolve）后再回主线")

    if cur != step:
        _die(f"当前节点是 {cur}，不能进入 {step}。下一步跑：{nodes.node_ref(cur, target)} {args.task}")

    state_lib.clear_pending_gate(st)
    # 记进入时刻：暂存于 main.entered_at，advance 时并入 history record 供服务端算节点耗时。
    # 无论是否清了闸门都要落盘（entered_at 是新增写）。
    st["main"]["entered_at"] = _now()
    # 「已过门禁」凭据：advance 起手断言它等于自己，拦住跳过 enter 直接 advance（静默绕过入口闸）。
    # 用节点名而非布尔——顺带覆盖「在 X 却 advance Y」，且回流搬 current_step 时由 clear_entered 作废。
    st["main"]["entered_step"] = step
    # 进入侧上下文快照：与 entered_at 同暂存，advance 时随之并入 record 的 entered_* 字段。
    # 软失败为 None → 不塞，不阻断进入。
    entered_ctx = telemetry_lib.collect_context_used()
    if entered_ctx["context_used"] is not None:
        st["main"]["entered_context_used"] = entered_ctx["context_used"]
        st["main"]["entered_session_id"] = entered_ctx["session_id"]
        if entered_ctx["model"] is not None:
            st["main"]["entered_model"] = entered_ctx["model"]
        if entered_ctx["turns"] is not None:
            st["main"]["entered_turns"] = entered_ctx["turns"]
    telemetry_lib.apply_token_snapshot(st, entered_ctx.get("token_usage"))
    state_lib.save_state(target, args.task, st)
    _report_state(target, args.task)

    pipeline = st["pipeline"]
    rounds = 1 + sum(1 for h in st["main"]["history"] if h.get("step") == step)
    my_issues = [it for it in fb_open if it.get("reroute_target") == step]
    tag = "（回流修复）" if my_issues else (f"（round {rounds}）" if rounds > 1 else "")
    print(f"✓ enter {step}{tag}")
    # 开工指令（放行的当下必打）：读 NODE.md / 跑 skill 是节点第一动作，却只在 PROTOCOL 散文里，
    # 切窗口/compact 后易漏——enter 是唯一机器必经的节点入口，把它焊在这里，形态分流见 open_worklet_hint。
    print(f"  ▶ 开工：{nodes.open_worklet_hint(pipeline, step, target)}（按其执行；未读别开工）")
    print(f"  {_env_line(st['env'])}")
    enter_hook = nodes.node_hook(pipeline, step, "enter")
    if enter_hook:
        print(f"  ▸ 项目附加指令（本节点须执行）：{_hook_chain(enter_hook, target)}")
    for it in my_issues:
        print(f"  待处理 issue：{it['id']}（from={it['from']}）→ delivery/{args.task}/feedback/{it['id']}.md，"
              f"修完 issue-resolve 再 advance")
    rem = nodes.subagent_reminder(step, pipeline)
    if rem:
        print(rem)
    if nodes.is_external_window(pipeline, step):
        print("  （独立窗口节点：本 skill 应在独立新窗口运行；advance 后照提示回主窗口跑 next 续接）")
    nxt = nodes.next_step(step, pipeline)
    gate = nodes.transition_gate(step, nxt, pipeline)
    print(f"  出口闸门预告：{gate}（干完调 advance --step {step}，照其打印的指令转场；run-loop 自驱时加 --loop 降级非钉死闸）")
    exit_hook = nodes.node_hook(pipeline, step, "exit")
    if exit_hook:
        print(f"  ▸ 收尾指令预告（advance 前须执行）：{_hook_chain(exit_hook, target)}")


def cmd_gate_ack(args):
    """人工放弃/接管转场时清除 pending_gate（幂等）。"""
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    if state_lib.clear_pending_gate(st):
        state_lib.save_state(target, args.task, st)
        _report_state(target, args.task)
        print("✓ pending_gate 已清除（转场由人工接管）")
    else:
        print("无待兑现闸门")


def cmd_advance(args):
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    fb_open = st["feedback"]["open"]
    if fb_open:
        ids = ", ".join(it["id"] for it in fb_open)
        _die(f"有 open feedback（{ids}），主线锁定不得推进。跑 /workflow-feedback-loop {args.task} 处置：\n"
             f"  ▶ 默认「原地修」（不回流）：在当前节点直接解决（改代码/用例/文档）→ 就地复验 → issue-resolve 闭环 → 再 advance 继续前进。\n"
             f"  ⚠ 「回退」（reroute 到上游）是例外，仅当用户明确要求才用——它会把 current_step 搬回上游、逼整条下游重走，拖慢交付、体验差。不要默认选它，也不要替用户做这个决定。\n"
             f"  （已修复的直接 issue-resolve）")
    # 入口门禁凭据校验（硬阻断，无旁路）：enter→干活→advance 是唯一正确序列，跳过 enter
    # 就绕过了 current_step 校验 / feedback 锁 / entry_gate 上下文包，还会把上一转场的
    # pending_gate 留成脏状态。凭据由 cmd_enter 写、advance_main 消费，故只需断言存在且同名。
    # 排在 feedback 锁之后：主线锁优先级高于任何闸门（见 PROTOCOL 总则），先报锁更贴合处置顺序。
    entered = st["main"].get("entered_step")
    if entered != args.step:
        why = (f"当前暂存的进入记录是 {entered}（那个节点没走完就跳来了？）" if entered
               else "本节点没有进入记录（跳过了 enter，或换窗口/compact 后接着干、把 enter 落下了）")
        _die(f"未过入口门禁，不得推进 {args.step}——{why}。\n"
             f"  先跑：python3 .workflow/engine/cli.py enter {args.task} --step {args.step}\n"
             f"  （enter 才会校验 current_step / feedback 锁、打印上下文包并清上一转场的 pending_gate；"
             f"这道闸没有 -y 旁路。enter 幂等，活已干完就补跑它、放行后直接重跑本条 advance）")
    # 离开侧上下文快照：塞进本条 history record 的 ended_* 字段，与进入侧 entered_* 配对成节点进/出水位。
    # 软失败为 None → 不塞，不阻断推进。
    ended_ctx = telemetry_lib.collect_context_used()
    extra = None
    if ended_ctx["context_used"] is not None:
        extra = {"ended_context_used": ended_ctx["context_used"], "ended_session_id": ended_ctx["session_id"]}
        if ended_ctx["model"] is not None:
            extra["ended_model"] = ended_ctx["model"]
        if ended_ctx["turns"] is not None:
            extra["ended_turns"] = ended_ctx["turns"]
    state_lib.advance_main(st, args.step, _now(), outcome=args.outcome, extra=extra)
    telemetry_lib.apply_token_snapshot(st, ended_ctx.get("token_usage"))
    nxt = st["main"]["current_step"]

    pipeline = st["pipeline"]
    gate = None
    instruction = None
    step_external = nodes.is_external_window(pipeline, args.step)
    if args.outcome == "ok" and nxt != nodes.DONE and not step_external:
        # --loop：本次 advance 由 run-loop 自驱发出，按自治语义降级非钉死出口闸（钉死闸经 _stricter 保留）。
        gate = nodes.transition_gate(args.step, nxt, pipeline, args.loop)
        instruction = nodes.gate_instruction(gate, args.step, nxt, pipeline, target)
        # 独立窗口转场只能人推，不设机械兜底
        if not nodes.is_external_window(pipeline, nxt):
            state_lib.set_pending_gate(st, gate, args.step, nxt, instruction, _now())
    elif nxt == nodes.DONE:
        # 终点兜底清书签：done 之后没有下一个 enter 来兑现它，不清则最后一转场的 pending_gate
        # 永久留在归档态 state 里（status 一直报「待兑现」）。此处清掉 → 这类残留自愈，不必人跑 gate-ack。
        state_lib.clear_pending_gate(st)
    state_lib.save_state(target, args.task, st)
    if nxt == nodes.DONE:
        # 摘出活跃清单：绑 done 这一刻，不等 archive-move（那条命令靠 AI 自己记得跑，
        # 漏跑就永久残留在清单里；done 是引擎自己算出来的，绑它才可靠）。
        _tasks_index_remove(target, args.task)
    _report_state(target, args.task)

    print(f"✓ {args.step} 已推进（outcome={args.outcome}）")
    exit_hook = nodes.node_hook(pipeline, args.step, "exit")
    if exit_hook and args.outcome == "ok":
        print(f"  ▸ 收尾指令（转场前须执行）：{_hook_chain(exit_hook, target)}")
    if nxt == nodes.DONE:
        print(f"  当前节点：done（已是最后一个节点，可跑 /workflow-engine next {args.task} 确认）")
        for line in _archive_move_pending_hint(target, args.task):
            print(line)
        return
    if args.outcome == "ok" and step_external:
        print(f"  本节点已在独立窗口完成 —— 请回到原来的主任务窗口，"
              f"跑 /workflow-engine next {args.task} 续接主线（本窗口到此为止，不在本窗口调下一节点）。")
        return
    rem = nodes.subagent_reminder(nxt, pipeline) or nodes.external_window_reminder(nxt, pipeline)
    # auto（及 outcome 非 ok 的无闸门场景）才打印「下一步：/X」直接引导；
    # 需人工介入的闸门（confirm/confirm-locked）不打印那行——它形如可立即执行的命令，会诱导 AI
    # 跳过确认直接调用，只留闸门指令（instruction 内已含「确认后再调 /X」）。
    auto_flow = gate is None or gate == nodes.GATE_AUTO
    if auto_flow:
        print(f"  下一步：{_next_phrase(pipeline, nxt, args.task, target)}")
        if rem:
            print(rem)
    if gate:
        print(f"  闸门：{gate}")
        print(f"  → {instruction}")
        if not auto_flow and rem:
            print(rem)
        if st["main"].get("pending_gate"):
            print(f"  （已记 pending_gate：下一节点起手的 enter 自动清除；"
                  f"用户拒绝/放弃转场则跑 gate-ack {args.task} 清）")
    # 沙箱下即将进入归档 → 提醒先 commit+push 好在本地看代码。刻意放在闸门块之后、
    # 不受 auto_flow 分支影响：归档入口闸通常是 confirm-locked，提示要和闸门指令一起被看到。
    sandbox_hint = nodes.sandbox_commit_reminder(nxt, pipeline)
    if sandbox_hint:
        print(sandbox_hint)
    # 刚跨入无人区入口（且非 run-loop 自驱发出的本次 advance）→ 强制确认一次是否要进 run-loop，
    # 不是软提示：AI 须停下用 AskUserQuestion 问用户，不能自己悄悄决定就往下跑。
    if not args.loop and nodes.is_zone_entry(pipeline, nxt):
        hint = nodes.run_loop_zone_entry_confirm(nxt, pipeline)
        if hint:
            print(hint.replace("<task>", args.task))


def cmd_issue_add(args):
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    sources = nodes.feedback_sources(st["pipeline"])
    if args.from_node not in sources:
        _die(f"--from 必须是 {sorted(sources)} 之一（profile 内 can_report 节点 + manual），收到：{args.from_node}")
    issue_id = state_lib.allocate_issue_id(st)
    now = _now()  # state 条目 created_at 与 issue.md frontmatter created_at 共用同一时刻
    state_lib.add_open_issue(st, issue_id, args.from_node, created_at=now, severity=args.severity)
    state_lib.clear_pending_gate(st)  # issue 锁主线，优先级高于任何待兑现转场
    issue_lib.write_new_issue(
        target, args.task, issue_id,
        from_node=args.from_node, today=now,
        phenomenon=args.phenomenon, context=args.context,
    )
    # 无条件写：severity 恒有值（缺省 normal），frontmatter 不留 null 占位
    issue_lib.update_issue_frontmatter(target, args.task, issue_id, {"severity": args.severity})
    state_lib.save_state(target, args.task, st)
    issue_lib.write_index(target, args.task, st)
    _report_state(target, args.task)
    print(issue_id)
    if args.severity == "high":
        print(f"⚠️ {issue_id} 粗判 high 危（数据丢失/安全/资金/不可逆）——当场停下、当场处置，不拖到最后。")


def cmd_issue_triage(args):
    if not ISSUE_ID_RE.match(args.issue_id):
        _die(f"issue-id 格式错：{args.issue_id}")
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    pipeline = st["pipeline"]

    reroute_target = args.target_node
    if reroute_target is not None:
        # 回退目标校验：真实存在 + rerouteable!=false + 不在当前下游（防前跳）。
        if reroute_target not in nodes._seq(pipeline):
            _die(f"--target {reroute_target} 不是本任务节点（见 state.pipeline）")
        if not nodes.is_rerouteable(pipeline, reroute_target):
            _die(f"--target {reroute_target} 声明 rerouteable:false，不能作回退目标")
        cur = st["main"]["current_step"]
        # done 终态同样不许指定回退目标——与 cmd_reroute 的 done 守卫成对。
        # 不挡这里的话会分诊成功（打印 target=X）、紧接着 reroute 被拒，自相矛盾。
        if cur == nodes.DONE:
            _die(f"任务已归档（current_step=done），不能指定回退目标——归档是终态收口。\n"
                 f"       只记录沉淀（不回退）：去掉 --target 重跑本条。\n"
                 f"       确需返工：另起一个任务处理。")
        if nodes.is_downstream(pipeline, cur, reroute_target):
            _die(f"--target {reroute_target} 在当前节点 {cur} 下游，回退不得前跳（会跳过未完成节点）")

    state_lib.set_issue_triage(st, args.issue_id, args.category, args.severity, reroute_target)
    issue_lib.update_issue_frontmatter(target, args.task, args.issue_id, {
        "category": args.category,
        "reroute_target": reroute_target,
        "severity": args.severity,
    })
    # 「仅记录」（无 target）→ 同操作内直接闭环，防主线死锁（设计二审 D）。
    # 刻意不传 resolved_mode，留 null：这条支路压根没走解决路径（问题本就不用修 / 早修完了），
    # 记 auto 会把它算进自治率分子、把数字做虚。语义见 nodes.RESOLVE_MODES。
    if reroute_target is None:
        state_lib.resolve_issue(st, args.issue_id, _now())
        issue_lib.update_issue_frontmatter(target, args.task, args.issue_id, {
            "status": "resolved",
            "resolved_by": "feedback-loop (仅记录，沉淀闭环)",
        })
    state_lib.save_state(target, args.task, st)
    issue_lib.write_index(target, args.task, st)
    _report_state(target, args.task)
    if reroute_target is None:
        print(f"✓ {args.issue_id} 分诊：category={args.category} severity={args.severity}（仅记录 → 已闭环 resolved）")
    else:
        print(f"✓ {args.issue_id} 分诊：category={args.category} severity={args.severity} target={reroute_target}")


def cmd_issue_resolve(args):
    if not ISSUE_ID_RE.match(args.issue_id):
        _die(f"issue-id 格式错：{args.issue_id}")
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    # resolved_mode 两处都写：frontmatter 给人看，state 条目给服务端看。
    # 只写 frontmatter 不行——上报走 _report_state 发全量 state 原文，issue.md 从不出用户仓库
    # （`resolved_by` 至今就只活在 frontmatter 里，服务端根本看不到「谁修的」）。
    fm = {"status": "resolved", "resolved_by": args.by, "resolved_mode": args.mode}
    if args.category is not None:
        # 顺手记归属：先解决、后沉淀落到一个动作（category 进漏点地图）。
        state_lib.set_issue_category(st, args.issue_id, args.category)
        fm["category"] = args.category
    state_lib.resolve_issue(st, args.issue_id, _now(), args.mode)
    issue_lib.update_issue_frontmatter(target, args.task, args.issue_id, fm)
    state_lib.save_state(target, args.task, st)
    issue_lib.write_index(target, args.task, st)
    _report_state(target, args.task)
    cat = f" category={args.category}" if args.category else ""
    print(f"✓ {args.issue_id} -> resolved (by {args.by}, {args.mode}){cat}")


def cmd_reroute(args):
    if not ISSUE_ID_RE.match(args.issue_id):
        _die(f"issue-id 格式错：{args.issue_id}")
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    pipeline = st["pipeline"]
    to = args.to
    # 回退目标校验：真实存在 + rerouteable!=false + 不在当前下游（防前跳，设计⑦）。
    if to not in nodes._seq(pipeline):
        _die(f"--to {to} 不是本任务节点（见 state.pipeline）")
    if not nodes.is_rerouteable(pipeline, to):
        _die(f"--to {to} 声明 rerouteable:false，不能作回退目标")
    cur = st["main"]["current_step"]
    # done 是终态：不许回流复活。与 enter / set-env 的 done 守卫同语义——
    # 那两条早就挡了（「任务已归档」），唯独本条漏挡：防前跳的 is_downstream 对 done
    # 完全不设防（done 不在 pipeline 序列里，判定恒为 False），于是 done 态能一路放行、
    # 把 current_step 改回上游。而改回去也走不通——照它打印的下一步走会立刻撞上 enter
    # 的 done 守卫，两处守卫互相矛盾，说明这是漏挡而非设计。
    # 归档后又发现问题应另起任务（任务名须全仓库唯一，见 _gate_task_name）。
    if cur == nodes.DONE:
        _die(f"任务已归档（current_step=done），不回流——归档是终态收口。\n"
             f"       归档后又发现问题：另起一个任务处理（旧任务的 issue 记录留在 "
             f"delivery/{_norm_task(args.task)}/feedback/ 里，可在新任务中引用）。")
    if nodes.is_downstream(pipeline, cur, to):
        _die(f"--to {to} 在当前节点 {cur} 下游，回退不得前跳（会跳过未完成节点）")
    count = state_lib.reroute_to(st, to, _now(), args.issue_id)
    state_lib.save_state(target, args.task, st)
    _report_state(target, args.task)
    print(f"✓ 已回流到 {to}")
    if count >= nodes.REROUTE_ESCALATE_THRESHOLD:
        print(f"⚠️ {args.issue_id} 已第 {count} 次回流仍未闭环（阈值 {nodes.REROUTE_ESCALATE_THRESHOLD}）。"
              f"停止自治自修、升级人工：把现象 + 历次修复尝试列给用户，由人介入定夺，不要再自动推进到 {to}。")
        return
    print(f"下一步：{_next_phrase(pipeline, to, args.task, target)}")
    rem = nodes.subagent_reminder(to, pipeline) or nodes.external_window_reminder(to, pipeline)
    if rem:
        print(rem)


def _loop_report(st: dict) -> list[str]:
    """从 state.main.history + feedback 确定性生成无人区运行报告（不靠 AI 记账）。"""
    hist = st["main"]["history"]
    seq = nodes._seq(st["pipeline"])
    ran = [h["step"] for h in hist if h.get("step") in seq and h.get("outcome") == "ok"]
    reroutes = [h for h in hist if h.get("outcome") == "回流"]
    resolved = st["feedback"]["resolved"]
    open_n = len(st["feedback"]["open"])
    lines = ["  ── 无人区运行报告 ──"]
    lines.append(f"  · 跑过节点：{len(ran)} 个" + (f"（{' → '.join(ran)}）" if ran else ""))
    lines.append(f"  · 自修 issue：已闭环 {len(resolved)} 个 / 回流 {len(reroutes)} 次 / 当前 open {open_n} 个")
    lines.append(f"  · 当前节点：{st['main']['current_step']}")
    return lines


def _issue_brief(it: dict) -> str:
    return (f"{it['id']}（from={it['from']} category={it.get('category') or '未分诊'} "
            f"severity={it.get('severity') or '?'} reroute={it.get('reroute_count', 0)}）")


def cmd_run_loop(args):
    """无人区外层驱动器：每个节点边界 AI 调一次，CONTINUE 则驱动当前节点再回来，余皆终止。

    run-loop ⇒ 自治：非钉死节点机械流转、不问人；只在两道钉死人工闸（重思考 propose/test-plan、
    上线 deploy）与高危/升级/待分诊 issue 处停人。
    """
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    status, detail = nodes.classify_loop(st)

    if status == nodes.LOOP_CONTINUE:
        step = detail["step"]
        nxt = detail["next"]
        print(f"▶ 驱动 {step}(无人区自驱中)")
        print(f"  起手 /workflow-engine enter {args.task} --step {step}，干完 /workflow-engine advance "
              f"{args.task} --step {step} --loop，然后再调 /workflow-engine run-loop {args.task} 续循环。")
        print(f"  （下一节点 {nxt}；--loop 让本次转场按自治降级非钉死闸、不问人）")
        rem = nodes.subagent_reminder(step, st["pipeline"]) or nodes.external_window_reminder(step, st["pipeline"])
        if rem:
            print(rem)
        print("  · 上下文经济：本节点边界若上下文已长，先 compact 或把重活派给 subagent 只取蒸馏结论，再驱动。")
        return

    if status == nodes.LOOP_TRIAGE:
        ids = ", ".join(it["id"] for it in detail["issues"])
        print(f"▶ 分诊 open issue（{ids}）")
        print(f"  跑 /workflow-feedback-loop {args.task}：normal 自裁→默认原地修（在当前节点直接解决+就地复验），"
              f"high 锁主线停人。解决闭环（issue-resolve）后再调 /workflow-engine run-loop {args.task} 续循环。")
        for it in detail["issues"]:
            print(f"    {_issue_brief(it)}")
        return

    # 以下皆为终止：打报告 + 该原因的人工动作
    print("■ run-loop 终止")
    if status == nodes.LOOP_DONE:
        print("  原因：任务已走到 done（全节点完成）。")
        for line in _archive_move_pending_hint(target, args.task):
            print(line)
    elif status == nodes.LOOP_ESCALATED:
        print(f"  原因：issue 反复回流达上限（reroute ≥ {nodes.REROUTE_ESCALATE_THRESHOLD}）仍未闭环，停止自治、升级人工。")
        for it in detail["issues"]:
            print(f"    {_issue_brief(it)}")
        print("  → 把现象 + 历次修复尝试列给用户，由人介入定夺，不要再自动重试。")
    elif status == nodes.LOOP_HIGH_ISSUE:
        print("  原因：存在 high 严重度 issue（数据丢失/安全/资金/不可逆），不自裁，锁主线停人。")
        for it in detail["issues"]:
            print(f"    {_issue_brief(it)}")
        print(f"  → 跑 /workflow-feedback-loop {args.task} 由人处置（high 危需人确认怎么修）。")
    elif status == nodes.LOOP_PINNED:
        step = detail["step"]
        print(f"  原因：当前节点 {step} 是停人节点（非 autopilot，产物/推进只有人能认证）。")
        print(f"  → 交人工认证 {step} 产物（若为上线节点，先走 {nodes.node_ref(step, target)} 的上线前置清单逐条确认）；"
              f"认证通过后由人续走 /workflow-engine next {args.task}（继续走 run-loop 也从下个节点接着跑，无需在此选择）。")
    for line in _loop_report(st):
        print(line)


def cmd_index_render(args):
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    issue_lib.write_index(target, args.task, st)
    print(f"✓ INDEX.md 已重渲")


def cmd_state_dump(args):
    target = _resolve_target(args.target)
    st = state_lib.load_state(target, args.task)
    print(json.dumps(st, ensure_ascii=False, indent=2))


def cmd_telemetry_ping(args):
    """手动探测采集通道（排查入口）。无需 task——验的是通道，与任务无关。

    与内嵌在 list-profiles 里那次探测的唯一区别：这里**成功也打一行**。
    诊断命令不回话等于没跑，用户会怀疑自己到底跑没跑。
    """
    target = _resolve_target(args.target)
    t0 = time.monotonic()
    msg = telemetry_lib.ping(target)
    elapsed = int((time.monotonic() - t0) * 1000)
    if msg:
        print(msg, file=sys.stderr)
        sys.exit(1)
    print(f"✓ 采集通道正常（{elapsed}ms）")


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(prog="workflow-engine", description="workflow-engine CLI（状态机壳）")
    p.add_argument("--target", help="项目根（默认当前目录）")
    sub = p.add_subparsers(dest="cmd", required=True)

    sp = sub.add_parser("init", help="创建 delivery/<task>/ 骨架 + state.json")
    sp.add_argument("task")
    sp.add_argument("--profile", default=None,
                    help="编排模式名（.workflow/profile/ 的 profile 之一）；不给则用 default_profile / 向导选")
    sp.add_argument("--env", choices=["local", "test", "gamma"], default=None,
                    help="运行环境；项目 has_dedicated=false 时 test/local 被拒")
    sp.add_argument("--scope", default=None, help="测试范围，逗号分隔（http,jsf）")
    sp.add_argument("--http-domain", dest="http_domain", default=None, help="HTTP 域名（范围含 http 时）")
    sp.add_argument("--jsf-alias", dest="jsf_alias", default=None, help="JSF 别名 alias（范围含 jsf 时）")
    sp.add_argument("--eone", default=None, help="eone 特性环境（仅 env=test 生效，非必填）")
    sp.add_argument("--skip-app-info", dest="skip_app_info", action="store_true",
                    help="逃生开关：app_info.policy=required 但应用信息临时拿不到（登录不了/网不通）时跳过采集门禁；"
                         "留痕进 state 并告警下游退化。项目本无 JDOS 应用请改 app_info.policy=off 而非用此开关")
    sp.set_defaults(func=cmd_init)

    sp = sub.add_parser("status", help="打印任务状态 3 行")
    sp.add_argument("task")
    sp.set_defaults(func=cmd_status)

    sp = sub.add_parser("roles", help="打印本任务产物区域清单（role → 解析后路径）")
    sp.add_argument("task")
    sp.set_defaults(func=cmd_roles)

    sp = sub.add_parser("profile", help="打印本任务冻结的 profile 名（供节点判血统，如名字含 openspec）")
    sp.add_argument("task")
    sp.set_defaults(func=cmd_profile)

    sp = sub.add_parser("list-profiles",
                        help="打印项目级 .workflow/profile/ 下所有启用编排模式的编号列表；供 onboarding 转贴给用户选，无需 task")
    sp.add_argument("--skip-ping", action="store_true",
                    help="跳过采集通道探测（通道不通时的逃生口，仅在用户明确要求时使用）")
    sp.set_defaults(func=cmd_list_profiles)

    sp = sub.add_parser("project-type", help="打印项目类型（backend|frontend）；项目级固定属性，读 .workflow/config.json，无需 task")
    sp.set_defaults(func=cmd_project_type)

    sp = sub.add_parser("remote-config-files",
                        help="打印 JDOS 实例配置三态（unknown|true|false）；--set 写入。无需 task")
    sp.add_argument("--set", dest="set_value", choices=["unknown", "true", "false"], default=None,
                    help="把开关写成 unknown / true / false")
    sp.set_defaults(func=cmd_remote_config_files)

    sp = sub.add_parser("set-env", help="更新任务级 env（target/scope/域名/别名/eone）；全程可改（除已归档），勿手改 state.json")
    sp.add_argument("task")
    sp.add_argument("--env", choices=["local", "test", "gamma"], default=None,
                    help="运行环境；项目 has_dedicated=false 时 test/local 被拒")
    sp.add_argument("--scope", default=None, help="测试范围，逗号分隔（http,jsf）")
    sp.add_argument("--http-domain", dest="http_domain", default=None, help="HTTP 域名（传空串清空）")
    sp.add_argument("--jsf-alias", dest="jsf_alias", default=None, help="JSF 别名 alias（传空串清空）")
    sp.add_argument("--eone", default=None, help="eone 特性环境（仅 target=test 生效，传空串清空）")
    sp.set_defaults(func=cmd_set_env)

    sp = sub.add_parser("archive-move", help="把 delivery/<task>/ 迁入 delivery/archive/<task>/（需 done，--force 绕过）")
    sp.add_argument("task")
    sp.add_argument("--force", action="store_true", help="跳过 current_step==done 校验强制迁移")
    sp.set_defaults(func=cmd_archive_move)

    sp = sub.add_parser("next", help="按规则打印下一步建议")
    sp.add_argument("task")
    sp.set_defaults(func=cmd_next)

    sp = sub.add_parser("node", help="文档节点别名：还原「读 .workflow/node/<名>/NODE.md」（转场提示打此干净命令，藏路径细节）")
    sp.add_argument("step", help="文档节点名（.workflow/node/<名>/NODE.md）")
    sp.add_argument("task")
    sp.set_defaults(func=cmd_node)

    sp = sub.add_parser("run-loop",
                        help="无人区外层驱动器：分类当前终态，CONTINUE 则给出驱动当前节点的指令、余皆终止打报告")
    sp.add_argument("task")
    sp.set_defaults(func=cmd_run_loop)

    sp = sub.add_parser("enter", help="节点入口门禁：校验 current_step / feedback 锁，放行打印上下文包并清 pending_gate")
    sp.add_argument("task")
    sp.add_argument("--step", required=True,
                    help="本节点名（本任务 profile 节点之一，或 workflow-feedback-loop）")
    sp.set_defaults(func=cmd_enter)

    sp = sub.add_parser("gate-ack", help="人工放弃/接管转场：清除 pending_gate（幂等）")
    sp.add_argument("task")
    sp.set_defaults(func=cmd_gate_ack)

    sp = sub.add_parser("advance", help="节点出口推进 main.current_step")
    sp.add_argument("task")
    sp.add_argument("--step", required=True, help="当前节点名，必须等于 state.main.current_step")
    sp.add_argument("--outcome", default="ok")
    sp.add_argument("--loop", action="store_true",
                    help="本次推进由 run-loop 自驱发出：按自治语义降级非钉死出口闸（钉死闸不动）。不固化任何配置")
    sp.set_defaults(func=cmd_advance)

    sp = sub.add_parser("issue-add", help="登记一条 open issue + 渲染 issue-<n>.md")
    sp.add_argument("task")
    # default 必须是真值 normal、不能是 None：调用方只在 high 时才带这个 flag（各 NODE.md 的口径），
    # 留 None 就等于「normal 从没被人写进 state」——字段实际取值退化成 high/null，服务端拿不到严重度分布。
    # 与 issue-resolve 的 --mode required 同一个教训，只是 severity 有安全缺省、不必逼调用方每次表态。
    sp.add_argument("--severity", choices=list(nodes.SEVERITIES), default="normal",
                    help="粗判严重度：high（数据丢失/安全/资金/不可逆）当场停、当场处置；缺省 normal（记一笔继续）")
    sp.add_argument("--from", dest="from_node", required=True,
                    help="报告来源：profile 内 can_report=true 的节点，或 manual（人主动报）")
    sp.add_argument("--phenomenon", required=True)
    sp.add_argument("--context", default="")
    sp.set_defaults(func=cmd_issue_add)

    sp = sub.add_parser("issue-triage",
                        help="分诊一次原子写入：category（沉淀归属）+ severity + 可选 target（回退目标；不给=仅记录，同操作闭环）")
    sp.add_argument("task")
    sp.add_argument("issue_id")
    sp.add_argument("--category", required=True, choices=list(nodes.CATEGORIES), help=CATEGORY_HELP)
    sp.add_argument("--severity", choices=list(nodes.SEVERITIES), default="normal",
                    help="二元严重度；命中 数据丢失/安全/资金/不可逆 取 high，默认 normal")
    sp.add_argument("--target", dest="target_node", default=None,
                    help="回退目标节点名（可选）；不给=仅记录沉淀、同操作内直接闭环，不 reroute")
    sp.set_defaults(func=cmd_issue_triage)

    sp = sub.add_parser("issue-resolve",
                        help="把 issue 从 open 移到 resolved（必表 --mode 解决方式；可选 --category 顺手记归属）")
    sp.add_argument("task")
    sp.add_argument("issue_id")
    sp.add_argument("--by", required=True, help="resolved_by 文本，如 'workflow-implement (round 2)'")
    # required：设成可选就会重蹈 severity 的覆辙——默认路径没人主动填，字段常年为 null。
    sp.add_argument("--mode", required=True, choices=list(nodes.RESOLVE_MODES),
                    help="解决方式：auto=AI 自己发现自己解决、全程没打断人；"
                         "asked=为这条 issue 向人提过问/等过人回话才搞定（含人自己动手改的）")
    sp.add_argument("--category", choices=list(nodes.CATEGORIES), default=None,
                    help="沉淀归属（design/code/test）：给了则写进漏点地图；不给=只解决、事后可补")
    sp.set_defaults(func=cmd_issue_resolve)

    sp = sub.add_parser("reroute", help="workflow-feedback-loop 分诊完成后按 --to 目标节点回流")
    sp.add_argument("task")
    sp.add_argument("issue_id")
    sp.add_argument("--to", required=True,
                    help="回退目标节点名（须为本任务真实节点、rerouteable!=false、不在当前下游）")
    sp.set_defaults(func=cmd_reroute)

    sp = sub.add_parser("index-render", help="从 state + issue/*.md 重渲 INDEX.md")
    sp.add_argument("task")
    sp.set_defaults(func=cmd_index_render)

    sp = sub.add_parser("state-dump", help="打印 state.json（调试用）")
    sp.add_argument("task")
    sp.set_defaults(func=cmd_state_dump)

    sp = sub.add_parser("telemetry-ping", help="探测采集通道是否通（排查用，无需 task）")
    sp.set_defaults(func=cmd_telemetry_ping)

    return p


def main(argv=None):
    parser = build_parser()
    args = parser.parse_args(argv)
    try:
        args.func(args)
    except state_lib.StateError as e:
        _die(str(e))


if __name__ == "__main__":
    main()
