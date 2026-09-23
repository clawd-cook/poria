#!/usr/bin/env python3
"""
verify-iterative: propose 出口硬校验（仅 iterative 需求）—— 迭代协议 host/siblings 两维「衔接被 own」举证。

背景（为什么不是「查有没有一个宿主帧 nodeId」）：
  只查「§2.0 有没有 [host] nodeId」是换汤不换药——随手写 `[host] 195:513` 引个孤立组件帧就能过，
  而孤立帧根本不含既有兄弟，居中/楼层顺序照样丢（本脚本诞生自这个真实踩坑）。且很多需求设计稿
  压根没画「新模块 + 既有兄弟同框」的整页帧，强要一个 nodeId 反而逼出假值。

  所以本脚本核的是「衔接被真正 own 了没」，接受两条合法举证**二选一**：
    (a) 设计稿有跨新旧的同框帧：宿主帧 nodeId（对照表 reuse=「既有·只读」的锚点行，或 §2.0 树手写的
        [host] 行）在 parentmap 里，且其子树**既含新组件、又含至少一个既有节点**（= 证明它真是同框帧、
        不是孤立组件帧；既有节点既可是 reuse 锚点行如 [sibling]，也可是并入 parentmap 的非对照表既有节点）。
    (b) 无同框帧：§4「衔接约束」小节必须给出**具体的既有锚点**——引宿主文件 + 至少一个既有兄弟的
        具体标识（[sibling] nodeId ∨ CSS 选择器/className ∨ 既有文件路径），证明读了既有布局，
        而不是「沿用既有节奏」这类空话。**且**——因路径(b) 只证明「衔接被识别」、编码不出新模块插在
        既有兄弟上/下的**楼层顺序**（无同框帧时顺序无设计真源）——§4 还须有 `[用户已确认楼层顺序: …]`
        签收，否则阻断（拦 AI 猜位置：踩坑过支线任务区插反到领取列表下方）。路径(a) 同框帧本身直出顺序、无需此签收。
    两条都没有 → 阻断。此外允许 `[用户已确认无宿主帧: <理由>]` 的**显式用户签收**逃生（非 AI 自授，且已含顺序裁定）。

  静态检查天花板：即便 (a)/(b) 过了，间距/居中/顺序渲染对不对，静态判不了——交 visual-check 页面级 + 人。
  本脚本只拦「衔接根本没被 own」这一类（最省、最该机检的一环）。

用法：
  python3 verify-iterative.py --context delivery/<task>/context.md \
                              --design  openspec/changes/<task>/design.md \
                              --table   delivery/<task>/schema/component-table.md \
                              --parentmap delivery/<task>/schema/parentmap.json

判定：
  - 非 iterative（context.md 无「需求形态：iterative」）→ N/A，exit 0
  - (a) 或 (b) 或 用户签收 任一成立 → exit 0
  - 三者皆不成立 → 阻断 exit 1
  - 必要文件缺失 → exit 2
"""

import argparse
import json
import os
import re
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, SCRIPT_DIR)
from tree_util import parse_component_table  # noqa: E402

NODEID_RE = r'\d+:\d+(?:;\d+:\d+)*'


def read(p):
    return open(p, encoding='utf-8').read()


def is_iterative(context_text):
    """context.md 记「需求形态：iterative」→ True，并回传大致宿主串（可能为空）。"""
    m = re.search(r'需求形态\s*[:：]\s*iterative', context_text)
    if not m:
        return False, ''
    # 宿主：iterative｜大致宿主：<host>  或  iterative | host: <host>
    hm = re.search(r'iterative[^\n]*?(?:宿主|host)\s*[:：]\s*([^\n）)]+)', context_text)
    return True, (hm.group(1).strip() if hm else '')


def extract_section(text, title_kw):
    """抽取标题含 title_kw 的 markdown 小节正文（到下一个同级或更浅标题止）。"""
    lines = text.splitlines()
    start = None
    start_level = 0
    for i, ln in enumerate(lines):
        m = re.match(r'^(#{2,6})\s+.*' + re.escape(title_kw), ln)
        if m:
            start = i
            start_level = len(m.group(1))
            break
    if start is None:
        return ''
    out = []
    for ln in lines[start + 1:]:
        m = re.match(r'^(#{2,6})\s+', ln)
        if m and len(m.group(1)) <= start_level:
            break
        out.append(ln)
    return '\n'.join(out)


def extract_tree_block(text):
    """抽取 §2.0 组件树代码块（含 [host]/[sibling] 锚点行）。"""
    m = re.search(r'###?\s*2\.0[^\n]*\n(.*?)```(.*?)```', text, re.DOTALL)
    return m.group(2) if m else ''


def descendants(pm, root):
    """parentmap {nid:{parent,...}} 里 root 的全部后代 nodeId 集合。"""
    kids = {}
    for nid, meta in pm.items():
        par = meta.get('parent')
        if par:
            kids.setdefault(par, []).append(nid)
    seen, stack = set(), list(kids.get(root, []))
    while stack:
        n = stack.pop()
        if n in seen:
            continue
        seen.add(n)
        stack.extend(kids.get(n, []))
    return seen


def check_span_frame(tree_block, pm, new_nodeids, existing_anchor_ids=frozenset()):
    """路径 (a)：宿主帧在 parentmap 里，且子树既含新组件、又含既有节点（= 跨新旧同框帧）。

    宿主候选取两处并集，不再只认树里的字面 [host]（树由 relay-schema-gen tree 生成、
    标签只富化 reuse/备注、并不写 [host]，只认字面 [host] 会让本判据在真实产物上打不着）：
      1. 对照表里 reuse 标「既有·只读」的锚点行 nodeId（真源：host/sibling 都在这）；
      2. §2.0 树若手写了 [host] 行，其 nodeId 也并入（兼容手标）。
    「既有节点」= 子树里不属于新组件的 nodeId——既含 reuse 锚点行（如 [sibling]，之前它被
    误算进新组件、导致既有兄弟只以锚点行出现时本判据永远打不着），也含并入 parentmap
    但未建对照表行的既有节点。
    """
    host_ids = list(existing_anchor_ids)
    for ln in tree_block.splitlines():
        if '[host]' in ln:
            host_ids += re.findall(NODEID_RE, ln)
    host_ids = list(dict.fromkeys(host_ids))  # 去重保序
    if not host_ids:
        return False, 'no [host] 行 / reuse 锚点行'
    for h in host_ids:
        if h not in pm:
            continue
        sub = descendants(pm, h)
        if not sub:
            continue
        # 子树里存在「不属于新组件」的 nodeId → 证明是跨新旧同框帧
        existing = [n for n in sub if n not in new_nodeids]
        if any(n in new_nodeids for n in sub) and existing:
            return True, f'宿主帧 {h} 子树跨新旧（既有兄弟 {existing[0]} 等 {len(existing)} 个）'
    # 宿主候选都不在 parentmap，或子树不跨新旧
    return False, (f'宿主候选 {host_ids} 不在 parentmap 或子树不含既有兄弟'
                   f'（疑似引了孤立组件帧、或宿主帧 raw 未并入 parentmap）')


def check_code_evidence(link_section, host_hint):
    """路径 (b)：§4 衔接约束引宿主文件 + ≥1 具体既有锚点（[sibling] nodeId ∨ 选择器 ∨ 文件路径）。"""
    if not link_section.strip():
        return False, 'no 衔接约束 section'
    # 宿主引用：context.md host 的 basename（去扩展名）出现在 §4 或以 .tsx/.vue/.ts 文件名出现
    host_base = ''
    if host_hint:
        host_base = os.path.splitext(os.path.basename(host_hint.strip().strip('`')))[0]
    host_ref = bool(host_base and host_base in link_section) or \
        bool(re.search(r'[\w./-]+\.(tsx|ts|jsx|js|vue|scss|less)', link_section))
    # 具体既有锚点
    anchors = []
    if re.search(r'\[sibling\][^\n]*' + NODEID_RE, link_section):
        anchors.append('[sibling] nodeId')
    if re.search(r'(?<![\w#])\.[a-zA-Z_][\w-]{2,}', link_section):  # CSS 选择器/className，如 .main_swiper
        anchors.append('选择器/className')
    if not host_ref:
        return False, '衔接约束未引宿主文件（无 host basename / 文件路径）'
    if not anchors:
        return False, ('衔接约束无具体既有锚点（缺 [sibling] nodeId / CSS 选择器 / className）'
                       '——「沿用既有节奏」这类空话不算举证')
    return True, f'衔接约束引宿主 + 既有锚点（{", ".join(anchors)}）'


def check_user_signoff(text):
    return bool(re.search(r'\[用户已确认无宿主帧[:：]', text))


def check_floor_order_signoff(text):
    """§4 是否有 [用户已确认楼层顺序: ...] 签收。路径(b) 代码锚点只证明「衔接被识别」、
    编码不出新模块插在既有兄弟上/下的顺序——无同框帧时楼层顺序无设计真源，须用户签收，
    否则就是 AI 猜位置（真实踩坑：支线任务区被插反到领取列表下方，visual 用新组件帧当
    pageNodeId 也看不出顺序、漏检）。"""
    return bool(re.search(r'\[用户已确认楼层顺序[:：]', text))


def main():
    ap = argparse.ArgumentParser(description='迭代协议 host/siblings 衔接举证校验（两条二选一）')
    ap.add_argument('--context', required=True, help='context.md 路径（读需求形态 + 宿主）')
    ap.add_argument('--design', required=True, help='design.md 路径（§2.0 树 + §4 衔接约束）')
    ap.add_argument('--table', required=True, help='component-table.md 路径（新组件 nodeId 集）')
    ap.add_argument('--parentmap', required=True, help='parentmap.json 路径（判子树跨新旧）')
    args = ap.parse_args()

    if not os.path.exists(args.context):
        print(f"ERROR: 文件不存在: {args.context}", file=sys.stderr)
        sys.exit(2)
    context_text = read(args.context)

    iterative, host_hint = is_iterative(context_text)
    if not iterative:
        print("N/A：需求形态非 iterative，跳过迭代协议衔接校验。")
        sys.exit(0)

    for p in (args.design, args.table, args.parentmap):
        if not os.path.exists(p):
            print(f"ERROR: 文件不存在: {p}", file=sys.stderr)
            sys.exit(2)

    design = read(args.design)
    pm = json.load(open(args.parentmap, encoding='utf-8'))
    rows = parse_component_table(read(args.table))
    # reuse 列标「既有·只读」的是 host/sibling 锚点行（既有代码、只读），不算新组件；
    # 其余（含复用物料库组件的 reuse 值）才是本迭代真正新增/改写的组件。
    new_nodeids = {r['nodeId'] for r in rows if '既有' not in (r.get('reuse') or '')}
    existing_anchor_ids = {r['nodeId'] for r in rows if '既有' in (r.get('reuse') or '')}

    tree_block = extract_tree_block(design)
    link_section = extract_section(design, '衔接约束')

    print(f"=== 迭代协议衔接校验（iterative｜宿主：{host_hint or '未记'}）===")

    # 用户显式签收 → 直接放行
    if check_user_signoff(design):
        print("✓ 检出 [用户已确认无宿主帧: ...] 显式签收 → 放行（衔接由用户裁定）。")
        sys.exit(0)

    ok_a, msg_a = check_span_frame(tree_block, pm, new_nodeids, existing_anchor_ids)
    if ok_a:
        print(f"✓ 路径(a) 跨新旧同框帧成立：{msg_a}")
        sys.exit(0)

    ok_b, msg_b = check_code_evidence(link_section, host_hint)
    if ok_b:
        # 路径(b) 只证明「衔接被识别」，不编码楼层顺序（新模块插既有兄弟上/下，代码锚点看不出）——
        # 无同框帧时顺序无设计真源，须用户签收，否则即 AI 猜位置（踩坑：支线任务区插反到领取列表下方）。
        if check_floor_order_signoff(design):
            print(f"✓ 路径(b) 既有代码举证成立：{msg_b}；且楼层顺序已用户签收")
            sys.exit(0)
        print("\n=== ❌ 楼层顺序无真源（阻断）===")
        print(f"  路径(b) 衔接举证成立（{msg_b}），但无同框帧 → 楼层顺序（新模块插既有兄弟上/下）无设计真源。")
        print("  代码锚点只证明「衔接被识别」、编码不出顺序；AI 擅自定位会插反（真实踩坑）。补齐二选一后重跑：")
        print("    · §4「衔接约束」记 [用户已确认楼层顺序: <新模块在 X 之上/之下…>]（须用户签收）；或")
        print("    · 改用路径(a) 同框帧（整页帧含新旧楼层，顺序由帧直出，visual 的 pageNodeId 亦取该帧）。")
        sys.exit(1)

    print("\n=== ❌ 衔接未被 own（阻断）===")
    print(f"  路径(a) 跨新旧同框帧：不成立 —— {msg_a}")
    print(f"  路径(b) 既有代码举证：不成立 —— {msg_b}")
    print("\niterative 需求的 host/siblings 衔接必须被 own（居中/楼层顺序的真源在既有兄弟布局里）。")
    print("三选一补齐后重跑：")
    print("  (a) 让 relay 分析捞「新模块+既有兄弟同框」的整页帧，其 raw 并入 parentmap，§2.0 [host] 引该帧 nodeId；或")
    print("  (b) §4「衔接约束」引宿主文件 + 具体既有锚点（[sibling] nodeId / 既有 CSS 选择器如 .main_swiper / 既有文件路径）；或")
    print("  (c) 确认设计确实未定衔接、由用户裁定 → 记 [用户已确认无宿主帧: <理由>]（须用户签收，不可 AI 自授）。")
    sys.exit(1)


if __name__ == '__main__':
    main()
