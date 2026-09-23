"""节点编排引擎：纯读节点声明字段，不认节点名。

引擎对节点拓扑不做任何合理性假设，只把「节点数组」解析成冻结的运行时对象列表，
所有行为（闸门/停人/执行方式/回流）都由节点自身声明字段驱动。
不同编排通过 .workflow/profile/<名>.json 的多 profile 表达；init 选中一份冻进 state.json。
"""

from __future__ import annotations

import os
import re

DONE = "done"

# 节点名格式：kebab-case（小写字母开头，仅含小写字母/数字/连字符）。
NODE_NAME_RE = re.compile(r"^[a-z][a-z0-9-]*$")

SEVERITIES = ("high", "normal")

# 沉淀归属三分类（团队铁律，硬编码在知识沉淀层；不通用化，见设计二审 E）。
CATEGORIES = ("design", "code", "test")

# 解决方式（自治率轴，与 category 的「根因」轴正交）：这条 issue 从建档到闭环之间，
# AI 是否为它主动向人提过问 / 等过人回话——有就是 asked（含人自己动手改的），没有就是 auto。
# 口径写死在 workflow-feedback-loop skill，别在这里各自解读。
# 注意两个已知的读数陷阱：
#   ① issue-triage 的「仅记录」闭环不走解决路径，resolved_mode 恒 null，算自治率时不进分母；
#   ② 无人区（run-loop 自驱）AI 无人可问、必然记 auto，要和「有人在场仍自己搞定」区分
#      得靠 main.history 里的 run-loop advance 痕迹按时间窗口关联，本字段自己分不出来。
RESOLVE_MODES = ("auto", "asked")

# 同一 issue 自治回流上限：达此次数仍未闭环 → 升级人工（防 run-loop 自驱死循环空转烧 token）。
REROUTE_ESCALATE_THRESHOLD = 3

# 归档节点名。**本模块唯一的主线节点名硬编码**，与「引擎不认节点名」的总原则相悖，
# 故收成一处具名常量、只服务 sandbox_commit_reminder 一个用途：归档是不可逆收口，
# 沙箱下要在**进它之前**提醒人先提交/验证，而「归档是不是最后一步」只有它自己知道。
# 用户自写 profile 若把收尾节点改名，这条提醒静默失效（不报错、不影响流转）——已知取舍。
ARCHIVE_NODE = "workflow-archive"

# ---- 闸门档位（进出通用，值冻进 state.json）----
GATE_AUTO = "auto"
GATE_CONFIRM = "confirm"
GATE_CONFIRM_LOCKED = "confirm-locked"

GATE_ORDER = {
    GATE_AUTO: 0,
    GATE_CONFIRM: 1,
    GATE_CONFIRM_LOCKED: 2,
}
ALL_GATE_VALUES = set(GATE_ORDER)
# 锁定闸值：人工闸，不可被 run-loop 自驱降级（判定看闸门值本身，与「节点是否在某集合」解耦）。
PINNED_GATE_VALUES = {GATE_CONFIRM_LOCKED}

# ---- execution 三态 ----
EXEC_INLINE = "inline"
EXEC_SUBAGENT = "subagent"
EXEC_EXTERNAL_WINDOW = "external-window"
EXECUTION_VALUES = {EXEC_INLINE, EXEC_SUBAGENT, EXEC_EXTERNAL_WINDOW}

HOOK_PHASES = ("enter", "exit")

# 执行单元「文档节点 vs skill」判定：探磁盘，不认名。约定——.workflow/node/<名>/NODE.md
# 存在即文档节点（node_ref 打斜杠别名「/workflow-engine node <名>」，命令内部代指读该 NODE.md），
# 否则视为 skill（打 `/名`）。文档节点是封闭小
# 目录、可枚举；skill 是开放大池子、不好枚举，故探小的那头。判据现读磁盘、永不陈旧，不落任何
# 缓存字段（曾用 SKILL_NODES 白名单硬编码，会与磁盘漂移，已废）。
# 起步节点（explore/planmode/brainstorming/bugfix/test-scope）在状态机外、不经 skill_chain，无需判定。
def _is_doc_node(name: str, target_root=None) -> bool:
    """探 .workflow/node/<名>/NODE.md 是否存在。target_root=None（纯逻辑测试/无根上下文）
    时一律判否 → 全走 skill 形态，保持 node_ref 在无 IO 场景可用。"""
    if target_root is None:
        return False
    return os.path.isfile(os.path.join(str(target_root), ".workflow", "node", name, "NODE.md"))

# 单个节点对象合法字段。
NODE_FIELDS = {"node", "skill", "exit_gate", "entry_gate", "autopilot", "execution", "hooks", "feedback"}
FEEDBACK_FIELDS = {"can_report", "reroute_options", "rerouteable"}


def _stricter(a: str, b: str) -> str:
    return a if GATE_ORDER[a] >= GATE_ORDER[b] else b


# ----------------------------------------------------------------------------
# 冻结 pipeline 上的字段读取（运行时统一形态）
# ----------------------------------------------------------------------------

def _pnode(pipeline: list | None, node: str) -> dict | None:
    """从冻结 pipeline 取某节点对象；不存在返回 None。"""
    for obj in pipeline or []:
        if obj.get("node") == node:
            return obj
    return None


def _seq(pipeline: list | None) -> list:
    """冻结 pipeline 的完整节点名序列。"""
    return [obj["node"] for obj in (pipeline or [])]


def _as_skill_list(val) -> list:
    """归一化 skill 引用：None→[]、字符串→[串]、列表原样（去空）。"""
    if val is None:
        return []
    if isinstance(val, str):
        return [val] if val else []
    if isinstance(val, list):
        return [s for s in val if s]
    return []


def skills_for(pipeline: list | None, node: str) -> list:
    """某节点运行时要跑的执行单元列表（声明 skill / 回退 [node]，恒非空）。
    单元可能是保留为 skill 的节点，也可能是 .workflow/node/ 下的文档节点——
    展示层 node_ref 会按「同名 skill 是否存在」自动区分调用方式。"""
    obj = _pnode(pipeline, node)
    skills = _as_skill_list(obj.get("skill")) if obj else []
    return skills or [node]


def node_ref(name: str, target_root=None) -> str:
    """单个执行单元的调用文本。约定优先、探磁盘定形态：
    .workflow/node/<名>/NODE.md 存在 → 文档节点，打印斜杠别名「/workflow-engine node <名>」
    （该命令内部即代指「读 .workflow/node/<名>/NODE.md 开工」——藏掉路径细节，语义就是「进入该节点」）；
    否则视为 skill，打 `/名`（如 workflow-feedback-loop / workflow-pit-record / open-claude-window）。"""
    if _is_doc_node(name, target_root):
        return f"/workflow-engine node {name}"
    return f"/{name}"


def skill_chain(pipeline: list | None, node: str, target_root=None) -> str:
    """节点执行单元列表的展示文本：单个直出，多个用 → 串联。"""
    return " → ".join(node_ref(s, target_root) for s in skills_for(pipeline, node))


def open_worklet_hint(pipeline: list | None, node: str, target_root=None) -> str:
    """enter 放行后的「开工指令」文本：把「读 NODE.md / 跑哪个 skill」从 PROTOCOL 散文
    挪进机器必经的 enter 输出——门禁放行的时点 AI 注意力必在此，最不该漏这句。
    与 skill_chain（转场预告用 node_ref 打文档节点别名）差在：文档节点这里直接给 Read 路径，
    比别名 `/workflow-engine node <名>` 少一跳（否则 AI 还得再调 cmd_node 才吐路径）；skill 形态给 `/skill`。"""
    parts = []
    for u in skills_for(pipeline, node):
        if _is_doc_node(u, target_root):
            parts.append(f"Read .workflow/node/{u}/NODE.md")
        else:
            parts.append(f"/{u}")
    return " → ".join(parts)


def node_hook(pipeline: list | None, node: str, phase: str) -> list:
    """取某节点某阶段（enter/exit）的钩子执行单元列表；缺省返回空列表。"""
    obj = _pnode(pipeline, node)
    if not obj:
        return []
    return _as_skill_list((obj.get("hooks") or {}).get(phase))


def execution_of(pipeline: list | None, node: str) -> str:
    obj = _pnode(pipeline, node)
    return (obj.get("execution") if obj else None) or EXEC_INLINE


def is_external_window(pipeline: list | None, node: str) -> bool:
    return execution_of(pipeline, node) == EXEC_EXTERNAL_WINDOW


def is_valid_step(step, pipeline: list | None):
    return step in _seq(pipeline) or step == DONE


def active_sequence(pipeline: list | None) -> list:
    """本任务节点序列（options 机制已废，直接返回冻结序列）。"""
    return _seq(pipeline)


def next_step(current, pipeline: list | None):
    if current == DONE:
        return None
    seq = _seq(pipeline)
    if current not in seq:
        return None
    idx = seq.index(current)
    if idx == len(seq) - 1:
        return DONE
    return seq[idx + 1]


# ----------------------------------------------------------------------------
# feedback（回流）字段读取
# ----------------------------------------------------------------------------

def _feedback(pipeline: list | None, node: str) -> dict:
    obj = _pnode(pipeline, node)
    return (obj.get("feedback") if obj else None) or {}


def can_report(pipeline: list | None, node: str) -> bool:
    """节点能否报 issue（取代 FEEDBACK_SOURCES）。"""
    return bool(_feedback(pipeline, node).get("can_report"))


def feedback_sources(pipeline: list | None) -> set:
    """所有可报 issue 的节点 + manual 伪来源。"""
    srcs = {n for n in _seq(pipeline) if can_report(pipeline, n)}
    srcs.add("manual")
    return srcs


def reroute_options_for(pipeline: list | None, node: str) -> list:
    """本节点报出的问题可定向回退到哪些节点（自动 issue 用）。空列表 = 只能原地修。"""
    return list(_feedback(pipeline, node).get("reroute_options") or [])


def is_rerouteable(pipeline: list | None, node: str) -> bool:
    """本节点能否作为回退目标（缺省 true）。"""
    fb = _feedback(pipeline, node)
    return fb.get("rerouteable", True) is not False


def rerouteable_nodes(pipeline: list | None) -> list:
    """所有可作回退目标的节点（manual 自由挑用，宽）。"""
    return [n for n in _seq(pipeline) if is_rerouteable(pipeline, n)]


def is_downstream(pipeline: list | None, current: str, target: str) -> bool:
    """target 是否在 current 下游（含相等之后）——用于回退防前跳。"""
    seq = _seq(pipeline)
    if current not in seq or target not in seq:
        return False
    return seq.index(target) > seq.index(current)


# ----------------------------------------------------------------------------
# 校验：放开 11 节点约束，只拦「会在运行期静默炸」的引用错误
# ----------------------------------------------------------------------------

def _validate_skill_ref(where: str, val):
    if isinstance(val, str):
        return
    if isinstance(val, list) and all(isinstance(s, str) and s for s in val) and val:
        return
    raise ValueError(f"pipeline：{where} 必须是 skill 字符串或非空字符串数组，收到 {val!r}")


def _validate_hooks(node: str, hooks):
    if hooks is None:
        return
    if not isinstance(hooks, dict):
        raise ValueError(f"pipeline：{node}.hooks 必须是对象（含 enter/exit skill 引用），收到 {hooks!r}")
    for phase, skill in hooks.items():
        if phase not in HOOK_PHASES:
            raise ValueError(f"pipeline：{node}.hooks 阶段只能是 {list(HOOK_PHASES)}，收到 {phase!r}")
        _validate_skill_ref(f"{node}.hooks.{phase}", skill)


def _validate_feedback(node: str, fb):
    if fb is None:
        return
    if not isinstance(fb, dict):
        raise ValueError(f"pipeline：{node}.feedback 必须是对象，收到 {fb!r}")
    extra = {k for k in fb if k not in FEEDBACK_FIELDS}
    if extra:
        raise ValueError(f"pipeline：{node}.feedback 含未知字段 {sorted(extra)}（合法：{sorted(FEEDBACK_FIELDS)}）")
    for key in ("can_report", "rerouteable"):
        if key in fb and not isinstance(fb[key], bool):
            raise ValueError(f"pipeline：{node}.feedback.{key} 必须是 true/false，收到 {fb[key]!r}")
    opts = fb.get("reroute_options")
    if opts is not None and (not isinstance(opts, list) or any(not isinstance(s, str) for s in opts)):
        raise ValueError(f"pipeline：{node}.feedback.reroute_options 必须是节点名字符串数组，收到 {opts!r}")


def validate_pipeline(pipeline_cfg):
    """校验单个 profile 的节点数组；坏配置直接抛错（loud fail）。

    只做「引用完整性」，不做「拓扑合理性」（见设计④）。规则：
    1. 非空对象数组；每项含合法 kebab-case node，节点名唯一。
    2. exit_gate/entry_gate ∈ 3 档；execution ∈ 3 态；autopilot 为 bool；skill/hooks 合法。
    3. feedback.reroute_options 指向同 profile 真实节点，且目标 rerouteable != false（矛盾报错）。
    4. execution=external-window 不能声明 autopilot:true（独立窗口不可被 run-loop 自驱跨入）→ 否则报错。
    """
    if not isinstance(pipeline_cfg, list) or not pipeline_cfg:
        raise ValueError(f"profile 必须是非空的有序节点对象数组，收到 {pipeline_cfg!r}")

    names = []
    for obj in pipeline_cfg:
        if not isinstance(obj, dict):
            raise ValueError(f"pipeline 每项必须是对象，收到 {obj!r}")
        node = obj.get("node")
        if not node or not isinstance(node, str):
            raise ValueError(f"pipeline 项缺 node 或 node 非字符串：{obj!r}")
        if not NODE_NAME_RE.match(node):
            raise ValueError(f"pipeline：节点名 {node!r} 须 kebab-case（小写字母开头，仅小写字母/数字/连字符）")
        if node in names:
            raise ValueError(f"pipeline：节点 {node!r} 重复出现")
        names.append(node)

    name_set = set(names)
    for obj in pipeline_cfg:
        node = obj["node"]
        extra = {k for k in obj if k not in NODE_FIELDS and not k.startswith("_")}
        if extra:
            raise ValueError(f"pipeline：{node} 含未知字段 {sorted(extra)}（合法：{sorted(NODE_FIELDS)}）")

        if obj.get("skill") is not None:
            _validate_skill_ref(f"{node}.skill", obj["skill"])

        for gk in ("exit_gate", "entry_gate"):
            g = obj.get(gk)
            if g is not None and g not in ALL_GATE_VALUES:
                raise ValueError(f"pipeline：{node}.{gk} 只能是 {sorted(ALL_GATE_VALUES)}，收到 {g!r}")

        execution = obj.get("execution")
        if execution is not None and execution not in EXECUTION_VALUES:
            raise ValueError(f"pipeline：{node}.execution 只能是 {sorted(EXECUTION_VALUES)}，收到 {execution!r}")

        autopilot = obj.get("autopilot")
        if autopilot is not None and not isinstance(autopilot, bool):
            raise ValueError(f"pipeline：{node}.autopilot 必须是 true/false，收到 {autopilot!r}")

        # external-window 节点不能自驱（独立窗口不可被 run-loop 自驱跨入）。
        exec_v = execution or EXEC_INLINE
        if exec_v == EXEC_EXTERNAL_WINDOW and autopilot is True:
            raise ValueError(
                f"pipeline：{node} 是 execution=external-window 的独立窗口节点，"
                f"不能声明 autopilot:true（独立窗口只能人驱动，不可被 run-loop 自驱跨入）"
            )

        _validate_hooks(node, obj.get("hooks"))
        _validate_feedback(node, obj.get("feedback"))

        fb = obj.get("feedback") or {}
        for opt in (fb.get("reroute_options") or []):
            if opt not in name_set:
                raise ValueError(
                    f"pipeline：{node}.feedback.reroute_options 指向不存在的节点 {opt!r}（同 profile 无此节点）"
                )
            tgt = _pnode(pipeline_cfg, opt)
            tgt_fb = (tgt.get("feedback") if tgt else None) or {}
            if tgt_fb.get("rerouteable") is False:
                raise ValueError(
                    f"pipeline：{node}.feedback.reroute_options 定向回退到 {opt!r}，"
                    f"但 {opt!r} 声明 rerouteable:false（自相矛盾）"
                )


# profile 对象合法字段：nodes（必填，节点数组）+ 声明性字段（可选）。
PROFILE_FIELDS = {"desc", "start_node", "roles", "nodes", "has_e2etest", "enabled"}


def _validate_roles(name: str, roles):
    """roles 可选：dict[str→str]，role 语义名 → 产物区域路径（可含 <task> 占位）。"""
    if roles is None:
        return
    if not isinstance(roles, dict):
        raise ValueError(f"profile {name!r} 的 roles 必须是对象（role名 → 路径字符串），收到 {roles!r}")
    for role, path in roles.items():
        if not isinstance(role, str) or not role:
            raise ValueError(f"profile {name!r} 的 roles 键必须是非空字符串，收到 {role!r}")
        if not isinstance(path, str) or not path:
            raise ValueError(f"profile {name!r} 的 roles[{role!r}] 必须是非空路径字符串，收到 {path!r}")


def _validate_start_node(name: str, start_node):
    """start_node 可选：单个非空 skill 名字符串（该 profile 的起步/触发节点，状态机外）。只做类型校验。"""
    if start_node is None:
        return
    if not isinstance(start_node, str) or not start_node:
        raise ValueError(f"profile {name!r} 的 start_node 必须是非空 skill 名字符串（该模式的起步/触发节点），收到 {start_node!r}")


def _validate_profile_obj(name: str, prof):
    """校验单份 profile 对象结构：nodes 必填走 validate_pipeline，desc/start_node/roles 可选轻校验。

    检测到旧的「节点数组」结构 → loud fail 提示升级（见设计「兼容策略」，不做自动迁移）。
    """
    if isinstance(prof, list):
        raise ValueError(
            f"profile {name!r} 是旧的节点数组结构，已升级为对象 {{desc, start_node, roles, nodes}}——"
            f"把原节点数组挪到 nodes 字段下（desc/start_node/roles 可选）。样例见模板 .workflow/profile/。"
        )
    if not isinstance(prof, dict):
        raise ValueError(f"profile {name!r} 必须是对象 {{desc, start_node, roles, nodes}}，收到 {type(prof).__name__}")
    extra = {k for k in prof if k not in PROFILE_FIELDS and not k.startswith("_")}
    if extra:
        raise ValueError(f"profile {name!r} 含未知字段 {sorted(extra)}（合法：{sorted(PROFILE_FIELDS)}）")
    if "nodes" not in prof:
        raise ValueError(f"profile {name!r} 缺 nodes 字段（有序节点对象数组）")
    validate_pipeline(prof["nodes"])
    _validate_roles(name, prof.get("roles"))
    desc = prof.get("desc")
    if desc is not None and not isinstance(desc, str):
        raise ValueError(f"profile {name!r} 的 desc 必须是字符串，收到 {desc!r}")
    e2e = prof.get("has_e2etest")
    if e2e is not None and not isinstance(e2e, bool):
        raise ValueError(f"profile {name!r} 的 has_e2etest 必须是 true/false，收到 {e2e!r}")
    en = prof.get("enabled")
    if en is not None and not isinstance(en, bool):
        raise ValueError(f"profile {name!r} 的 enabled 必须是 true/false，收到 {en!r}")
    _validate_start_node(name, prof.get("start_node"))


def has_e2etest(prof) -> bool:
    """profile 是否含端到端自动化测试（缺省 true）。false=无 e2e：init 向导不收集环境、deploy 默认走本地。"""
    if not isinstance(prof, dict):
        return True
    return prof.get("has_e2etest", True) is not False


def is_enabled(prof) -> bool:
    """profile 是否对用户可见可选（缺省 true）。false=仅从 list-profiles/向导选单隐藏，
    显式 init --profile <名> 仍可用；仅展示层过滤，引擎/校验层照常认所有已定义 profile。"""
    if not isinstance(prof, dict):
        return True
    return prof.get("enabled", True) is not False


def validate_profiles(profiles, default_profile):
    """校验 profiles 结构（非空对象、每份 profile 对象合法、default_profile 存在且 enabled）。"""
    if not isinstance(profiles, dict):
        raise ValueError(f"profiles 必须是对象（模式名 → profile 对象），收到 {type(profiles).__name__}")
    real = {k: v for k, v in profiles.items() if not k.startswith("_")}
    if not real:
        raise ValueError("profiles 为空（至少定义一个模式，见 .workflow/profile/）")
    for name, prof in real.items():
        _validate_profile_obj(name, prof)
    if not default_profile or default_profile not in real:
        raise ValueError(
            f".workflow/config.json 的 default_profile={default_profile!r} 未指向已定义 profile（现有：{sorted(real)}）"
        )
    if not is_enabled(real[default_profile]):
        raise ValueError(
            f".workflow/config.json 的 default_profile={default_profile!r} 指向的 profile 被禁用（enabled:false），"
            f"默认 profile 必须启用"
        )


# ----------------------------------------------------------------------------
# 解析冻结
# ----------------------------------------------------------------------------

def _freeze_hooks(raw) -> dict:
    """归一化 hooks：保留 enter/exit，值统一成非空 skill 列表（空阶段丢弃）。"""
    out = {}
    for phase in HOOK_PHASES:
        skills = _as_skill_list((raw or {}).get(phase))
        if skills:
            out[phase] = skills
    return out


def _freeze_feedback(raw) -> dict:
    """归一化 feedback：三字段补默认（can_report=false / reroute_options=[] / rerouteable=true）。"""
    fb = raw or {}
    return {
        "can_report": bool(fb.get("can_report", False)),
        "reroute_options": [s for s in (fb.get("reroute_options") or []) if s],
        "rerouteable": fb.get("rerouteable", True) is not False,
    }


def resolve_pipeline(pipeline_cfg) -> list:
    """把一份 profile 的节点数组解析成冻结的完整有序对象列表（运行时统一形态）。

    校验后逐项解析、缺字段补默认（exit_gate 默认 confirm-locked、autopilot 默认 false）、skill 归一为列表。
    """
    validate_pipeline(pipeline_cfg)
    frozen = []
    for obj in pipeline_cfg:
        node = obj["node"]
        exit_gate = obj.get("exit_gate") or GATE_CONFIRM_LOCKED
        entry_gate = obj.get("entry_gate") or GATE_AUTO
        execution = obj.get("execution") or EXEC_INLINE
        autopilot = bool(obj.get("autopilot", False))
        frozen.append({
            "node": node,
            "skill": _as_skill_list(obj.get("skill")) or [node],
            "exit_gate": exit_gate,
            "entry_gate": entry_gate,
            "autopilot": autopilot,
            "execution": execution,
            "hooks": _freeze_hooks(obj.get("hooks")),
            "feedback": _freeze_feedback(obj.get("feedback")),
        })
    return frozen


def resolve_roles(roles: dict | None, task: str) -> dict:
    """把 profile.roles 的路径模板里的 <task> 占位替换成真实任务名，返回冻结 dict。"""
    return {role: path.replace("<task>", task) for role, path in (roles or {}).items()}


def resolve_profile(profiles, profile_name, default_profile) -> list:
    """选中 profile（profile_name 优先，否则 default_profile）→ 返回冻结节点列表。"""
    return resolve_profile_full(profiles, profile_name, default_profile)["pipeline"]


def resolve_profile_full(profiles, profile_name, default_profile) -> dict:
    """选中 profile → 返回 {pipeline（冻结节点列表）, roles（原始 roles dict，未替换 <task>）, start_node（起步节点名，可空）}。"""
    validate_profiles(profiles, default_profile)
    real = {k: v for k, v in profiles.items() if not k.startswith("_")}
    name = profile_name or default_profile
    if name not in real:
        raise ValueError(f"未知 profile {name!r}（现有：{sorted(real)}）")
    prof = real[name]
    return {
        "pipeline": resolve_pipeline(prof["nodes"]),
        "roles": dict(prof.get("roles") or {}),
        "start_node": prof.get("start_node"),
    }


# ----------------------------------------------------------------------------
# 闸门 / 转场
# ----------------------------------------------------------------------------

def resolve_exit_gate(node: str, pipeline: list | None, autonomous: bool = False) -> str:
    obj = _pnode(pipeline, node)
    base = obj["exit_gate"] if obj else GATE_CONFIRM
    if base in PINNED_GATE_VALUES:
        return base  # 出口锁定闸：autonomous 也不降级（含跑完确认类节点）
    if autonomous:
        return GATE_AUTO  # run-loop 自驱：非锁定出口闸一律机械流转
    return base


def transition_gate(finished: str, nxt: str, pipeline: list | None, autonomous: bool = False) -> str:
    if nxt in (None, DONE):
        return GATE_AUTO
    nxt_obj = _pnode(pipeline, nxt)
    entry = nxt_obj["entry_gate"] if nxt_obj else GATE_AUTO
    # 入口闸（如 confirm-locked）经 _stricter 取严，autonomous 下锁定闸仍被保留。
    return _stricter(resolve_exit_gate(finished, pipeline, autonomous), entry)


def _invoke_phrase(nxt: str, pipeline: list | None, target_root=None) -> str:
    """闸门通过后对下一节点的动作短语；独立窗口节点不在本窗口调用。
    chain 项可能是 `/skill`（保留 skill）或「读 .workflow/node/…/NODE.md」（文档节点），
    故统一用「执行」承接，两种形态都通顺。"""
    chain = skill_chain(pipeline, nxt, target_root)
    if is_external_window(pipeline, nxt):
        return f"提示用户新开一个窗口执行 {chain}（独立窗口节点，本窗口不调用、停驻等待）"
    if execution_of(pipeline, nxt) == EXEC_SUBAGENT:
        return f"派一个 subagent 执行 {chain} 并返回蒸馏结论"
    return f"再依次执行 {chain}" if len(skills_for(pipeline, nxt)) > 1 else f"再执行 {chain}"


def gate_instruction(gate: str, finished: str, nxt: str, pipeline: list | None, target_root=None) -> str:
    chain = skill_chain(pipeline, nxt, target_root)
    if gate == GATE_AUTO:
        if is_external_window(pipeline, nxt):
            return f"机械流转 —— {_invoke_phrase(nxt, pipeline, target_root)}，无需额外询问。"
        if execution_of(pipeline, nxt) == EXEC_SUBAGENT:
            return f"机械流转 —— {_invoke_phrase(nxt, pipeline, target_root)}，无需询问用户。"
        return f"机械流转 —— 执行 {chain}，无需询问用户。"
    if gate == GATE_CONFIRM:
        return (f"一键确认 —— 用 AskUserQuestion 问「{finished} 完成，进入 {nxt}？」，"
                f"用户确认后{_invoke_phrase(nxt, pipeline, target_root)}；否则停下。")
    if gate == GATE_CONFIRM_LOCKED:
        return (f"锁定人工闸（不可自动跳过）—— {finished} 产物只有人能判定够不够好，审核要花真实精力。"
                f"把产物要点与审阅入口列给用户后停下等待，不要用 AskUserQuestion 一键确认催促；"
                f"用户明确批准后{_invoke_phrase(nxt, pipeline, target_root)}。")
    return f"未知闸门 {gate}，按 confirm 处理：人工确认后再继续。"


# ----------------------------------------------------------------------------
# 提示行（纯读节点 execution / autopilot 字段）
# ----------------------------------------------------------------------------

def subagent_reminder(node: str, pipeline: list | None) -> str:
    """subagent 驱动节点的下一步提醒行；非此类节点返回空串。"""
    if execution_of(pipeline, node) == EXEC_SUBAGENT:
        return "  （单 subagent 驱动：主窗口派一个 subagent 执行并返回蒸馏结论，详见该 skill「执行方式」）"
    return ""


def external_window_reminder(node: str, pipeline: list | None) -> str:
    """独立窗口节点的下一步提醒行；非此类节点返回空串。"""
    if is_external_window(pipeline, node):
        return ("  （独立窗口节点：本窗口不执行——请用户新开一个窗口运行上述命令，"
                "与本窗口上下文隔离；完成后回本窗口跑 /workflow-engine next 续接）")
    return ""


def run_loop_hint(node: str, pipeline: list | None) -> str:
    """可自驱节点（autopilot）的 run-loop 推荐行；停人节点返回空串。
    纯提示——不改状态、不固化开关。用于「非跨入时刻」的软提醒（如 next 查询时人已在区内）；
    真正跨入无人区那一刻要用 run_loop_zone_entry_confirm（强制确认，不是软提示）。"""
    obj = _pnode(pipeline, node)
    if not obj or not obj.get("autopilot"):
        return ""
    return ("  💡 这段可跑 /workflow-engine run-loop <task> 让它自动连跑+自修，"
            "撞到停人节点（非 autopilot）或 high 问题会自动停回你。")


def sandbox_commit_reminder(nxt: str, pipeline: list | None) -> str:
    """沙箱下「即将进入归档」的提交提醒行；其余情形返回空串。

    时机是**归档的前一个节点 advance 完那一刻**（此函数由调用方在 nxt 上调用），
    刻意早于归档本身：归档是不可逆收口（archive-move 迁走整个 delivery/<task>/），
    而任务刚做完，用户往往还想看看代码、或真机验一下——沙箱里这两件事都不方便，
    通常要先把代码推到远端在本地过。等归档末尾那次兜底提交才提就晚了
    （schema 迁移 / retro / archive-move 都已落盘）。

    纯提示——不改状态、不执行 git、不阻断流转，用户说不需要就照常进归档。
    真正的提交纪律（禁 git add -A、人工确认后 commit + push）在归档节点文档里，
    这里不复述、不代劳。

    沙箱判定与 CLI 侧 execution.ts / telemetry.detect_run_env 同语义（truthy 仅
    1/true/on/yes，trim 后大小写不敏感），但**刻意不复用 detect_run_env**：那条路径认
    WORKFLOW_SKIP_TELEMETRY=1 恒返回 local，借它会让「关掉上报」顺带吃掉这条提醒。
    """
    if nxt != ARCHIVE_NODE or nxt not in _seq(pipeline):
        return ""
    raw = os.environ.get("OPENCLI_SANDBOX_MODE")
    if not (isinstance(raw, str) and raw.strip().lower() in ("1", "true", "on", "yes")):
        return ""
    return ("  💡 沙箱环境 + 下一步就是归档 —— 任务不可能刚做完就直接归档，用户可能需要看看代码或者真实验证一下，所以先别急着进归档："
            "用 AskUserQuestion 问用户要不要**现在**把代码 commit + push 到远端，好在本地过一遍。"
            "（提交纪律见归档节点：禁 git add -A、人工确认后才 commit + push。用户表示不需要就照常进归档，别追问。）")


def is_zone_entry(pipeline: list | None, node: str) -> bool:
    """节点是否「刚进无人区」：自身可自驱（autopilot），前驱不可自驱（或无前驱）。
    advance 落到此节点时精准提示一次 run-loop。"""
    seq = _seq(pipeline)
    if node not in seq:
        return False
    obj = _pnode(pipeline, node)
    if not obj or not obj.get("autopilot"):
        return False
    idx = seq.index(node)
    if idx == 0:
        return True
    prev = _pnode(pipeline, seq[idx - 1])
    return bool(prev and not prev.get("autopilot"))


def _zone_nodes(pipeline: list | None, node: str) -> list:
    """从 node 起连续的 autopilot 节点名列表（含 node 自身），到第一个非 autopilot 节点或序列尾为止。"""
    seq = _seq(pipeline)
    if node not in seq:
        return []
    out = []
    for n in seq[seq.index(node):]:
        obj = _pnode(pipeline, n)
        if not obj or not obj.get("autopilot"):
            break
        out.append(n)
    return out


def run_loop_zone_entry_confirm(node: str, pipeline: list | None) -> str:
    """刚跨入无人区入口（is_zone_entry 为真）时的强制确认指令——不是软提示。

    要求 AI 停下用 AskUserQuestion 问用户是否要进入 run-loop，而不是把选择权悄悄留给自己。
    该问题只决定「AI 怎么驱动」（借 run-loop 命令批跑 vs 自己逐节点跑但更显式汇报进度），
    不改变区内各节点自身的闸门——该 auto 仍是 auto，选「否」也不会被逐节点追加确认，
    因为「无人区无问题顺路自动跑通」是这些节点的既定设计，这道确认只管「要不要让人知情且点头」。
    """
    zone = _zone_nodes(pipeline, node)
    if not zone:
        return ""
    names = "→".join(zone)
    return (
        f"  🛑 即将连续跑过无人区（{names}）——先停下，用 AskUserQuestion 问用户"
        f"「是否要用 run-loop 自动模式跑完这段？」：\n"
        f"     · 选「是」→ 调 /workflow-engine run-loop <task> 驱动到底"
        f"（撞停人节点/high 问题会自动停回你）。\n"
        f"     · 选「否」→ 你自己按 enter→干活→advance 逐节点跑，每步都在聊天里明确汇报进度；"
        f"区内节点闸门本身不受影响（该 auto 仍是 auto，不会因为选「否」而逐节点追加确认）。"
    )


# ----------------------------------------------------------------------------
# run-loop 终态分类
# ----------------------------------------------------------------------------

LOOP_DONE = "DONE"
LOOP_ESCALATED = "ESCALATED"
LOOP_HIGH_ISSUE = "HIGH_ISSUE"
LOOP_TRIAGE = "TRIAGE"
LOOP_PINNED = "PINNED"
LOOP_CONTINUE = "CONTINUE"


def classify_loop(state: dict):
    """run-loop 终态分类（纯 state 判定，不碰 IO）。返回 (status, detail)。

    run-loop ⇒ 自治：非锁定节点出口本就可机械流转，故 run-loop 只在「非自驱」（not autopilot）
    节点停人——走到（advance 进入）非 autopilot 节点即停（统一 PINNED）。

    优先级：DONE > issue 类（open issue 本就锁主线）> 停人节点 > CONTINUE。
    issue 类内部：ESCALATED > HIGH_ISSUE > TRIAGE。
    """
    cur = state["main"]["current_step"]
    if cur == DONE:
        return LOOP_DONE, {}

    open_issues = state["feedback"]["open"]
    if open_issues:
        escalated = [it for it in open_issues
                     if it.get("reroute_count", 0) >= REROUTE_ESCALATE_THRESHOLD]
        if escalated:
            return LOOP_ESCALATED, {"issues": escalated}
        high = [it for it in open_issues if it.get("severity") == "high"]
        if high:
            return LOOP_HIGH_ISSUE, {"issues": high}
        return LOOP_TRIAGE, {"issues": open_issues}

    pipeline = state["pipeline"]
    nxt = next_step(cur, pipeline)
    obj = _pnode(pipeline, cur)
    if obj and not obj.get("autopilot"):
        return LOOP_PINNED, {"step": cur, "next": nxt}
    return LOOP_CONTINUE, {"step": cur, "next": nxt}
