#!/usr/bin/env python3
"""组件树 join 的公共件：§2.1 对照表解析 + parentmap 父链遍历。

被同目录的 relay-schema-gen.py（`tree` 子命令从 parentmap join 生成 §2.0 组件树）与
verify-tree.py（对照表 nodeId 合法性 + owner 校验）共用——避免各写一份 parse_component_table。
"""

import re

# relay nodeId 可为 `;` 连接的复合 id（实例作用域），如 `45:144;30:719`——整串捕获、整串比对
NODEID_RE = re.compile(r'(\d+:\d+(?:;\d+:\d+)*)')


def cells(line: str) -> list:
    """拆一行 markdown 表格为 cell 列表（去首尾 `|` 与空白）。"""
    return [c.strip() for c in line.strip().strip('|').split('|')]


def is_separator(cs: list) -> bool:
    """`|---|:--|` 这类分隔行。"""
    return bool(cs) and set(''.join(cs)) <= set('-: ')


def parse_component_table(design_content: str) -> list:
    """扫 design.md 所有「层级 + nodeId」列的对照表子表 → [{nodeId, name, skeleton, reuse, note}]（按序去重）。

    只认同时有 层级 与 nodeId 两列的表——借此和变体表/跨状态差异表/数据绑定表区分开。
    列按表头名定位；parentId 列去留、列序变化都不影响。`note`（备注）供 §2.0 组件树富化标注。
    """
    nodes, seen = [], set()
    lines = design_content.split('\n')
    i, n = 0, len(lines)
    while i < n:
        if lines[i].strip().startswith('|'):
            headers = cells(lines[i])
            lvl = next((k for k, h in enumerate(headers) if '层级' in h), None)
            nid = next((k for k, h in enumerate(headers) if 'nodeid' in h.lower()), None)
            nm = next((k for k, h in enumerate(headers) if '名称' in h), None)
            ru = next((k for k, h in enumerate(headers) if 'reuse' in h.lower() or '复用' in h), None)
            bz = next((k for k, h in enumerate(headers) if '备注' in h), None)
            if lvl is not None and nid is not None:
                j = i + 1
                while j < n and lines[j].strip().startswith('|'):
                    rc = cells(lines[j])
                    if not is_separator(rc) and nid < len(rc):
                        m = NODEID_RE.search(rc[nid])
                        if m and m.group(1) not in seen:
                            seen.add(m.group(1))
                            nodes.append({
                                'nodeId': m.group(1),
                                'name': (rc[nm] if nm is not None and nm < len(rc) else '') or m.group(1),
                                'skeleton': '骨架' in (rc[lvl] if lvl < len(rc) else ''),
                                'reuse': (rc[ru].strip() if ru is not None and ru < len(rc) else ''),
                                'note': (rc[bz].strip() if bz is not None and bz < len(rc) else ''),
                            })
                    j += 1
                i = j
                continue
        i += 1
    return nodes


def nearest_ancestor_in_set(nodeid: str, node_set: set, pm: dict):
    """沿 parentmap parent 链找最近的、也在 node_set 里的祖先；无则 None（→ 挂 Page 根）。"""
    cur = pm.get(nodeid, {}).get('parent')
    seen = set()
    while cur and cur not in seen:
        if cur in node_set:
            return cur
        seen.add(cur)
        cur = pm.get(cur, {}).get('parent')
    return None
