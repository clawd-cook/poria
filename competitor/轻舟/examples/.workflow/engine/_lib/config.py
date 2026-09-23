"""workflow 配置读取：.workflow/config.json（默认 profile + 测试目标环境）+ .workflow/profile/ 编排库。

- 只读，不写（app_info.policy 除外——那是项目级 scaffold 的写入结果，本模块只读不算）。
  项目级配置位于 <target_root>/.workflow/config.json、profile 位于 <target_root>/.workflow/profile/<名>.json。
- profile 名 = 文件名 stem（standard-openspec.json → standard-openspec）。
- profiles 全部来自项目级 .workflow/profile/；用户自写 profile 直接放该目录即可，
  `lb init` 只覆盖框架自带的几份、不删用户自写文件。
- profiles 是必配项（至少一份）；default_profile（config.json）指定 init 不带 --profile 时的默认。
- test_env 缺失时退化为内置默认（has_dedicated=true、target=test）。
- project_type（backend|frontend）缺失退化 backend；项目级固定属性、与 task 无关。
- app_info（{policy, app, ...}）：JDOS 应用信息块。policy 由项目级 `lb init`（scaffold）
  按 project_type 一次性落定写入 config.json，是唯一事实源——本模块不做运行时缺省计算，
  缺失/非法 policy 直接 loud fail（提示重新跑 `lb init`）。返回值补 collected（app+app_id
  均非空才算已采：app 允许兜底回显用户输入的裸文本、app_id 只来自 JDOS 接口真实返回，
  双字段防止半失败/瞎填被误判成已采集）。
- remote_config_files：三态 unknown / true / false。缺省 unknown（未确认）。
  查询/改值走 `cli.py remote-config-files [--set …]`。
"""

from __future__ import annotations

import json
from pathlib import Path

from . import nodes


def config_path(target_root: Path) -> Path:
    return target_root / ".workflow" / "config.json"


def profile_dir(target_root: Path) -> Path:
    return target_root / ".workflow" / "profile"


def _load_profiles_from_dir(d: Path) -> dict:
    """遍历目录下 *.json，profile 名 = 文件名 stem。目录缺失即空 dict（零影响）。"""
    if not d.is_dir():
        return {}
    profiles = {}
    for p in sorted(d.glob("*.json")):
        name = p.stem
        with p.open() as f:
            profiles[name] = json.load(f)
    return profiles


def load_project_config(target_root: Path) -> dict:
    """读 .workflow/config.json + .workflow/profile/，回填 test_env 缺省。

    profiles 全部来自项目级 .workflow/profile/。profiles/default_profile 缺失或非法 loud fail。
    """
    p = config_path(target_root)
    raw = {}
    if p.exists():
        with p.open() as f:
            raw = json.load(f)

    raw_env = raw.get("test_env") or {}
    test_env = {
        "has_dedicated": bool(raw_env.get("has_dedicated", True)),
        "target": raw_env.get("target") or "test",
        "note": raw_env.get("note", ""),
    }

    # 旧顶层键废弃：pipeline/defaults/node_gates/node_hooks 全并入 profiles，检测到即 loud fail。
    for legacy in ("pipeline", "defaults", "node_gates", "node_hooks", "profiles"):
        val = raw.get(legacy)
        has_content = (isinstance(val, dict) and any(not k.startswith("_") for k in val)) or \
                      (isinstance(val, list) and val)
        if has_content:
            raise ValueError(
                f".workflow/config.json 的顶层键 `{legacy}` 已废弃。"
                f"profile 编排改为拆分到 .workflow/profile/<名>.json（每文件一份，名=文件名 stem）；"
                f"config.json 只留 test_env/unittest/default_profile；"
                f"options（propose/autotest/unittest）机制取消，不同组合用不同 profile 表达。"
                f"样例见模板 .workflow/config.json 的 _doc_profiles。"
            )

    profiles = _load_profiles_from_dir(profile_dir(target_root))

    default_profile = raw.get("default_profile")
    nodes.validate_profiles(profiles, default_profile)

    project_type = raw.get("project_type") or "backend"

    return {
        "profiles": profiles,
        "default_profile": default_profile,
        "test_env": test_env,
        "project_type": project_type,
        "app_info": _resolve_app_info(raw.get("app_info")),
        "auto_upgrade": _resolve_auto_upgrade(raw.get("auto_upgrade")),
        "upgrade_channel": _resolve_upgrade_channel(raw.get("upgrade_channel")),
        "lb_version": raw.get("lb_version"),
        "remote_config_files": _resolve_remote_config_files(raw.get("remote_config_files")),
    }


# 自动升级开关：缺省 true。与 app_info.policy 不同——升级开关缺失是正常的（老项目无此字段），
# 退默认即可、不 loud fail。
def _resolve_auto_upgrade(raw) -> bool:
    return False if raw is False else True


def _resolve_remote_config_files(raw) -> str:
    """三态：unknown（未确认）/ true（在远程）/ false（全在本地）。"""
    if raw is False or raw == "false":
        return "false"
    if raw is True or raw == "true":
        return "true"
    return "unknown"


def _encode_remote_config_files(value: str):
    if value == "true":
        return True
    if value == "false":
        return False
    if value == "unknown":
        return None
    raise ValueError(f"remote_config_files 只能是 unknown / true / false，收到：{value!r}")


def read_remote_config_files(target_root: Path) -> str:
    """只读 remote_config_files，不走完整 config 校验。缺文件 / 坏 JSON / 缺字段 → unknown。"""
    p = config_path(target_root)
    if not p.exists():
        return "unknown"
    try:
        with p.open() as f:
            raw = json.load(f)
    except (OSError, json.JSONDecodeError):
        return "unknown"
    if not isinstance(raw, dict):
        return "unknown"
    return _resolve_remote_config_files(raw.get("remote_config_files"))


def write_remote_config_files(target_root: Path, value: str) -> str:
    """把开关写成 JSON 原生 true / false / null，返回归一后的 unknown|true|false。"""
    encoded = _encode_remote_config_files(value)
    p = config_path(target_root)
    if not p.exists():
        raise ValueError(".workflow/config.json 不存在，请先 `lb init`")
    try:
        with p.open() as f:
            raw = json.load(f)
    except (OSError, json.JSONDecodeError) as e:
        raise ValueError(f".workflow/config.json 无法解析：{e}") from e
    if not isinstance(raw, dict):
        raise ValueError(".workflow/config.json 不是对象")
    raw["remote_config_files"] = encoded
    with p.open("w") as f:
        json.dump(raw, f, ensure_ascii=False, indent=2)
        f.write("\n")
    return _resolve_remote_config_files(encoded)


UPGRADE_CHANNELS = ("stable", "beta")


def _resolve_upgrade_channel(raw) -> str:
    return raw if raw in UPGRADE_CHANNELS else "stable"


# app_info.policy 三态：required 强制、suggested 建议、off 不采。
APP_INFO_POLICIES = ("required", "suggested", "off")


def _resolve_app_info(raw_app_info) -> dict:
    """归一 app_info 块：policy 是唯一事实源（由项目级 `lb init` 写入，本函数不做运行时
    缺省计算），非法/缺失直接 loud fail；并算出 app 种子是否已采（app+app_id 均非空才算已采）。

    整块缺失（老项目、无 app_info 键）→ policy 视为缺失，同样 loud fail。
    """
    ai = raw_app_info if isinstance(raw_app_info, dict) else {}
    policy = ai.get("policy")
    if policy not in APP_INFO_POLICIES:
        raise ValueError(
            f".workflow/config.json 的 app_info.policy 非法或缺失：{policy!r}"
            f"（只能是 {' / '.join(APP_INFO_POLICIES)}；缺失/空值请重新跑一次项目级 "
            f"`lb init` 落定该字段，本模块不再做按 project_type 的运行时缺省）"
        )
    app = (ai.get("app") or "").strip()
    app_id = str(ai.get("app_id") or "").strip()
    return {**ai, "policy": policy, "collected": bool(app) and bool(app_id)}
