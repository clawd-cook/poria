"""state.json 读写 + schema 校验 + 原子推进。

state.json 是单一事实源；本模块所有写操作走 save_state 一次性落盘，避免半建状态。
"""

from __future__ import annotations

import json
import re
from pathlib import Path

from . import nodes

TASK_NAME_RE = re.compile(r"^[a-z][a-z0-9-]*$")

ENV_TARGETS = ("local", "test", "gamma")
ENV_SCOPES = ("http", "jsf")

# 进入侧暂存键：cmd_enter 写进 state.main，advance 时消费（entered_step 作门禁凭据、其余并入 history）。
# 集中在此，免得新增一个快照字段时漏掉某处清理。
ENTERED_KEYS = ("entered_step", "entered_at", "entered_context_used",
                "entered_session_id", "entered_model", "entered_turns")


class StateError(Exception):
    pass


def state_path(target_root: Path, task: str) -> Path:
    return target_root / "delivery" / task / "state.json"


def default_env() -> dict:
    """任务级环境块默认值。target=test/scope=[http]，三个坐标留空。"""
    return {
        "target": "test",
        "scope": ["http"],
        "http_domain": None,
        "jsf_alias": None,
        "eone_feature_env": None,
    }


def default_state(task: str, today: str, frozen_pipeline: list, env: dict | None = None,
                  roles: dict | None = None, profile: str | None = None,
                  npm_lb_version: str | None = None, npm_lbcli_version: str | None = None,
                  lb_version: str | None = None,
                  telemetry: dict | None = None, identity: dict | None = None,
                  init_ts: str | None = None, start_node: str | None = None) -> dict:
    """建初始 state。frozen_pipeline 已由 cli 用 resolve_profile 冻结；
    roles 已由 cli 用 resolve_roles 替换 <task>（可空）；profile 是选中的编排模式名（可空）；
    三个版本字段都是 init 时的一次性快照（之后不再刷新；探测/读取失败为 None，不阻断 init），语义分两类：
      · npm_lb_version/npm_lbcli_version —— 机器态：init 时探测 `lb -V` / `lbcli -V`，即当前机器上 npm 全局装的工具版本；
      · lb_version —— 仓库态：init 时快照 .workflow/config.json 的 lb_version，即本仓库上次 `lb init` 落地的模板版本。
    两者并列，服务端可对比出「机器已升级但仓库模板未跟进」的 gap。
    telemetry（{is_report, report_url}）/identity（git 身份块）/init_ts（秒级时间戳）是 init 时冻结的上报配置与身份快照，
    后续流转只读、不刷新；探测失败为 None/空，不阻断 init。
    start_node 是选中 profile 声明的起步/触发节点名（状态机外正门，init 之前跑）——一次性快照，只记名字不记执行时刻，可空。"""
    seed = nodes.active_sequence(frozen_pipeline)[0]  # seed = 首个节点
    return {
        "task": task,
        "created_at": today,
        "init_ts": init_ts,
        "profile": profile,
        "start_node": start_node,
        "npm_lb_version": npm_lb_version,
        "npm_lbcli_version": npm_lbcli_version,
        "lb_version": lb_version,
        "identity": identity if identity is not None else {},
        "telemetry": telemetry if telemetry is not None else {},
        "env": env if env is not None else default_env(),
        "pipeline": frozen_pipeline,
        "roles": roles if roles is not None else {},
        "main": {"current_step": seed, "history": [], "pending_gate": None},
        "feedback": {"next_id": 1, "open": [], "resolved": []},
    }


def validate_task_name(task: str):
    if not TASK_NAME_RE.match(task):
        raise StateError("task 名必须 kebab-case（小写字母+数字+连字符，字母开头）")


def _normalize(state: dict) -> dict:
    """migration-on-read：回填可选字段（env / pending_gate / issue 分诊字段）。

    不做旧结构（options / verdict）迁移——升级前进行中任务须走完归档（见设计「兼容策略」）。
    """
    env = state.setdefault("env", default_env())
    base_env = default_env()
    for k, v in base_env.items():
        env.setdefault(k, v)
    state.setdefault("roles", {})  # 老任务无 roles 块 → 回填空 dict（role 查询降级为无表）
    state.setdefault("profile", None)  # 老任务无 profile 字段 → None（profile 查询降级为未知）
    state.setdefault("start_node", None)  # 老任务无起步节点快照 → None
    # 版本快照字段迁移：旧任务的 lb_version/lbcli_version 是「机器态」（探测 lb -V / lbcli -V），
    # 改名加 npm_ 前缀腾出 lb_version 给「仓库态」（config.json 落地模板版本）。老任务无仓库态 → None。
    if "npm_lb_version" not in state:
        state["npm_lb_version"] = state.pop("lb_version", None)
    if "npm_lbcli_version" not in state:
        state["npm_lbcli_version"] = state.pop("lbcli_version", None)
    state.setdefault("lb_version", None)  # 老任务无仓库态模板版本快照 → None
    state.setdefault("init_ts", None)  # 老任务无上报主键时间戳 → None
    state.setdefault("identity", {})  # 老任务无身份块 → 空 dict
    state.setdefault("telemetry", {})  # 老任务无上报配置块 → 空 dict（is_report 缺省视为关）
    state["main"].setdefault("pending_gate", None)
    for obj in state.get("pipeline") or []:
        if isinstance(obj.get("skill"), str):
            obj["skill"] = [obj["skill"]] if obj["skill"] else [obj["node"]]
        obj.setdefault("execution", nodes.EXEC_INLINE)
        obj.setdefault("feedback", nodes._freeze_feedback(obj.get("feedback")))
        hooks = obj.get("hooks") or {}
        for phase in ("enter", "exit"):
            if isinstance(hooks.get(phase), str):
                hooks[phase] = [hooks[phase]] if hooks[phase] else []
    for it in state["feedback"].get("open", []):
        it.setdefault("reroute_count", 0)
        it.setdefault("severity", None)
        it.setdefault("category", None)  # 沉淀归属（design/code/test），分诊后落
        it.setdefault("reroute_target", None)  # 回退目标节点名，仅回退时有
        it.setdefault("created_at", None)  # 老任务无写入时刻 → None
        it.setdefault("resolved_at", None)  # open 恒未解决 → None
        it.setdefault("resolved_mode", None)  # open 恒未闭环 → None
    for it in state["feedback"].get("resolved", []):
        it.setdefault("reroute_count", 0)
        it.setdefault("severity", None)
        it.setdefault("category", None)
        it.setdefault("reroute_target", None)
        it.setdefault("created_at", None)  # 老任务无写入时刻 → None
        it.setdefault("resolved_at", None)  # 老任务无解决时刻 → None
        # 老任务 + issue-triage「仅记录」闭环都留 None，两者都不算进自治率分母（见 nodes.RESOLVE_MODES）
        it.setdefault("resolved_mode", None)
    state.pop("test_plan", None)
    return state


def load_state(target_root: Path, task: str) -> dict:
    p = state_path(target_root, task)
    if not p.exists():
        raise StateError(f"state.json 不存在：{p}")
    with p.open() as f:
        state = json.load(f)
    _normalize(state)
    _validate_state(state)
    return state


def save_state(target_root: Path, task: str, state: dict):
    _validate_state(state)
    p = state_path(target_root, task)
    p.parent.mkdir(parents=True, exist_ok=True)
    with p.open("w") as f:
        json.dump(state, f, ensure_ascii=False, indent=2)
        f.write("\n")


def _validate_state(state: dict):
    for key in ("task", "created_at", "env", "pipeline", "main", "feedback"):
        if key not in state:
            raise StateError(f"state.json 缺字段: {key}")
    if not isinstance(state.get("roles", {}), dict):
        raise StateError(f"非法 roles（应为对象 role名→路径）: {state.get('roles')!r}")
    pipeline = state["pipeline"]
    if not isinstance(pipeline, list) or not pipeline:
        raise StateError("非法 pipeline（应为非空对象列表）")
    for obj in pipeline:
        if not isinstance(obj, dict):
            raise StateError(f"非法 pipeline 项（应为对象）: {obj!r}")
        for f in ("node", "skill", "exit_gate", "entry_gate", "autopilot"):
            if f not in obj:
                raise StateError(f"pipeline 项缺字段 {f}: {obj!r}")
        if not isinstance(obj["skill"], list) or not obj["skill"]:
            raise StateError(f"pipeline 项 skill 应为非空列表: {obj!r}")
    if not nodes.is_valid_step(state["main"]["current_step"], pipeline):
        raise StateError(f"非法 current_step: {state['main']['current_step']}")
    env = state["env"]
    if env.get("target") not in ENV_TARGETS:
        raise StateError(f"非法 env.target（应 ∈ {'/'.join(ENV_TARGETS)}）: {env.get('target')}")
    scope = env.get("scope")
    if not isinstance(scope, list) or not scope or any(s not in ENV_SCOPES for s in scope):
        raise StateError(f"非法 env.scope（应为 {'/'.join(ENV_SCOPES)} 的非空子集）: {scope}")


def clear_entered(state: dict):
    """丢弃进入侧暂存（entered_step / entered_at / 上下文快照）。

    供 reroute 用：回流搬走 current_step，被弃节点的暂存必须一并清掉——否则它会冒充
    下一个节点的进入水位（污染遥测），还会让 advance 的入口门禁断言误判为「已过门禁」。
    """
    for key in ENTERED_KEYS:
        state["main"].pop(key, None)


def advance_main(state: dict, step: str, today: str, outcome: str = "ok", extra: dict | None = None):
    """节点出口推进。step 必须等于 current_step；outcome=ok 推进到下一节点。"""
    cur = state["main"]["current_step"]
    if cur != step:
        raise StateError(f"当前节点是 {cur}，不能从 {step} 推进")
    record = {"step": step, "ended_at": today, "outcome": outcome}
    # entered_step 是 cmd_enter 写下的「已过门禁」凭据，出口在此消费（不进 history，只是凭据）。
    # 调用方（cmd_advance）负责在此之前断言它等于本节点——本函数只管取出清除。
    state["main"].pop("entered_step", None)
    # entered_at 是 cmd_enter 时暂存在 main 的进入时刻，并入本条 history 供服务端算节点耗时；
    # 取出后清除，避免残留污染下一节点。
    entered_at = state["main"].pop("entered_at", None)
    if entered_at is not None:
        record["entered_at"] = entered_at
    # 进入侧上下文快照（enter 时与 entered_at 同暂存）随之并入并清除，与 ended_* 配对成节点进/出水位。
    for key in ("entered_context_used", "entered_session_id", "entered_model", "entered_turns"):
        val = state["main"].pop(key, None)
        if val is not None:
            record[key] = val
    if extra:
        record.update(extra)
    state["main"]["history"].append(record)
    if outcome == "ok":
        nxt = nodes.next_step(cur, state["pipeline"])
        state["main"]["current_step"] = nxt if nxt else nodes.DONE


def set_pending_gate(state: dict, gate: str, frm: str, to: str, instruction: str, today: str):
    """advance 转场时记下待兑现指令；兑现 = 下一节点 enter 清除，人工放弃 = gate-ack 清除。"""
    state["main"]["pending_gate"] = {
        "gate": gate, "from": frm, "to": to,
        "instruction": instruction, "set_at": today,
    }


def clear_pending_gate(state: dict) -> bool:
    """清除待兑现闸门；返回是否真有东西被清（幂等）。"""
    had = bool(state["main"].get("pending_gate"))
    state["main"]["pending_gate"] = None
    return had


def allocate_issue_id(state: dict) -> str:
    n = state["feedback"]["next_id"]
    state["feedback"]["next_id"] = n + 1
    return f"issue-{n}"


def add_open_issue(state: dict, issue_id: str, from_node: str, created_at: str,
                   severity: str | None = None):
    state["feedback"]["open"].append(
        {"id": issue_id, "from": from_node, "category": None, "reroute_target": None,
         "severity": severity, "reroute_count": 0,
         "created_at": created_at, "resolved_at": None, "resolved_mode": None}
    )


def set_issue_triage(state: dict, issue_id: str, category: str, severity: str | None = None,
                     reroute_target: str | None = None):
    """分诊一次原子写入：category（沉淀归属，必）+ severity（可选）+ reroute_target（回退目标，可选）。"""
    for it in state["feedback"]["open"]:
        if it["id"] == issue_id:
            it["category"] = category
            if severity is not None:
                it["severity"] = severity
            it["reroute_target"] = reroute_target
            return
    raise StateError(f"open feedback 中找不到 {issue_id}")


def set_issue_category(state: dict, issue_id: str, category: str):
    """单独记归属（漏点地图），不动 severity/target——供 issue-resolve --category 先解决后沉淀。"""
    for it in state["feedback"]["open"]:
        if it["id"] == issue_id:
            it["category"] = category
            return
    raise StateError(f"open feedback 中找不到 {issue_id}")


def resolve_issue(state: dict, issue_id: str, resolved_at: str, resolved_mode: str | None = None):
    """把条目从 open 原对象搬到 resolved（两个数组结构因此天然一致）。

    resolved_mode 缺省不写 → 保持 None：留给 issue-triage 的「仅记录」闭环，那条支路没走
    解决路径，记 auto 会把「压根没解决过」算进自治率分子。issue-resolve 一律显式传。
    """
    for i, it in enumerate(state["feedback"]["open"]):
        if it["id"] == issue_id:
            it["resolved_at"] = resolved_at
            if resolved_mode is not None:
                it["resolved_mode"] = resolved_mode
            state["feedback"]["resolved"].append(state["feedback"]["open"].pop(i))
            return
    raise StateError(f"open feedback 中找不到 {issue_id}")


def reroute_to(state: dict, target: str, today: str, issue_id: str) -> int:
    """feedback-loop 分诊完调：把 current_step 直接改到 target 节点，history 记一笔。

    自增该 issue 的 reroute_count 并返回——供 CLI 判定是否触发逃生（反复修不好升级人工）。
    """
    state["main"]["current_step"] = target
    # 被弃节点的进入侧暂存一并丢弃：回流搬走了 current_step，那份 entered_* 已无出口可消费。
    # 不清则它会冒充 target 的进入水位（遥测失真），且 entered_step 残留会让 advance 门禁误判放行。
    clear_entered(state)
    count = 0
    for it in state["feedback"]["open"]:
        if it["id"] == issue_id:
            count = it.get("reroute_count", 0) + 1
            it["reroute_count"] = count
            break
    state["main"]["history"].append({
        "step": "workflow-feedback-loop",
        "ended_at": today,
        "outcome": "回流",
        "reroute_target": target,
        "issue": issue_id,
    })
    return count
