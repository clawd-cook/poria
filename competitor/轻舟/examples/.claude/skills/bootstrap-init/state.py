#!/usr/bin/env python3
"""bootstrap-state.json 的确定性读写器（纯标准库，无第三方依赖）。

存在意义：知识底座初始化的进度账本 .workflow/bootstrap-state.json 是框架公共契约
（lb bootstrap CLI 与其它环节都读它）。让 AI 在会话里手搓这份 JSON 容易格式漂移、
键写错、并发下相互覆盖——本脚本把「标记某步完成 / 查某步是否完成」下沉为确定性动作，
逻辑与 CLI 的 src/bootstrap/state.ts（stateMark / stateDone / migrateBootstrapState）
逐字对齐，两条路径写出的账本互认。

用法（在项目根跑，或用 --root 指定）：
  python3 state.py mark  overview            # 标记 overview 完成
  python3 state.py mark  modules.1           # 标记第 1 个业务模块完成
  python3 state.py mark  tech.2              # 标记第 2 个 tech 完成
  python3 state.py mark  material_map
  python3 state.py done  overview            # 查询：完成打印 done、退出码 0；否则 not-done、退出码 1
  python3 state.py show                       # 打印整份 state（不存在则打印 {}）
  python3 state.py migrate                    # 仅把旧位置 .claude/ 迁到 .workflow/

key 支持点路径（modules.1 / tech.2）；mark 会自动建中间对象、并先执行一次 migrate。
"""
import argparse
import json
import os
import sys
from datetime import datetime


def state_path(root: str) -> str:
    return os.path.join(root, ".workflow", "bootstrap-state.json")


def legacy_path(root: str) -> str:
    return os.path.join(root, ".claude", "bootstrap-state.json")


def migrate(root: str) -> bool:
    """旧位置 .claude/bootstrap-state.json → .workflow/。仅当旧在、新不在时搬。"""
    legacy = legacy_path(root)
    current = state_path(root)
    if not os.path.exists(legacy) or os.path.exists(current):
        return False
    os.makedirs(os.path.dirname(current), exist_ok=True)
    os.replace(legacy, current)
    return True


def load(root: str) -> dict:
    p = state_path(root)
    if not os.path.exists(p):
        return {}
    try:
        with open(p, "r", encoding="utf-8") as f:
            data = json.load(f)
        return data if isinstance(data, dict) else {}
    except (json.JSONDecodeError, OSError):
        return {}


def save(root: str, data: dict) -> None:
    p = state_path(root)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
        f.write("\n")


def is_done(data: dict, key: str) -> bool:
    node = data
    for part in key.split("."):
        if not isinstance(node, dict) or part not in node:
            return False
        node = node[part]
    return isinstance(node, dict) and node.get("done") is True


def mark(root: str, key: str) -> None:
    migrate(root)
    data = load(root)
    parts = key.split(".")
    node = data
    for part in parts[:-1]:
        if not isinstance(node.get(part), dict):
            node[part] = {}
        node = node[part]
    # 时间戳格式与 CLI 的 toLocaleString('sv-SE') 对齐：YYYY-MM-DD HH:MM:SS（本地时区）
    ts = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    node[parts[-1]] = {"done": True, "at": ts}
    save(root, data)


def main() -> int:
    ap = argparse.ArgumentParser(description="bootstrap-state.json 确定性读写器")
    ap.add_argument("--root", default=os.getcwd(), help="项目根，缺省为当前目录")
    sub = ap.add_subparsers(dest="cmd", required=True)
    m = sub.add_parser("mark", help="标记某步完成")
    m.add_argument("key", help="点路径键，如 overview / modules.1 / tech.2 / material_map")
    d = sub.add_parser("done", help="查询某步是否完成（完成退出码 0，否则 1）")
    d.add_argument("key")
    sub.add_parser("show", help="打印整份 state")
    sub.add_parser("migrate", help="仅执行旧位置迁移")

    args = ap.parse_args()
    root = os.path.abspath(args.root)

    if args.cmd == "mark":
        mark(root, args.key)
        print(f"marked {args.key} @ {state_path(root)}")
        return 0
    if args.cmd == "done":
        migrate(root)
        ok = is_done(load(root), args.key)
        print("done" if ok else "not-done")
        return 0 if ok else 1
    if args.cmd == "show":
        migrate(root)
        print(json.dumps(load(root), ensure_ascii=False, indent=2))
        return 0
    if args.cmd == "migrate":
        print("migrated" if migrate(root) else "no-migration-needed")
        return 0
    return 1


if __name__ == "__main__":
    sys.exit(main())
