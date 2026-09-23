"""issue-<n>.md 文件维护 + INDEX.md 渲染。

issue frontmatter 是 YAML-lite（key: value 行），不引 yaml 依赖；
INDEX.md 每次从 state.feedback + issue/*.md 整体重渲，避免增量追加错位。
"""

import re
from pathlib import Path

FRONTMATTER_RE = re.compile(r"^---\n(.*?)\n---\n", re.DOTALL)


def issue_dir(target_root: Path, task: str) -> Path:
    return target_root / "delivery" / task / "feedback"


def issue_path(target_root: Path, task: str, issue_id: str) -> Path:
    return issue_dir(target_root, task) / f"{issue_id}.md"


def index_path(target_root: Path, task: str) -> Path:
    return issue_dir(target_root, task) / "INDEX.md"


def render_new_issue(issue_id: str, from_node: str, today: str, phenomenon: str, context: str) -> str:
    return (
        "---\n"
        f"id: {issue_id}\n"
        f"from: {from_node}\n"
        f"created_at: {today}\n"
        "category: null\n"
        "reroute_target: null\n"
        "severity: null\n"
        "status: open\n"
        "resolved_by: null\n"
        "resolved_mode: null\n"
        "trap_written_to: null\n"
        "---\n"
        "\n"
        "## 现象\n"
        f"{phenomenon}\n"
        "\n"
        "## 关联上下文\n"
        f"{context}\n"
        "\n"
        "## 分诊过程\n"
        "（待 feedback-loop 填）\n"
        "\n"
        "## 人工确认\n"
        "（待 feedback-loop 填）\n"
        "\n"
        "## 修复结果\n"
        "（待下游节点回填）\n"
    )


def write_new_issue(target_root: Path, task: str, issue_id: str, from_node: str, today: str, phenomenon: str, context: str):
    p = issue_path(target_root, task, issue_id)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(render_new_issue(issue_id, from_node, today, phenomenon, context))


def parse_frontmatter(md_text: str) -> dict:
    m = FRONTMATTER_RE.match(md_text)
    if not m:
        return {}
    out = {}
    for line in m.group(1).splitlines():
        if ":" not in line:
            continue
        k, _, v = line.partition(":")
        v = v.strip()
        if v == "null" or v == "":
            v = None
        out[k.strip()] = v
    return out


def update_issue_frontmatter(target_root: Path, task: str, issue_id: str, updates: dict):
    p = issue_path(target_root, task, issue_id)
    text = p.read_text()
    m = FRONTMATTER_RE.match(text)
    if not m:
        raise ValueError(f"{p} 缺 frontmatter")
    fm = parse_frontmatter(text)
    fm.update(updates)
    new_fm_lines = []
    for k in ("id", "from", "created_at", "category", "reroute_target", "severity", "status", "resolved_by", "resolved_mode", "trap_written_to"):
        v = fm.get(k)
        new_fm_lines.append(f"{k}: {'null' if v is None else v}")
    new_text = "---\n" + "\n".join(new_fm_lines) + "\n---\n" + text[m.end():]
    p.write_text(new_text)


def render_index(target_root: Path, task: str, state: dict) -> str:
    lines = [
        f"# Feedback Index — {task}",
        "",
        "由可报问题的节点登记（建档即落 severity，缺省 normal），feedback-loop 闭环时记 category（design/code/test 三分类），修完移到 resolved。",
        "",
        "from = 拦截阶段；根因 = category（design/code/test 三分类，与沉淀去向对齐）。两轴构成漏点地图。",
        "",
        "解决方式 = auto（AI 自己发现自己解决、全程没打断人）/ asked（为它问过人、等过人回话才搞定，含人自己动手改）。"
        "与 from 交叉：from=验证节点 + auto 才是完全自治。「仅记录」闭环的条目此列为空（没走解决路径）。",
        "",
        "| id | from | 根因 | severity | status | 解决方式 | resolved_by |",
        "|---|---|---|---|---|---|---|",
    ]
    rows = []
    for it in state["feedback"]["open"]:
        rows.append(_index_row(target_root, task, it, status="open"))
    for it in state["feedback"]["resolved"]:
        rows.append(_index_row(target_root, task, it, status="resolved"))
    rows.sort(key=lambda r: _issue_num(r[0]))
    for r in rows:
        lines.append("| " + " | ".join(_cell(x) for x in r) + " |")
    return "\n".join(lines) + "\n"


def _index_row(target_root: Path, task: str, it: dict, status: str):
    issue_id = it["id"]
    fm = {}
    p = issue_path(target_root, task, issue_id)
    if p.exists():
        fm = parse_frontmatter(p.read_text())
    return (
        issue_id,
        it.get("from") or fm.get("from"),
        it.get("category") or fm.get("category"),
        fm.get("severity"),
        status,
        it.get("resolved_mode") or fm.get("resolved_mode"),
        fm.get("resolved_by"),
    )


def _issue_num(issue_id: str) -> int:
    try:
        return int(issue_id.split("-")[1])
    except (IndexError, ValueError):
        return 0


def _cell(v):
    if v is None:
        return "—"
    return str(v)


def write_index(target_root: Path, task: str, state: dict):
    p = index_path(target_root, task)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(render_index(target_root, task, state))
