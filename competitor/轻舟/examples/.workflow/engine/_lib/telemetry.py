"""workflow-engine 数据采集与上报（极简、非阻塞、可远程管控）。

设计约束：cli.py 跑在用户电脑上、难更新，故本地逻辑必须"傻"——只做读文件 + POST，
不做解析/聚合（放服务端）；开关与上报地址走远程配置中心化管控（改开关/换地址不用用户升级）。

- 采集点：state.json（唯一任务级真相源），上报全量原文（内网、env 不脱敏）。
- 开关/地址来源：REMOTE_CONFIG_URL 指向的远程 JSON（外部约定，key 用驼峰
  {"data":[{"isReport":bool,"reportUrl":"..."}]}）。init 时拉一次，在读取边界翻成 snake_case
  （is_report/report_url）冻进 state.json；后续流转只读冻结的 snake_case 值、零网络拉取。
- 上报网关（light-data-server）按公司网关惯例强校验三个请求头，缺一即被判 illegal：
  light-app-name（项目标识，语义固定取 init 时冻结的 identity.git_remote 解析出的 org/repo，
    如 lightboat/lightboat-framework——所有项目都必有的稳定标识；无 remote 则不带该头。
    取代过去的 jdos_ 应用名：前端应用普遍无 JDOS 应用，git remote 才通用）、
  light-env（运行环境标识：随 OPENCLI_SANDBOX_MODE 传 "sandbox"/"local"，见 detect_run_env；
    网关接受这两个值——实测 env=sandbox 被原样解析）、
  light-user（取 init 时冻结的 git 身份 identity.git_user，可能为空则不带该头）。
- 传输：spawn 一个 detached python 子进程用 urllib 发 POST，父进程立即返回。
  失败静默、不重试、不阻塞、不抛错——上报绝不能拖垮用户正在跑的 init/advance。
- 探测（ping）：上面这条旁路的代价是链路上没有任何一环会说话——采集不上来时用户
  和我们都不知情。ping() 是它的反面（同步、in-process、失败大声、默认阻断），
  在起新任务前验一次通道；两者共用 _report_headers 保证同源。见文件末尾那节。
- WORKFLOW_SKIP_TELEMETRY=1 时全部短路（自测用：不拉网络、不 fork 子进程）。
- WORKFLOW_TELEMETRY_DEBUG=1 时把实际发送的 url/headers/body（及发送异常）追加写到
  /tmp/workflow-telemetry-debug.log，默认关闭、不影响发送逻辑，纯本地调试用。
"""

from __future__ import annotations

import copy
import json
import os
import subprocess
import sys
from pathlib import Path

# "配置的配置"——唯一硬编码的 URL：去哪拉「开关 + 真实上报地址」。
# 换上报服务器只需改这个远程 JSON 的 reportUrl，不用用户升级 cli。
REMOTE_CONFIG_URL = (
    "http://storage.360buyimg.com/miniprogram-platform/"
    "configure-lowcode-platform-node/prod/json/prod/"
    "16782b6c-abe2-4ae8-b309-bbaeb68b5e4d.json"
)

# 子进程发送脚本：从 stdin 读 {url, body, headers}，POST 后退出；任何异常吞掉。
# WORKFLOW_TELEMETRY_DEBUG=1 时额外把请求内容/异常追加写到 /tmp 调试日志，不影响发送本身。
_SEND_SCRIPT = (
    "import sys,os,json,urllib.request\n"
    "debug=os.environ.get('WORKFLOW_TELEMETRY_DEBUG')=='1'\n"
    "log='/tmp/workflow-telemetry-debug.log'\n"
    "try:\n"
    "    d=json.load(sys.stdin)\n"
    "    if debug:\n"
    "        with open(log,'a') as f:\n"
    "            f.write(json.dumps({'url':d['url'],'headers':d['headers'],"
    "'body':d['body']},ensure_ascii=False)+'\\n')\n"
    "    req=urllib.request.Request(d['url'],"
    "data=json.dumps(d['body'],ensure_ascii=False).encode('utf-8'),"
    "headers=d['headers'],method='POST')\n"
    "    urllib.request.urlopen(req,timeout=10)\n"
    "except Exception as e:\n"
    "    if debug:\n"
    "        with open(log,'a') as f:\n"
    "            f.write('ERROR '+repr(e)+'\\n')\n"
)


def _skip() -> bool:
    return os.environ.get("WORKFLOW_SKIP_TELEMETRY") == "1"


def _git(target_root: Path, args: list[str]) -> str | None:
    """跑一条 git 命令取 stdout 首行；命令不存在/报错/超时/空输出都软失败返回 None，绝不阻断 init。"""
    try:
        result = subprocess.run(
            ["git", "-C", str(target_root), *args],
            capture_output=True, text=True, timeout=5,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    out = result.stdout.strip()
    return out.splitlines()[0].strip() if out else None


def collect_identity(target_root: Path) -> dict:
    """init 时一次性收集 git 身份，冻进 state.json 供上报带出。

    每项软失败为 None，不阻断 init。内网实名、不做哈希。
    git_user 取 user.name 作主标识；git_remote 作项目标识（无 remote 则 None）；
    git_branch 作分析维度（不进服务端主键）。
    """
    if _skip():
        return {}
    return {
        "git_user": _git(target_root, ["config", "user.name"]),
        "git_email": _git(target_root, ["config", "user.email"]),
        "git_remote": _git(target_root, ["remote", "get-url", "origin"]),
        "git_branch": _git(target_root, ["rev-parse", "--abbrev-ref", "HEAD"]),
    }


def detect_run_env() -> str:
    """判断上报客户端此刻跑在沙箱还是用户本地，复用 CLI 侧 OPENCLI_SANDBOX_MODE 语义（execution.ts）。

    truthy（1/true/on/yes，trim 后大小写不敏感）→ "sandbox"，否则 "local"。
    这是唯一真相源，不发明新值；WORKFLOW_SKIP_TELEMETRY 自测时恒 "local"。
    """
    if _skip():
        return "local"
    raw = os.environ.get("OPENCLI_SANDBOX_MODE")
    if isinstance(raw, str) and raw.strip().lower() in ("1", "true", "on", "yes"):
        return "sandbox"
    return "local"


COUNT_KEYS = (
    "input_tokens",
    "output_tokens",
    "cache_creation_input_tokens",
    "cache_read_input_tokens",
)


def _empty_counts() -> dict:
    c = {k: 0 for k in COUNT_KEYS}
    c["total_tokens"] = 0
    return c


def _counts_from_usage(u: dict) -> dict | None:
    if not isinstance(u, dict):
        return None
    try:
        c = {k: int(u.get(k, 0) or 0) for k in COUNT_KEYS}
    except (TypeError, ValueError):
        return None
    c["total_tokens"] = (
        c["input_tokens"] + c["output_tokens"]
        + c["cache_creation_input_tokens"] + c["cache_read_input_tokens"]
    )
    return c


def _water_level_from_usage(u) -> int | None:
    if not isinstance(u, dict):
        return None
    try:
        return (int(u.get("input_tokens", 0) or 0)
                + int(u.get("cache_creation_input_tokens", 0) or 0)
                + int(u.get("cache_read_input_tokens", 0) or 0))
    except (TypeError, ValueError):
        return None


def _add_counts(a: dict, b: dict) -> dict:
    c = {k: a.get(k, 0) + b.get(k, 0) for k in COUNT_KEYS}
    c["total_tokens"] = (
        c["input_tokens"] + c["output_tokens"]
        + c["cache_creation_input_tokens"] + c["cache_read_input_tokens"]
    )
    return c


def _counts_block(src: dict | None) -> dict:
    src = src if isinstance(src, dict) else {}
    c = {k: int(src.get(k, 0) or 0) for k in COUNT_KEYS}
    c["total_tokens"] = (
        c["input_tokens"] + c["output_tokens"]
        + c["cache_creation_input_tokens"] + c["cache_read_input_tokens"]
    )
    return c


def _models_block(src: dict | None) -> dict:
    if not isinstance(src, dict):
        return {}
    return {str(model): _counts_block(counts) for model, counts in src.items()}


def _collect_usage_line(obj: dict, by_id: dict) -> None:
    try:
        msg = obj.get("message") or {}
        u = msg.get("usage")
        if not u:
            return
        mid = msg.get("id")
        if not mid:
            return
        counts = _counts_from_usage(u)
        if counts is None:
            return
        model = msg.get("model") or "unknown"
        prev = by_id.get(mid)
        if prev is None or counts["output_tokens"] > prev[0]["output_tokens"]:
            by_id[mid] = (counts, model)
    except Exception:
        return


def _collect_usage_from_file(path: Path, by_id: dict) -> None:
    try:
        for line in path.read_text(encoding="utf-8").splitlines():
            try:
                obj = json.loads(line)
            except Exception:
                continue
            _collect_usage_line(obj, by_id)
    except Exception:
        return


def _build_token_usage(sid: str, by_id: dict) -> dict:
    total = _empty_counts()
    by_model: dict[str, dict] = {}
    for _mid, (counts, model) in by_id.items():
        total = _add_counts(total, counts)
        if model not in by_model:
            by_model[model] = _empty_counts()
        by_model[model] = _add_counts(by_model[model], counts)
    return {"session_id": sid, "summary": total, "models": by_model}


def apply_token_snapshot(state: dict, snapshot: dict | None) -> None:
    try:
        if not snapshot:
            return
        sid = snapshot.get("session_id")
        if not sid:
            return
        tu = state.setdefault("token_usage", {})
        if not isinstance(tu, dict):
            tu = {}
            state["token_usage"] = tu
        sessions = tu.setdefault("by_session", {})
        if not isinstance(sessions, dict):
            sessions = {}
        sessions[sid] = {
            "summary": _counts_block(snapshot.get("summary")),
            "models": _models_block(snapshot.get("models")),
        }
        total = _empty_counts()
        models: dict = {}
        for b in sessions.values():
            if not isinstance(b, dict):
                continue
            total = _add_counts(total, b.get("summary") or {})
            for model, counts in (b.get("models") or {}).items():
                models[model] = _add_counts(models.get(model) or _empty_counts(), counts)
        tu["summary"] = total
        tu["models"] = models
        tu["by_session"] = sessions
        for k in COUNT_KEYS + ("total_tokens", "by_model", "session_id"):
            tu.pop(k, None)
    except Exception:
        return


def token_usage_for_report(tu: dict | None) -> dict | None:
    """上报用：只保留 summary / models，丢掉本地累加用的 by_session。"""
    if not isinstance(tu, dict):
        return tu
    return {
        "summary": tu.get("summary") or _empty_counts(),
        "models": tu.get("models") or {},
    }


def _payload_for_report(payload: dict) -> dict:
    body = copy.deepcopy(payload)
    st = body.get("state")
    if isinstance(st, dict) and "token_usage" in st:
        st["token_usage"] = token_usage_for_report(st.get("token_usage"))
    return body


def collect_context_used() -> dict:
    """一次读会话 transcript，采集上下文占用/会话 id/模型/对话轮次，供流转/init 客观记录进 state.json。

    机制：Claude Code 给子进程透传 env CLAUDE_CODE_SESSION_ID（本工具跑在其工具子进程链里）；
    据此在 ~/.claude/projects/*/<session_id>.jsonl 定位当前会话 transcript，一次遍历取五项：
      - context_used：「末条带 usage 的 assistant 消息」三项 token 之和，即此刻喂进模型的上下文占用量：
          input_tokens + cache_creation_input_tokens + cache_read_input_tokens
        （output_tokens 是模型产出、不算上下文占用，不计入。只报客观 used、不报总量——总量
         context_window_size 只在 Claude Code 推给 statusLine 的 stdin，子进程/hook 拿不到，不猜、不算 ratio。）
      - baseline_context_used：**首条非零**水位，即"什么都还没干"时的起步占用——system prompt +
        常驻指令文档（CLAUDE.md/AGENTS.md）+ 工具定义 + skills 索引 + 用户第一条消息。
        与 context_used 同一算式，差值即整段会话真实烧掉的量。
        取「非零」而不是「首条」：Claude Code 会插 model=<synthetic> 的合成消息（非真实请求），
        它带 usage 但四项全零，当基线会把 12k+ 的真实占用误报成 0。实测 20 个会话里 3 个首条即合成。
      - model：末条带 message.model 的 assistant 消息的模型标识（如 claude-opus-4-8）。
      - turns：真实用户对话轮次——type==user 且 role==user、非 isMeta、非 isSidechain，
        且 content 非 tool_result（str，或 list 且不含 tool_result block）的条数。

    返回 {context_used, baseline_context_used, session_id, model, turns, token_usage}，session 字段同源绑定。
    任一步失败（无 env / 找不到文件 / 解析异常）返回六项全 None——纯客观采集，绝不阻断流转。
    token_usage 汇总主 jsonl + subagents/agent-*.jsonl，按 message.id 去重（无 id 丢弃，同 id 留更大 output_tokens）。
    """
    blank = {
        "context_used": None, "baseline_context_used": None,
        "session_id": None, "model": None, "turns": None,
        "token_usage": None,
    }
    if _skip():
        return blank
    sid = os.environ.get("CLAUDE_CODE_SESSION_ID")
    if not sid:
        return blank
    matches = list((Path.home() / ".claude" / "projects").glob(f"*/{sid}.jsonl"))
    if not matches:
        return blank
    main_path = matches[0]
    used = None
    baseline = None
    model = None
    turns = 0
    main_has_usage = False
    by_id: dict = {}
    try:
        for line in main_path.read_text(encoding="utf-8").splitlines():
            try:
                obj = json.loads(line)
            except Exception:
                continue
            msg = obj.get("message") or {}
            u = msg.get("usage")
            if u:
                wl = _water_level_from_usage(u)
                if wl is not None:
                    main_has_usage = True
                    used = wl
                    if baseline is None and wl > 0:
                        baseline = wl
                _collect_usage_line(obj, by_id)
            if obj.get("type") == "assistant" and msg.get("model"):
                model = msg["model"]
            if (obj.get("type") == "user" and msg.get("role") == "user"
                    and not obj.get("isMeta") and not obj.get("isSidechain")):
                c = msg.get("content")
                is_tool_result = isinstance(c, list) and any(
                    isinstance(b, dict) and b.get("type") == "tool_result" for b in c)
                if not is_tool_result:
                    turns += 1
    except Exception:
        return blank
    if not main_has_usage:
        return blank
    try:
        sub_dir = main_path.parent / sid / "subagents"
        if sub_dir.is_dir():
            for sub_path in sub_dir.glob("agent-*.jsonl"):
                try:
                    _collect_usage_from_file(sub_path, by_id)
                except Exception:
                    continue
    except Exception:
        pass
    token_usage = _build_token_usage(sid, by_id) if by_id else None
    return {
        "context_used": used, "baseline_context_used": baseline,
        "session_id": sid, "model": model, "turns": turns,
        "token_usage": token_usage,
    }


def _fetch_remote_config_raw(url: str | None = None, timeout: float = 3) -> dict:
    """拉远程配置并解析成 {is_report, report_url}——**异常照抛**。

    存在的理由只有一个：ping() 要拿到失败原因（拉不到配置是最高频的通道故障），
    而 fetch_remote_config 把异常吞成了 {}。两者共用这一份 URL 选取 + 驼峰→snake_case
    边界翻译，别在别处再抄一遍。软失败版见下面的 fetch_remote_config。
    """
    import urllib.request
    url = url or os.environ.get("WORKFLOW_TELEMETRY_CONFIG_URL") or REMOTE_CONFIG_URL
    with urllib.request.urlopen(url, timeout=timeout) as resp:
        data = json.loads(resp.read().decode("utf-8"))
    item = (data.get("data") or [{}])[0]
    result = {
        "is_report": bool(item.get("isReport")),
        "report_url": item.get("reportUrl") or None,
    }
    if isinstance(item.get("autoUpgradeRemote"), bool):
        result["auto_upgrade_remote"] = item["autoUpgradeRemote"]
    return result


def fetch_remote_config(url: str | None = None, timeout: int = 3) -> dict:
    """拉远程配置，解析出 {is_report, report_url}（取 data[0]）。

    边界翻译：远程配置是外部约定，key 用驼峰 isReport/reportUrl（读时照它）；
    落进 state.telemetry 的自有状态统一 snake_case（is_report/report_url），在此转换。
    超时/网络失败/格式异常 → 返回 {}（视为关闭），绝不阻断 init。
    URL 默认 REMOTE_CONFIG_URL，可用 WORKFLOW_TELEMETRY_CONFIG_URL 覆盖（测试用）。

    自动升级熔断（kill switch）：远程 data[0].autoUpgradeRemote 若为显式布尔 → 翻成 auto_upgrade_remote
    带上；非布尔/缺失则不带该键，让上层区分"远程说 false"(=熔断) vs "远程没说"(=不熔断)。
    只关不开：仅当远程显式 false 才熔断，缺失/拉不到一律不熔断（网络故障不误伤）。
    """
    if _skip():
        return {}
    try:
        return _fetch_remote_config_raw(url, timeout)
    except Exception:
        return {}


def _repo_slug_from_remote(remote: str | None) -> str | None:
    """从 git remote 地址解析出 org/repo 形式的项目标识，作 light-app-name 上报。

    取代过去的 JDOS 应用名（jdos_xxx）：前端应用普遍没有 JDOS 应用，git remote 才是所有项目
    都必有的稳定标识。取 host 之后、去掉 .git 尾巴的完整 path，嵌套分组也不丢：
      git@coding.jd.com:lightboat/lightboat-framework.git      → lightboat/lightboat-framework
      https://coding.jd.com/lightboat/lightboat-framework.git  → lightboat/lightboat-framework
    解析不出（空/无 path）返回 None。
    """
    if not remote:
        return None
    s = remote.strip()
    if s.endswith(".git"):
        s = s[:-len(".git")]
    if "://" in s:
        # scheme://[user@]host[:port]/org/repo → 去掉 scheme + host 段，留 path
        rest = s.split("://", 1)[1]
        s = rest.split("/", 1)[1] if "/" in rest else ""
    elif ":" in s and "@" in s:
        # scp-like git@host:org/repo → 冒号后即完整 path
        s = s.split(":", 1)[1]
    slug = "/".join(p for p in s.split("/") if p)
    return slug or None


def _report_headers(payload: dict) -> dict:
    """拼装网关强校验的三个请求头，缺了就不带（网关判 illegal，静默丢，不阻断主流程）。

    light-app-name：项目标识，语义固定为 payload 内嵌的冻结身份 state.identity.git_remote
      解析出的 org/repo（见 _repo_slug_from_remote，所有项目都必有的稳定标识）；无 remote 则不带该头。
    light-env：运行环境标识，实时探测 OPENCLI_SANDBOX_MODE → "sandbox"/"local"（见 detect_run_env）；
    light-user：从 payload 内嵌的冻结身份 state.identity.git_user 取，可能为空。
    """
    headers = {"Content-Type": "application/json", "light-env": detect_run_env()}
    identity = (payload.get("state") or {}).get("identity") or {}
    app_label = _repo_slug_from_remote(identity.get("git_remote"))
    if app_label:
        headers["light-app-name"] = app_label
    git_user = identity.get("git_user")
    if git_user:
        headers["light-user"] = git_user
    return headers


def report(telemetry: dict | None, payload: dict) -> None:
    """按冻结的 telemetry 配置上报 payload：spawn detached 子进程发 POST，父进程立即返回。

    守卫：WORKFLOW_SKIP_TELEMETRY / is_report 非真 / report_url 为空 → 直接 return，连子进程都不 spawn。
    项目标识（light-app-name）由 _report_headers 从 payload.state.identity.git_remote 解析，无需调用方传入。
    任何异常吞掉——上报失败绝不影响主流程。
    """
    if _skip():
        return
    telemetry = telemetry or {}
    if not telemetry.get("is_report"):
        return
    url = telemetry.get("report_url")
    if not url:
        return
    headers = _report_headers(payload)
    try:
        proc = subprocess.Popen(
            [sys.executable, "-c", _SEND_SCRIPT],
            stdin=subprocess.PIPE,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,  # 脱离父会话组，父进程退出不影响它
        )
        proc.stdin.write(json.dumps({
            "url": url, "body": _payload_for_report(payload), "headers": headers,
        }).encode("utf-8"))
        proc.stdin.close()
        # 不 wait——fire and forget
    except Exception:
        pass


# ── 通道探测（ping）：report 的反面 ──────────────────────────────────────────
# report 是旁路（spawn 子进程 / fire and forget / 失败静默），代价是链路上没有任何一环
# 会说话——用户采集不上来，用户和我们都不知情。ping 补的就是这个：同步、in-process、
# 失败大声。两者共用 _report_headers，所以「ping 通过」才等价于「上报会被网关放行」。

_PING_FAIL_TMPL = """❌ 采集通道不通，多半是本地环境问题（代理 / 证书 / 内网连通性）

   错误原因：{reason}

   处理后重跑本命令。确需先开工：加 --skip-ping 重跑
   （--skip-ping 仅在用户明确要求时使用，AI 不要自行添加）"""


def ping(target_root: Path, timeout: float = 1.5) -> str | None:
    """同步探测采集通道：通返回 None，不通返回一段可直接打印的失败文案。

    调用方一个 if 就完事——通道正常时一个字都不该打印（绝大多数情况都正常，
    正常还刷屏等于给用户和 AI 平白增加噪音）。

    探测包极简 {"event":"ping"}，回包 code==0 即通过。验的是**通道本身**
    （网关鉴权 + 连通性 + 服务端活着），不验某次上报的载荷——所以「全量 state.json
    随任务变大后被网关按 body 上限拒掉」那类失败 ping 是发现不了的。

    失败原因一律取异常原文、不分类不加工：故障大头是用户本地环境（挂了代理、没装
    公司根证书、没连内网），原始报错比我们猜的分类更有信息量；公司服务挂掉是长尾。

    两种「不算故障」的情况返回 None：WORKFLOW_SKIP_TELEMETRY=1 自测短路，
    以及远程配置 is_report 为假（远程主动关的上报，是预期状态）。

    总耗时封顶 2*timeout（拉配置 + POST 各一次）。
    """
    if _skip():
        return None
    try:
        cfg = _fetch_remote_config_raw(timeout=timeout)
    except Exception as e:
        return _PING_FAIL_TMPL.format(reason=f"拉取远程配置失败 —— {e!r}")
    if not cfg.get("is_report"):
        return None  # 远程主动关闭上报，不是故障
    url = cfg.get("report_url")
    if not url:
        return _PING_FAIL_TMPL.format(reason="远程配置未提供 reportUrl")

    # headers 必须与真实上报同源：网关强校验这三个头，缺一即判 illegal 并静默丢弃，
    # 这正是「用户以为在采集、其实一条没上去」的头号成因，必须在闸门这一步就点名。
    headers = _report_headers({"state": {"identity": collect_identity(target_root)}})
    if "light-user" not in headers:
        return _PING_FAIL_TMPL.format(
            reason="缺少请求头 light-user（本仓库未配置 git user.name），网关会判 illegal")
    if "light-app-name" not in headers:
        return _PING_FAIL_TMPL.format(
            reason="缺少请求头 light-app-name（本仓库无 git remote origin），网关会判 illegal")

    import urllib.request
    req = urllib.request.Request(
        url,
        data=json.dumps({"event": "ping"}).encode("utf-8"),
        headers=headers,
        method="POST",
    )
    try:
        with urllib.request.urlopen(req, timeout=timeout) as resp:
            body = json.loads(resp.read().decode("utf-8"))
    except Exception as e:
        return _PING_FAIL_TMPL.format(reason=f"上报请求失败 —— {e!r}")
    if not isinstance(body, dict) or body.get("code") != 0:
        detail = body.get("message") if isinstance(body, dict) else None
        return _PING_FAIL_TMPL.format(
            reason=f"服务端未确认（期望 code=0，实得 {body!r}）"
                   + (f"：{detail}" if detail else ""))
    return None
