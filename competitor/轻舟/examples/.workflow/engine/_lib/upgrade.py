"""workflow-start 自动升级 lb / lbcli（告知 + 代跑）。

设计约束（与 telemetry.py 同源——cli.py 跑在用户机器上、难更新，故本地逻辑必须"傻"、软失败）：
- 挂在 `/workflow-start` 必调的 `list-profiles` 正门（此刻无进行中流程会被 npm i 打断）。
- 全程软失败：拉取失败/npm 不存在/超时/非法 JSON 一律静默跳过，绝不阻断 list-profiles 输出。
- 不自动升 major：workflow 的 major 由人拍板，跨 major 只提示。
- 升到"同 major 最高正式版"（不是 @latest，那会绕过 major 闸门），channel=stable 时剔 prerelease。
- lbcli major 从属 workflow major（不变式）：lbcli major ≠ workflow major → 无条件对齐到
  workflow major 的最高正式版（修不变式，不受 major 闸门限制）。

两步（顺序：先包升级、后模板对齐——连续跑完再提示重开）：
- ① 包升级（机器级、联网、24h 节流）：全局 `lb -V` / `lbcli -V` vs registry 最高正式版，决定要不要
  npm i -g。升 lbcli 的同时刷 lbcli setup（把 lbcli skill 装到全局 skills 目录）。
  无升级动作也写 lastCheck，避免够不到时每次白跑网络。
- ② 模板对齐（仓库级、纯本地、不节流、承第①步连续跑）：比较本仓库 config.lb_version 与「升级后」
  的全局 lb 版本（第①步升了 lb 就用 lb_target，否则用原全局值）。三态方向闸门，只许朝上刷模板、
  禁止降级；刷前过在途任务闸门（扫 delivery/*/state.json 的 current_step）。

四种情况（lb=@lightboat/workflow, lbcli=@lightboat/cli）：
  lb 低 + lbcli 同 → 升 lb → 刷 workflow 模板
  lb 低 + lbcli 低 → 升两者 + lbcli setup → 刷 workflow 模板
  lb 同 + lbcli 低 → 升 lbcli + lbcli setup（不刷 workflow 模板）
  lb 同 + lbcli 同 → 不动

开关优先级（从高到低）：① 远程 kill switch（false=强制关）→ ② env LIGHTBOAT_AUTO_UPGRADE
→ ③ 本地 config auto_upgrade → ④ 默认 true。远程只关不开（不覆盖用户"我不想升"的本地意愿）。
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import NamedTuple

from . import telemetry

PKG_WORKFLOW = "@lightboat/workflow"
PKG_CLI = "@lightboat/cli"
SCOPE = "@lightboat"  # 私有 scope：registry 解析优先读 @lightboat:registry
# @lightboat/* 只发布在京东内网源，公网 npmjs 上根本没有 → 兜底直接写死内网源，
# 谁都不用配 scope registry；配了 scope/全局源（比如内网域名不同）则优先尊重。
FALLBACK_REGISTRY = "https://registry.m.jd.com"
METADATA_ACCEPT = "application/vnd.npm.install-v1+json"  # 精简 metadata（含全部版本 + dist-tags，约 1/4 体积）
FETCH_TIMEOUT = 3
THROTTLE_FILE = Path.home() / ".lb" / "wf-upgrade.json"
THROTTLE_INTERVAL = 24 * 60 * 60  # 24h（仅约束机器级第 ② 步联网 + npm i）


class SemVer(NamedTuple):
    major: int
    minor: int
    patch: int
    is_prerelease: bool


def parse_semver(v: str) -> SemVer | None:
    """拆 semver：'1.2.3' / '1.2.3-beta.0'。'-' 后为 prerelease。解析不出返回 None。"""
    if not v or not isinstance(v, str):
        return None
    s = v.strip().lstrip("v")
    core, _, pre = s.partition("-")
    parts = core.split(".")
    if len(parts) != 3:
        return None
    try:
        major, minor, patch = (int(p) for p in parts)
    except ValueError:
        return None
    return SemVer(major, minor, patch, bool(pre))


def _cmp_key(v: str):
    """排序键：只按 (major, minor, patch) 比较正式版之间的大小（prerelease 已在上游剔除）。"""
    sv = parse_semver(v)
    return (sv.major, sv.minor, sv.patch) if sv else (-1, -1, -1)


def pick_target(versions: list[str], current_major: int, channel: str) -> str | None:
    """从版本列表取「同 major 最高」：过滤 major==current_major，channel=stable 时剔 prerelease。

    channel=beta 时纳入 prerelease。无候选返回 None。
    """
    cands = []
    for v in versions:
        sv = parse_semver(v)
        if sv is None or sv.major != current_major:
            continue
        if channel != "beta" and sv.is_prerelease:
            continue
        cands.append(v)
    if not cands:
        return None
    return max(cands, key=_cmp_key)


def has_higher_major(versions: list[str], current_major: int) -> str | None:
    """返回更高 major 的最高正式版（仅提示、不自动升）。无则 None。"""
    cands = [v for v in versions
             if (sv := parse_semver(v)) and sv.major > current_major and not sv.is_prerelease]
    if not cands:
        return None
    return max(cands, key=_cmp_key)


def _npm_config(key: str) -> str | None:
    """跑 `npm config get <key>`，返回去尾斜杠的值；缺失 / undefined / null / 失败一律 None。"""
    try:
        r = subprocess.run(["npm", "config", "get", key],
                           capture_output=True, text=True, timeout=FETCH_TIMEOUT)
    except (OSError, subprocess.TimeoutExpired):
        return None
    out = (r.stdout or "").strip()
    if r.returncode != 0 or not out or out in ("undefined", "null"):
        return None
    return out.rstrip("/")


def get_registry() -> str:
    """解析 @lightboat 私有包该用的 registry：先读 scope 专属源 `@lightboat:registry`，
    没配就直接用写死的内网源——**不回落全局 registry**。

    修真 bug + 免配置：@lightboat/* 只在京东内网源发布，公网 npmjs 上根本没有。原实现回落全局
    registry，而企业机器全局源通常是公网 → 对私有包拿到公网源 → fetch 404 → 静默 return，
    内网环境永远检测不到新版。全局 registry 这一级对私有包只会帮倒忙，故直接跳过、写死内网兜底；
    仅当用户为 scope 显式配了源（内网域名不同）才尊重之。
    """
    return _npm_config(f"{SCOPE}:registry") or FALLBACK_REGISTRY


def fetch_metadata(registry: str, pkg: str) -> dict | None:
    """拉精简 metadata（带 Accept header）；软失败返回 None。"""
    import urllib.request

    url = f"{registry.rstrip('/')}/{pkg.replace('/', '%2f')}"
    try:
        req = urllib.request.Request(url, headers={"Accept": METADATA_ACCEPT})
        with urllib.request.urlopen(req, timeout=FETCH_TIMEOUT) as resp:
            return json.loads(resp.read().decode("utf-8"))
    except Exception:
        return None


def versions_of(meta: dict | None) -> list[str]:
    """从 metadata 取所有发布过的版本号。"""
    if not isinstance(meta, dict):
        return []
    return list((meta.get("versions") or {}).keys())


def read_local_versions() -> tuple[str | None, str | None]:
    """跑 `lb -V` / `lbcli -V` 拿本地全局版本。软失败返回 None（复用 cli._detect_cli_version 思路）。"""
    return _detect(["lb", "-V"]), _detect(["lbcli", "-V"])


def _detect(cmd: list[str]) -> str | None:
    if os.environ.get("WORKFLOW_SKIP_VERSION_DETECT") == "1":
        return None
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=5)
    except (OSError, subprocess.TimeoutExpired):
        return None
    if r.returncode != 0 or not r.stdout.strip():
        return None
    return r.stdout.strip().splitlines()[0].strip()


class UpgradePlan(NamedTuple):
    lb_target: str | None       # workflow 同 major 最高正式版（有则升）
    lbcli_target: str | None    # lbcli 目标（对齐 workflow major）
    higher_major: str | None    # workflow 有无更高 major（仅提示）


def decide(lb_local: str | None, lbcli_local: str | None,
           lb_meta: dict | None, lbcli_meta: dict | None, channel: str) -> UpgradePlan:
    """综合决策：产出 lb_target / lbcli_target / higher_major。

    lbcli_target 不变式：lbcli major ≠ workflow major → 无条件对齐到 workflow major 最高正式版；
    否则同（lbcli）major 最高正式版。
    """
    lb_sv = parse_semver(lb_local or "")
    if lb_sv is None:
        return UpgradePlan(None, None, None)
    wf_major = lb_sv.major
    lb_versions = versions_of(lb_meta)
    lbcli_versions = versions_of(lbcli_meta)

    lb_pick = pick_target(lb_versions, wf_major, channel)
    lb_target = lb_pick if lb_pick and _cmp_key(lb_pick) > _cmp_key(lb_local) else None

    lbcli_target = None
    lbcli_sv = parse_semver(lbcli_local or "")
    if lbcli_sv is not None:
        if lbcli_sv.major != wf_major:
            # 不变式修复：lbcli major 对齐 workflow major（无条件，不受 major 闸门限制）
            lbcli_target = pick_target(lbcli_versions, wf_major, channel)
        else:
            pick = pick_target(lbcli_versions, lbcli_sv.major, channel)
            lbcli_target = pick if pick and _cmp_key(pick) > _cmp_key(lbcli_local) else None

    return UpgradePlan(lb_target, lbcli_target, has_higher_major(lb_versions, wf_major))


def remote_kill_switch() -> bool:
    """远程熔断：仅当远程显式 auto_upgrade_remote=False 才返回 True（=熔断）。

    为 true / 缺失 / 拉不到一律 False（不熔断）。这是"只关不开"——远程无法强开本地已关的开关。
    """
    try:
        cfg = telemetry.fetch_remote_config()
    except Exception:
        return False
    return cfg.get("auto_upgrade_remote") is False


# ── 节流（仅约束机器级第 ② 步：联网 + npm i）─────────────────────────────
def _throttled() -> bool:
    try:
        data = json.loads(THROTTLE_FILE.read_text())
        return (time.time() - float(data.get("lastCheck", 0))) < THROTTLE_INTERVAL
    except Exception:
        return False


def _mark_checked() -> None:
    """记一次机器级检查时间戳——无论升没升都记，避免 config>global 够不到时每次白跑网络。"""
    try:
        THROTTLE_FILE.parent.mkdir(parents=True, exist_ok=True)
        THROTTLE_FILE.write_text(json.dumps({"lastCheck": time.time()}))
    except Exception:
        pass


# ── 在途任务闸门（仓库级第 ① 步刷模板前的安全阀）────────────────────────
def has_active_task(target: Path) -> str | None:
    """扫 delivery/*/state.json（不递归、不含 archive），返回第一个 current_step != 'done' 的任务名。

    无在途任务返回 None。用 current_step != 'done' 而非「有没有 state.json」：做完未归档（done 态）
    的任务目录还在外面但已结束，不挡自动刷（对用户更体贴）；严格排除真正在跑的任务。
    """
    delivery = target / "delivery"
    if not delivery.is_dir():
        return None
    for state_file in sorted(delivery.glob("*/state.json")):  # 单层 glob，天然扫不到 archive/<task>/
        try:
            st = json.loads(state_file.read_text())
            current = (st.get("main") or {}).get("current_step")
        except Exception:
            continue  # 读不了/坏 JSON 不当在途，别误挡
        if current is not None and current != "done":
            return state_file.parent.name
    return None


# ── 执行升级 ────────────────────────────────────────────────────────────
def _run(cmd: list[str]) -> bool:
    try:
        r = subprocess.run(cmd, timeout=300)
        return r.returncode == 0
    except (OSError, subprocess.TimeoutExpired):
        return False


def run_upgrade(plan: UpgradePlan, registry: str) -> bool:
    """执行 npm i -g（精确版本号 + registry）；成功不在此刷模板（交回 cli 提示重跑）。

    失败（EACCES 等）→ 打印手动命令 + 返回 False，不吞错、不 raise。
    """
    specs = []
    if plan.lb_target:
        specs.append(f"{PKG_WORKFLOW}@{plan.lb_target}")
    if plan.lbcli_target:
        specs.append(f"{PKG_CLI}@{plan.lbcli_target}")
    if not specs:
        return False

    cmd = ["npm", "i", "-g", *specs, f"--registry={registry}"]
    print(f"→ 发现新版，正在升级：{' '.join(specs)} ...")
    if _run(cmd):
        return True
    print("⚠️  自动升级失败。可手动执行：", file=sys.stderr)
    print(f"     {' '.join(cmd)}", file=sys.stderr)
    return False


def refresh_lbcli_setup() -> bool:
    """升 lbcli 后刷 setup：`lbcli setup -y`（把 lbcli skill 装到全局 skills 目录，幂等）。

    软失败返回 False、打印手动命令，不 raise（与 run_upgrade 同风格）。
    """
    cmd = ["lbcli", "setup", "-y"]
    print("→ 刷新 lbcli setup（lbcli setup -y）...")
    if _run(cmd):
        return True
    print("⚠️  lbcli setup 刷新失败，可手动执行 `lbcli setup -y`。", file=sys.stderr)
    return False


# ── 仓库级第 ① 步：模板对齐（三态方向闸门）──────────────────────────────
def align_template(target: Path, global_lb: str | None) -> str | None:
    """比较本仓库 config.lb_version 与全局 lb -V，只许朝上刷模板、禁止降级。

    返回一个动作码供上层决定后续：'refresh'（安全，需刷）/ 'blocked:<task>'（有在途任务，只提示）/
    None（无需动作：已对齐 / config 超前 / 版本读不出）。刷模板本身由上层 cli 调（避免子进程递归）。
    """
    cfg_v = _read_config_lb_version(target)
    g_sv = parse_semver(global_lb or "")
    c_sv = parse_semver(cfg_v or "")
    if g_sv is None or c_sv is None:
        return None
    cfg_key = (c_sv.major, c_sv.minor, c_sv.patch)
    g_key = (g_sv.major, g_sv.minor, g_sv.patch)
    if cfg_key >= g_key:
        return None  # config == global（对齐）或 config > global（禁止降级，交第 ② 步抬升）
    # config < global：全局更新 → 刷模板，但先过在途任务闸门
    active = has_active_task(target)
    if active:
        return f"blocked:{active}"
    return "refresh"


def _read_config_lb_version(target: Path) -> str | None:
    try:
        cfg = json.loads((target / ".workflow" / "config.json").read_text())
        v = cfg.get("lb_version")
        return v if isinstance(v, str) and v else None
    except Exception:
        return None


# ── 顶层编排 ────────────────────────────────────────────────────────────
def _enabled(config: dict) -> bool:
    """开关判定顺序：① 远程 kill switch → ② env LIGHTBOAT_AUTO_UPGRADE → ③ config → ④ 默认 true。"""
    if remote_kill_switch():
        return False  # 全局熔断，连版本都不用拉
    env = os.environ.get("LIGHTBOAT_AUTO_UPGRADE")
    if env is not None:
        return env not in ("0", "false", "no", "")
    return config.get("auto_upgrade", True) is not False


def _channel(config: dict) -> str:
    ch = os.environ.get("LIGHTBOAT_UPGRADE_CHANNEL") or config.get("upgrade_channel") or "stable"
    return ch if ch in ("stable", "beta") else "stable"


def maybe_auto_upgrade(target: Path, config: dict) -> dict | None:
    """workflow-start 正门入口。返回结果 dict（供 cli 决定是否刷模板/提示重跑）或 None（无动作）。

    顺序：先第①步包升级（升 lb/lbcli，升 lbcli 连带刷 setup），再第②步模板对齐（连续跑）。
    第②步的全局 lb 版本用「升级后」的值（effective_lb）——第①步升了 lb 就是 lb_target，否则原全局值。

    结果 dict 键：
      - 'upgraded': True 表示 npm 包已升级（lb 和/或 lbcli；lbcli setup 已在此刷完）。
      - 'refresh_template': True 表示需跑 lb init --yes 刷 workflow 模板（仓库模板落后于升级后的全局 lb）。
      - 'blocked_task': str 表示有在途任务挡住刷模板 → cli 打印提示、不刷。
      - 'higher_major': str 表示 workflow 有更高 major → cli 提示（不自动升）。
    全程软失败，任何异常上层 try/except 兜住。
    """
    if os.environ.get("WORKFLOW_SKIP_VERSION_DETECT") == "1":
        return None
    if not _enabled(config):
        return None

    channel = _channel(config)
    global_lb, global_lbcli = read_local_versions()
    result: dict = {}
    effective_lb = global_lb  # 第②步模板对齐比对的全局 lb（第①步升了 lb 则抬升为 lb_target）

    # 第①步：机器级包升级（联网、24h 节流）
    if not _throttled():
        registry = get_registry()
        lb_meta = fetch_metadata(registry, PKG_WORKFLOW)
        lbcli_meta = fetch_metadata(registry, PKG_CLI)
        _mark_checked()  # 无论升没升都记（关键：够不到时也别每次白跑）
        if lb_meta is not None or lbcli_meta is not None:
            plan = decide(global_lb, global_lbcli, lb_meta, lbcli_meta, channel)
            if plan.higher_major:
                result["higher_major"] = plan.higher_major
            if (plan.lb_target or plan.lbcli_target) and run_upgrade(plan, registry):
                result["upgraded"] = True
                if plan.lbcli_target:
                    refresh_lbcli_setup()  # 升 lbcli 连带刷 setup（软失败不阻断）
                if plan.lb_target:
                    effective_lb = plan.lb_target  # 用升级后的版本对齐模板（当前进程仍读旧全局）

    # 第②步：仓库级模板对齐（纯本地、不节流、承第①步连续跑）
    action = align_template(target, effective_lb)
    if action == "refresh":
        result["refresh_template"] = True
    elif action and action.startswith("blocked:"):
        result["blocked_task"] = action.split(":", 1)[1]

    return result or None
