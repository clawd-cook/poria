#!/usr/bin/env python3
"""cli.py 自测：profile 编排 / init / 闸门 / 回流 / run-loop（subprocess + 纯函数）。

引擎已零硬编码节点名：所有行为由节点声明字段驱动，编排靠 .workflow/profile/ 的多 profile。
测试用下方 FULL/NO_AUTOTEST/LITE 三份 profile（与模板 .workflow/profile/ 对齐）覆盖。
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

CLI = Path(__file__).resolve().parent / "cli.py"

sys.path.insert(0, str(CLI.parent))
from _lib import nodes  # noqa: E402
from _lib import state as state_lib  # noqa: E402


# ---- 测试用 profile（与模板 .workflow/profile/ 对齐）----
FULL_PROFILE = [
    {"node": "workflow-propose", "exit_gate": "confirm-locked"},
    {"node": "workflow-test-plan", "exit_gate": "confirm-locked", "execution": "external-window",
     "feedback": {"can_report": True, "reroute_options": []}},
    {"node": "workflow-implement", "exit_gate": "auto", "autopilot": True},
    {"node": "workflow-lint", "exit_gate": "auto", "autopilot": True},
    {"node": "workflow-unittest", "exit_gate": "auto", "autopilot": True},
    {"node": "workflow-code-review", "exit_gate": "auto", "autopilot": True, "execution": "subagent",
     "feedback": {"can_report": True, "reroute_options": ["workflow-implement"]}},
    {"node": "workflow-test-cases", "exit_gate": "auto", "autopilot": True, "execution": "subagent",
     "feedback": {"can_report": True, "reroute_options": ["workflow-implement"]}},
    {"node": "workflow-deploy", "entry_gate": "confirm-locked", "exit_gate": "auto",
     "feedback": {"rerouteable": False}},
    {"node": "workflow-run-autotest", "exit_gate": "confirm-locked", "autopilot": True,
     "feedback": {"can_report": True, "reroute_options": ["workflow-implement"]}},
    {"node": "workflow-handoff-qa", "exit_gate": "confirm-locked",
     "feedback": {"can_report": True, "reroute_options": ["workflow-implement"]}},
    {"node": "workflow-archive", "exit_gate": "confirm-locked",
     "feedback": {"rerouteable": False}},
]

NO_AUTOTEST_PROFILE = [
    {"node": "workflow-propose", "exit_gate": "confirm-locked"},
    {"node": "workflow-implement", "exit_gate": "auto", "autopilot": True},
    {"node": "workflow-lint", "exit_gate": "auto", "autopilot": True},
    {"node": "workflow-unittest", "exit_gate": "auto", "autopilot": True},
    {"node": "workflow-code-review", "exit_gate": "auto", "autopilot": True, "execution": "subagent",
     "feedback": {"can_report": True, "reroute_options": ["workflow-implement"]}},
    {"node": "workflow-deploy", "entry_gate": "confirm-locked", "exit_gate": "auto",
     "feedback": {"rerouteable": False}},
    {"node": "workflow-handoff-qa", "exit_gate": "confirm-locked",
     "feedback": {"can_report": True, "reroute_options": ["workflow-implement"]}},
    {"node": "workflow-archive", "exit_gate": "confirm-locked",
     "feedback": {"rerouteable": False}},
]

LITE_PROFILE = [
    {"node": "workflow-implement", "exit_gate": "auto", "autopilot": True},
    {"node": "workflow-lint", "exit_gate": "auto", "autopilot": True},
    {"node": "workflow-unittest", "exit_gate": "auto", "autopilot": True},
    {"node": "workflow-code-review", "exit_gate": "auto", "autopilot": True, "execution": "subagent",
     "feedback": {"can_report": True, "reroute_options": ["workflow-implement"]}},
    {"node": "workflow-deploy", "entry_gate": "confirm-locked", "exit_gate": "auto",
     "feedback": {"rerouteable": False}},
    {"node": "workflow-handoff-qa", "exit_gate": "confirm-locked",
     "feedback": {"can_report": True, "reroute_options": ["workflow-implement"]}},
    {"node": "workflow-archive", "exit_gate": "confirm-locked",
     "feedback": {"rerouteable": False}},
]

DEFAULT_ROLES = {
    "trd": "openspec/changes/<task>/",
    "test-plan": "delivery/<task>/test/test-plan.md",
    "materials": "delivery/<task>/test/materials.md",
    "schema-draft": "delivery/<task>/schema/",
    "deploy": "delivery/<task>/deploy.md",
}


def _prof(node_arr, roles=None, desc=None, start_node=None, has_e2etest=None, enabled=None):
    """把节点数组包成 profile 对象 {desc, start_node, roles, nodes}（可选字段按需带）。"""
    obj = {"nodes": node_arr}
    if roles is not None:
        obj["roles"] = roles
    if desc is not None:
        obj["desc"] = desc
    if start_node is not None:
        obj["start_node"] = start_node
    if has_e2etest is not None:
        obj["has_e2etest"] = has_e2etest
    if enabled is not None:
        obj["enabled"] = enabled
    return obj


DEFAULT_CONFIG = {
    "profiles": {
        "full": _prof(FULL_PROFILE, roles=DEFAULT_ROLES, desc="完整流",
                      start_node="workflow-explore"),
        "no-autotest": _prof(NO_AUTOTEST_PROFILE, desc="去自动化测试", has_e2etest=False),
        "lite": _prof(LITE_PROFILE, desc="轻量流", has_e2etest=False),
    },
    "default_profile": "full",
}


def _write_config(root: Path, cfg: dict | None = None):
    """把 {profiles, default_profile, test_env, unittest} 落成 .workflow 目录布局。

    profiles 每份写成 .workflow/profile/<名>.json；其余键写 .workflow/config.json。
    先清空 profile 目录再写，避免多次改写残留旧文件。
    """
    cfg = cfg if cfg is not None else DEFAULT_CONFIG
    wf = root / ".workflow"
    pdir = wf / "profile"
    if pdir.exists():
        shutil.rmtree(pdir)
    pdir.mkdir(parents=True, exist_ok=True)
    for name, prof in (cfg.get("profiles") or {}).items():
        (pdir / f"{name}.json").write_text(json.dumps(prof))
    top = {k: v for k, v in cfg.items() if k != "profiles"}
    # 缺省注入「已采集」的 app_info，让 init 门禁默认放行（代表正常已接入项目）；
    # app_info 门禁专项测试自行在 cfg 里显式覆盖 app_info（如省略 app 触发 required 拦截）。
    top.setdefault("app_info", {"policy": "required", "app": "test-app", "app_id": "test-app-id"})
    (wf / "config.json").write_text(json.dumps(top))


def _mk_state(profile=FULL_PROFILE, task="t", today="2026-06-25"):
    return state_lib.default_state(task, today, nodes.resolve_pipeline(profile))


# 全测试共用一个「装好 lbcli skill」的桩 skills-home，经 WORKFLOW_SKILLS_HOME 注入。
_SKILLS_HOME = None


def setUpModule():
    global _SKILLS_HOME
    _SKILLS_HOME = Path(tempfile.mkdtemp())
    lbcli = _SKILLS_HOME / ".claude" / "skills" / "lbcli"
    lbcli.mkdir(parents=True)
    (lbcli / "SKILL.md").write_text("stub")


def tearDownModule():
    if _SKILLS_HOME:
        shutil.rmtree(_SKILLS_HOME, ignore_errors=True)


def _cli_env():
    env = dict(os.environ)
    env["WORKFLOW_SKILLS_HOME"] = str(_SKILLS_HOME)
    env["WORKFLOW_SKIP_VERSION_DETECT"] = "1"  # 测试不依赖本机是否装了全局 lb/lbcli，也免得每次 init fork 子进程
    env["WORKFLOW_SKIP_TELEMETRY"] = "1"  # 测试不拉远程配置、不 fork 上报子进程、不碰网络
    return env


def _git_init(root: Path, branch: str = "feature-test"):
    """把 root 变成指定分支上的 git 仓库（无需 commit——闸门用 symbolic-ref 探测）。

    每个测试都显式钉死分支，避免隐式依赖「tempfile.mkdtemp() 不在任何 git 仓库内」：
    谁把 TMPDIR 指到某个仓库里（如仓库内的 tmp/），起步分支闸门就会按那个仓库的分支判，
    落在 master 上时全量测试集体翻车。
    """
    subprocess.run(["git", "init", "-b", branch, str(root)],
                   capture_output=True, text=True, check=True)


class _CliBase(unittest.TestCase):
    """共用 root + 默认配置 + run_cli/state。"""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        _git_init(self.root)
        _write_config(self.root)

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def run_cli(self, *args):
        return subprocess.run(
            [sys.executable, str(CLI), "--target", str(self.root), *args],
            capture_output=True, text=True, env=_cli_env(),
        )

    def state(self, task):
        return json.loads((self.root / "delivery" / task / "state.json").read_text())

    def step(self, task, step, *extra):
        """走完一个节点：enter（过入口门禁拿凭据）→ advance。返回 advance 的结果。

        advance 硬性要求先 enter（凭据 main.entered_step），故测试里凡「推进一个节点」都得成对调用。
        只想单测 advance 自身拒绝行为的用例别用这个，直接 run_cli("advance", ...)。
        """
        self.run_cli("enter", task, "--step", step)
        return self.run_cli("advance", task, "--step", step, *extra)

    def init(self, task, *args):
        # 非 tty 下需环境的 profile 无 env flag 会 loud fail；测试只关心 seed/骨架/警告，
        # 未显式给 env flag 时补默认，让 init 过关（显式带 flag 的调用原样透传）。
        if not any(a in ("--env", "--scope", "--http-domain", "--jsf-alias", "--eone") for a in args):
            args = (*args, "--env", "test", "--scope", "http", "--http-domain", "a.com")
        # 非 tty init 必须显式 --profile（护栏，不静默默认）；未指定时补默认 full。
        if "--profile" not in args:
            args = ("--profile", "full", *args)
        r = self.run_cli("init", task, *args)
        self.assertEqual(r.returncode, 0, r.stderr)
        return r

    def walk_to_done(self, task):
        for _ in range(20):
            cur = self.state(task)["main"]["current_step"]
            if cur == "done":
                return
            self.step(task, cur)
        self.fail("walk_to_done 未在 20 步内到达 done")


class CliTest(_CliBase):
    # --- init 守卫 ---
    def test_init_rejects_archive_name(self):
        r = self.run_cli("init", "archive")
        self.assertEqual(r.returncode, 1)
        self.assertIn("保留名", r.stderr)

    def test_init_rejects_when_lbcli_skill_missing(self):
        empty_home = Path(tempfile.mkdtemp())
        try:
            env = dict(os.environ)
            env["WORKFLOW_SKILLS_HOME"] = str(empty_home)
            r = subprocess.run(
                [sys.executable, str(CLI), "--target", str(self.root), "init", "t-nolbcli"],
                capture_output=True, text=True, env=env,
            )
            self.assertEqual(r.returncode, 1)
            self.assertIn("lbcli", r.stderr)
            self.assertFalse((self.root / "delivery" / "t-nolbcli").exists())
        finally:
            shutil.rmtree(empty_home, ignore_errors=True)

    # --- init seed（默认 full profile，seed=propose）---
    def test_init_seed_is_first_node(self):
        self.init("t0")
        self.assertEqual(self.state("t0")["main"]["current_step"], "workflow-propose")
        self.assertEqual(self.state("t0")["main"]["history"], [])

    def test_init_lite_seed_is_implement(self):
        self.init("t0b", "--profile", "lite")
        self.assertEqual(self.state("t0b")["main"]["current_step"], "workflow-implement")

    def test_init_unknown_profile_fails(self):
        r = self.run_cli("init", "t0c", "--profile", "nope")
        self.assertEqual(r.returncode, 1)
        self.assertIn("未知 --profile", r.stderr)

    def test_init_no_options_key_in_state(self):
        self.init("t0d")
        self.assertNotIn("options", self.state("t0d"))

    def test_init_snapshots_start_node(self):
        # full profile 声明 start_node=workflow-explore → init 快照进 state 顶层。
        self.init("t0s")
        self.assertEqual(self.state("t0s")["start_node"], "workflow-explore")

    def test_init_start_node_none_when_profile_omits(self):
        # lite profile 未声明 start_node → 快照为 None（字段存在）。
        self.init("t0sn", "--profile", "lite")
        st = self.state("t0sn")
        self.assertIn("start_node", st)
        self.assertIsNone(st["start_node"])

    def test_init_has_version_fields_in_state(self):
        # 探测走真实子进程调 lb/lbcli；自测环境统一注入 WORKFLOW_SKIP_VERSION_DETECT=1 跳过（见 _cli_env），
        # 故机器态落地值为 None——这里只断言字段存在、语义正确，探测本身的真实调用见 test_init_captures_real_tool_version。
        self.init("t0e")
        st = self.state("t0e")
        self.assertIn("npm_lb_version", st)     # 机器态：探测 lb -V
        self.assertIn("npm_lbcli_version", st)  # 机器态：探测 lbcli -V
        self.assertIn("lb_version", st)         # 仓库态：config.json 落地模板版本
        self.assertIsNone(st["npm_lb_version"])
        self.assertIsNone(st["npm_lbcli_version"])

    def test_init_snapshots_repo_lb_version_from_config(self):
        # 仓库态 lb_version 取自 config.json.lb_version（load_project_config 须透传该字段，
        # 不能只回一份白名单子集——回归 705604f 里 cfg.get("lb_version") 因白名单裁剪永远拿 None 的 bug）。
        cfg = dict(DEFAULT_CONFIG)
        cfg["lb_version"] = "1.1.0"
        _write_config(self.root, cfg)
        self.init("t0g")
        self.assertEqual(self.state("t0g")["lb_version"], "1.1.0")

    def test_init_captures_real_tool_version(self):
        # 拿掉 skip 开关，用桩 lb/lbcli 可执行文件占住 PATH，验证子进程探测真的读到了版本号首行。
        bin_dir = Path(tempfile.mkdtemp())
        try:
            for name, flag, version in (("lb", "-V", "9.9.9-stub"), ("lbcli", "-V", "1.2.3-stub")):
                script = bin_dir / name
                script.write_text(f"#!/bin/sh\nif [ \"$1\" = \"{flag}\" ]; then echo {version}; fi\n")
                script.chmod(0o755)
            env = dict(os.environ)
            env["WORKFLOW_SKILLS_HOME"] = str(_SKILLS_HOME)
            env["PATH"] = f"{bin_dir}{os.pathsep}{env.get('PATH', '')}"
            r = subprocess.run(
                [sys.executable, str(CLI), "--target", str(self.root), "init", "t0f",
                 "--profile", "full", "--env", "test", "--scope", "http", "--http-domain", "a.com"],
                capture_output=True, text=True, env=env,
            )
            self.assertEqual(r.returncode, 0, r.stderr)
            st = self.state("t0f")
            self.assertEqual(st["npm_lb_version"], "9.9.9-stub")
            self.assertEqual(st["npm_lbcli_version"], "1.2.3-stub")
        finally:
            shutil.rmtree(bin_dir, ignore_errors=True)

    # --- 项目未初始化提示 ---
    def _write_bootstrap_state(self, data):
        p = self.root / ".workflow" / "bootstrap-state.json"
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(json.dumps(data), encoding="utf-8")

    @staticmethod
    def _done():
        return {"done": True, "at": "2026-06-22 10:00:00"}

    def test_init_warns_when_no_bootstrap_state(self):
        r = self.init("t-uninit")
        self.assertIn("项目知识底座未初始化", r.stdout)
        self.assertIn("独立窗口", r.stdout)

    def test_init_warns_when_only_overview_done(self):
        self._write_bootstrap_state({"overview": self._done()})
        r = self.init("t-overview-only")
        self.assertIn("项目知识底座未初始化", r.stdout)

    def test_init_warns_when_missing_tech(self):
        self._write_bootstrap_state({
            "overview": self._done(),
            "modules": {"1": self._done()},
        })
        r = self.init("t-no-tech")
        self.assertIn("项目知识底座未初始化", r.stdout)

    def test_init_no_warn_when_overview_module_tech_done(self):
        self._write_bootstrap_state({
            "overview": self._done(),
            "modules": {"1": self._done()},
            "tech": {"1": self._done()},
        })
        r = self.init("t-real")
        self.assertNotIn("项目知识底座未初始化", r.stdout)

    def test_init_migrates_legacy_bootstrap_state(self):
        # 旧位置 .claude/bootstrap-state.json → init 时搬到 .workflow/，底座判为已落地
        legacy = self.root / ".claude" / "bootstrap-state.json"
        legacy.parent.mkdir(parents=True, exist_ok=True)
        legacy.write_text(json.dumps({
            "overview": self._done(),
            "modules": {"1": self._done()},
            "tech": {"1": self._done()},
        }), encoding="utf-8")
        r = self.init("t-migrate")
        self.assertFalse(legacy.exists())
        self.assertTrue((self.root / ".workflow" / "bootstrap-state.json").exists())
        self.assertNotIn("项目知识底座未初始化", r.stdout)

    # --- archive-move ---
    def test_archive_move_requires_done(self):
        self.init("t5", "--profile", "lite")
        r = self.run_cli("archive-move", "t5")
        self.assertEqual(r.returncode, 1)
        self.assertIn("done", r.stderr)
        self.assertTrue((self.root / "delivery" / "t5").exists())

    def test_archive_move_at_done(self):
        self.init("t6", "--profile", "lite")
        self.walk_to_done("t6")
        self.assertEqual(self.state("t6")["main"]["current_step"], "done")
        r = self.run_cli("archive-move", "t6")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertFalse((self.root / "delivery" / "t6").exists())
        self.assertTrue((self.root / "delivery" / "archive" / "t6" / "state.json").exists())

    def test_archive_move_force_skips_done_check(self):
        self.init("t7", "--profile", "lite")
        r = self.run_cli("archive-move", "t7", "--force")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertTrue((self.root / "delivery" / "archive" / "t7").exists())

    def test_archive_move_rejects_existing_dst(self):
        self.init("t8", "--profile", "lite")
        (self.root / "delivery" / "archive" / "t8").mkdir(parents=True)
        r = self.run_cli("archive-move", "t8", "--force")
        self.assertEqual(r.returncode, 1)
        self.assertIn("已存在", r.stderr)
        self.assertTrue((self.root / "delivery" / "t8").exists())


class ActiveTasksIndexTest(_CliBase):
    """活跃任务清单（delivery/active-tasks.json）在 advance 推到 done 那一刻摘除。

    回归的真实故障：任务归档了却还挂在清单上——原先只有 archive-move 摘除，
    advance 到 done 后漏跑那条命令就永久残留，且尾斜杠会让摘除静默失效。
    """

    def index(self):
        p = self.root / "delivery" / "active-tasks.json"
        return json.loads(p.read_text()) if p.exists() else None

    def names(self):
        idx = self.index()
        return [t["task"] for t in idx["tasks"]] if idx else []

    def test_init_registers_task(self):
        self.init("t-idx-a", "--profile", "lite")
        self.assertEqual(self.names(), ["t-idx-a"])
        self.assertEqual(self.index()["latest_task"], "t-idx-a")

    def test_advance_to_done_removes_without_archive_move(self):
        """核心回归：done 那一刻就摘，不依赖 AI 记得跑 archive-move。"""
        self.init("t-idx-b", "--profile", "lite")
        self.walk_to_done("t-idx-b")
        self.assertEqual(self.names(), [])
        self.assertEqual(self.index()["latest_task"], None)
        # 目录仍在原位——摘清单不等于物理归档，两件事刻意解耦
        self.assertTrue((self.root / "delivery" / "t-idx-b").exists())

    def test_done_keeps_other_active_tasks_and_rolls_back_latest(self):
        self.init("t-idx-c", "--profile", "lite")
        self.init("t-idx-d", "--profile", "lite")
        self.assertEqual(self.index()["latest_task"], "t-idx-d")
        self.walk_to_done("t-idx-d")
        self.assertEqual(self.names(), ["t-idx-c"])
        self.assertEqual(self.index()["latest_task"], "t-idx-c")

    def test_archive_move_tolerates_trailing_slash(self):
        """tab 补全带出的尾斜杠：Path 拼接会吞掉它，故打印与清单比对都须先归一化。"""
        self.init("t-idx-f", "--profile", "lite")
        r = self.run_cli("archive-move", "t-idx-f/", "--force")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn("//", r.stdout)
        self.assertEqual(self.names(), [])
        self.assertTrue((self.root / "delivery" / "archive" / "t-idx-f" / "state.json").exists())

    def test_force_archive_move_without_done_still_removes(self):
        """--force 直迁没走到 done、没触发那次摘除，靠 archive-move 里的兜底。"""
        self.init("t-idx-g", "--profile", "lite")
        self.assertEqual(self.names(), ["t-idx-g"])
        r = self.run_cli("archive-move", "t-idx-g", "--force")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.names(), [])

    def test_write_failure_warns_on_stderr_without_blocking(self):
        """软失败要出声：全静默会让「归档了却还挂着」看起来完全正常。"""
        self.init("t-idx-h", "--profile", "lite")
        idx_path = self.root / "delivery" / "active-tasks.json"
        idx_path.chmod(0o444)
        try:
            self.walk_to_done("t-idx-h")
        finally:
            idx_path.chmod(0o644)
        self.assertEqual(self.state("t-idx-h")["main"]["current_step"], "done")  # 流转未被阻断
        self.assertEqual(self.names(), ["t-idx-h"])  # 写不动 → 清单确实脏了，故必须告警

    def test_done_hints_archive_move_still_pending(self):
        """done 的三个出口都要提示还差物理归档一步（原先一句都没提）。"""
        self.init("t-idx-i", "--profile", "lite")
        self.walk_to_done("t-idx-i")
        for argv in (("next", "t-idx-i"), ("run-loop", "t-idx-i")):
            r = self.run_cli(*argv)
            self.assertEqual(r.returncode, 0, r.stderr)
            self.assertIn("尚未物理归档", r.stdout, f"{argv[0]} 应提示 archive-move")
            self.assertIn("archive-move", r.stdout)

    def test_hint_gone_after_archive_move(self):
        """迁完就该闭嘴——判据是目录是否还在原位，不是状态位。"""
        self.init("t-idx-j", "--profile", "lite")
        self.walk_to_done("t-idx-j")
        self.run_cli("archive-move", "t-idx-j")
        r = self.run_cli("next", "t-idx-j")
        self.assertNotIn("尚未物理归档", r.stdout)


class DoneIsTerminalTest(_CliBase):
    """done 是终态：不许借回流复活。

    enter / set-env 早就挡了 done，唯独 reroute 与 issue-triage --target 漏挡——
    防前跳的 is_downstream 对 done 不设防（done 不在 pipeline 序列里，判定恒 False）。
    """

    def test_reroute_from_done_rejected(self):
        self.init("t-term-a", "--profile", "lite")
        self.walk_to_done("t-term-a")
        r = self.run_cli("issue-add", "t-term-a", "--from", "manual", "--phenomenon", "临门一脚")
        self.assertEqual(r.returncode, 0, r.stderr)
        r = self.run_cli("reroute", "t-term-a", "issue-1", "--to", "workflow-implement")
        self.assertEqual(r.returncode, 1)
        self.assertIn("已归档", r.stderr)
        # 状态没被改动：仍是 done
        self.assertEqual(self.state("t-term-a")["main"]["current_step"], "done")

    def test_issue_triage_with_target_from_done_rejected(self):
        """另一道平行的门：不挡的话会「分诊说 target=X 成功、紧接着 reroute 被拒」。"""
        self.init("t-term-b", "--profile", "lite")
        self.walk_to_done("t-term-b")
        self.run_cli("issue-add", "t-term-b", "--from", "manual", "--phenomenon", "x")
        r = self.run_cli("issue-triage", "t-term-b", "issue-1",
                         "--category", "code", "--target", "workflow-implement")
        self.assertEqual(r.returncode, 1)
        self.assertIn("已归档", r.stderr)

    def test_issue_triage_record_only_still_allowed_at_done(self):
        """只记录（无 --target）仍放行：归档任务上留痕不改主线，是纯沉淀用途。"""
        self.init("t-term-c", "--profile", "lite")
        self.walk_to_done("t-term-c")
        self.run_cli("issue-add", "t-term-c", "--from", "manual", "--phenomenon", "x")
        r = self.run_cli("issue-triage", "t-term-c", "issue-1", "--category", "code")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.state("t-term-c")["main"]["current_step"], "done")

    def test_enter_and_set_env_already_guard_done(self):
        """对照基线：这两条本来就挡，确保本次改动没把它们弄松。"""
        self.init("t-term-d", "--profile", "lite")
        self.walk_to_done("t-term-d")
        r = self.run_cli("enter", "t-term-d", "--step", "workflow-implement")
        self.assertEqual(r.returncode, 1)
        self.assertIn("已归档", r.stderr)
        r = self.run_cli("set-env", "t-term-d", "--env", "gamma")
        self.assertEqual(r.returncode, 1)
        self.assertIn("已归档", r.stderr)


class AppInfoGateTest(_CliBase):
    """app_info 三态门禁 + --skip-app-info 逃生开关。"""

    def _config(self, app_info):
        cfg = {**DEFAULT_CONFIG, "app_info": app_info}
        _write_config(self.root, cfg)

    def _init_args(self, task, *extra):
        return ("init", task, "--profile", "lite", *extra)

    def test_required_missing_app_blocks_init(self):
        self._config({"policy": "required", "app": ""})
        r = self.run_cli(*self._init_args("g1"))
        self.assertEqual(r.returncode, 1)
        self.assertIn("app_info", r.stderr)
        self.assertFalse((self.root / "delivery" / "g1").exists())  # 失败不留孤儿目录

    def test_required_missing_app_escapes_with_skip_flag(self):
        self._config({"policy": "required", "app": ""})
        r = self.run_cli(*self._init_args("g2", "--skip-app-info"))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertTrue(self.state("g2")["app_info_skipped"])  # 逃生留痕

    def test_required_collected_passes_silently(self):
        self._config({"policy": "required", "app": "some-app", "app_id": "some-app-id"})
        r = self.run_cli(*self._init_args("g3"))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn("app_info_skipped", self.state("g3"))

    def test_required_app_without_appid_blocks_init(self):
        # app 非空但 app_id 空——半失败/瞎填不算已采，required 仍应拦截
        self._config({"policy": "required", "app": "some-app", "app_id": ""})
        r = self.run_cli(*self._init_args("g3b"))
        self.assertEqual(r.returncode, 1)
        self.assertIn("app_info", r.stderr)

    def test_suggested_missing_app_warns_but_passes(self):
        self._config({"policy": "suggested", "app": ""})
        r = self.run_cli(*self._init_args("g4"))
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn("app_info_skipped", self.state("g4"))

    def test_off_missing_app_silent(self):
        self._config({"policy": "off", "app": ""})
        r = self.run_cli(*self._init_args("g5"))
        self.assertEqual(r.returncode, 0, r.stderr)

    def test_missing_policy_loud_fails_regardless_of_project_type(self):
        # policy 是唯一事实源，由项目级 `lb init` 落定；缺失/空字符串不再按 project_type
        # 运行时缺省，不管 backend 还是 frontend 都应直接 loud fail。
        for project_type, task in (("backend", "g6"), ("frontend", "g7")):
            with self.subTest(project_type=project_type):
                cfg = {**DEFAULT_CONFIG, "project_type": project_type, "app_info": {"app": ""}}
                _write_config(self.root, cfg)
                r = self.run_cli(*self._init_args(task))
                self.assertEqual(r.returncode, 1)
                self.assertIn("policy", r.stderr)

    def test_invalid_policy_loud_fails(self):
        self._config({"policy": "bogus", "app": "x"})
        r = self.run_cli(*self._init_args("g8"))
        self.assertEqual(r.returncode, 1)
        self.assertIn("policy", r.stderr)


class EnvProfileTest(_CliBase):
    """env flag 路径 / profile 选择 / has_dedicated 门控 / set-env。"""

    def write_no_dedicated(self):
        cfg = {**DEFAULT_CONFIG, "test_env": {"has_dedicated": False, "target": "gamma"}}
        _write_config(self.root, cfg)

    def test_e2etest_no_flag_loud_fails(self):
        # 需环境的 profile（full，has_e2etest 缺省 true）非 tty 无 env flag：loud fail 逼先收集
        r = self.run_cli("init", "e1", "--profile", "full")
        self.assertEqual(r.returncode, 1)
        self.assertIn("未收到任何 env flag", r.stderr)
        self.assertFalse((self.root / "delivery" / "e1").exists())

    def test_default_profile_when_no_flag(self):
        self.run_cli("init", "e2", "--profile", "full", "--env", "test", "--scope", "http", "--http-domain", "a.com")
        seq = [o["node"] for o in self.state("e2")["pipeline"]]
        self.assertEqual(seq[0], "workflow-propose")
        self.assertEqual(len(seq), 11)

    def test_profile_flag_selects(self):
        self.run_cli("init", "e3", "--profile", "no-autotest")
        seq = [o["node"] for o in self.state("e3")["pipeline"]]
        self.assertNotIn("workflow-test-plan", seq)
        self.assertNotIn("workflow-run-autotest", seq)

    def test_env_flags_full(self):
        r = self.run_cli("init", "e5", "--profile", "full", "--env", "test", "--scope", "http,jsf",
                         "--http-domain", "a.com", "--jsf-alias", "x")
        self.assertEqual(r.returncode, 0, r.stderr)
        env = self.state("e5")["env"]
        self.assertEqual(env["target"], "test")
        self.assertEqual(env["scope"], ["http", "jsf"])
        self.assertEqual(env["http_domain"], "a.com")
        self.assertEqual(env["jsf_alias"], "x")

    def test_invalid_scope(self):
        r = self.run_cli("init", "e6", "--scope", "rpc")
        self.assertEqual(r.returncode, 1)

    def test_invalid_env(self):
        r = self.run_cli("init", "e7", "--env", "prod")
        self.assertEqual(r.returncode, 2)

    def test_no_dedicated_rejects_test(self):
        self.write_no_dedicated()
        r = self.run_cli("init", "e8", "--profile", "full", "--env", "test")
        self.assertEqual(r.returncode, 1)
        self.assertIn("has_dedicated", r.stderr)

    def test_no_dedicated_rejects_local(self):
        self.write_no_dedicated()
        r = self.run_cli("init", "e9", "--env", "local")
        self.assertEqual(r.returncode, 1)

    def test_no_dedicated_allows_gamma(self):
        self.write_no_dedicated()
        r = self.run_cli("init", "e10", "--profile", "full", "--env", "gamma", "--scope", "http", "--http-domain", "a.com")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.state("e10")["env"]["target"], "gamma")

    def test_no_dedicated_no_flag_loud_fails(self):
        # 无独立测试环境的项目 + 需环境 profile + 无 env flag：仍 loud fail（scope/域名待收集）
        self.write_no_dedicated()
        r = self.run_cli("init", "e11", "--profile", "full")
        self.assertEqual(r.returncode, 1)
        self.assertIn("未收到任何 env flag", r.stderr)

    # --- has_e2etest=false：无端到端自动化测试的 profile 钉本地环境 ---
    def _write_e2e_config(self):
        cfg = {
            "profiles": {
                "full": _prof(FULL_PROFILE, roles=DEFAULT_ROLES, desc="完整流",
                              start_node="workflow-explore", has_e2etest=True),
                "lite": _prof(LITE_PROFILE, desc="轻量流", has_e2etest=False),
            },
            "default_profile": "full",
        }
        _write_config(self.root, cfg)

    def test_no_e2etest_pins_local_no_tty(self):
        self._write_e2e_config()
        r = self.run_cli("init", "ne1", "--profile", "lite")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.state("ne1")["env"]["target"], "local")

    def test_e2etest_no_flag_loud_fails_no_tty(self):
        self._write_e2e_config()
        r = self.run_cli("init", "ne2", "--profile", "full")
        self.assertEqual(r.returncode, 1)
        self.assertIn("未收到任何 env flag", r.stderr)

    def test_no_e2etest_default_profile_pins_local(self):
        cfg = {
            "profiles": {"lite": _prof(LITE_PROFILE, desc="轻量流", has_e2etest=False)},
            "default_profile": "lite",
        }
        _write_config(self.root, cfg)
        r = self.run_cli("init", "ne3", "--profile", "lite")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.state("ne3")["env"]["target"], "local")

    def test_no_e2etest_explicit_env_flag_overrides(self):
        self._write_e2e_config()
        r = self.run_cli("init", "ne4", "--profile", "lite", "--env", "test",
                         "--scope", "http", "--http-domain", "a.com")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.state("ne4")["env"]["target"], "test")

    # --- set-env ---
    def test_set_env_partial_update(self):
        self.run_cli("init", "s1", "--profile", "full", "--env", "test", "--scope", "http", "--http-domain", "a.com")
        r = self.run_cli("set-env", "s1", "--scope", "http,jsf", "--jsf-alias", "x")
        self.assertEqual(r.returncode, 0, r.stderr)
        env = self.state("s1")["env"]
        self.assertEqual(env["scope"], ["http", "jsf"])
        self.assertEqual(env["jsf_alias"], "x")
        self.assertEqual(env["http_domain"], "a.com")

    def test_set_env_clear_with_empty(self):
        self.run_cli("init", "s2", "--profile", "full", "--env", "test", "--scope", "http", "--http-domain", "a.com")
        r = self.run_cli("set-env", "s2", "--http-domain", "")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIsNone(self.state("s2")["env"]["http_domain"])

    def test_set_env_target_leaving_test_clears_eone(self):
        self.run_cli("init", "s3", "--profile", "full", "--env", "test", "--scope", "http", "--http-domain", "a.com", "--eone", "feat-1")
        self.assertEqual(self.state("s3")["env"]["eone_feature_env"], "feat-1")
        r = self.run_cli("set-env", "s3", "--env", "gamma")
        self.assertEqual(r.returncode, 0, r.stderr)
        env = self.state("s3")["env"]
        self.assertEqual(env["target"], "gamma")
        self.assertIsNone(env["eone_feature_env"])

    def test_set_env_no_args(self):
        self.run_cli("init", "s4", "--profile", "full", "--env", "test", "--scope", "http", "--http-domain", "a.com")
        r = self.run_cli("set-env", "s4")
        self.assertEqual(r.returncode, 1)
        self.assertIn("至少", r.stderr)

    def test_set_env_invalid_scope(self):
        self.run_cli("init", "s5", "--profile", "full", "--env", "test", "--scope", "http", "--http-domain", "a.com")
        r = self.run_cli("set-env", "s5", "--scope", "rpc")
        self.assertEqual(r.returncode, 1)

    def test_set_env_no_dedicated_rejects_test(self):
        self.write_no_dedicated()
        self.run_cli("init", "s6", "--profile", "full", "--env", "gamma", "--scope", "http", "--http-domain", "a.com")
        r = self.run_cli("set-env", "s6", "--env", "test")
        self.assertEqual(r.returncode, 1)
        self.assertIn("has_dedicated", r.stderr)


class AutonomousGateTest(unittest.TestCase):
    """自治闸门降级（纯函数）：非锁定出口降 auto，锁定闸（confirm-locked）不降。"""

    def setUp(self):
        self.full = nodes.resolve_pipeline(FULL_PROFILE)
        # implement 配 confirm 验证降级
        cfg = [dict(o) for o in LITE_PROFILE]
        cfg[0] = {"node": "workflow-implement", "exit_gate": "confirm"}
        self.confirm_impl = nodes.resolve_pipeline(cfg)

    def test_non_pinned_exit_downgraded_to_auto(self):
        self.assertEqual(nodes.resolve_exit_gate("workflow-implement", self.confirm_impl), nodes.GATE_CONFIRM)
        self.assertEqual(nodes.resolve_exit_gate("workflow-implement", self.confirm_impl, autonomous=True), nodes.GATE_AUTO)

    def test_locked_exit_not_downgraded(self):
        for n in ("workflow-propose", "workflow-test-plan"):
            self.assertEqual(nodes.resolve_exit_gate(n, self.full, autonomous=True), nodes.GATE_CONFIRM_LOCKED)

    def test_run_autotest_exit_locked_survives_loop(self):
        self.assertEqual(nodes.resolve_exit_gate("workflow-run-autotest", self.full, autonomous=True), nodes.GATE_CONFIRM_LOCKED)

    def test_deploy_entry_locked_preserved_in_autonomous(self):
        gate = nodes.transition_gate("workflow-test-cases", "workflow-deploy", self.full, autonomous=True)
        self.assertEqual(gate, nodes.GATE_CONFIRM_LOCKED)

    def test_autonomous_overrides_configured_gate(self):
        gate = nodes.resolve_exit_gate("workflow-implement", self.confirm_impl, autonomous=True)
        self.assertEqual(gate, nodes.GATE_AUTO)

    def test_run_autotest_to_handoff_stays_locked(self):
        gate = nodes.transition_gate("workflow-run-autotest", "workflow-handoff-qa", self.full, autonomous=True)
        self.assertEqual(gate, nodes.GATE_CONFIRM_LOCKED)



class CmdCoreDirectTest(_CliBase):
    """直调 6 个核心 cmd_*（不经 subprocess）——专供 mutmut 把变异关联到 cli.py。

    现有大批用例走 subprocess，父进程栈碰不到 cmd_*，变异会变成 no tests。
    这里用 argparse.Namespace 进进程调用，覆盖各命令主路径 + 关键硬拒。
    """

    def setUp(self):
        super().setUp()
        os.environ.setdefault("WORKFLOW_SKIP_VERSION_DETECT", "1")
        os.environ.setdefault("WORKFLOW_SKIP_TELEMETRY", "1")
        # 与 unittest discover / mutmut 同 cwd 时能 import cli
        import cli as cli_mod
        self.cli = cli_mod

    def _ns(self, **kw):
        base = {
            "target": str(self.root),
            "task": "t-core",
            "profile": "lite",
            "env": "test",
            "scope": "http",
            "http_domain": "a.com",
            "jsf_alias": None,
            "eone": None,
            "skip_app_info": False,
            "skip_ping": True,
            "force": False,
            "step": "workflow-implement",
            "loop": False,
            "outcome": "ok",
            "set_value": None,
        }
        base.update(kw)
        from argparse import Namespace
        return Namespace(**base)

    def test_init_happy_path(self):
        self.cli.cmd_init(self._ns(task="t-init"))
        st = self.state("t-init")
        self.assertEqual(st["main"]["current_step"], "workflow-implement")

    def test_init_rejects_archive_name(self):
        with self.assertRaises(SystemExit):
            self.cli.cmd_init(self._ns(task="archive"))

    def test_list_profiles_prints_enabled(self):
        import io, contextlib
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            self.cli.cmd_list_profiles(self._ns(skip_ping=True))
        out = buf.getvalue()
        self.assertIn("可用编排模式", out)
        self.assertIn("lite", out)

    def test_remote_config_files_prints_unknown_by_default(self):
        import io, contextlib
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            self.cli.cmd_remote_config_files(self._ns())
        self.assertEqual(buf.getvalue().strip(), "unknown")

    def test_remote_config_files_prints_false_when_off(self):
        cfg_path = self.root / ".workflow" / "config.json"
        raw = json.loads(cfg_path.read_text())
        raw["remote_config_files"] = False
        cfg_path.write_text(json.dumps(raw))
        import io, contextlib
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            self.cli.cmd_remote_config_files(self._ns())
        self.assertEqual(buf.getvalue().strip(), "false")

    def test_remote_config_files_subprocess(self):
        r = self.run_cli("remote-config-files")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "unknown")

    def test_remote_config_files_set_true_then_false(self):
        r = self.run_cli("remote-config-files", "--set", "true")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "true")
        self.assertIs(json.loads((self.root / ".workflow" / "config.json").read_text())["remote_config_files"], True)
        r = self.run_cli("remote-config-files", "--set", "false")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "false")
        r = self.run_cli("remote-config-files", "--set", "unknown")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(r.stdout.strip(), "unknown")
        self.assertIsNone(json.loads((self.root / ".workflow" / "config.json").read_text())["remote_config_files"])

    def test_node_missing_dies(self):
        import io, contextlib
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            with self.assertRaises(SystemExit):
                self.cli.cmd_node(self._ns(step="no-such-node", task="x"))
        self.assertIn("no-such-node", err.getvalue())
        self.assertIn("NODE.md", err.getvalue())

    def test_node_prints_read_path(self):
        node_dir = self.root / ".workflow" / "node" / "workflow-implement"
        node_dir.mkdir(parents=True)
        (node_dir / "NODE.md").write_text("# implement\n")
        import io, contextlib
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            self.cli.cmd_node(self._ns(step="workflow-implement", task="any"))
        self.assertIn("读 .workflow/node/workflow-implement/NODE.md", buf.getvalue())

    def test_enter_and_advance_pair(self):
        self.cli.cmd_init(self._ns(task="t-ea"))
        self.cli.cmd_enter(self._ns(task="t-ea", step="workflow-implement"))
        self.assertEqual(self.state("t-ea")["main"]["entered_step"], "workflow-implement")
        self.cli.cmd_advance(self._ns(task="t-ea", step="workflow-implement"))
        self.assertEqual(self.state("t-ea")["main"]["current_step"], "workflow-lint")
        self.assertIsNone(self.state("t-ea")["main"].get("entered_step"))

    def test_advance_without_enter_dies(self):
        self.cli.cmd_init(self._ns(task="t-ae"))
        with self.assertRaises(SystemExit):
            self.cli.cmd_advance(self._ns(task="t-ae", step="workflow-implement"))

    def test_archive_move_requires_done(self):
        self.cli.cmd_init(self._ns(task="t-am"))
        with self.assertRaises(SystemExit):
            self.cli.cmd_archive_move(self._ns(task="t-am", force=False))

    def test_archive_move_force(self):
        self.cli.cmd_init(self._ns(task="t-amf"))
        self.cli.cmd_archive_move(self._ns(task="t-amf", force=True))
        self.assertFalse((self.root / "delivery" / "t-amf").exists())
        self.assertTrue((self.root / "delivery" / "archive" / "t-amf").exists())



class EnterAdvanceDirectTest(_CliBase):
    """enter/advance 门禁矩阵（直调）——专杀 cmd_enter/cmd_advance 逻辑变异。"""

    def setUp(self):
        super().setUp()
        os.environ.setdefault("WORKFLOW_SKIP_VERSION_DETECT", "1")
        os.environ.setdefault("WORKFLOW_SKIP_TELEMETRY", "1")
        import cli as cli_mod
        self.cli = cli_mod

    def _ns(self, **kw):
        from argparse import Namespace
        base = {
            "target": str(self.root), "task": "t-ea2", "profile": "lite",
            "env": "test", "scope": "http", "http_domain": "a.com",
            "jsf_alias": None, "eone": None, "skip_app_info": False,
            "skip_ping": True, "force": False, "step": "workflow-implement",
            "loop": False, "outcome": "ok",
        }
        base.update(kw)
        return Namespace(**base)

    def _die_err(self, call):
        import io, contextlib
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            with self.assertRaises(SystemExit):
                call()
        return err.getvalue()

    def _init(self, task="t-ea2"):
        self.cli.cmd_init(self._ns(task=task))
        return task

    def _add_open(self, task, issue_id="issue-1", from_node="workflow-code-review"):
        st = self.state(task)
        state_lib.add_open_issue(st, issue_id, from_node, "2026-06-25 10:00:00")
        state_lib.save_state(self.root, task, st)

    # --- enter ---

    def test_enter_unknown_step_dies(self):
        task = self._init("t-unk")
        err = self._die_err(lambda: self.cli.cmd_enter(self._ns(task=task, step="nope")))
        self.assertIn("未知节点", err)
        self.assertIn("nope", err)

    def test_enter_wrong_step_dies(self):
        task = self._init("t-ws")
        err = self._die_err(lambda: self.cli.cmd_enter(self._ns(task=task, step="workflow-lint")))
        self.assertIn("当前节点是", err)
        self.assertIn("workflow-implement", err)

    def test_enter_done_dies(self):
        task = self._init("t-done")
        st = self.state(task)
        st["main"]["current_step"] = "done"
        state_lib.save_state(self.root, task, st)
        err = self._die_err(lambda: self.cli.cmd_enter(self._ns(task=task, step="workflow-implement")))
        self.assertIn("已归档", err)

    def test_enter_untriaged_open_locks(self):
        task = self._init("t-ut")
        self._add_open(task)
        err = self._die_err(lambda: self.cli.cmd_enter(self._ns(task=task, step="workflow-implement")))
        self.assertIn("未分诊", err)
        self.assertIn("issue-1", err)

    def test_enter_triaged_wrong_target_locks(self):
        task = self._init("t-tt")
        self._add_open(task)
        st = self.state(task)
        state_lib.set_issue_triage(st, "issue-1", "code", "medium", "workflow-implement")
        # 人为把 current 拨到 lint（不经 advance：open feedback 会挡推进）
        st["main"]["current_step"] = "workflow-lint"
        state_lib.save_state(self.root, task, st)
        err = self._die_err(lambda: self.cli.cmd_enter(self._ns(task=task, step="workflow-lint")))
        self.assertIn("主线锁定", err)
        self.assertIn("workflow-implement", err)

    def test_enter_triaged_correct_target_ok(self):
        task = self._init("t-tok")
        self._add_open(task)
        st = self.state(task)
        state_lib.set_issue_triage(st, "issue-1", "code", "medium", "workflow-implement")
        state_lib.save_state(self.root, task, st)
        self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
        self.assertEqual(self.state(task)["main"]["entered_step"], "workflow-implement")

    def test_enter_feedback_loop_requires_open(self):
        task = self._init("t-fl0")
        err = self._die_err(lambda: self.cli.cmd_enter(
            self._ns(task=task, step="workflow-feedback-loop")))
        self.assertIn("无 open feedback", err)

    def test_enter_feedback_loop_with_open(self):
        task = self._init("t-fl1")
        self._add_open(task)
        import io, contextlib
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            self.cli.cmd_enter(self._ns(task=task, step="workflow-feedback-loop"))
        self.assertIn("issue-1", buf.getvalue())
        # feedback-loop 早退，不写 enter 凭据
        self.assertIsNone(self.state(task)["main"].get("entered_step"))

    def test_enter_clears_pending_gate_and_sets_credential(self):
        task = self._init("t-pg")
        st = self.state(task)
        state_lib.set_pending_gate(st, "confirm", "prev", "workflow-implement", "go", "2026-06-25")
        state_lib.save_state(self.root, task, st)
        self.assertIsNotNone(self.state(task)["main"].get("pending_gate"))
        self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
        st2 = self.state(task)
        self.assertIsNone(st2["main"].get("pending_gate"))
        self.assertEqual(st2["main"]["entered_step"], "workflow-implement")
        self.assertIn("entered_at", st2["main"])

    # --- advance ---

    def test_advance_open_feedback_locks(self):
        task = self._init("t-aof")
        self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
        self._add_open(task)
        err = self._die_err(lambda: self.cli.cmd_advance(
            self._ns(task=task, step="workflow-implement")))
        self.assertIn("open feedback", err)
        self.assertIn("issue-1", err)
        # 不得推进
        self.assertEqual(self.state(task)["main"]["current_step"], "workflow-implement")

    def test_advance_wrong_credential_dies(self):
        task = self._init("t-awc")
        self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
        err = self._die_err(lambda: self.cli.cmd_advance(
            self._ns(task=task, step="workflow-lint")))
        self.assertIn("未过入口门禁", err)
        self.assertIn("workflow-implement", err)

    def test_advance_sets_pending_gate(self):
        task = self._init("t-apg")
        self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
        self.cli.cmd_advance(self._ns(task=task, step="workflow-implement"))
        pg = self.state(task)["main"].get("pending_gate")
        self.assertIsNotNone(pg)
        self.assertEqual(pg["to"], "workflow-lint")
        self.assertEqual(pg["from"], "workflow-implement")
        self.assertEqual(pg["gate"], "auto")
        self.assertIsInstance(pg.get("instruction"), str)
        self.assertIn("机械流转", pg["instruction"])

    def test_advance_loop_still_advances_and_keeps_locked_exit(self):
        """--loop 降级非钉死闸，但仍推进；lite 的 implement→lint 出口为 auto。"""
        task = self._init("t-alp")
        self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
        self.cli.cmd_advance(self._ns(task=task, step="workflow-implement", loop=True))
        self.assertEqual(self.state(task)["main"]["current_step"], "workflow-lint")

    def test_advance_loop_degrades_confirm_exit(self):
        """--loop 必须传入 transition_gate：confirm 出口降为 auto（杀漏传 autonomous 的变异）。"""
        task = self._init("t-ald")
        st = self.state(task)
        for obj in st["pipeline"]:
            if obj["node"] == "workflow-implement":
                obj["exit_gate"] = "confirm"
        state_lib.save_state(self.root, task, st)
        self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
        self.cli.cmd_advance(self._ns(task=task, step="workflow-implement", loop=True))
        pg = self.state(task)["main"]["pending_gate"]
        self.assertEqual(pg["gate"], "auto")
        self.assertIn("机械流转", pg["instruction"])

    def test_advance_confirm_exit_without_loop(self):
        """无 --loop 时 confirm 出口保留，instruction 走一键确认文案。"""
        task = self._init("t-acn")
        st = self.state(task)
        for obj in st["pipeline"]:
            if obj["node"] == "workflow-implement":
                obj["exit_gate"] = "confirm"
        state_lib.save_state(self.root, task, st)
        self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
        self.cli.cmd_advance(self._ns(task=task, step="workflow-implement"))
        pg = self.state(task)["main"]["pending_gate"]
        self.assertEqual(pg["gate"], "confirm")
        self.assertIn("AskUserQuestion", pg["instruction"])

    def test_init_writes_baseline_context_used_once(self):
        """init 落 baseline_context_used（起步基线）；enter/advance 不再重复写——它是会话属性。"""
        from unittest import mock
        fake = {
            "context_used": 30000, "baseline_context_used": 15467,
            "session_id": "sess-b", "model": "m-test", "turns": 3, "token_usage": None,
        }
        with mock.patch.object(self.cli.telemetry_lib, "collect_context_used", return_value=fake):
            task = self._init("t-base")
            st = self.state(task)
            self.assertEqual(st["baseline_context_used"], 15467)
            self.assertEqual(st["init_context_used"], 30000)
            self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
            self.assertNotIn("baseline_context_used", self.state(task)["main"])

    def test_init_omits_baseline_when_uncollectable(self):
        """采不到基线（全零/无 transcript）→ 不塞字段，不拿 0 冒充。"""
        from unittest import mock
        fake = {
            "context_used": 30000, "baseline_context_used": None,
            "session_id": "sess-c", "model": None, "turns": None, "token_usage": None,
        }
        with mock.patch.object(self.cli.telemetry_lib, "collect_context_used", return_value=fake):
            task = self._init("t-nobase")
        self.assertNotIn("baseline_context_used", self.state(task))

    def test_enter_advance_context_snapshot_fields(self):
        """enter/advance 须把 telemetry 水位写进 main → history（杀 entered_*/ended_* 键变异）。"""
        from unittest import mock
        task = self._init("t-ctx")
        fake = {
            "context_used": 0.42,
            "baseline_context_used": 100,
            "session_id": "sess-1",
            "model": "m-test",
            "turns": 7,
        }
        with mock.patch.object(self.cli.telemetry_lib, "collect_context_used", return_value=fake):
            self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
            st = self.state(task)
            self.assertEqual(st["main"]["entered_context_used"], 0.42)
            self.assertEqual(st["main"]["entered_session_id"], "sess-1")
            self.assertEqual(st["main"]["entered_model"], "m-test")
            self.assertEqual(st["main"]["entered_turns"], 7)
            self.cli.cmd_advance(self._ns(task=task, step="workflow-implement"))
        rec = self.state(task)["main"]["history"][-1]
        self.assertEqual(rec["entered_context_used"], 0.42)
        self.assertEqual(rec["entered_session_id"], "sess-1")
        self.assertEqual(rec["entered_model"], "m-test")
        self.assertEqual(rec["entered_turns"], 7)
        self.assertEqual(rec["ended_context_used"], 0.42)
        self.assertEqual(rec["ended_session_id"], "sess-1")
        self.assertEqual(rec["ended_model"], "m-test")
        self.assertEqual(rec["ended_turns"], 7)

    def test_advance_non_ok_outcome_still_records(self):
        task = self._init("t-aok")
        self.cli.cmd_enter(self._ns(task=task, step="workflow-implement"))
        self.cli.cmd_advance(self._ns(task=task, step="workflow-implement", outcome="skip"))
        st = self.state(task)
        # outcome!=ok 不推进 current_step（advance_main 仅 ok 时前进）
        self.assertEqual(st["main"]["current_step"], "workflow-implement")
        self.assertEqual(st["main"]["history"][-1]["outcome"], "skip")
        self.assertIsNone(st["main"].get("entered_step"))

    def test_advance_to_done_clears_pending_gate(self):
        task = self._init("t-adon")
        # 快进到 archive
        for step in ("workflow-implement", "workflow-lint", "workflow-unittest",
                     "workflow-code-review", "workflow-deploy", "workflow-handoff-qa"):
            self.cli.cmd_enter(self._ns(task=task, step=step))
            self.cli.cmd_advance(self._ns(task=task, step=step))
        st = self.state(task)
        self.assertEqual(st["main"]["current_step"], "workflow-archive")
        # 塞一个脏 pending_gate，advance 到 done 必须清掉
        state_lib.set_pending_gate(st, "confirm-locked", "handoff", "archive", "x", "2026-06-25")
        state_lib.save_state(self.root, task, st)
        self.cli.cmd_enter(self._ns(task=task, step="workflow-archive"))
        self.cli.cmd_advance(self._ns(task=task, step="workflow-archive"))
        st2 = self.state(task)
        self.assertEqual(st2["main"]["current_step"], "done")
        self.assertIsNone(st2["main"].get("pending_gate"))


class ClearEnteredTest(unittest.TestCase):
    """clear_entered：回流前必须作废 enter 凭据（直测，不经 subprocess）。

    变异测试曾发现：`pop(key, None)` → `pop(None, None)` 仍能通过仅断言
    reroute_count 的用例——凭据残留会让下一节点 advance 白拿放行。
    """

    def test_clears_all_entered_keys(self):
        st = _mk_state()
        st["main"].update({
            "entered_step": "workflow-lint",
            "entered_at": "2026-06-25 10:00:00",
            "entered_context_used": 42,
            "entered_session_id": "sess",
            "entered_model": "m",
            "entered_turns": 3,
        })
        state_lib.clear_entered(st)
        for key in state_lib.ENTERED_KEYS:
            self.assertNotIn(key, st["main"], key)

    def test_reroute_to_clears_credential(self):
        """reroute_to 必须调用 clear_entered——否则搬走 current_step 后凭据仍在。"""
        st = _mk_state()
        state_lib.add_open_issue(st, "issue-1", "workflow-code-review", "2026-06-25 10:00:00")
        st["main"]["entered_step"] = "workflow-lint"
        st["main"]["entered_at"] = "2026-06-25 10:00:00"
        state_lib.reroute_to(st, "workflow-implement", "2026-06-25", "issue-1")
        self.assertEqual(st["main"]["current_step"], "workflow-implement")
        self.assertNotIn("entered_step", st["main"])
        self.assertNotIn("entered_at", st["main"])


class RerouteCountTest(unittest.TestCase):
    """reroute_to 逃生计数（state 纯函数）。"""

    def _state_with_issue(self):
        st = _mk_state()
        state_lib.add_open_issue(st, "issue-1", "workflow-code-review", "2026-06-25 10:00:00")
        return st

    def test_reroute_count_increments_and_returns(self):
        st = self._state_with_issue()
        for expected in (1, 2, 3):
            self.assertEqual(
                state_lib.reroute_to(st, "workflow-implement", "2026-06-25", "issue-1"), expected
            )
        self.assertEqual(st["feedback"]["open"][0]["reroute_count"], 3)

    def test_add_open_issue_seeds_zero(self):
        st = self._state_with_issue()
        self.assertEqual(st["feedback"]["open"][0]["reroute_count"], 0)

    def test_normalize_backfills_old_issue(self):
        st = _mk_state()
        st["feedback"]["open"].append({"id": "issue-9", "from": "manual"})
        state_lib._normalize(st)
        it = st["feedback"]["open"][0]
        self.assertEqual(it["reroute_count"], 0)
        self.assertIsNone(it["category"])
        self.assertIsNone(it["reroute_target"])
        self.assertIsNone(it["created_at"])
        self.assertIsNone(it["resolved_at"])

    def test_normalize_backfills_old_resolved_issue(self):
        st = _mk_state()
        st["feedback"]["resolved"].append({"id": "issue-8", "from": "manual"})
        state_lib._normalize(st)
        it = st["feedback"]["resolved"][0]
        self.assertIsNone(it["created_at"])
        self.assertIsNone(it["resolved_at"])


class IssueTriageTest(_CliBase):
    """issue-triage（原子写 category/severity/target）+ reroute --to（防前跳/rerouteable）。"""

    def test_triage_with_target(self):
        self.run_cli("init", "i1", "--profile", "lite")
        rid = self.run_cli("issue-add", "i1", "--from", "workflow-code-review", "--phenomenon", "x").stdout.strip()
        r = self.run_cli("issue-triage", "i1", rid, "--category", "code", "--target", "workflow-implement")
        self.assertEqual(r.returncode, 0, r.stderr)
        it = self.state("i1")["feedback"]["open"][0]
        self.assertEqual(it["category"], "code")
        self.assertEqual(it["reroute_target"], "workflow-implement")

    def test_issue_add_stamps_created_at(self):
        # issue-add → open 条目落 created_at（非空），resolved_at 仍空。
        self.run_cli("init", "it1", "--profile", "lite")
        self.run_cli("issue-add", "it1", "--from", "workflow-code-review", "--phenomenon", "x")
        it = self.state("it1")["feedback"]["open"][0]
        self.assertTrue(it["created_at"])
        self.assertIsNone(it["resolved_at"])

    def test_issue_resolve_stamps_resolved_at(self):
        # issue-resolve → 条目进 resolved 且 resolved_at 落上、created_at 保留。
        self.run_cli("init", "it2", "--profile", "lite")
        rid = self.run_cli("issue-add", "it2", "--from", "workflow-code-review", "--phenomenon", "x").stdout.strip()
        self.run_cli("issue-resolve", "it2", rid, "--by", "implement", "--mode", "auto")
        it = self.state("it2")["feedback"]["resolved"][0]
        self.assertTrue(it["created_at"])
        self.assertTrue(it["resolved_at"])

    def test_triage_record_only_stamps_resolved_at(self):
        # 仅记录闭环也要落 resolved_at。
        self.run_cli("init", "it3", "--profile", "lite")
        rid = self.run_cli("issue-add", "it3", "--from", "manual", "--phenomenon", "note").stdout.strip()
        self.run_cli("issue-triage", "it3", rid, "--category", "design")
        it = self.state("it3")["feedback"]["resolved"][0]
        self.assertTrue(it["resolved_at"])

    def test_triage_record_only_autocloses(self):
        # 无 target = 仅记录 → 同操作内闭环（设计二审 D 防死锁）
        self.run_cli("init", "i2", "--profile", "lite")
        rid = self.run_cli("issue-add", "i2", "--from", "manual", "--phenomenon", "note").stdout.strip()
        r = self.run_cli("issue-triage", "i2", rid, "--category", "design")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("已闭环", r.stdout)
        st = self.state("i2")
        self.assertEqual(len(st["feedback"]["open"]), 0)
        self.assertEqual(len(st["feedback"]["resolved"]), 1)

    def test_triage_rejects_rerouteable_false_target(self):
        self.run_cli("init", "i3", "--profile", "lite")
        rid = self.run_cli("issue-add", "i3", "--from", "workflow-code-review", "--phenomenon", "x").stdout.strip()
        r = self.run_cli("issue-triage", "i3", rid, "--category", "code", "--target", "workflow-deploy")
        self.assertEqual(r.returncode, 1)
        self.assertIn("rerouteable:false", r.stderr)

    def test_triage_rejects_forward_jump(self):
        # current=implement，target=handoff-qa 在下游 → 拒绝前跳
        self.run_cli("init", "i4", "--profile", "lite")
        rid = self.run_cli("issue-add", "i4", "--from", "workflow-code-review", "--phenomenon", "x").stdout.strip()
        r = self.run_cli("issue-triage", "i4", rid, "--category", "code", "--target", "workflow-handoff-qa")
        self.assertEqual(r.returncode, 1)
        self.assertIn("下游", r.stderr)

    def test_reroute_to_moves_current_step(self):
        self.run_cli("init", "i5", "--profile", "lite")
        # 先推进到 lint，再报 issue 回退到 implement
        self.step("i5", "workflow-implement")
        rid = self.run_cli("issue-add", "i5", "--from", "workflow-code-review", "--phenomenon", "x").stdout.strip()
        self.run_cli("issue-triage", "i5", rid, "--category", "code", "--target", "workflow-implement")
        r = self.run_cli("reroute", "i5", rid, "--to", "workflow-implement")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.state("i5")["main"]["current_step"], "workflow-implement")

    def test_reroute_to_rejects_unknown_node(self):
        self.run_cli("init", "i6", "--profile", "lite")
        rid = self.run_cli("issue-add", "i6", "--from", "workflow-code-review", "--phenomenon", "x").stdout.strip()
        self.run_cli("issue-triage", "i6", rid, "--category", "code", "--target", "workflow-implement")
        r = self.run_cli("reroute", "i6", rid, "--to", "no-such-node")
        self.assertEqual(r.returncode, 1)
        self.assertIn("不是本任务节点", r.stderr)

    def test_issue_add_rejects_non_can_report_node(self):
        # lint 无 can_report → issue-add 拒绝
        self.run_cli("init", "i7", "--profile", "lite")
        r = self.run_cli("issue-add", "i7", "--from", "workflow-lint", "--phenomenon", "x")
        self.assertEqual(r.returncode, 1)
        self.assertIn("can_report", r.stderr)

    def test_issue_add_severity_high_lands_and_warns(self):
        # --severity high：落 state severity + 打印「当场停」提示
        self.run_cli("init", "i8", "--profile", "lite")
        r = self.run_cli("issue-add", "i8", "--from", "workflow-code-review",
                         "--phenomenon", "资金写错", "--severity", "high")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("当场", r.stdout)
        it = self.state("i8")["feedback"]["open"][0]
        self.assertEqual(it["severity"], "high")

    def test_issue_add_default_severity_normal(self):
        # 缺省落真值 normal（记一笔继续）——留 None 会让字段退化成 high/null，服务端拿不到分布
        self.run_cli("init", "i9", "--profile", "lite")
        rid = self.run_cli("issue-add", "i9", "--from", "workflow-code-review",
                           "--phenomenon", "x").stdout.strip()
        it = self.state("i9")["feedback"]["open"][0]
        self.assertEqual(it["severity"], "normal")
        self.assertIn("severity: normal", self._issue_md("i9", rid))

    def test_issue_add_does_not_move_current_step(self):
        # 记账不中断当前节点：issue-add 后 current_step 不动
        self.run_cli("init", "i10", "--profile", "lite")
        before = self.state("i10")["main"]["current_step"]
        self.run_cli("issue-add", "i10", "--from", "workflow-code-review", "--phenomenon", "x")
        self.assertEqual(self.state("i10")["main"]["current_step"], before)

    def test_advance_blocked_by_open_issue_is_checkpoint(self):
        # 收口点：记账不挡当前节点，但干完想 advance 被 open issue 挡住
        self.run_cli("init", "i13", "--profile", "lite")
        # 先正常进节点（此时无 open issue，门禁放行）——还原「干活中途记账、干完想走」的真实序列，
        # 免得这条用例实际是被入口门禁挡下、却假装验证了 feedback 锁。
        self.run_cli("enter", "i13", "--step", "workflow-implement")
        self.run_cli("issue-add", "i13", "--from", "workflow-code-review", "--phenomenon", "x")
        r = self.run_cli("advance", "i13", "--step", "workflow-implement")
        self.assertEqual(r.returncode, 1)
        self.assertIn("open feedback", r.stderr)

    def test_resolve_with_category_closes_and_records(self):
        # 默认原地修路径：issue-resolve --category 一步闭环 + 记归属
        self.run_cli("init", "i11", "--profile", "lite")
        rid = self.run_cli("issue-add", "i11", "--from", "workflow-code-review",
                           "--phenomenon", "x").stdout.strip()
        r = self.run_cli("issue-resolve", "i11", rid, "--by", "workflow-code-review 原地修",
                         "--mode", "asked", "--category", "code")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("category=code", r.stdout)
        st = self.state("i11")
        self.assertEqual(len(st["feedback"]["open"]), 0)
        self.assertEqual(st["feedback"]["resolved"][0]["category"], "code")

    def test_resolve_without_category_closes_only(self):
        # 不给 category：只闭环、不记归属（事后可补）
        self.run_cli("init", "i12", "--profile", "lite")
        rid = self.run_cli("issue-add", "i12", "--from", "workflow-code-review",
                           "--phenomenon", "x").stdout.strip()
        r = self.run_cli("issue-resolve", "i12", rid, "--by", "workflow-code-review 原地修",
                         "--mode", "auto")
        self.assertEqual(r.returncode, 0, r.stderr)
        st = self.state("i12")
        self.assertEqual(len(st["feedback"]["resolved"]), 1)
        self.assertIsNone(st["feedback"]["resolved"][0]["category"])

    def _issue_md(self, task, issue_id):
        return (self.root / "delivery" / task / "feedback" / f"{issue_id}.md").read_text()

    def _index_md(self, task):
        return (self.root / "delivery" / task / "feedback" / "INDEX.md").read_text()

    def test_resolve_mode_lands_in_state_and_frontmatter(self):
        """resolved_mode 必须两处都写：frontmatter 给人看，state 条目给服务端看。

        只写 frontmatter 是无效的——上报发的是全量 state 原文，issue.md 从不出用户仓库
        （resolved_by 至今就只在 frontmatter，服务端根本看不到）。这条用例就是钉住这一点。
        """
        self.run_cli("init", "m1", "--profile", "lite")
        rid = self.run_cli("issue-add", "m1", "--from", "workflow-code-review",
                           "--phenomenon", "x").stdout.strip()
        self.assertIn("resolved_mode: null", self._issue_md("m1", rid))  # 建档时先占位
        r = self.run_cli("issue-resolve", "m1", rid, "--by", "implement", "--mode", "asked")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.state("m1")["feedback"]["resolved"][0]["resolved_mode"], "asked")
        self.assertIn("resolved_mode: asked", self._issue_md("m1", rid))
        self.assertIn("asked", self._index_md("m1"))
        self.assertIn("| 解决方式 |", self._index_md("m1"))

    def test_resolve_requires_mode(self):
        """--mode 必填：设成可选就会重蹈 severity 的覆辙（默认路径没人填 → 常年 null）。"""
        self.run_cli("init", "m2", "--profile", "lite")
        rid = self.run_cli("issue-add", "m2", "--from", "workflow-code-review",
                           "--phenomenon", "x").stdout.strip()
        r = self.run_cli("issue-resolve", "m2", rid, "--by", "implement")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("--mode", r.stderr)
        # 拒绝后 issue 必须还躺在 open 里，不能半闭环
        self.assertEqual(len(self.state("m2")["feedback"]["open"]), 1)

    def test_resolve_rejects_unknown_mode(self):
        self.run_cli("init", "m3", "--profile", "lite")
        rid = self.run_cli("issue-add", "m3", "--from", "workflow-code-review",
                           "--phenomenon", "x").stdout.strip()
        r = self.run_cli("issue-resolve", "m3", rid, "--by", "implement", "--mode", "half-auto")
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(len(self.state("m3")["feedback"]["open"]), 1)

    def test_triage_record_only_leaves_mode_null(self):
        """「仅记录」闭环没走解决路径 → resolved_mode 留 null，不进自治率分母。

        这里若被改成硬填 auto，自治率的分子就会混进「压根没解决过」的条目。
        """
        self.run_cli("init", "m4", "--profile", "lite")
        rid = self.run_cli("issue-add", "m4", "--from", "manual", "--phenomenon", "note").stdout.strip()
        self.run_cli("issue-triage", "m4", rid, "--category", "design")
        it = self.state("m4")["feedback"]["resolved"][0]
        self.assertEqual(it["resolved_at"] is not None, True)  # 确实闭环了
        self.assertIsNone(it["resolved_mode"])                 # 但不表态
        self.assertIn("resolved_mode: null", self._issue_md("m4", rid))


class AdvanceLoopTest(_CliBase):
    """advance --loop 自治降级 + reroute 升级提示（subprocess）。"""

    def _walk(self, task, steps, loop=False):
        for s in steps:
            extra = ("--loop",) if loop else ()
            self.step(task, s, *extra)

    def test_loop_does_not_downgrade_handoff_locked(self):
        self.run_cli("init", "a5", "--profile", "lite")
        self._walk("a5", ["workflow-implement", "workflow-lint", "workflow-unittest",
                          "workflow-code-review", "workflow-deploy"])
        r = self.step("a5", "workflow-handoff-qa", "--loop")
        self.assertIn("闸门：confirm-locked", r.stdout)

    def test_no_loop_keeps_handoff_locked(self):
        self.run_cli("init", "a6", "--profile", "lite")
        self._walk("a6", ["workflow-implement", "workflow-lint", "workflow-unittest",
                          "workflow-code-review", "workflow-deploy"])
        r = self.step("a6", "workflow-handoff-qa")
        self.assertIn("闸门：confirm-locked", r.stdout)

    def test_loop_preserves_deploy_entry_locked(self):
        self.run_cli("init", "a7", "--profile", "lite")
        self._walk("a7", ["workflow-implement", "workflow-lint", "workflow-unittest"], loop=True)
        r = self.step("a7", "workflow-code-review", "--loop")
        self.assertIn("confirm-locked", r.stdout)

    def test_reroute_escalation_warning_at_threshold(self):
        self.run_cli("init", "a3", "--profile", "lite")
        self.step("a3", "workflow-implement")
        rid = self.run_cli("issue-add", "a3", "--from", "workflow-code-review", "--phenomenon", "x").stdout.strip()
        self.run_cli("issue-triage", "a3", rid, "--category", "code", "--target", "workflow-implement")
        last = None
        for _ in range(nodes.REROUTE_ESCALATE_THRESHOLD):
            last = self.run_cli("reroute", "a3", rid, "--to", "workflow-implement")
        self.assertIn("升级人工", last.stdout)
        self.assertIn(str(nodes.REROUTE_ESCALATE_THRESHOLD), last.stdout)

    def test_reroute_no_warning_below_threshold(self):
        self.run_cli("init", "a4", "--profile", "lite")
        self.step("a4", "workflow-implement")
        rid = self.run_cli("issue-add", "a4", "--from", "workflow-code-review", "--phenomenon", "x").stdout.strip()
        self.run_cli("issue-triage", "a4", rid, "--category", "code", "--target", "workflow-implement")
        r1 = self.run_cli("reroute", "a4", rid, "--to", "workflow-implement")
        self.assertNotIn("升级人工", r1.stdout)


class RunLoopHintTest(_CliBase):
    """run-loop 提示：advance/init 跨入无人区入口是强制确认（🛑 + AskUserQuestion）；
    next 查询区内节点是软提示（💡）；自驱（--loop）/区外都不提示。"""

    def _walk(self, task, steps, loop=False):
        for s in steps:
            extra = ("--loop",) if loop else ()
            self.step(task, s, *extra)

    def test_advance_into_implement_confirms(self):
        # no-autotest: propose(非autopilot)→implement(autopilot) 是无人区入口
        self.run_cli("init", "h1", "--profile", "no-autotest")
        r = self.step("h1", "workflow-propose")
        self.assertEqual(self.state("h1")["main"]["current_step"], "workflow-implement")
        self.assertIn("🛑", r.stdout)
        self.assertIn("AskUserQuestion", r.stdout)
        self.assertIn("run-loop h1", r.stdout)

    def test_advance_into_run_autotest_confirms(self):
        self.run_cli("init", "h2", "--profile", "full", "--env", "test", "--scope", "http", "--http-domain", "a.com")
        self._walk("h2", ["workflow-propose", "workflow-test-plan", "workflow-implement",
                          "workflow-lint", "workflow-unittest", "workflow-code-review",
                          "workflow-test-cases"])
        self.assertEqual(self.state("h2")["main"]["current_step"], "workflow-deploy")
        r = self.step("h2", "workflow-deploy")
        self.assertIn("🛑", r.stdout)
        self.assertIn("run-loop h2", r.stdout)

    def test_advance_loop_suppresses_confirm(self):
        self.run_cli("init", "h3", "--profile", "no-autotest")
        r = self.step("h3", "workflow-propose", "--loop")
        self.assertEqual(self.state("h3")["main"]["current_step"], "workflow-implement")
        self.assertNotIn("🛑", r.stdout)
        self.assertNotIn("💡", r.stdout)

    def test_advance_into_handoff_no_confirm(self):
        self.run_cli("init", "h4", "--profile", "full", "--env", "test", "--scope", "http", "--http-domain", "a.com")
        self._walk("h4", ["workflow-propose", "workflow-test-plan", "workflow-implement",
                          "workflow-lint", "workflow-unittest", "workflow-code-review",
                          "workflow-test-cases", "workflow-deploy", "workflow-run-autotest"])
        r = self.step("h4", "workflow-run-autotest")
        self.assertNotIn("🛑", r.stdout)
        self.assertNotIn("💡", r.stdout)

    def test_init_seed_in_zone_confirms(self):
        # lite: seed=implement（首节点、autopilot）是无人区入口——init 当场就要强制确认，不等 advance。
        r = self.run_cli("init", "h5", "--profile", "lite")
        self.assertEqual(self.state("h5")["main"]["current_step"], "workflow-implement")
        self.assertIn("🛑", r.stdout)
        self.assertIn("run-loop h5", r.stdout)

    def test_next_in_zone_soft_hints(self):
        # next 查询时人已在区内（非跨入时刻）：软提示 💡，不是强制确认。
        self.run_cli("init", "h5b", "--profile", "lite")
        r = self.run_cli("next", "h5b")
        self.assertEqual(self.state("h5b")["main"]["current_step"], "workflow-implement")
        self.assertIn("💡", r.stdout)
        self.assertIn("run-loop h5b", r.stdout)

    def test_next_outside_zone_no_hint(self):
        self.run_cli("init", "h6", "--env", "test", "--scope", "http", "--http-domain", "a.com")  # seed=propose（非autopilot）区外
        r = self.run_cli("next", "h6")
        self.assertNotIn("💡", r.stdout)
        self.assertNotIn("🛑", r.stdout)


class SandboxCommitReminderTest(unittest.TestCase):
    """沙箱下「即将进入归档」的提交提醒（纯函数直调）。

    只在 nxt==归档节点 且 OPENCLI_SANDBOX_MODE truthy 时出；两条件缺一即空串。
    """

    def setUp(self):
        self._saved = {k: os.environ.get(k) for k in
                       ("OPENCLI_SANDBOX_MODE", "WORKFLOW_SKIP_TELEMETRY")}

    def tearDown(self):
        for k, v in self._saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v

    def _pipe(self, profile=FULL_PROFILE):
        return nodes.resolve_pipeline(profile)

    def test_sandbox_truthy_values_remind(self):
        for v in ("1", "true", "on", "yes", "TRUE", " Yes "):
            os.environ["OPENCLI_SANDBOX_MODE"] = v
            out = nodes.sandbox_commit_reminder("workflow-archive", self._pipe())
            self.assertIn("commit + push", out, f"{v!r} 应提醒")
            self.assertIn("AskUserQuestion", out)

    def test_local_values_silent(self):
        for v in ("0", "false", "off", "no", "", "bogus"):
            os.environ["OPENCLI_SANDBOX_MODE"] = v
            self.assertEqual(nodes.sandbox_commit_reminder("workflow-archive", self._pipe()), "",
                             f"{v!r} 应静默")
        os.environ.pop("OPENCLI_SANDBOX_MODE", None)
        self.assertEqual(nodes.sandbox_commit_reminder("workflow-archive", self._pipe()), "")

    def test_non_archive_node_silent(self):
        os.environ["OPENCLI_SANDBOX_MODE"] = "1"
        for n in ("workflow-implement", "workflow-handoff-qa", "workflow-deploy", nodes.DONE):
            self.assertEqual(nodes.sandbox_commit_reminder(n, self._pipe()), "", f"{n} 不该提醒")

    def test_archive_not_in_pipeline_silent(self):
        # 归档不在本档位序列里（用户裁掉了）→ 不提醒，不因名字命中就硬打
        os.environ["OPENCLI_SANDBOX_MODE"] = "1"
        pipe = nodes.resolve_pipeline([{"node": "workflow-implement", "exit_gate": "auto"}])
        self.assertEqual(nodes.sandbox_commit_reminder("workflow-archive", pipe), "")

    def test_skip_telemetry_does_not_suppress(self):
        # 刻意不复用 detect_run_env：关掉上报不该顺带吃掉这条提醒
        os.environ["OPENCLI_SANDBOX_MODE"] = "1"
        os.environ["WORKFLOW_SKIP_TELEMETRY"] = "1"
        self.assertIn("commit + push", nodes.sandbox_commit_reminder("workflow-archive", self._pipe()))


class SandboxCommitReminderCliTest(_CliBase):
    """提醒的实际落点：归档**前一个**节点 advance 完那一刻打印（不是进归档时）。"""

    def _walk_to_pre_archive(self, task):
        """推进到 handoff-qa（lite 档里归档的前驱），current_step 停在它上面。"""
        self.run_cli("init", task, "--profile", "lite")
        for s in ("workflow-implement", "workflow-lint", "workflow-unittest",
                  "workflow-code-review", "workflow-deploy", "workflow-run-autotest"):
            self.step(task, s)

    def _env(self, **extra):
        env = _cli_env()
        env.update(extra)
        return env

    def _advance(self, task, step, **extra):
        """带自定义环境变量的 enter+advance（沙箱开关经 env 注入，故不能复用 self.step）。"""
        subprocess.run(
            [sys.executable, str(CLI), "--target", str(self.root), "enter", task, "--step", step],
            capture_output=True, text=True, env=self._env(**extra),
        )
        return subprocess.run(
            [sys.executable, str(CLI), "--target", str(self.root), "advance", task, "--step", step],
            capture_output=True, text=True, env=self._env(**extra),
        )

    def test_advance_into_archive_reminds_in_sandbox(self):
        self._walk_to_pre_archive("s1")
        self.assertEqual(self.state("s1")["main"]["current_step"], "workflow-handoff-qa")
        r = self._advance("s1", "workflow-handoff-qa", OPENCLI_SANDBOX_MODE="1")
        self.assertEqual(self.state("s1")["main"]["current_step"], "workflow-archive")
        self.assertIn("commit + push", r.stdout)
        self.assertIn("AskUserQuestion", r.stdout)

    def test_advance_into_archive_silent_when_local(self):
        self._walk_to_pre_archive("s2")
        r = self._advance("s2", "workflow-handoff-qa", OPENCLI_SANDBOX_MODE="0")
        self.assertEqual(self.state("s2")["main"]["current_step"], "workflow-archive")
        self.assertNotIn("commit + push", r.stdout)

    def test_earlier_advance_silent_in_sandbox(self):
        # 沙箱下也只在归档前一步提醒，中途各节点不打扰
        self.run_cli("init", "s3", "--profile", "lite")
        r = self._advance("s3", "workflow-implement", OPENCLI_SANDBOX_MODE="1")
        self.assertEqual(self.state("s3")["main"]["current_step"], "workflow-lint")
        self.assertNotIn("commit + push", r.stdout)

    def test_advance_out_of_archive_silent(self):
        # 归档自己 advance → done，此时提交已由归档节点兜底，不再重复提醒
        self._walk_to_pre_archive("s4")
        self._advance("s4", "workflow-handoff-qa", OPENCLI_SANDBOX_MODE="1")
        r = self._advance("s4", "workflow-archive", OPENCLI_SANDBOX_MODE="1")
        self.assertEqual(self.state("s4")["main"]["current_step"], nodes.DONE)
        self.assertNotIn("commit + push", r.stdout)

    def _next(self, task, **extra):
        return subprocess.run(
            [sys.executable, str(CLI), "--target", str(self.root), "next", task],
            capture_output=True, text=True, env=self._env(**extra),
        )

    def test_next_at_pending_archive_gate_reminds(self):
        # 换窗口/被 compact 后靠 next 找回进度：闸门待兑现（还没 enter 归档）时补提醒
        self._walk_to_pre_archive("s5")
        self._advance("s5", "workflow-handoff-qa", OPENCLI_SANDBOX_MODE="1")
        r = self._next("s5", OPENCLI_SANDBOX_MODE="1")
        self.assertIn("workflow-handoff-qa → workflow-archive", r.stdout)  # 闸门仍待兑现
        self.assertIn("commit + push", r.stdout)

    def test_next_at_pending_archive_gate_silent_when_local(self):
        self._walk_to_pre_archive("s6")
        self._advance("s6", "workflow-handoff-qa", OPENCLI_SANDBOX_MODE="0")
        r = self._next("s6", OPENCLI_SANDBOX_MODE="0")
        self.assertNotIn("commit + push", r.stdout)

    def test_next_inside_archive_silent(self):
        # 已 enter 归档（闸门被清）→ 人已在归档里，提交交给归档节点自己兜底，next 不再唠
        self._walk_to_pre_archive("s7")
        self._advance("s7", "workflow-handoff-qa", OPENCLI_SANDBOX_MODE="1")
        subprocess.run(
            [sys.executable, str(CLI), "--target", str(self.root), "enter", "s7",
             "--step", "workflow-archive"],
            capture_output=True, text=True, env=self._env(OPENCLI_SANDBOX_MODE="1"),
        )
        r = self._next("s7", OPENCLI_SANDBOX_MODE="1")
        self.assertNotIn("commit + push", r.stdout)

    def test_next_mid_pipeline_silent_in_sandbox(self):
        # 中途节点的待兑现闸门（非指向归档）→ 不提醒
        self.run_cli("init", "s8", "--profile", "lite")
        self._advance("s8", "workflow-implement", OPENCLI_SANDBOX_MODE="1")
        r = self._next("s8", OPENCLI_SANDBOX_MODE="1")
        self.assertNotIn("commit + push", r.stdout)


class EnterGateTest(_CliBase):
    """advance 的入口门禁凭据（main.entered_step）：跳过 enter 直接 advance 必须硬阻断。

    这道闸是为了拦住实际发生过的漏检：AI 直接 advance 归档节点，绕过 entry_gate=confirm-locked，
    还把上一转场的 pending_gate 留成永久脏状态。
    """

    def test_advance_without_enter_is_rejected(self):
        self.run_cli("init", "g1", "--profile", "lite")
        r = self.run_cli("advance", "g1", "--step", "workflow-implement")
        self.assertEqual(r.returncode, 1)
        self.assertIn("未过入口门禁", r.stderr)
        # 状态不动：被拒的 advance 不得推进 current_step
        self.assertEqual(self.state("g1")["main"]["current_step"], "workflow-implement")

    def test_enter_then_advance_passes(self):
        self.run_cli("init", "g2", "--profile", "lite")
        self.run_cli("enter", "g2", "--step", "workflow-implement")
        r = self.run_cli("advance", "g2", "--step", "workflow-implement")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.state("g2")["main"]["current_step"], "workflow-lint")

    def test_credential_is_consumed_once(self):
        """凭据一次性：advance 消费掉后，同节点再 advance（或下一节点不 enter）都得被拒。"""
        self.run_cli("init", "g3", "--profile", "lite")
        self.step("g3", "workflow-implement")
        self.assertIsNone(self.state("g3")["main"].get("entered_step"))
        r = self.run_cli("advance", "g3", "--step", "workflow-lint")
        self.assertEqual(r.returncode, 1)
        self.assertIn("未过入口门禁", r.stderr)

    def test_credential_is_node_specific(self):
        """进了 X 不能 advance Y——凭据带节点名，顺带覆盖「串节点」。"""
        self.run_cli("init", "g4", "--profile", "lite")
        self.run_cli("enter", "g4", "--step", "workflow-implement")
        r = self.run_cli("advance", "g4", "--step", "workflow-lint")
        self.assertEqual(r.returncode, 1)
        self.assertIn("workflow-implement", r.stderr)  # 提示里点名当前暂存的是谁

    def test_reroute_invalidates_credential(self):
        """回流搬走 current_step → 被弃节点的凭据必须作废，否则新节点能白拿一次放行。"""
        self.run_cli("init", "g5", "--profile", "lite")
        self.step("g5", "workflow-implement")
        self.run_cli("enter", "g5", "--step", "workflow-lint")  # 进了 lint，拿到凭据
        rid = self.run_cli("issue-add", "g5", "--from", "workflow-code-review",
                           "--phenomenon", "x").stdout.strip()
        self.run_cli("issue-triage", "g5", rid, "--category", "code",
                     "--target", "workflow-implement")
        self.run_cli("reroute", "g5", rid, "--to", "workflow-implement")
        self.assertIsNone(self.state("g5")["main"].get("entered_step"))
        self.run_cli("issue-resolve", "g5", rid, "--by", "fix", "--mode", "auto")
        r = self.run_cli("advance", "g5", "--step", "workflow-implement")
        self.assertEqual(r.returncode, 1)
        self.assertIn("未过入口门禁", r.stderr)

    def test_enter_prints_open_worklet_hint(self):
        """enter 放行必打「开工」指令行——把「读 NODE.md / 跑 skill」焊进机器必经的入口输出，
        拦住实际发生过的漏检：AI 拿 enter 上下文包当交底、跳过读 NODE.md 直接干活。
        集成 root 未铺 .workflow/node/ → 探否走 skill 形态，验证 /节点名 出现在开工行。"""
        self.run_cli("init", "gw", "--profile", "lite")
        r = self.run_cli("enter", "gw", "--step", "workflow-implement")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("▶ 开工", r.stdout)
        self.assertIn("/workflow-implement", r.stdout)

    def test_lost_credential_recovers_via_enter(self):
        """凭据丢了（换窗口/compact 后接着干，把 enter 落下）：被拒一次，补跑 enter 即通行。

        不测「升级前在途的老任务」——那个状态到不了：lb init 的在途任务闸门（init.ts::findActiveTask
        / upgrade.py::has_active_task）硬阻断刷模板、无 --force/--yes 旁路，任务只能在旧模板下跑完归档。
        """
        self.run_cli("init", "g6", "--profile", "lite")
        p = self.root / "delivery" / "g6" / "state.json"
        st = json.loads(p.read_text())
        st["main"].pop("entered_step", None)
        p.write_text(json.dumps(st))
        self.assertEqual(self.run_cli("advance", "g6", "--step", "workflow-implement").returncode, 1)
        self.run_cli("enter", "g6", "--step", "workflow-implement")
        self.assertEqual(self.run_cli("advance", "g6", "--step", "workflow-implement").returncode, 0)


class DonePendingGateTest(_CliBase):
    """终点兜底：advance 到 done 时清 pending_gate（done 之后没有 enter 会来兑现它）。"""

    def test_pending_gate_cleared_at_done(self):
        self.run_cli("init", "z1", "--profile", "lite")
        for s in ("workflow-implement", "workflow-lint", "workflow-unittest",
                  "workflow-code-review", "workflow-deploy", "workflow-run-autotest",
                  "workflow-handoff-qa"):
            self.step("z1", s)
        # 末节点前：书签在（handoff-qa → archive 的 confirm-locked 待兑现）
        self.assertIsNotNone(self.state("z1")["main"]["pending_gate"])
        self.step("z1", "workflow-archive")
        st = self.state("z1")
        self.assertEqual(st["main"]["current_step"], "done")
        self.assertIsNone(st["main"]["pending_gate"])

    def test_status_at_done_has_no_pending_line(self):
        self.run_cli("init", "z2", "--profile", "lite")
        for s in ("workflow-implement", "workflow-lint", "workflow-unittest",
                  "workflow-code-review", "workflow-deploy", "workflow-run-autotest",
                  "workflow-handoff-qa", "workflow-archive"):
            self.step("z2", s)
        r = self.run_cli("status", "z2")
        self.assertNotIn("待兑现", r.stdout)


class ClassifyLoopTest(unittest.TestCase):
    """run-loop 终态分类（纯函数）。"""

    def _state(self, profile=FULL_PROFILE):
        return _mk_state(profile)

    def test_done(self):
        st = self._state()
        st["main"]["current_step"] = nodes.DONE
        self.assertEqual(nodes.classify_loop(st)[0], nodes.LOOP_DONE)

    def test_pinned_at_propose(self):
        st = self._state()
        self.assertEqual(st["main"]["current_step"], "workflow-propose")
        self.assertEqual(nodes.classify_loop(st)[0], nodes.LOOP_PINNED)

    def test_pinned_at_test_plan(self):
        st = self._state(NO_AUTOTEST_PROFILE)  # 无 test-plan；用 propose 验证 pinned
        st["main"]["current_step"] = "workflow-propose"
        self.assertEqual(nodes.classify_loop(st)[0], nodes.LOOP_PINNED)

    def test_pinned_at_deploy(self):
        st = self._state()
        st["main"]["current_step"] = "workflow-deploy"
        status, detail = nodes.classify_loop(st)
        self.assertEqual(status, nodes.LOOP_PINNED)
        self.assertEqual(detail["step"], "workflow-deploy")

    def test_continue_at_implement(self):
        st = self._state()
        st["main"]["current_step"] = "workflow-implement"
        status, detail = nodes.classify_loop(st)
        self.assertEqual(status, nodes.LOOP_CONTINUE)
        self.assertEqual(detail["step"], "workflow-implement")
        self.assertEqual(detail["next"], "workflow-lint")

    def test_continue_at_test_cases_not_pinned(self):
        st = self._state()
        st["main"]["current_step"] = "workflow-test-cases"
        self.assertEqual(nodes.classify_loop(st)[0], nodes.LOOP_CONTINUE)

    def test_continue_at_run_autotest(self):
        st = self._state()
        st["main"]["current_step"] = "workflow-run-autotest"
        status, detail = nodes.classify_loop(st)
        self.assertEqual(status, nodes.LOOP_CONTINUE)
        self.assertEqual(detail["step"], "workflow-run-autotest")

    def test_pinned_at_handoff_and_archive(self):
        for step in ("workflow-handoff-qa", "workflow-archive"):
            st = self._state()
            st["main"]["current_step"] = step
            self.assertEqual(nodes.classify_loop(st)[0], nodes.LOOP_PINNED, step)

    def test_triage_when_open_issue(self):
        st = self._state()
        st["main"]["current_step"] = "workflow-implement"
        state_lib.add_open_issue(st, "issue-1", "workflow-code-review", "2026-06-25 10:00:00")
        self.assertEqual(nodes.classify_loop(st)[0], nodes.LOOP_TRIAGE)

    def test_high_issue_beats_triage(self):
        st = self._state()
        st["main"]["current_step"] = "workflow-implement"
        state_lib.add_open_issue(st, "issue-1", "workflow-code-review", "2026-06-25 10:00:00")
        st["feedback"]["open"][0]["severity"] = "high"
        self.assertEqual(nodes.classify_loop(st)[0], nodes.LOOP_HIGH_ISSUE)

    def test_escalated_beats_high(self):
        st = self._state()
        st["main"]["current_step"] = "workflow-implement"
        state_lib.add_open_issue(st, "issue-1", "workflow-code-review", "2026-06-25 10:00:00")
        it = st["feedback"]["open"][0]
        it["severity"] = "high"
        it["reroute_count"] = nodes.REROUTE_ESCALATE_THRESHOLD
        self.assertEqual(nodes.classify_loop(st)[0], nodes.LOOP_ESCALATED)

    def test_issue_class_beats_pinned(self):
        st = self._state()  # current_step=propose（非autopilot）
        state_lib.add_open_issue(st, "issue-1", "workflow-code-review", "2026-06-25 10:00:00")
        self.assertEqual(nodes.classify_loop(st)[0], nodes.LOOP_TRIAGE)


class TriageInStateTest(unittest.TestCase):
    """category / severity / reroute_target 入 state。"""

    def test_triage_persists_fields(self):
        st = _mk_state()
        state_lib.add_open_issue(st, "issue-1", "workflow-code-review", "2026-06-25 10:00:00")
        state_lib.set_issue_triage(st, "issue-1", "code", "high", "workflow-implement")
        it = st["feedback"]["open"][0]
        self.assertEqual(it["category"], "code")
        self.assertEqual(it["severity"], "high")
        self.assertEqual(it["reroute_target"], "workflow-implement")

    def test_triage_record_only_no_target(self):
        st = _mk_state()
        state_lib.add_open_issue(st, "issue-1", "manual", "2026-06-25 10:00:00")
        state_lib.set_issue_triage(st, "issue-1", "design")
        it = st["feedback"]["open"][0]
        self.assertEqual(it["category"], "design")
        self.assertIsNone(it["reroute_target"])

    def test_add_open_issue_seeds_none(self):
        st = _mk_state()
        state_lib.add_open_issue(st, "issue-1", "manual", "2026-06-25 10:00:00")
        it = st["feedback"]["open"][0]
        self.assertIsNone(it["severity"])
        self.assertIsNone(it["category"])
        self.assertIsNone(it["reroute_target"])


class RunLoopCliTest(_CliBase):
    """run-loop 子命令端到端（subprocess）。"""

    def advance_all(self, task, steps):
        for s in steps:
            self.step(task, s)

    def test_continue_block(self):
        self.run_cli("init", "r1", "--profile", "lite")  # seed=implement(非停人)
        r = self.run_cli("run-loop", "r1")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("驱动 workflow-implement", r.stdout)
        self.assertIn("--loop", r.stdout)
        self.assertIn("run-loop r1", r.stdout)

    def test_pinned_at_deploy(self):
        self.run_cli("init", "r2", "--profile", "lite")
        self.advance_all("r2", ["workflow-implement", "workflow-lint",
                                "workflow-unittest", "workflow-code-review"])
        self.assertEqual(self.state("r2")["main"]["current_step"], "workflow-deploy")
        r = self.run_cli("run-loop", "r2")
        self.assertIn("终止", r.stdout)
        self.assertIn("停人节点", r.stdout)
        self.assertIn("无人区运行报告", r.stdout)

    def test_done_report(self):
        self.run_cli("init", "r3", "--profile", "lite")
        self.advance_all("r3", ["workflow-implement", "workflow-lint", "workflow-unittest",
                                "workflow-code-review", "workflow-deploy",
                                "workflow-handoff-qa", "workflow-archive"])
        r = self.run_cli("run-loop", "r3")
        self.assertIn("done", r.stdout)
        self.assertIn("跑过节点：7 个", r.stdout)

    def test_triage_then_high_stop(self):
        self.run_cli("init", "r4", "--profile", "lite")
        rid = self.run_cli("issue-add", "r4", "--from", "workflow-code-review",
                           "--phenomenon", "x").stdout.strip()
        r = self.run_cli("run-loop", "r4")
        self.assertIn("分诊", r.stdout)
        self.run_cli("issue-triage", "r4", rid, "--category", "code",
                     "--severity", "high", "--target", "workflow-implement")
        r2 = self.run_cli("run-loop", "r4")
        self.assertIn("high", r2.stdout)
        self.assertIn("终止", r2.stdout)


class PipelineValidationTest(unittest.TestCase):
    """validate_pipeline / validate_profiles 引用完整性校验（loud fail）。"""

    def test_empty_pipeline_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_pipeline([])

    def test_valid_ok(self):
        nodes.validate_pipeline(FULL_PROFILE)

    def test_duplicate_node_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_pipeline([{"node": "a"}, {"node": "a"}])

    def test_not_kebab_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_pipeline([{"node": "Bad_Name"}])

    def test_bad_gate_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_pipeline([{"node": "a", "exit_gate": "weird"}])

    def test_bad_execution_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_pipeline([{"node": "a", "execution": "weird"}])

    def test_reroute_options_unknown_node_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_pipeline([{"node": "a", "feedback": {"reroute_options": ["zzz"]}}])

    def test_reroute_options_to_non_rerouteable_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_pipeline([
                {"node": "a", "feedback": {"reroute_options": ["b"]}},
                {"node": "b", "feedback": {"rerouteable": False}},
            ])

    def test_external_window_default_ok(self):
        # external-window 缺省（不写 autopilot）→ 合法，走默认 false（停人）
        nodes.validate_pipeline([{"node": "a", "execution": "external-window"}])

    def test_external_window_autopilot_true_fails(self):
        # external-window 标 autopilot:true → 报错（独立窗口不可自驱）
        with self.assertRaises(ValueError):
            nodes.validate_pipeline([{"node": "a", "execution": "external-window", "autopilot": True}])

    def test_external_window_autopilot_false_ok(self):
        nodes.validate_pipeline([{"node": "a", "execution": "external-window", "autopilot": False}])

    def test_unknown_field_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_pipeline([{"node": "a", "bogus": 1}])

    def test_underscore_field_ignored(self):
        nodes.validate_pipeline([{"node": "a", "_note": "ok"}])

    def test_can_report_non_bool_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_pipeline([{"node": "a", "feedback": {"can_report": "yes"}}])

    # --- validate_profiles ---
    def test_profiles_empty_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_profiles({}, "full")

    def test_default_profile_missing_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_profiles({"full": _prof(FULL_PROFILE)}, "nope")

    def test_profiles_ok(self):
        nodes.validate_profiles({"full": _prof(FULL_PROFILE), "lite": _prof(LITE_PROFILE)}, "full")

    def test_profiles_underscore_keys_ignored(self):
        nodes.validate_profiles({"_doc": "x", "full": _prof(FULL_PROFILE)}, "full")

    def test_legacy_array_profile_loud_fails(self):
        # profile 值是旧的节点数组结构 → loud fail 提示升级为对象
        with self.assertRaises(ValueError):
            nodes.validate_profiles({"full": FULL_PROFILE}, "full")

    def test_profile_missing_nodes_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_profiles({"full": {"desc": "x", "roles": {}}}, "full")

    def test_profile_bad_roles_type_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_profiles({"full": {"nodes": LITE_PROFILE, "roles": ["not-a-dict"]}}, "full")

    def test_profile_bad_start_node_type_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_profiles({"full": {"nodes": LITE_PROFILE, "start_node": ["not-a-str"]}}, "full")

    def test_profile_unknown_field_fails(self):
        with self.assertRaises(ValueError):
            nodes.validate_profiles({"full": {"nodes": LITE_PROFILE, "bogus": 1}}, "full")

    def test_profile_desc_start_node_roles_optional(self):
        # 只有 nodes、无 desc/start_node/roles → 合法
        nodes.validate_profiles({"full": {"nodes": LITE_PROFILE}}, "full")

    def test_resolve_roles_replaces_task(self):
        roles = nodes.resolve_roles({"trd": "openspec/changes/<task>/"}, "my-task")
        self.assertEqual(roles["trd"], "openspec/changes/my-task/")

    def test_resolve_profile_full_returns_pipeline_and_roles(self):
        profs = {"full": _prof(FULL_PROFILE, roles=DEFAULT_ROLES)}
        out = nodes.resolve_profile_full(profs, "full", "full")
        self.assertEqual([o["node"] for o in out["pipeline"]][0], "workflow-propose")
        self.assertEqual(out["roles"]["trd"], "openspec/changes/<task>/")


class ResolvePipelineTest(unittest.TestCase):
    """resolve_pipeline 冻结对象 + 行为等价（default profile 语义对齐改造前）。"""

    def test_frozen_fields_present(self):
        frozen = nodes.resolve_pipeline(LITE_PROFILE)
        for o in frozen:
            for f in ("node", "skill", "exit_gate", "entry_gate", "autopilot", "execution", "hooks", "feedback"):
                self.assertIn(f, o)
            self.assertEqual(o["skill"], [o["node"]])

    def test_default_profile_behavior_equivalence(self):
        """full profile 冻结后 (autopilot, exit_gate, entry_gate) 逐节点符合声明。"""
        frozen = nodes.resolve_pipeline(FULL_PROFILE)
        # 停人节点（非 autopilot）
        pinned = {"workflow-propose", "workflow-test-plan", "workflow-deploy",
                  "workflow-handoff-qa", "workflow-archive"}
        exit_locked = {"workflow-propose", "workflow-test-plan", "workflow-run-autotest",
                       "workflow-handoff-qa", "workflow-archive"}
        for o in frozen:
            n = o["node"]
            self.assertEqual(o["autopilot"], n not in pinned, f"{n}.autopilot")
            self.assertEqual(o["exit_gate"],
                             nodes.GATE_CONFIRM_LOCKED if n in exit_locked else nodes.GATE_AUTO, f"{n}.exit_gate")
            self.assertEqual(o["entry_gate"],
                             nodes.GATE_CONFIRM_LOCKED if n == "workflow-deploy" else nodes.GATE_AUTO, f"{n}.entry_gate")
        # run-autotest 出口锁定但可自驱
        ra = nodes._pnode(frozen, "workflow-run-autotest")
        self.assertTrue(ra["autopilot"])
        self.assertEqual(ra["exit_gate"], nodes.GATE_CONFIRM_LOCKED)

    def test_classify_loop_equivalence(self):
        """default profile 各节点 classify_loop 终态。"""
        expected = {
            "workflow-propose": nodes.LOOP_PINNED,
            "workflow-test-plan": nodes.LOOP_PINNED,
            "workflow-implement": nodes.LOOP_CONTINUE,
            "workflow-lint": nodes.LOOP_CONTINUE,
            "workflow-unittest": nodes.LOOP_CONTINUE,
            "workflow-code-review": nodes.LOOP_CONTINUE,
            "workflow-test-cases": nodes.LOOP_CONTINUE,
            "workflow-deploy": nodes.LOOP_PINNED,
            "workflow-run-autotest": nodes.LOOP_CONTINUE,
            "workflow-handoff-qa": nodes.LOOP_PINNED,
            "workflow-archive": nodes.LOOP_PINNED,
        }
        st = _mk_state(FULL_PROFILE)
        for node, want in expected.items():
            st["main"]["current_step"] = node
            self.assertEqual(nodes.classify_loop(st)[0], want, node)

    def test_skill_override(self):
        cfg = [{"node": "impl", "skill": "my-impl"}]
        frozen = nodes.resolve_pipeline(cfg)
        self.assertEqual(nodes.skills_for(frozen, "impl"), ["my-impl"])

    def test_node_ref_doc_vs_skill(self):
        # 探磁盘定形态：给一个含 .workflow/node/<名>/NODE.md 的临时根 → 判文档（打斜杠别名）；
        # 不含该文件（或 root=None）→ 判 skill。
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / ".workflow" / "node" / "workflow-implement").mkdir(parents=True)
            (root / ".workflow" / "node" / "workflow-implement" / "NODE.md").write_text("x")
            self.assertEqual(nodes.node_ref("workflow-implement", root),
                             "/workflow-engine node workflow-implement")
            # 无 NODE.md 的名字 → skill 形态
            self.assertEqual(nodes.node_ref("workflow-feedback-loop", root), "/workflow-feedback-loop")
        # root=None（纯逻辑场景）→ 一律 skill
        self.assertEqual(nodes.node_ref("workflow-implement"), "/workflow-implement")

    def test_multi_skill_chain(self):
        # 多执行单元用 → 串联，各自走 node_ref；探到 NODE.md 的判文档、其余判 skill。
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            for n in ("a", "b"):
                (root / ".workflow" / "node" / n).mkdir(parents=True)
                (root / ".workflow" / "node" / n / "NODE.md").write_text("x")
            frozen = nodes.resolve_pipeline([{"node": "impl", "skill": ["a", "b"]}])
            self.assertEqual(nodes.skills_for(frozen, "impl"), ["a", "b"])
            self.assertEqual(nodes.skill_chain(frozen, "impl", root),
                             "/workflow-engine node a → /workflow-engine node b")

    def test_single_skill_chain_no_arrow(self):
        # 单执行单元、文档节点：无 → 箭头，直出别名命令。
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / ".workflow" / "node" / "impl").mkdir(parents=True)
            (root / ".workflow" / "node" / "impl" / "NODE.md").write_text("x")
            frozen = nodes.resolve_pipeline([{"node": "impl"}])
            self.assertEqual(nodes.skill_chain(frozen, "impl", root), "/workflow-engine node impl")

    def test_open_worklet_hint_doc_vs_skill(self):
        # enter 放行后的开工指令：文档节点直接给 Read 路径（比别名少一跳），skill 形态给 /skill，
        # 多单元用 → 串联、逐个分流。这是拦「不读 NODE.md 直接干活」的机器落点。
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / ".workflow" / "node" / "impl").mkdir(parents=True)
            (root / ".workflow" / "node" / "impl" / "NODE.md").write_text("x")
            # 单文档节点 → Read 路径，不打斜杠别名
            doc = nodes.resolve_pipeline([{"node": "impl"}])
            self.assertEqual(nodes.open_worklet_hint(doc, "impl", root),
                             "Read .workflow/node/impl/NODE.md")
            # skill 形态（无 NODE.md）→ /skill
            sk = nodes.resolve_pipeline([{"node": "impl", "skill": ["openspec-apply-change"]}])
            self.assertEqual(nodes.open_worklet_hint(sk, "impl", root), "/openspec-apply-change")
            # 多单元混合 → 逐个分流、→ 串联
            mixed = nodes.resolve_pipeline([{"node": "impl", "skill": ["impl", "openspec-apply-change"]}])
            self.assertEqual(nodes.open_worklet_hint(mixed, "impl", root),
                             "Read .workflow/node/impl/NODE.md → /openspec-apply-change")

    def test_hooks_frozen(self):
        frozen = nodes.resolve_pipeline([{"node": "impl", "hooks": {"enter": "prep", "exit": "clean"}}])
        self.assertEqual(nodes.node_hook(frozen, "impl", "enter"), ["prep"])
        self.assertEqual(nodes.node_hook(frozen, "impl", "exit"), ["clean"])

    def test_execution_default_inline(self):
        frozen = nodes.resolve_pipeline([{"node": "impl"}])
        self.assertEqual(nodes.execution_of(frozen, "impl"), nodes.EXEC_INLINE)

    def test_default_exit_gate_confirm_locked(self):
        frozen = nodes.resolve_pipeline([{"node": "d"}])
        self.assertEqual(nodes._pnode(frozen, "d")["exit_gate"], nodes.GATE_CONFIRM_LOCKED)

    def test_default_autopilot_false(self):
        frozen = nodes.resolve_pipeline([{"node": "d"}])
        self.assertFalse(nodes._pnode(frozen, "d")["autopilot"])

    def test_default_entry_gate_auto(self):
        frozen = nodes.resolve_pipeline([{"node": "d"}])
        self.assertEqual(nodes._pnode(frozen, "d")["entry_gate"], nodes.GATE_AUTO)

    def test_external_window_default_not_autopilot(self):
        frozen = nodes.resolve_pipeline([{"node": "w", "execution": "external-window"}])
        self.assertFalse(nodes._pnode(frozen, "w")["autopilot"])

    def test_resolve_profile_selects(self):
        profs = {"full": _prof(FULL_PROFILE), "lite": _prof(LITE_PROFILE)}
        frozen = nodes.resolve_profile(profs, "lite", "full")
        self.assertEqual([o["node"] for o in frozen][0], "workflow-implement")

    def test_resolve_profile_default(self):
        profs = {"full": _prof(FULL_PROFILE), "lite": _prof(LITE_PROFILE)}
        frozen = nodes.resolve_profile(profs, None, "full")
        self.assertEqual([o["node"] for o in frozen][0], "workflow-propose")


class FeedbackFieldTest(unittest.TestCase):
    """feedback 字段读取：can_report / reroute_options / rerouteable / sources。"""

    def setUp(self):
        self.p = nodes.resolve_pipeline(FULL_PROFILE)

    def test_can_report(self):
        self.assertTrue(nodes.can_report(self.p, "workflow-code-review"))
        self.assertFalse(nodes.can_report(self.p, "workflow-lint"))

    def test_feedback_sources_includes_manual(self):
        srcs = nodes.feedback_sources(self.p)
        self.assertIn("manual", srcs)
        self.assertIn("workflow-code-review", srcs)
        self.assertNotIn("workflow-lint", srcs)

    def test_reroute_options(self):
        self.assertEqual(nodes.reroute_options_for(self.p, "workflow-code-review"), ["workflow-implement"])
        self.assertEqual(nodes.reroute_options_for(self.p, "workflow-test-plan"), [])

    def test_rerouteable(self):
        self.assertFalse(nodes.is_rerouteable(self.p, "workflow-deploy"))
        self.assertTrue(nodes.is_rerouteable(self.p, "workflow-implement"))

    def test_is_downstream(self):
        self.assertTrue(nodes.is_downstream(self.p, "workflow-implement", "workflow-deploy"))
        self.assertFalse(nodes.is_downstream(self.p, "workflow-deploy", "workflow-implement"))


class ConfigTest(unittest.TestCase):
    """config 读取 profiles / 废弃旧顶层键 loud fail。"""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def _write(self, cfg):
        _write_config(self.root, cfg)

    def test_loads_profiles(self):
        from _lib import config as config_lib
        self._write(DEFAULT_CONFIG)
        cfg = config_lib.load_project_config(self.root)
        self.assertEqual(cfg["default_profile"], "full")
        self.assertIn("full", cfg["profiles"])

    def test_legacy_pipeline_fails(self):
        from _lib import config as config_lib
        self._write({"pipeline": [{"node": "workflow-implement"}]})
        with self.assertRaises(ValueError):
            config_lib.load_project_config(self.root)

    def test_legacy_defaults_fails(self):
        from _lib import config as config_lib
        self._write({"defaults": {"propose": True}, "profiles": {"full": _prof(FULL_PROFILE)}, "default_profile": "full"})
        with self.assertRaises(ValueError):
            config_lib.load_project_config(self.root)

    def test_legacy_node_gates_fails(self):
        from _lib import config as config_lib
        self._write({"node_gates": {"workflow-code-review": "auto"}})
        with self.assertRaises(ValueError):
            config_lib.load_project_config(self.root)

    def test_missing_profiles_fails(self):
        from _lib import config as config_lib
        self._write({"test_env": {"has_dedicated": True}})
        with self.assertRaises(ValueError):
            config_lib.load_project_config(self.root)

    def test_project_custom_profile_loads(self):
        """用户自写 profile 直接放项目级 .workflow/profile/（非模板名）即可被加载。"""
        from _lib import config as config_lib
        self._write(DEFAULT_CONFIG)
        (self.root / ".workflow" / "profile" / "my-flow.json").write_text(
            json.dumps(_prof(LITE_PROFILE, desc="我的自定义流"))
        )
        cfg = config_lib.load_project_config(self.root)
        self.assertIn("my-flow", cfg["profiles"])
        self.assertIn("full", cfg["profiles"])

    def test_all_profiles_from_project(self):
        """profiles 全部来自项目级；不再有用户级来源合并。"""
        from _lib import config as config_lib
        self._write(DEFAULT_CONFIG)
        cfg = config_lib.load_project_config(self.root)
        self.assertEqual(set(k for k in cfg["profiles"] if not k.startswith("_")),
                         {"full", "no-autotest", "lite"})

    def test_remote_config_files_defaults_unknown(self):
        from _lib import config as config_lib
        self._write(DEFAULT_CONFIG)
        cfg = config_lib.load_project_config(self.root)
        self.assertEqual(cfg["remote_config_files"], "unknown")

    def test_remote_config_files_explicit_false(self):
        from _lib import config as config_lib
        self._write({**DEFAULT_CONFIG, "remote_config_files": False})
        cfg = config_lib.load_project_config(self.root)
        self.assertEqual(cfg["remote_config_files"], "false")

    def test_read_remote_config_files_soft_defaults(self):
        """单读开关：缺文件 / 坏 JSON / 缺字段 → unknown；true/false 原样。"""
        from _lib import config as config_lib
        empty = Path(tempfile.mkdtemp())
        self.addCleanup(lambda: shutil.rmtree(empty, ignore_errors=True))
        self.assertEqual(config_lib.read_remote_config_files(empty), "unknown")

        wf = empty / ".workflow"
        wf.mkdir()
        (wf / "config.json").write_text("{not json")
        self.assertEqual(config_lib.read_remote_config_files(empty), "unknown")

        (wf / "config.json").write_text("{}")
        self.assertEqual(config_lib.read_remote_config_files(empty), "unknown")

        (wf / "config.json").write_text(json.dumps({"remote_config_files": False}))
        self.assertEqual(config_lib.read_remote_config_files(empty), "false")

        (wf / "config.json").write_text(json.dumps({"remote_config_files": True}))
        self.assertEqual(config_lib.read_remote_config_files(empty), "true")

        (wf / "config.json").write_text(json.dumps({"remote_config_files": None}))
        self.assertEqual(config_lib.read_remote_config_files(empty), "unknown")

    def test_write_remote_config_files(self):
        from _lib import config as config_lib
        self._write(DEFAULT_CONFIG)
        self.assertEqual(config_lib.write_remote_config_files(self.root, "true"), "true")
        self.assertIs(json.loads((self.root / ".workflow" / "config.json").read_text())["remote_config_files"], True)
        self.assertEqual(config_lib.write_remote_config_files(self.root, "unknown"), "unknown")
        self.assertIsNone(json.loads((self.root / ".workflow" / "config.json").read_text())["remote_config_files"])
        with self.assertRaises(ValueError):
            config_lib.write_remote_config_files(self.root, "maybe")


class RolesCliTest(_CliBase):
    """roles 冻结进 state + roles 命令查询 + no-prompt 目录 + context.md 引导。"""

    def test_init_freezes_roles_with_task_substituted(self):
        self.run_cli("init", "rc1", "--profile", "full", "--scope", "http", "--http-domain", "a.com")
        roles = self.state("rc1")["roles"]
        self.assertEqual(roles["trd"], "openspec/changes/rc1/")
        self.assertEqual(roles["deploy"], "delivery/rc1/deploy.md")

    def test_roles_command_prints_resolved_paths(self):
        self.run_cli("init", "rc2", "--profile", "full", "--scope", "http", "--http-domain", "a.com")
        r = self.run_cli("roles", "rc2")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("openspec/changes/rc2/", r.stdout)
        self.assertIn("trd", r.stdout)

    def test_init_no_prompt_dir(self):
        self.run_cli("init", "rc3", "--profile", "lite")
        self.assertFalse((self.root / "delivery" / "rc3" / "prompt").exists())
        self.assertTrue((self.root / "delivery" / "rc3" / "prd").exists())

    def test_init_output_mentions_context_md(self):
        r = self.run_cli("init", "rc4", "--profile", "lite")
        self.assertIn("context.md", r.stdout)

    def test_roles_empty_profile_degrades(self):
        # lite profile 的 roles 只有 schema-draft/deploy（本测试 DEFAULT_CONFIG 未给 lite roles）
        self.run_cli("init", "rc5", "--profile", "lite")
        r = self.run_cli("roles", "rc5")
        self.assertEqual(r.returncode, 0, r.stderr)


class PipelineE2ETest(_CliBase):
    """自定义领域 profile（全新节点名，非 workflow-*）能 init 跑通——验证引擎不认节点名。"""

    def setUp(self):
        super().setUp()
        custom = {
            "profiles": {
                "domain-x": _prof([
                    {"node": "gather-facts", "exit_gate": "auto",
                     "feedback": {"can_report": True, "reroute_options": ["gather-facts"]}},
                    {"node": "draft-answer", "exit_gate": "confirm"},
                    {"node": "publish", "entry_gate": "confirm-locked",
                     "feedback": {"rerouteable": False}},
                ], roles={"answer": "delivery/<task>/answer.md"}, has_e2etest=False)
            },
            "default_profile": "domain-x",
        }
        _write_config(self.root, custom)

    def test_custom_domain_profile_runs(self):
        r = self.run_cli("init", "d1", "--profile", "domain-x")
        self.assertEqual(r.returncode, 0, r.stderr)
        seq = [o["node"] for o in self.state("d1")["pipeline"]]
        self.assertEqual(seq, ["gather-facts", "draft-answer", "publish"])
        self.assertEqual(self.state("d1")["main"]["current_step"], "gather-facts")

    def test_custom_enter_and_advance(self):
        self.run_cli("init", "d2", "--profile", "domain-x")
        r = self.run_cli("enter", "d2", "--step", "gather-facts")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("enter gather-facts", r.stdout)
        r2 = self.step("d2", "gather-facts")
        self.assertEqual(r2.returncode, 0, r2.stderr)
        self.assertEqual(self.state("d2")["main"]["current_step"], "draft-answer")


class EnabledProfileTest(_CliBase):
    """enabled 开关：禁用 profile 不进 list-profiles / 向导候选，但显式 --profile 仍可 init。"""

    def _write_with_disabled(self):
        cfg = {
            "profiles": {
                "full": _prof(FULL_PROFILE, roles=DEFAULT_ROLES, desc="完整流",
                              start_node="workflow-explore"),
                "lite": _prof(LITE_PROFILE, desc="轻量流"),
                "sp": _prof(NO_AUTOTEST_PROFILE, desc="禁用模式", enabled=False),
            },
            "default_profile": "full",
        }
        _write_config(self.root, cfg)

    def test_list_profiles_hides_disabled(self):
        self._write_with_disabled()
        r = self.run_cli("list-profiles")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("full", r.stdout)
        self.assertIn("lite", r.stdout)
        self.assertNotIn("sp", r.stdout)

    def test_explicit_profile_flag_allows_disabled(self):
        self._write_with_disabled()
        r = self.run_cli("init", "en1", "--profile", "sp", "--env", "local")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertEqual(self.state("en1")["profile"], "sp")

    def test_default_profile_disabled_fails(self):
        from _lib import nodes as nodes_lib
        with self.assertRaises(ValueError):
            nodes_lib.validate_profiles(
                {"a": _prof(LITE_PROFILE, enabled=False)}, "a")

    def test_is_enabled_defaults_true(self):
        from _lib import nodes as nodes_lib
        self.assertTrue(nodes_lib.is_enabled(_prof(LITE_PROFILE)))
        self.assertFalse(nodes_lib.is_enabled(_prof(LITE_PROFILE, enabled=False)))


class StartBranchGateTest(_CliBase):
    """起步分支闸门：master*/main*/release* 上不许起任务（list-profiles + init 两个入口）。"""

    def _switch_branch(self, branch: str):
        subprocess.run(["git", "-C", str(self.root), "checkout", "-b", branch],
                       capture_output=True, text=True, check=True)

    def _init_on(self, branch: str, task: str):
        self._switch_branch(branch)
        return self.run_cli("init", task, "--profile", "full", "--env", "test",
                            "--scope", "http", "--http-domain", "a.com")

    def test_init_blocked_on_master(self):
        r = self._init_on("master", "b1")
        self.assertEqual(r.returncode, 1)
        self.assertIn("保护分支", r.stderr)
        # 拦下时一个目录都别留（与其他 init 前置闸门一致：失败不留孤儿目录）
        self.assertFalse((self.root / "delivery" / "b1").exists())

    def test_init_blocked_on_protected_prefixes(self):
        # 前缀匹配：这些都不是裸 master/main/release，也一律命中
        for i, branch in enumerate(("master-fix", "main-2", "release-1.2", "releases", "Master")):
            with self.subTest(branch=branch):
                r = self._init_on(branch, f"b2-{i}")
                self.assertEqual(r.returncode, 1, r.stdout)
                self.assertIn("保护分支", r.stderr)

    def test_init_allowed_on_feature_branch(self):
        # setUp 已把仓库钉在 feature-test 上
        r = self.run_cli("init", "b3", "--profile", "full", "--env", "test",
                         "--scope", "http", "--http-domain", "a.com")
        self.assertEqual(r.returncode, 0, r.stderr)

    def test_init_allowed_when_not_a_git_repo(self):
        # 探测不到分支（非 git 仓库）→ 软失败放行，绝不误挡
        plain = Path(tempfile.mkdtemp())
        try:
            _write_config(plain)
            r = subprocess.run(
                [sys.executable, str(CLI), "--target", str(plain), "init", "b4",
                 "--profile", "full", "--env", "test", "--scope", "http", "--http-domain", "a.com"],
                capture_output=True, text=True, env=_cli_env(),
            )
            self.assertEqual(r.returncode, 0, r.stderr)
        finally:
            shutil.rmtree(plain, ignore_errors=True)

    def test_list_profiles_blocked_on_master_before_upgrade(self):
        # 闸门在 _maybe_auto_upgrade 之前：保护分支上不许起任务，也不在其工作区刷模板。
        self._switch_branch("master")
        r = self.run_cli("list-profiles")
        self.assertEqual(r.returncode, 1)
        self.assertIn("保护分支", r.stderr)
        self.assertNotIn("可用编排模式", r.stdout)
        self.assertNotIn("正在升级", r.stdout)

    def test_list_profiles_allowed_on_feature_branch(self):
        r = self.run_cli("list-profiles")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("可用编排模式", r.stdout)


class TaskNameUniqueTest(_CliBase):
    """任务名闸门：不与当前分支同名 + 全仓库唯一（活跃 + 归档）。"""

    def init_ok(self, task):
        r = self.run_cli("init", task, "--profile", "full", "--env", "test",
                         "--scope", "http", "--http-domain", "a.com")
        self.assertEqual(r.returncode, 0, r.stderr)
        return r

    def init_raw(self, task):
        return self.run_cli("init", task, "--profile", "full", "--env", "test",
                            "--scope", "http", "--http-domain", "a.com")

    def test_rejects_task_named_like_branch(self):
        # setUp 的分支是 feature-test，且它本身是合法 kebab-case 任务名
        r = self.init_raw("feature-test")
        self.assertEqual(r.returncode, 1)
        self.assertIn("与当前分支名相同", r.stderr)
        self.assertFalse((self.root / "delivery" / "feature-test").exists())

    def test_rejects_duplicate_active_task(self):
        self.init_ok("dup1")
        r = self.init_raw("dup1")
        self.assertEqual(r.returncode, 1)
        self.assertIn("已初始化", r.stderr)

    def test_rejects_name_taken_by_archive(self):
        # 早失败：不挡的话要等任务跑到末尾 archive-move 才撞车
        (self.root / "delivery" / "archive" / "dup2").mkdir(parents=True)
        r = self.init_raw("dup2")
        self.assertEqual(r.returncode, 1)
        self.assertIn("同名归档任务", r.stderr)
        self.assertFalse((self.root / "delivery" / "dup2").exists())

    def test_adopts_half_built_dir_without_state(self):
        # 目录存在但无 state.json（如 prd-pre-review 先落了 prd/）不算重名，仍走半成品收编
        prd = self.root / "delivery" / "half" / "prd"
        prd.mkdir(parents=True)
        (prd / "prd.md").write_text("draft")
        r = self.init_raw("half")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("收编", r.stdout)
        self.assertEqual((prd / "prd.md").read_text(), "draft")

    def test_reports_all_violations_at_once(self):
        # 双违规（撞分支名 + 撞归档名）一次报全，别让 AI 撞一条修一条多跑一轮
        (self.root / "delivery" / "archive" / "feature-test").mkdir(parents=True)
        r = self.init_raw("feature-test")
        self.assertEqual(r.returncode, 1)
        self.assertIn("（2 项）", r.stderr)
        self.assertIn("与当前分支名相同", r.stderr)
        self.assertIn("同名归档任务", r.stderr)


class MigrationTest(unittest.TestCase):
    """_normalize：老 state 的 skill 单串→列表、issue 回填 category/reroute_target。"""

    def test_normalize_string_skill_to_list(self):
        st = _mk_state(LITE_PROFILE)
        st["pipeline"][0]["skill"] = "legacy-impl"
        st["pipeline"][0]["hooks"] = {"enter": "legacy-hook"}
        state_lib._normalize(st)
        self.assertEqual(st["pipeline"][0]["skill"], ["legacy-impl"])
        self.assertEqual(st["pipeline"][0]["hooks"]["enter"], ["legacy-hook"])

    def test_normalize_backfills_issue_fields(self):
        st = _mk_state()
        st["feedback"]["open"].append({"id": "issue-9", "from": "manual"})
        state_lib._normalize(st)
        it = st["feedback"]["open"][0]
        self.assertIn("category", it)
        self.assertIn("reroute_target", it)
        self.assertEqual(it["reroute_count"], 0)

    def test_normalize_backfills_resolved_mode_both_arrays(self):
        # 老任务（加字段前闭环的）两个数组都得回填 None，否则读老 state 直接 KeyError。
        # open/resolved 两个 loop 必须同步维护——resolve_issue 是把 open 的原对象搬过去的，
        # 只补一边会让两个数组的结构在老任务上分叉。
        st = _mk_state()
        st["feedback"]["open"].append({"id": "issue-8", "from": "manual"})
        st["feedback"]["resolved"].append({"id": "issue-7", "from": "workflow-code-review"})
        state_lib._normalize(st)
        self.assertIsNone(st["feedback"]["open"][0]["resolved_mode"])
        self.assertIsNone(st["feedback"]["resolved"][0]["resolved_mode"])

    def test_normalize_backfills_version_fields(self):
        # 老任务（升级前 init）只有旧的机器态 lb_version/lbcli_version，无 npm_ 前缀字段、无仓库态字段。
        # 迁移须：① 把旧的机器态值搬到 npm_lb_version/npm_lbcli_version（改名保值）；
        #         ② 腾出的 lb_version 语义变仓库态，老任务无此快照 → 回填 None。
        st = _mk_state()
        for k in ("npm_lb_version", "npm_lbcli_version", "lb_version"):
            st.pop(k, None)
        st["lb_version"] = "1.0.0"       # 老任务里 lb_version 是机器态探测值
        st["lbcli_version"] = "2.0.0"
        state_lib._normalize(st)
        self.assertEqual(st["npm_lb_version"], "1.0.0")     # 机器态改名保值
        self.assertEqual(st["npm_lbcli_version"], "2.0.0")
        self.assertIsNone(st["lb_version"])                 # 仓库态：老任务无 → None
        self.assertNotIn("lbcli_version", st)               # 旧无前缀键已被搬走

    def test_normalize_preserves_new_version_fields(self):
        # 新任务已有 npm_/仓库态字段时迁移不覆盖。
        st = _mk_state()
        st["npm_lb_version"] = "9.9.9"
        st["npm_lbcli_version"] = "8.8.8"
        st["lb_version"] = "1.1.0"
        state_lib._normalize(st)
        self.assertEqual(st["npm_lb_version"], "9.9.9")
        self.assertEqual(st["npm_lbcli_version"], "8.8.8")
        self.assertEqual(st["lb_version"], "1.1.0")


class TelemetryCollectTest(unittest.TestCase):
    """telemetry 采集纯函数：detect_run_env / collect_context_used（直调，不走 CLI）。"""

    def setUp(self):
        import importlib
        from _lib import telemetry as tel
        self.tel = importlib.reload(tel)
        # 这些采集在 WORKFLOW_SKIP_TELEMETRY=1 时短路，直调测试须清掉
        self._saved = {k: os.environ.get(k) for k in
                       ("WORKFLOW_SKIP_TELEMETRY", "OPENCLI_SANDBOX_MODE", "CLAUDE_CODE_SESSION_ID")}
        os.environ.pop("WORKFLOW_SKIP_TELEMETRY", None)

    def tearDown(self):
        for k, v in self._saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v

    def test_detect_run_env_sandbox_values(self):
        for v in ("1", "true", "on", "yes", "TRUE", " Yes "):
            os.environ["OPENCLI_SANDBOX_MODE"] = v
            self.assertEqual(self.tel.detect_run_env(), "sandbox", f"{v!r} 应判 sandbox")

    def test_detect_run_env_local_values(self):
        for v in ("0", "false", "off", "no", "", "bogus"):
            os.environ["OPENCLI_SANDBOX_MODE"] = v
            self.assertEqual(self.tel.detect_run_env(), "local", f"{v!r} 应判 local")
        os.environ.pop("OPENCLI_SANDBOX_MODE", None)
        self.assertEqual(self.tel.detect_run_env(), "local")  # 未设 → local

    def test_detect_run_env_skip_is_local(self):
        os.environ["WORKFLOW_SKIP_TELEMETRY"] = "1"
        os.environ["OPENCLI_SANDBOX_MODE"] = "1"
        self.assertEqual(self.tel.detect_run_env(), "local")  # 自测短路恒 local

    def test_report_headers_light_env_tracks_sandbox(self):
        # light-env 实时探测 OPENCLI_SANDBOX_MODE：sandbox 模式 → "sandbox"，否则 "local"
        payload = {"state": {"identity": {"git_remote": "git@x:o/r.git", "git_user": "u"}}}
        os.environ["OPENCLI_SANDBOX_MODE"] = "1"
        h = self.tel._report_headers(payload)
        self.assertEqual(h["light-env"], "sandbox")
        self.assertEqual(h["light-app-name"], "o/r")  # 附带确认其余头照常
        self.assertEqual(h["light-user"], "u")
        os.environ["OPENCLI_SANDBOX_MODE"] = "0"
        self.assertEqual(self.tel._report_headers(payload)["light-env"], "local")
        os.environ.pop("OPENCLI_SANDBOX_MODE", None)
        self.assertEqual(self.tel._report_headers(payload)["light-env"], "local")  # 未设 → local

    def test_collect_context_used_no_session_returns_blank(self):
        os.environ.pop("CLAUDE_CODE_SESSION_ID", None)
        self.assertEqual(
            self.tel.collect_context_used(),
            {"context_used": None, "baseline_context_used": None,
             "session_id": None, "model": None, "turns": None,
             "token_usage": None},
        )

    def test_collect_context_used_parses_model_turns_used(self):
        from unittest import mock
        sid = "test-session-abc"
        home = Path(tempfile.mkdtemp())
        proj = home / ".claude" / "projects" / "some-proj"
        proj.mkdir(parents=True)
        lines = [
            {"type": "user", "message": {"role": "user", "content": "第一问"}},
            {"type": "assistant", "message": {"id": "m1", "role": "assistant", "model": "claude-opus-4-8",
             "content": [{"type": "text"}], "usage": {"input_tokens": 100, "cache_read_input_tokens": 20}}},
            # 工具结果的 user 消息不算轮次
            {"type": "user", "message": {"role": "user", "content": [{"type": "tool_result"}]}},
            # meta / sidechain 不算
            {"type": "user", "isMeta": True, "message": {"role": "user", "content": "meta"}},
            {"type": "user", "isSidechain": True, "message": {"role": "user", "content": "side"}},
            {"type": "user", "message": {"role": "user", "content": "第二问"}},
            {"type": "assistant", "message": {"id": "m2", "role": "assistant", "model": "claude-opus-4-8",
             "content": [{"type": "text"}], "usage": {"input_tokens": 200, "cache_creation_input_tokens": 5,
                                                       "cache_read_input_tokens": 300}}},
        ]
        (proj / f"{sid}.jsonl").write_text(
            "\n".join(json.dumps(o) for o in lines), encoding="utf-8")
        os.environ["CLAUDE_CODE_SESSION_ID"] = sid
        try:
            with mock.patch.object(self.tel.Path, "home", return_value=home):
                got = self.tel.collect_context_used()
        finally:
            shutil.rmtree(home, ignore_errors=True)
        self.assertEqual(got["context_used"], 505)  # 末条 usage: 200+5+300
        self.assertEqual(got["baseline_context_used"], 120)  # 首条 usage: 100+0+20
        self.assertEqual(got["session_id"], sid)
        self.assertEqual(got["model"], "claude-opus-4-8")
        self.assertEqual(got["turns"], 2)  # 只有「第一问」「第二问」
        tu = got["token_usage"]
        self.assertEqual(tu["session_id"], sid)
        self.assertEqual(tu["summary"]["input_tokens"], 300)
        self.assertEqual(tu["summary"]["cache_creation_input_tokens"], 5)
        self.assertEqual(tu["summary"]["cache_read_input_tokens"], 320)
        self.assertEqual(tu["summary"]["output_tokens"], 0)
        self.assertEqual(tu["summary"]["total_tokens"], 625)
        self.assertEqual(got["context_used"], 505)  # 水位仍是末条 200+5+300

    def test_collect_dedupes_same_message_id_keeps_max_output(self):
        from unittest import mock
        sid = "dup-msg"
        home = Path(tempfile.mkdtemp())
        proj = home / ".claude" / "projects" / "p"
        proj.mkdir(parents=True)
        mid = "msg_1"
        lines = [
            {"type": "assistant", "message": {"id": mid, "model": "claude-opus-4-8",
             "usage": {"input_tokens": 1, "output_tokens": 10, "cache_read_input_tokens": 100}}},
            {"type": "assistant", "message": {"id": mid, "model": "claude-opus-4-8",
             "usage": {"input_tokens": 1, "output_tokens": 40, "cache_read_input_tokens": 100}}},
        ]
        (proj / f"{sid}.jsonl").write_text("\n".join(json.dumps(o) for o in lines), encoding="utf-8")
        os.environ["CLAUDE_CODE_SESSION_ID"] = sid
        try:
            with mock.patch.object(self.tel.Path, "home", return_value=home):
                tu = self.tel.collect_context_used()["token_usage"]
        finally:
            shutil.rmtree(home, ignore_errors=True)
        self.assertEqual(tu["summary"]["output_tokens"], 40)
        self.assertEqual(tu["summary"]["input_tokens"], 1)
        self.assertEqual(tu["summary"]["total_tokens"], 141)

    def test_collect_includes_subagent_jsonl(self):
        from unittest import mock
        sid = "with-sub"
        home = Path(tempfile.mkdtemp())
        proj = home / ".claude" / "projects" / "p"
        proj.mkdir(parents=True)
        (proj / f"{sid}.jsonl").write_text(json.dumps({
            "type": "assistant",
            "message": {"id": "m1", "model": "claude-opus-4-8",
                        "usage": {"input_tokens": 2, "output_tokens": 8,
                                  "cache_creation_input_tokens": 0, "cache_read_input_tokens": 10}},
        }), encoding="utf-8")
        sub = proj / sid / "subagents"
        sub.mkdir(parents=True)
        (sub / "agent-x.jsonl").write_text(json.dumps({
            "type": "assistant", "isSidechain": True,
            "message": {"id": "m2", "model": "claude-opus-4-8",
                        "usage": {"input_tokens": 3, "output_tokens": 9,
                                  "cache_creation_input_tokens": 1, "cache_read_input_tokens": 20}},
        }), encoding="utf-8")
        (sub / "agent-broken.jsonl").write_text("{not json\n", encoding="utf-8")
        os.environ["CLAUDE_CODE_SESSION_ID"] = sid
        try:
            with mock.patch.object(self.tel.Path, "home", return_value=home):
                got = self.tel.collect_context_used()
        finally:
            shutil.rmtree(home, ignore_errors=True)
        tu = got["token_usage"]
        self.assertEqual(tu["summary"]["input_tokens"], 5)
        self.assertEqual(tu["summary"]["output_tokens"], 17)
        self.assertEqual(tu["summary"]["cache_creation_input_tokens"], 1)
        self.assertEqual(tu["summary"]["cache_read_input_tokens"], 30)
        self.assertEqual(tu["summary"]["total_tokens"], 53)
        self.assertEqual(got["context_used"], 12)  # 水位只看主文件末条 2+0+10

    def test_collect_splits_by_model(self):
        from unittest import mock
        sid = "two-models"
        home = Path(tempfile.mkdtemp())
        proj = home / ".claude" / "projects" / "p"
        proj.mkdir(parents=True)
        lines = [
            {"type": "assistant", "message": {"id": "a", "model": "claude-opus-4-8",
             "usage": {"input_tokens": 1, "output_tokens": 2, "cache_read_input_tokens": 3}}},
            {"type": "assistant", "message": {"id": "b", "model": "claude-sonnet-4-6",
             "usage": {"input_tokens": 4, "output_tokens": 5, "cache_read_input_tokens": 6}}},
        ]
        (proj / f"{sid}.jsonl").write_text("\n".join(json.dumps(o) for o in lines), encoding="utf-8")
        os.environ["CLAUDE_CODE_SESSION_ID"] = sid
        try:
            with mock.patch.object(self.tel.Path, "home", return_value=home):
                tu = self.tel.collect_context_used()["token_usage"]
        finally:
            shutil.rmtree(home, ignore_errors=True)
        self.assertEqual(tu["summary"]["input_tokens"], 5)
        self.assertEqual(tu["models"]["claude-opus-4-8"]["total_tokens"], 6)
        self.assertEqual(tu["models"]["claude-sonnet-4-6"]["total_tokens"], 15)
        self.assertEqual(tu["summary"]["total_tokens"], 21)
        self.assertNotIn("input_tokens", tu)
        self.assertNotIn("by_model", tu)

    def test_collect_skips_malformed_usage(self):
        from unittest import mock
        sid = "malformed-usage"
        home = Path(tempfile.mkdtemp())
        proj = home / ".claude" / "projects" / "p"
        proj.mkdir(parents=True)
        lines = [
            {"type": "assistant", "message": {"id": "m1", "model": "claude-opus-4-8",
             "usage": {"input_tokens": 10, "output_tokens": 1, "cache_read_input_tokens": 5}}},
            {"type": "assistant", "message": {"id": "m-bad", "model": "claude-opus-4-8",
             "usage": "not-a-dict"}},
            {"type": "assistant", "message": {"id": "m2", "model": "claude-opus-4-8",
             "usage": {"input_tokens": 20, "output_tokens": 2, "cache_read_input_tokens": 8}}},
            {"type": "assistant", "message": {"id": "m3", "model": "claude-opus-4-8",
             "usage": {"input_tokens": "bad", "output_tokens": 0, "cache_read_input_tokens": 0}}},
        ]
        (proj / f"{sid}.jsonl").write_text("\n".join(json.dumps(o) for o in lines), encoding="utf-8")
        os.environ["CLAUDE_CODE_SESSION_ID"] = sid
        try:
            with mock.patch.object(self.tel.Path, "home", return_value=home):
                got = self.tel.collect_context_used()
        finally:
            shutil.rmtree(home, ignore_errors=True)
        self.assertEqual(got["context_used"], 28)  # 末条有效 usage: 20+0+8
        tu = got["token_usage"]
        self.assertEqual(tu["summary"]["input_tokens"], 30)
        self.assertEqual(tu["summary"]["output_tokens"], 3)
        self.assertEqual(tu["summary"]["cache_read_input_tokens"], 13)
        self.assertEqual(tu["summary"]["total_tokens"], 46)

    def test_baseline_skips_synthetic_zero_usage(self):
        """基线取首条**非零**水位：Claude Code 插的 model=<synthetic> 合成消息带 usage 但四项全零，
        当基线会把真实起步占用误报成 0（实测 20 个会话里 3 个首条即合成）。"""
        from unittest import mock
        sid = "synthetic-first"
        home = Path(tempfile.mkdtemp())
        proj = home / ".claude" / "projects" / "p"
        proj.mkdir(parents=True)
        lines = [
            {"type": "assistant", "message": {"id": "s1", "model": "<synthetic>",
             "usage": {"input_tokens": 0, "output_tokens": 0,
                       "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0}}},
            {"type": "assistant", "message": {"id": "m1", "model": "claude-opus-4-8",
             "usage": {"input_tokens": 2, "cache_creation_input_tokens": 15465}}},
            {"type": "assistant", "message": {"id": "m2", "model": "claude-opus-4-8",
             "usage": {"input_tokens": 1, "cache_read_input_tokens": 70000}}},
        ]
        (proj / f"{sid}.jsonl").write_text("\n".join(json.dumps(o) for o in lines), encoding="utf-8")
        os.environ["CLAUDE_CODE_SESSION_ID"] = sid
        try:
            with mock.patch.object(self.tel.Path, "home", return_value=home):
                got = self.tel.collect_context_used()
        finally:
            shutil.rmtree(home, ignore_errors=True)
        self.assertEqual(got["baseline_context_used"], 15467)  # 跳过合成零行，取 2+15465
        self.assertEqual(got["context_used"], 70001)          # 末条水位照旧

    def test_baseline_none_when_all_usage_zero(self):
        """全是合成零行 → 基线为 None（不塞字段），水位仍报 0：不拿 0 冒充基线。"""
        from unittest import mock
        sid = "all-zero"
        home = Path(tempfile.mkdtemp())
        proj = home / ".claude" / "projects" / "p"
        proj.mkdir(parents=True)
        lines = [
            {"type": "assistant", "message": {"id": "s1", "model": "<synthetic>",
             "usage": {"input_tokens": 0, "output_tokens": 0, "cache_read_input_tokens": 0}}},
        ]
        (proj / f"{sid}.jsonl").write_text("\n".join(json.dumps(o) for o in lines), encoding="utf-8")
        os.environ["CLAUDE_CODE_SESSION_ID"] = sid
        try:
            with mock.patch.object(self.tel.Path, "home", return_value=home):
                got = self.tel.collect_context_used()
        finally:
            shutil.rmtree(home, ignore_errors=True)
        self.assertIsNone(got["baseline_context_used"])
        self.assertEqual(got["context_used"], 0)

    def test_apply_token_snapshot_merges_two_sessions(self):
        st = {}
        self.tel.apply_token_snapshot(st, {
            "session_id": "A",
            "summary": {
                "total_tokens": 10,
                "input_tokens": 1, "output_tokens": 2,
                "cache_creation_input_tokens": 3, "cache_read_input_tokens": 4,
            },
            "models": {"opus": {
                "total_tokens": 10, "input_tokens": 1, "output_tokens": 2,
                "cache_creation_input_tokens": 3, "cache_read_input_tokens": 4,
            }},
        })
        self.tel.apply_token_snapshot(st, {
            "session_id": "B",
            "summary": {
                "total_tokens": 100,
                "input_tokens": 10, "output_tokens": 20,
                "cache_creation_input_tokens": 30, "cache_read_input_tokens": 40,
            },
            "models": {
                "opus": {
                    "total_tokens": 55, "input_tokens": 5, "output_tokens": 10,
                    "cache_creation_input_tokens": 15, "cache_read_input_tokens": 25,
                },
                "sonnet": {
                    "total_tokens": 45, "input_tokens": 5, "output_tokens": 10,
                    "cache_creation_input_tokens": 15, "cache_read_input_tokens": 15,
                },
            },
        })
        tu = st["token_usage"]
        self.assertEqual(tu["summary"]["input_tokens"], 11)
        self.assertEqual(tu["summary"]["output_tokens"], 22)
        self.assertEqual(tu["summary"]["total_tokens"], 110)
        self.assertEqual(tu["models"]["opus"]["input_tokens"], 6)
        self.assertEqual(tu["models"]["sonnet"]["output_tokens"], 10)
        self.assertEqual(set(tu["by_session"]), {"A", "B"})
        self.tel.apply_token_snapshot(st, {
            "session_id": "B",
            "summary": {
                "total_tokens": 4,
                "input_tokens": 1, "output_tokens": 1,
                "cache_creation_input_tokens": 1, "cache_read_input_tokens": 1,
            },
            "models": {"sonnet": {
                "total_tokens": 4, "input_tokens": 1, "output_tokens": 1,
                "cache_creation_input_tokens": 1, "cache_read_input_tokens": 1,
            }},
        })
        tu = st["token_usage"]
        self.assertEqual(tu["by_session"]["A"]["summary"]["total_tokens"], 10)
        self.assertEqual(tu["by_session"]["B"]["summary"]["total_tokens"], 4)
        self.assertEqual(tu["summary"]["total_tokens"], 14)
        self.assertEqual(tu["models"]["opus"]["input_tokens"], 1)
        self.assertEqual(tu["models"]["sonnet"]["total_tokens"], 4)
        self.assertEqual(set(tu["models"]), {"opus", "sonnet"})
        published = self.tel.token_usage_for_report(tu)
        self.assertEqual(published["summary"]["total_tokens"], 14)
        self.assertEqual(set(published["models"]), {"opus", "sonnet"})
        self.assertNotIn("by_session", published)
        self.assertIn("by_session", tu)

    def test_apply_token_snapshot_none_is_noop(self):
        st = {"token_usage": {"summary": {"total_tokens": 9}, "models": {}}}
        self.tel.apply_token_snapshot(st, None)
        self.assertEqual(st["token_usage"]["summary"]["total_tokens"], 9)

    def test_apply_token_snapshot_repairs_null_token_usage(self):
        st = {"token_usage": None}
        self.tel.apply_token_snapshot(st, {
            "session_id": "S",
            "summary": {
                "input_tokens": 1, "output_tokens": 1,
                "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0,
            },
            "models": {},
        })
        self.assertEqual(st["token_usage"]["summary"]["total_tokens"], 2)
        self.assertEqual(st["token_usage"]["by_session"]["S"]["summary"]["total_tokens"], 2)

    def test_apply_token_snapshot_after_advance_stays_out_of_history(self):
        st = _mk_state(LITE_PROFILE)
        state_lib.advance_main(st, "workflow-implement", "2026-08-20")
        self.tel.apply_token_snapshot(st, {
            "session_id": "S",
            "summary": {
                "input_tokens": 1, "output_tokens": 1,
                "cache_creation_input_tokens": 0, "cache_read_input_tokens": 0,
            },
            "models": {},
        })
        self.assertEqual(st["token_usage"]["summary"]["total_tokens"], 2)
        self.assertNotIn("token_usage", st["main"]["history"][-1])

    def test_payload_for_report_strips_by_session_without_mutating_state(self):
        st = {"token_usage": {
            "summary": {"total_tokens": 3, "input_tokens": 1, "output_tokens": 1,
                        "cache_creation_input_tokens": 0, "cache_read_input_tokens": 1},
            "models": {"opus": {"total_tokens": 3, "input_tokens": 1, "output_tokens": 1,
                                "cache_creation_input_tokens": 0, "cache_read_input_tokens": 1}},
            "by_session": {"A": {"summary": {"total_tokens": 3}}},
        }}
        payload = {"event": "advance", "state": st}
        body = self.tel._payload_for_report(payload)
        self.assertNotIn("by_session", body["state"]["token_usage"])
        self.assertEqual(body["state"]["token_usage"]["summary"]["total_tokens"], 3)
        self.assertIn("by_session", st["token_usage"])


class TelemetryPingTest(unittest.TestCase):
    """采集通道探测 telemetry.ping：通返回 None、不通返回文案。全程打桩，不碰网络。"""

    OK_CFG = {"is_report": True, "report_url": "http://light-data-server/report"}

    def setUp(self):
        import importlib
        import urllib.request
        from _lib import telemetry as tel
        # reload 把 telemetry 上的猴补还原；urlopen 补在 urllib 上，得自己存自己还
        self.tel = importlib.reload(tel)
        self._real_urlopen = urllib.request.urlopen
        # ping 在 WORKFLOW_SKIP_TELEMETRY=1 时短路，直调测试须清掉
        self._saved = {k: os.environ.get(k) for k in
                       ("WORKFLOW_SKIP_TELEMETRY", "OPENCLI_SANDBOX_MODE")}
        os.environ.pop("WORKFLOW_SKIP_TELEMETRY", None)
        self.target = Path(tempfile.mkdtemp())
        # 默认给一套齐全身份，个别用例再按需覆盖
        self.tel.collect_identity = lambda _t: {
            "git_user": "u", "git_email": "e", "git_remote": "git@x:o/r.git", "git_branch": "b"}

    def tearDown(self):
        import urllib.request
        urllib.request.urlopen = self._real_urlopen  # 别把桩漏给同进程的其他用例
        shutil.rmtree(self.target, ignore_errors=True)
        for k, v in self._saved.items():
            if v is None:
                os.environ.pop(k, None)
            else:
                os.environ[k] = v

    def _stub_post(self, resp_body):
        """打桩 POST：返回给定回包；同时记录实际发出的 Request 供断言。"""
        sent = {}

        class _Resp:
            def __enter__(_s): return _s
            def __exit__(*_a): return False
            def read(_s): return json.dumps(resp_body).encode("utf-8")

        def _urlopen(req, timeout=None):
            sent["req"] = req
            return _Resp()

        import urllib.request
        urllib.request.urlopen = _urlopen
        return sent

    # ── 不算故障的两种情况 ──
    def test_skip_env_short_circuits(self):
        os.environ["WORKFLOW_SKIP_TELEMETRY"] = "1"
        def boom(*_a, **_k): raise AssertionError("短路时不该拉配置")
        self.tel._fetch_remote_config_raw = boom
        self.assertIsNone(self.tel.ping(self.target))

    def test_remote_switch_off_is_not_a_failure(self):
        # 远程主动关闭上报是预期状态，不该拦住用户起任务
        self.tel._fetch_remote_config_raw = lambda **_k: {"is_report": False, "report_url": "http://x"}
        self.assertIsNone(self.tel.ping(self.target))

    # ── 配置环 ──
    def test_config_fetch_failure_reports_raw_reason_and_skips_post(self):
        def boom(**_k): raise OSError("[SSL: CERTIFICATE_VERIFY_FAILED] self signed cert")
        self.tel._fetch_remote_config_raw = boom
        import urllib.request
        urllib.request.urlopen = lambda *_a, **_k: self.fail("配置都拉不到，不该再发 POST")
        msg = self.tel.ping(self.target)
        self.assertIn("采集通道不通", msg)
        self.assertIn("CERTIFICATE_VERIFY_FAILED", msg)  # 原始异常原文透传，不加工
        self.assertIn("--skip-ping", msg)                # 逃生口自带在文案里

    def test_missing_report_url(self):
        self.tel._fetch_remote_config_raw = lambda **_k: {"is_report": True, "report_url": None}
        self.assertIn("reportUrl", self.tel.ping(self.target))

    # ── 请求头环（网关强校验，缺一即静默丢——badcase 头号成因）──
    def test_missing_light_user_is_named(self):
        self.tel._fetch_remote_config_raw = lambda **_k: dict(self.OK_CFG)
        self.tel.collect_identity = lambda _t: {"git_user": None, "git_remote": "git@x:o/r.git"}
        msg = self.tel.ping(self.target)
        self.assertIn("light-user", msg)
        self.assertIn("git user.name", msg)

    def test_missing_light_app_name_is_named(self):
        self.tel._fetch_remote_config_raw = lambda **_k: dict(self.OK_CFG)
        self.tel.collect_identity = lambda _t: {"git_user": "u", "git_remote": None}
        self.assertIn("light-app-name", self.tel.ping(self.target))

    # ── POST 环 ──
    def test_post_failure_reports_raw_reason(self):
        self.tel._fetch_remote_config_raw = lambda **_k: dict(self.OK_CFG)
        import urllib.request
        def boom(*_a, **_k): raise OSError("[Errno 8] nodename nor servname provided")
        urllib.request.urlopen = boom
        msg = self.tel.ping(self.target)
        self.assertIn("nodename nor servname", msg)  # DNS 失败原文透传

    def test_code_zero_passes_and_sends_same_headers_as_report(self):
        self.tel._fetch_remote_config_raw = lambda **_k: dict(self.OK_CFG)
        sent = self._stub_post({"code": 0})
        self.assertIsNone(self.tel.ping(self.target))
        req = sent["req"]
        self.assertEqual(json.loads(req.data.decode("utf-8")), {"event": "ping"})
        # headers 必须与真实上报同源（urllib 会把 header 名首字母大写化）
        self.assertEqual(req.get_header("Light-app-name"), "o/r")
        self.assertEqual(req.get_header("Light-user"), "u")
        self.assertEqual(req.get_header("Light-env"), "local")

    def test_nonzero_code_fails_with_server_message(self):
        self.tel._fetch_remote_config_raw = lambda **_k: dict(self.OK_CFG)
        self._stub_post({"code": 1, "message": "illegal request"})
        msg = self.tel.ping(self.target)
        self.assertIn("服务端未确认", msg)
        self.assertIn("illegal request", msg)

    # ── 重构不回归：fetch_remote_config 仍是 raw 的软失败包装 ──
    def test_fetch_remote_config_still_swallows(self):
        def boom(*_a, **_k): raise OSError("boom")
        self.tel._fetch_remote_config_raw = boom
        self.assertEqual(self.tel.fetch_remote_config(), {})


class ListProfilesPingGateTest(_CliBase):
    """list-profiles 的采集通道闸门：不通即拦，--skip-ping 是逃生口。

    走真 ping（不猴补）：把配置地址指到必然连不上的 127.0.0.1:1（立即 ECONNREFUSED，
    不用等超时），构造出「用户环境不通」这一最高频局面。
    """

    DEAD_URL = "http://127.0.0.1:1/nope"

    def _run(self, *extra, ping_alive: bool):
        env = _cli_env()
        if not ping_alive:
            env.pop("WORKFLOW_SKIP_TELEMETRY", None)  # 放开短路，让真 ping 跑起来
            env["WORKFLOW_TELEMETRY_CONFIG_URL"] = self.DEAD_URL
        return subprocess.run(
            [sys.executable, str(CLI), "--target", str(self.root), "list-profiles", *extra],
            capture_output=True, text=True, env=env,
        )

    def test_ping_ok_prints_nothing_extra(self):
        r = self._run(ping_alive=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("可用编排模式", r.stdout)
        self.assertNotIn("采集通道", r.stdout + r.stderr)  # 通道正常时一个字都不打

    def test_ping_failure_blocks_the_menu(self):
        r = self._run(ping_alive=False)
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("采集通道不通", r.stderr)
        self.assertIn("--skip-ping", r.stderr)          # 逃生口自带在阻断文案里
        # 闸门必须拦在选单之前：先打选单再报错的话，AI 会照着到手的选单往下走
        self.assertNotIn("可用编排模式", r.stdout)
        self.assertNotIn("回复编号选择", r.stdout)

    def test_skip_ping_bypasses_the_gate(self):
        r = self._run("--skip-ping", ping_alive=False)
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("可用编排模式", r.stdout)
        self.assertNotIn("采集通道", r.stdout + r.stderr)


if __name__ == "__main__":
    unittest.main()
