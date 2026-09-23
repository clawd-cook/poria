"""init 自助向导：纯 input() 问答，仅 cmd_init 在 tty 下调用。

收集：profile（编排模式）、环境、测试范围、HTTP 域名/JSF 别名、eone 特性环境。
无 tty 时 cmd_init 不调用本模块，走默认值；故本模块不处理 EOF 降级。
"""

from __future__ import annotations

from . import state as state_lib

TARGET_LABELS = {"local": "本地", "test": "测试", "gamma": "预发(gamma)"}


def _local_env() -> dict:
    """无 e2e 测试的 profile 的环境块：钉本地、坐标全空。deploy 据 target==local 退化为本地启动。"""
    env = state_lib.default_env()
    env["target"] = "local"
    env["eone_feature_env"] = None
    return env


def _ask(prompt: str, default: str) -> str:
    """问一行，回车取默认。"""
    raw = input(f"{prompt}（默认 {default}）: ").strip()
    return raw or default


def _ask_choice(prompt: str, choices, labels, default: str) -> str:
    opts = " / ".join(f"{c}={labels[c]}" for c in choices)
    while True:
        val = _ask(f"{prompt} [{opts}]", default)
        if val in choices:
            return val
        print(f"  ✗ 只能填 {' / '.join(choices)}，重来")


def _ask_scope(default_scope) -> list:
    default = ",".join(default_scope)
    while True:
        raw = _ask("测试范围（可多选，逗号分隔）[http / jsf]", default)
        items = [s.strip() for s in raw.split(",") if s.strip()]
        if items and all(s in state_lib.ENV_SCOPES for s in items):
            # 去重保序
            seen = []
            for s in items:
                if s not in seen:
                    seen.append(s)
            return seen
        print("  ✗ 只能填 http / jsf 的非空子集，重来")


def _ask_required(prompt: str) -> str:
    while True:
        raw = input(f"{prompt}: ").strip()
        if raw:
            return raw
        print("  ✗ 必填，不能为空")


def _ask_profile(profile_names: list, default_profile: str, descs: dict | None = None) -> str:
    """列出可选 profile（default 高亮 + desc 帮理解），选一个。"""
    descs = descs or {}
    print("可选编排模式（profile）：")
    for name in profile_names:
        mark = "（默认）" if name == default_profile else ""
        print(f"  · {name}{mark}")
        if descs.get(name):
            print(f"      {descs[name]}")
    while True:
        val = _ask("选择编排模式", default_profile)
        if val in profile_names:
            return val
        print(f"  ✗ 只能填 {' / '.join(profile_names)}，重来")


def run_wizard(profile_names: list, default_profile: str, has_dedicated: bool,
               profile_descs: dict | None = None,
               preselected_profile: str | None = None,
               e2etest_by_profile: dict | None = None) -> dict:
    """跑向导，返回 {profile, env}。

    has_dedicated=false（项目只有预发）时跳过环境提问，直接钉 gamma，并连带跳过 eone。
    profile_descs（可选）：profile名 → desc，展示给用户帮理解各模式含哪些环节。
    preselected_profile（可选）：profile 已由 --profile flag 定死时传入——跳过 profile 选择步，只收集环境。
    e2etest_by_profile（可选）：profile名 → 是否含端到端自动化测试（缺省视为 true）。
        选中的 profile 无 e2e 时跳过全部环境提问，env 钉本地默认（target=local）——
        无自动化测试、无需触发部署，环境坐标无处消费。判定在选完 profile 之后，故按选中值查。
    """
    print("── 新任务自助向导（回车取默认）──")
    if preselected_profile is not None:
        profile = preselected_profile
        print(f"  · 编排模式已由 --profile 指定：{profile}（跳过选择，仅收集环境）")
    else:
        profile = _ask_profile(profile_names, default_profile, profile_descs)

    if e2etest_by_profile is not None and e2etest_by_profile.get(profile, True) is False:
        print("  · 该模式无端到端自动化测试、无需触发部署，跳过环境收集（环境钉 本地(local)）")
        return {"profile": profile, "env": _local_env()}


    if has_dedicated:
        target = _ask_choice("运行环境", state_lib.ENV_TARGETS, TARGET_LABELS, "test")
    else:
        target = "gamma"
        print("  · 项目无独立测试环境（has_dedicated=false），环境钉死 预发(gamma)")

    scope = _ask_scope(["http"])

    http_domain = _ask_required("HTTP 域名") if "http" in scope else None
    jsf_alias = _ask_required("JSF 别名(alias)") if "jsf" in scope else None

    eone = None
    if target == "test":
        raw = input("eone 特性环境（非必填，回车跳过）: ").strip()
        eone = raw or None

    env = {
        "target": target,
        "scope": scope,
        "http_domain": http_domain,
        "jsf_alias": jsf_alias,
        "eone_feature_env": eone,
    }
    return {"profile": profile, "env": env}
