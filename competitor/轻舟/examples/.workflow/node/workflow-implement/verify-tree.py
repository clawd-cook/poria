#!/usr/bin/env python3
"""
verify-tree: propose 出口硬校验 —— §2.1 对照表 nodeId 合法性 + 带视觉容器 owner 归属。

#9 翻转后：组件树由 `relay-schema-gen.py tree` 从 parentmap(拓扑真源) join 对照表(语义) 生成，
**拓扑正确 by construction**——原「树 vs parentmap 父子对差」已无意义、删除。本脚本改查翻转后
subagent 唯一还可能手误的东西：
  - 对照表里出现的 nodeId 必须真实存在于 parentmap（抓拼错/臆造 nodeId）→ 硬失败(exit 1)
  - 带视觉容器(parentmap hasVisual)且在对照表标结构层/复用(非新增组件) → owner 警告(不失败)

用法：
  python3 verify-tree.py --table delivery/<task>/schema/component-table.md \
                         --parentmap delivery/<task>/schema/parentmap.json
  （对照表已移出 design.md 正文、落 schema 层；未传 --table 时回退 --design 读内联 §2.1，兼容老稿）

判定：
  - 对照表 nodeId 不在 parentmap → 硬失败(exit 1)（拼错/臆造，或 parentmap 覆盖不全）
  - 带视觉容器无 owner（hasVisual + 结构层/复用）→ 警告(不失败)
  - 文件缺失/无对照表 → exit 2
"""

import argparse
import json
import os
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
# 组件树 join 公共件（与 relay-schema-gen.py 共用，同目录）——sys.path 保证 importlib 加载时也能找到
sys.path.insert(0, SCRIPT_DIR)
from tree_util import parse_component_table  # noqa: E402


def main():
    ap = argparse.ArgumentParser(description='对照表 nodeId 合法性 + owner 归属校验（#9 翻转后）')
    ap.add_argument('--table', help='component-table.md 路径（对照表源，schema 层；优先于 --design）')
    ap.add_argument('--design', help='design.md 路径（对照表仍内联 §2.1 的老稿回退源）')
    ap.add_argument('--parentmap', required=True, help='parentmap.json 路径（拓扑真源）')
    args = ap.parse_args()

    table_path = args.table or args.design
    if not table_path:
        print("ERROR: 需 --table（对照表源）或 --design（老稿回退）之一", file=sys.stderr)
        sys.exit(2)
    for p in (table_path, args.parentmap):
        if not os.path.exists(p):
            print(f"ERROR: 文件不存在: {p}", file=sys.stderr)
            sys.exit(2)

    design = open(table_path, encoding='utf-8').read()
    pm = json.load(open(args.parentmap, encoding='utf-8'))
    rows = parse_component_table(design)
    if not rows:
        print(f"ERROR: {table_path} 未找到「层级 + nodeId」列的对照表", file=sys.stderr)
        sys.exit(2)

    illegal, warnings = [], []
    for r in rows:
        nid = r['nodeId']
        if nid not in pm:
            illegal.append(f"{nid} 「{r['name']}」不在 parentmap（拼错/臆造 nodeId，或 parentmap 覆盖不全）")
            continue
        if pm[nid].get('hasVisual') and (r['skeleton'] or r['reuse']):
            kind = '结构层' if r['skeleton'] else f"复用({r['reuse']})"
            warnings.append(
                f"带视觉容器无 owner: {nid} 「{r['name']}」标{kind}，"
                f"须确认由骨架 wrapper 渲染其视觉，否则底色/内边距整片丢失")

    print(f"=== 对照表 nodeId 合法性校验（共 {len(rows)} 条节点） ===")
    if warnings:
        print("\n=== ⚠ owner 警告(不阻断，人工确认) ===")
        for w in warnings:
            print("  " + w)
    if illegal:
        print("\n=== ❌ 非法 nodeId(阻断) ===")
        for x in illegal:
            print("  " + x)
        print(f"\n共 {len(illegal)} 条 nodeId 不在 parentmap → propose 不得 advance。"
              f"修对照表 nodeId，或补全 parentmap（见 relay-component-analysis.md"
              f"「组件拓扑parentmap」的整页抓取）后重跑。")
        sys.exit(1)

    print("\n✓ 对照表 nodeId 全部合法（存在于 parentmap）。" +
          (f"（{len(warnings)} 条 owner 警告待人工确认）" if warnings else ""))
    sys.exit(0)


if __name__ == '__main__':
    main()
