#!/usr/bin/env python3
"""upgrade.py 自测：纯函数（semver/pick_target/decide）+ 三态方向闸门 + kill switch + 在途任务闸门。

纯函数为主，避免真联网/真跑 npm。联网/子进程路径用 monkeypatch 打桩。
"""

from __future__ import annotations

import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from _lib import upgrade  # noqa: E402


class ParseSemverTest(unittest.TestCase):
    def test_plain(self):
        self.assertEqual(upgrade.parse_semver("1.2.3"), (1, 2, 3, False))

    def test_prerelease(self):
        sv = upgrade.parse_semver("1.0.3-beta.0")
        self.assertEqual((sv.major, sv.minor, sv.patch), (1, 0, 3))
        self.assertTrue(sv.is_prerelease)

    def test_v_prefix(self):
        self.assertEqual(upgrade.parse_semver("v2.0.0"), (2, 0, 0, False))

    def test_garbage(self):
        self.assertIsNone(upgrade.parse_semver("nope"))
        self.assertIsNone(upgrade.parse_semver("1.2"))
        self.assertIsNone(upgrade.parse_semver(""))
        self.assertIsNone(upgrade.parse_semver(None))


class PickTargetTest(unittest.TestCase):
    VERSIONS = ["1.0.1", "1.0.2", "1.0.3-beta.0", "2.0.0"]

    def test_stable_same_major(self):
        # current_major=1 + stable → 剔 beta、取同 major 最高 = 1.0.2
        self.assertEqual(upgrade.pick_target(self.VERSIONS, 1, "stable"), "1.0.2")

    def test_beta_includes_prerelease(self):
        # channel=beta → 纳入 1.0.3-beta.0（它比 1.0.2 高）
        self.assertEqual(upgrade.pick_target(self.VERSIONS, 1, "beta"), "1.0.3-beta.0")

    def test_no_candidate(self):
        self.assertIsNone(upgrade.pick_target(self.VERSIONS, 5, "stable"))

    def test_higher_major_not_picked_for_same_major(self):
        # 2.0.0 不该被 major=1 的查询选中
        self.assertNotEqual(upgrade.pick_target(self.VERSIONS, 1, "stable"), "2.0.0")


class HasHigherMajorTest(unittest.TestCase):
    def test_finds_higher(self):
        self.assertEqual(upgrade.has_higher_major(["1.0.1", "2.0.0", "2.1.0"], 1), "2.1.0")

    def test_none_when_capped(self):
        self.assertIsNone(upgrade.has_higher_major(["1.0.1", "1.0.2"], 1))

    def test_ignores_higher_major_prerelease(self):
        self.assertIsNone(upgrade.has_higher_major(["1.0.1", "2.0.0-beta.1"], 1))


class DecideTest(unittest.TestCase):
    def _meta(self, versions):
        return {"versions": {v: {} for v in versions}}

    def test_lb_same_major_upgrade_plus_higher_major_hint(self):
        # workflow 1.0.1、同 major 有 1.0.3、另有 2.0.0 → lb_target=1.0.3 + higher_major=2.0.0
        plan = upgrade.decide(
            "1.0.1", "1.0.1",
            self._meta(["1.0.1", "1.0.3", "2.0.0"]),
            self._meta(["1.0.1", "1.0.3"]),
            "stable",
        )
        self.assertEqual(plan.lb_target, "1.0.3")
        self.assertEqual(plan.higher_major, "2.0.0")

    def test_lbcli_major_mismatch_realigns(self):
        # lbcli major=1、workflow major=2 → lbcli_target 指向 workflow major(2) 的最高正式版
        plan = upgrade.decide(
            "2.0.0", "1.5.0",
            self._meta(["2.0.0", "2.0.1"]),
            self._meta(["1.5.0", "2.0.0", "2.0.1"]),
            "stable",
        )
        self.assertEqual(plan.lbcli_target, "2.0.1")

    def test_no_upgrade_when_already_latest(self):
        plan = upgrade.decide(
            "1.0.3", "1.0.3",
            self._meta(["1.0.1", "1.0.3"]),
            self._meta(["1.0.1", "1.0.3"]),
            "stable",
        )
        self.assertIsNone(plan.lb_target)
        self.assertIsNone(plan.lbcli_target)

    def test_local_unparseable_gives_empty_plan(self):
        plan = upgrade.decide(None, None, None, None, "stable")
        self.assertEqual(plan, (None, None, None))


class AlignTemplateTest(unittest.TestCase):
    """仓库级第 ① 步：三态方向闸门（只许朝上刷、禁止降级）+ 在途任务闸门。"""

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())

    def tearDown(self):
        shutil.rmtree(self.root, ignore_errors=True)

    def _write_cfg(self, lb_version):
        wf = self.root / ".workflow"
        wf.mkdir(parents=True, exist_ok=True)
        (wf / "config.json").write_text(json.dumps({"lb_version": lb_version}))

    def _write_state(self, task, current_step, archived=False):
        base = self.root / "delivery"
        if archived:
            base = base / "archive"
        d = base / task
        d.mkdir(parents=True, exist_ok=True)
        (d / "state.json").write_text(json.dumps({"main": {"current_step": current_step}}))

    def test_config_behind_no_active_refresh(self):
        # config < global + 无在途任务 → refresh
        self._write_cfg("1.0.2")
        self.assertEqual(upgrade.align_template(self.root, "1.0.3"), "refresh")

    def test_config_equal_noop(self):
        self._write_cfg("1.0.3")
        self.assertIsNone(upgrade.align_template(self.root, "1.0.3"))

    def test_config_ahead_no_downgrade(self):
        # config > global → 绝不刷模板（禁止降级）→ None
        self._write_cfg("1.0.4")
        self.assertIsNone(upgrade.align_template(self.root, "1.0.3"))

    def test_active_task_blocks_refresh(self):
        # config < global 但有在途任务（current_step != done）→ blocked
        self._write_cfg("1.0.2")
        self._write_state("foo", "workflow-implement")
        self.assertEqual(upgrade.align_template(self.root, "1.0.3"), "blocked:foo")

    def test_done_task_does_not_block(self):
        # 做完未归档（done 态）→ 不挡刷
        self._write_cfg("1.0.2")
        self._write_state("foo", "done")
        self.assertEqual(upgrade.align_template(self.root, "1.0.3"), "refresh")

    def test_no_state_refresh(self):
        self._write_cfg("1.0.2")
        self.assertEqual(upgrade.align_template(self.root, "1.0.3"), "refresh")

    def test_archived_state_not_counted_as_active(self):
        # 只有 delivery/archive/foo/state.json（多一层）→ glob 不递归、不当在途
        self._write_cfg("1.0.2")
        self._write_state("foo", "workflow-implement", archived=True)
        self.assertEqual(upgrade.align_template(self.root, "1.0.3"), "refresh")


class KillSwitchTest(unittest.TestCase):
    """远程只关不开：仅远程显式 False 才熔断。"""

    def test_explicit_false_trips(self):
        upgrade.telemetry.fetch_remote_config = lambda: {"auto_upgrade_remote": False}
        self.assertTrue(upgrade.remote_kill_switch())

    def test_true_does_not_trip(self):
        upgrade.telemetry.fetch_remote_config = lambda: {"auto_upgrade_remote": True}
        self.assertFalse(upgrade.remote_kill_switch())

    def test_missing_does_not_trip(self):
        upgrade.telemetry.fetch_remote_config = lambda: {}
        self.assertFalse(upgrade.remote_kill_switch())

    def test_fetch_raises_does_not_trip(self):
        def boom():
            raise RuntimeError("network")
        upgrade.telemetry.fetch_remote_config = boom
        self.assertFalse(upgrade.remote_kill_switch())


class EnabledTest(unittest.TestCase):
    """开关优先级：远程 kill switch > env > config > 默认 true。仅关不开。"""

    def setUp(self):
        upgrade.telemetry.fetch_remote_config = lambda: {}
        for k in ("LIGHTBOAT_AUTO_UPGRADE",):
            import os
            os.environ.pop(k, None)

    def test_default_true(self):
        self.assertTrue(upgrade._enabled({}))

    def test_config_false(self):
        self.assertFalse(upgrade._enabled({"auto_upgrade": False}))

    def test_env_overrides_config(self):
        import os
        os.environ["LIGHTBOAT_AUTO_UPGRADE"] = "0"
        try:
            # config 说 true，env 说 0 → 关
            self.assertFalse(upgrade._enabled({"auto_upgrade": True}))
        finally:
            os.environ.pop("LIGHTBOAT_AUTO_UPGRADE", None)

    def test_remote_kill_overrides_local_on(self):
        # 远程熔断 → 即便本地开着也关（只关不开的「关」方向）
        upgrade.telemetry.fetch_remote_config = lambda: {"auto_upgrade_remote": False}
        self.assertFalse(upgrade._enabled({"auto_upgrade": True}))

    def test_remote_true_cannot_force_on(self):
        # 远程 true 无法强开本地已关的开关（只关不开的「不开」方向）
        upgrade.telemetry.fetch_remote_config = lambda: {"auto_upgrade_remote": True}
        self.assertFalse(upgrade._enabled({"auto_upgrade": False}))


class ChannelTest(unittest.TestCase):
    def setUp(self):
        import os
        os.environ.pop("LIGHTBOAT_UPGRADE_CHANNEL", None)

    def test_default_stable(self):
        self.assertEqual(upgrade._channel({}), "stable")

    def test_config_beta(self):
        self.assertEqual(upgrade._channel({"upgrade_channel": "beta"}), "beta")

    def test_illegal_falls_back(self):
        self.assertEqual(upgrade._channel({"upgrade_channel": "nightly"}), "stable")

    def test_env_overrides(self):
        import os
        os.environ["LIGHTBOAT_UPGRADE_CHANNEL"] = "beta"
        try:
            self.assertEqual(upgrade._channel({"upgrade_channel": "stable"}), "beta")
        finally:
            os.environ.pop("LIGHTBOAT_UPGRADE_CHANNEL", None)


class RunUpgradeCommandTest(unittest.TestCase):
    """精确断言 npm i 命令行：精确版本（非 @latest）、锁 major、带 --registry（修 registry bug）。

    mock 掉最底层 subprocess.run，捕获真正会执行的 argv，不真跑 npm。
    """

    def setUp(self):
        self._orig_run = upgrade.subprocess.run
        self.calls: list[list[str]] = []

        class _R:
            returncode = 0

        def fake_run(cmd, *a, **kw):
            self.calls.append(list(cmd))
            return _R()

        upgrade.subprocess.run = fake_run

    def tearDown(self):
        upgrade.subprocess.run = self._orig_run

    def test_builds_exact_pinned_command_with_registry(self):
        plan = upgrade.UpgradePlan(lb_target="1.0.3", lbcli_target="1.0.3", higher_major=None)
        ok = upgrade.run_upgrade(plan, "https://registry.m.jd.com")
        self.assertTrue(ok)
        self.assertEqual(self.calls, [[
            "npm", "i", "-g",
            "@lightboat/workflow@1.0.3", "@lightboat/cli@1.0.3",
            "--registry=https://registry.m.jd.com",
        ]])
        # 反向断言：不是 @latest（那会绕过 major 闸门）
        argv = " ".join(self.calls[0])
        self.assertNotIn("@latest", argv)
        self.assertNotIn("registry.npmjs.org", argv)  # 用的是内网源，不是硬编码 npmjs（registry bug 已修）

    def test_only_lb_target_omits_lbcli_spec(self):
        plan = upgrade.UpgradePlan(lb_target="1.0.3", lbcli_target=None, higher_major=None)
        upgrade.run_upgrade(plan, "https://r/")
        self.assertEqual(self.calls[0][:4], ["npm", "i", "-g", "@lightboat/workflow@1.0.3"])
        self.assertNotIn("@lightboat/cli", " ".join(self.calls[0]))

    def test_no_targets_is_noop(self):
        plan = upgrade.UpgradePlan(lb_target=None, lbcli_target=None, higher_major=None)
        self.assertFalse(upgrade.run_upgrade(plan, "https://r/"))
        self.assertEqual(self.calls, [])

    def test_npm_failure_returns_false(self):
        class _R:
            returncode = 1
        upgrade.subprocess.run = lambda *a, **kw: _R()
        plan = upgrade.UpgradePlan(lb_target="1.0.3", lbcli_target=None, higher_major=None)
        self.assertFalse(upgrade.run_upgrade(plan, "https://r/"))


class MaybeAutoUpgradeChainTest(unittest.TestCase):
    """顶层编排 maybe_auto_upgrade：mock 全部 IO，验证「先包升级、后模板对齐」的新顺序 + 四种情况。

    覆盖 lb/lbcli 升级四象限、lbcli setup 连带刷新、effective_lb 连贯传递（升级后版本对齐模板）。
    """

    def setUp(self):
        self.root = Path(tempfile.mkdtemp())
        self._orig = {
            name: getattr(upgrade, name)
            for name in ("read_local_versions", "get_registry", "fetch_metadata",
                         "run_upgrade", "refresh_lbcli_setup", "_throttled",
                         "_mark_checked", "align_template")
        }
        self._orig_remote = upgrade.telemetry.fetch_remote_config
        upgrade.telemetry.fetch_remote_config = lambda: {}  # 默认远程不熔断
        self.upgrade_calls: list = []
        self.setup_calls: list = []
        self.align_args: list = []  # 捕获 align_template 收到的 effective_lb
        self.align_return = None    # 可被用例覆盖：'refresh' / 'blocked:X' / None
        self.marked = []
        upgrade._mark_checked = lambda: self.marked.append(True)
        upgrade.run_upgrade = lambda plan, reg: (self.upgrade_calls.append((plan, reg)) or True)
        upgrade.refresh_lbcli_setup = lambda: (self.setup_calls.append(True) or True)
        upgrade.align_template = lambda target, gl: (self.align_args.append(gl) or self.align_return)
        for k in ("LIGHTBOAT_AUTO_UPGRADE", "LIGHTBOAT_UPGRADE_CHANNEL",
                  "WORKFLOW_SKIP_VERSION_DETECT"):
            import os
            os.environ.pop(k, None)

    def tearDown(self):
        for name, fn in self._orig.items():
            setattr(upgrade, name, fn)
        upgrade.telemetry.fetch_remote_config = self._orig_remote
        shutil.rmtree(self.root, ignore_errors=True)

    def _meta(self, versions):
        return {"versions": {v: {} for v in versions}}

    # ── 四种情况 ──────────────────────────────────────────────────────────
    def test_case1_lb_low_lbcli_same(self):
        # lb 低(1.0.1→1.0.3) + lbcli 同(1.0.3) → 升 lb、不刷 setup、effective_lb=lb_target、刷模板
        self.align_return = "refresh"
        upgrade.read_local_versions = lambda: ("1.0.1", "1.0.3")
        upgrade.get_registry = lambda: "https://registry.m.jd.com"
        upgrade.fetch_metadata = lambda reg, pkg: (
            self._meta(["1.0.1", "1.0.3"]) if pkg == upgrade.PKG_WORKFLOW
            else self._meta(["1.0.3"]))
        upgrade._throttled = lambda: False

        result = upgrade.maybe_auto_upgrade(self.root, {})
        self.assertEqual(result, {"upgraded": True, "refresh_template": True})
        self.assertEqual(len(self.upgrade_calls), 1)
        self.assertEqual(self.upgrade_calls[0][0].lb_target, "1.0.3")
        self.assertIsNone(self.upgrade_calls[0][0].lbcli_target)
        self.assertEqual(self.setup_calls, [])          # lbcli 没升 → 不刷 setup
        self.assertEqual(self.align_args[-1], "1.0.3")  # 第②步用升级后的 lb 版本对齐

    def test_case2_lb_low_lbcli_low(self):
        # lb 低 + lbcli 低 → 升两者 + 刷 setup + 刷模板
        self.align_return = "refresh"
        upgrade.read_local_versions = lambda: ("1.0.1", "1.0.1")
        upgrade.get_registry = lambda: "https://r/"
        upgrade.fetch_metadata = lambda reg, pkg: self._meta(["1.0.1", "1.0.3"])
        upgrade._throttled = lambda: False

        result = upgrade.maybe_auto_upgrade(self.root, {})
        self.assertEqual(result, {"upgraded": True, "refresh_template": True})
        plan = self.upgrade_calls[0][0]
        self.assertEqual(plan.lb_target, "1.0.3")
        self.assertEqual(plan.lbcli_target, "1.0.3")
        self.assertEqual(self.setup_calls, [True])      # lbcli 升了 → 刷 setup
        self.assertEqual(self.align_args[-1], "1.0.3")

    def test_case3_lb_same_lbcli_low(self):
        # lb 同(1.0.3) + lbcli 低(1.0.1→1.0.3) → 只升 lbcli + 刷 setup、effective_lb 不变、不刷模板
        self.align_return = None  # config == effective_lb（workflow 模板未落后）
        upgrade.read_local_versions = lambda: ("1.0.3", "1.0.1")
        upgrade.get_registry = lambda: "https://r/"
        upgrade.fetch_metadata = lambda reg, pkg: self._meta(["1.0.1", "1.0.3"])
        upgrade._throttled = lambda: False

        result = upgrade.maybe_auto_upgrade(self.root, {})
        self.assertEqual(result, {"upgraded": True})    # 无 refresh_template
        plan = self.upgrade_calls[0][0]
        self.assertIsNone(plan.lb_target)               # lb 没升
        self.assertEqual(plan.lbcli_target, "1.0.3")
        self.assertEqual(self.setup_calls, [True])
        self.assertEqual(self.align_args[-1], "1.0.3")  # lb 没升 → 用原全局值

    def test_case4_all_same_noop(self):
        # lb 同 + lbcli 同 → 无升级动作、无模板刷新 → None
        self.align_return = None
        upgrade.read_local_versions = lambda: ("1.0.3", "1.0.3")
        upgrade.get_registry = lambda: "https://r/"
        upgrade.fetch_metadata = lambda reg, pkg: self._meta(["1.0.1", "1.0.3"])
        upgrade._throttled = lambda: False

        self.assertIsNone(upgrade.maybe_auto_upgrade(self.root, {}))
        self.assertEqual(self.upgrade_calls, [])
        self.assertEqual(self.setup_calls, [])

    # ── 开关 / 节流 / 模板对齐边界 ────────────────────────────────────────
    def test_remote_kill_switch_short_circuits_before_any_io(self):
        # 远程熔断 → 直接 return，连版本/registry/align 都不碰
        upgrade.telemetry.fetch_remote_config = lambda: {"auto_upgrade_remote": False}
        def boom(*a, **kw):
            raise AssertionError("熔断后不该碰任何 IO")
        upgrade.read_local_versions = boom
        upgrade.get_registry = boom
        upgrade.align_template = boom
        self.assertIsNone(upgrade.maybe_auto_upgrade(self.root, {"auto_upgrade": True}))

    def test_template_behind_only_refreshes_without_pkg_upgrade(self):
        # 包已最新（第①步无动作）但仓库模板落后 → 第②步仍 refresh（不 upgraded）
        self.align_return = "refresh"
        upgrade.read_local_versions = lambda: ("1.0.3", "1.0.3")
        upgrade.get_registry = lambda: "https://r/"
        upgrade.fetch_metadata = lambda reg, pkg: self._meta(["1.0.1", "1.0.3"])
        upgrade._throttled = lambda: False
        self.assertEqual(upgrade.maybe_auto_upgrade(self.root, {}), {"refresh_template": True})
        self.assertEqual(self.upgrade_calls, [])

    def test_blocked_task_when_template_refresh_blocked(self):
        self.align_return = "blocked:foo"
        upgrade.read_local_versions = lambda: ("1.0.3", "1.0.3")
        upgrade.get_registry = lambda: "https://r/"
        upgrade.fetch_metadata = lambda reg, pkg: self._meta(["1.0.3"])
        upgrade._throttled = lambda: False
        self.assertEqual(upgrade.maybe_auto_upgrade(self.root, {}), {"blocked_task": "foo"})

    def test_throttled_skips_pkg_upgrade_but_template_still_aligns(self):
        # 节流内跳过第①步联网/升包，但第②步模板对齐仍连续跑（用原全局 lb）
        self.align_return = "refresh"
        upgrade.read_local_versions = lambda: ("1.0.3", "1.0.3")
        upgrade._throttled = lambda: True
        def boom(*a, **kw):
            raise AssertionError("节流内不该联网")
        upgrade.get_registry = boom
        upgrade.fetch_metadata = boom
        self.assertEqual(upgrade.maybe_auto_upgrade(self.root, {}), {"refresh_template": True})
        self.assertEqual(self.align_args[-1], "1.0.3")  # 用原全局值对齐

    def test_fetch_fails_marks_checked_and_still_aligns(self):
        # registry 拉不到 → 记节流 + 第②步 align 照跑（align_return=None → 整体 None）
        self.align_return = None
        upgrade.read_local_versions = lambda: ("1.0.1", "1.0.1")
        upgrade.get_registry = lambda: "https://r/"
        upgrade.fetch_metadata = lambda reg, pkg: None
        upgrade._throttled = lambda: False
        self.assertIsNone(upgrade.maybe_auto_upgrade(self.root, {}))
        self.assertTrue(self.marked)          # 拉不到也写了 lastCheck
        self.assertEqual(self.upgrade_calls, [])
        self.assertEqual(len(self.align_args), 1)  # 第②步仍跑了

    def test_skip_env_disables_entirely(self):
        import os
        os.environ["WORKFLOW_SKIP_VERSION_DETECT"] = "1"
        try:
            def boom(*a, **kw):
                raise AssertionError("SKIP 下不该做任何事")
            upgrade.align_template = boom
            upgrade.read_local_versions = boom
            self.assertIsNone(upgrade.maybe_auto_upgrade(self.root, {}))
        finally:
            os.environ.pop("WORKFLOW_SKIP_VERSION_DETECT", None)


class RefreshLbcliSetupTest(unittest.TestCase):
    """升 lbcli 后刷 setup：跑 `lbcli setup -y`，软失败不 raise。"""

    def setUp(self):
        self._orig_run = upgrade._run
        self.calls: list[list[str]] = []

    def tearDown(self):
        upgrade._run = self._orig_run

    def test_runs_setup_command(self):
        upgrade._run = lambda cmd: (self.calls.append(list(cmd)) or True)
        self.assertTrue(upgrade.refresh_lbcli_setup())
        self.assertEqual(self.calls, [["lbcli", "setup", "-y"]])

    def test_soft_fails_on_nonzero(self):
        upgrade._run = lambda cmd: False
        self.assertFalse(upgrade.refresh_lbcli_setup())


class GetRegistryTest(unittest.TestCase):
    """registry 解析：@lightboat:registry 优先 → 写死内网源兜底（**不回落全局 registry**）。

    mock subprocess.run，按 `npm config get <key>` 的 key 返回不同值，验证优先级。
    """

    def setUp(self):
        self._orig = upgrade.subprocess.run

    def tearDown(self):
        upgrade.subprocess.run = self._orig

    def _stub_by_key(self, mapping):
        """mapping: {config_key: (stdout, returncode)}；未列出的 key 返回 undefined（模拟未配）。"""
        class _R:
            pass

        def fake_run(cmd, *a, **kw):
            key = cmd[3] if len(cmd) > 3 else ""
            stdout, rc = mapping.get(key, ("undefined\n", 0))
            r = _R()
            r.stdout = stdout
            r.returncode = rc
            return r
        upgrade.subprocess.run = fake_run

    def test_scope_registry_wins(self):
        # 配了 scope 源（内网域名可能不同）→ 尊重之，不用写死兜底
        self._stub_by_key({
            "@lightboat:registry": ("https://registry.other.jd.com/\n", 0),
            "registry": ("https://registry.npmjs.org/\n", 0),
        })
        self.assertEqual(upgrade.get_registry(), "https://registry.other.jd.com")

    def test_no_scope_uses_hardcoded_internal_not_global(self):
        # 关键：未配 scope + 全局源是公网 → 直接用写死内网源，**绝不回落公网全局**（正是原 bug）
        self._stub_by_key({"registry": ("https://registry.npmjs.org/\n", 0)})
        self.assertEqual(upgrade.get_registry(), upgrade.FALLBACK_REGISTRY)
        self.assertEqual(upgrade.FALLBACK_REGISTRY, "https://registry.m.jd.com")

    def test_both_missing_uses_hardcoded_internal(self):
        self._stub_by_key({})
        self.assertEqual(upgrade.get_registry(), "https://registry.m.jd.com")

    def test_scope_undefined_uses_hardcoded_internal(self):
        # scope 显式 undefined → 写死内网，不看全局
        self._stub_by_key({
            "@lightboat:registry": ("undefined\n", 0),
            "registry": ("https://registry.npmjs.org/\n", 0),
        })
        self.assertEqual(upgrade.get_registry(), "https://registry.m.jd.com")

    def test_strips_trailing_slash(self):
        self._stub_by_key({"@lightboat:registry": ("https://registry.m.jd.com/\n", 0)})
        self.assertEqual(upgrade.get_registry(), "https://registry.m.jd.com")

    def test_nonzero_scope_uses_hardcoded_internal(self):
        self._stub_by_key({
            "@lightboat:registry": ("", 1),
            "registry": ("https://registry.npmjs.org/\n", 0),
        })
        self.assertEqual(upgrade.get_registry(), "https://registry.m.jd.com")


if __name__ == "__main__":
    unittest.main()
