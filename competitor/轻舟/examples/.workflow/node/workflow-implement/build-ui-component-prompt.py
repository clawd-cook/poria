#!/usr/bin/env python3
"""
UI Subagent Prompt Builder

把 subagent prompt 组装从"手动拼"变成"调脚本拿"。
主 agent spawn 组件 subagent 前必须先调本脚本，将 stdout 原样传给 Agent()。

两步走流程：静态稿 subagent 生成静态稿 → 本 subagent 基于静态稿改写为业务组件。
"""

import argparse
import json
import os
import re
import sys

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
PROJECT_ROOT = os.path.abspath(os.path.join(SCRIPT_DIR, '..', '..', '..'))


def emit_prompt(prompt: str, task: str, entry_id: str, stage: str, emit_to=None, to_stdout=False):
    """默认把 prompt 落盘、stdout 只回 {prompt_file, next}——主 agent 手里只有路径：
    既无从改写，prompt 全文也不进主 agent 上下文；spawn 时让 subagent 自己 Read 该文件。
    --stdout 强制回全文（调试/兜底）。
    """
    if to_stdout:
        print(prompt)
        return
    rel = emit_to or f'delivery/{task}/workspace/prompts/{entry_id}.{stage}.md'
    full = rel if os.path.isabs(rel) else os.path.join(PROJECT_ROOT, rel)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, 'w', encoding='utf-8') as f:
        f.write(prompt)
    print(json.dumps({
        'prompt_file': rel,
        'summary': f'{stage} prompt for {entry_id} 已落盘（{len(prompt)} 字符）',
        'next': f'spawn subagent，prompt 传 "Read {rel} 并严格照其执行即可"；'
                f'勿把文件内容读进主 agent 上下文、勿改写。',
    }, ensure_ascii=False, indent=2))


def read_file(path: str) -> str:
    full = os.path.join(PROJECT_ROOT, path) if not os.path.isabs(path) else path
    if not os.path.exists(full):
        print(f"ERROR: file not found: {full}", file=sys.stderr)
        sys.exit(1)
    with open(full, 'r', encoding='utf-8') as f:
        return f.read()


def load_component_table(task: str) -> str:
    """读 §2.1 对照表源 `delivery/<task>/schema/component-table.md`（对照表已移出 design.md 正文）。

    软读取：文件不存在返回 ''（对照表仍内联在 design.md §2.1 的老稿走此回退，
    projection 装配 / 组件枚举再以 design.md 兜底）。
    """
    full = os.path.join(PROJECT_ROOT, f'delivery/{task}/schema/component-table.md')
    if not os.path.exists(full):
        return ''
    with open(full, 'r', encoding='utf-8') as f:
        return f.read()


def extract_rewrite_rules(protocol_content: str) -> str:
    """Extract content from '## 组件 subagent 改写规则' heading to the next '## ' heading.

    以整行标题为锚（re.M + 行尾），避免误命中「subagent 编排」段里反引号包裹的
    `## 组件 subagent 改写规则` 提及（它出现在真正标题之前）。
    """
    marker = '## 组件 subagent 改写规则'
    m = re.search(r'^' + re.escape(marker) + r'\s*$', protocol_content, re.M)
    if not m:
        print("ERROR: '## 组件 subagent 改写规则' section not found in ui-protocol.md", file=sys.stderr)
        sys.exit(1)
    rest = protocol_content[m.end():]
    next_h2 = re.search(r'\n## ', rest)
    section = rest[:next_h2.start()] if next_h2 else rest
    return (marker + section).strip()


def extract_design_params(context_content: str) -> dict:
    """Extract CDN scale from context.md."""
    result = {'cdn_scale': '2'}

    cdn_match = re.search(r'CDN\s*(?:图片导出)?倍率[：:]\s*(\d+)', context_content)
    if cdn_match:
        result['cdn_scale'] = cdn_match.group(1)

    return result


def extract_sections(design_content: str, numbers: list) -> str:
    """按章节号（如 '2.3'、'4.4'）从 design.md 摘出整节 markdown，拼接返回。

    目的：把改写 subagent 需要的 TRD 章节（Props/数据绑定/被 task 引用的业务规则）
    直接内联进 prompt，subagent 就不必再 Read 整份 500+ 行 design.md（省 subagent 上下文）。
    章节边界：从 `## / ### / #### <num>` 标题起，到下一个**同级或更浅**标题止。
    找不到的号跳过（不报错）。
    """
    lines = design_content.split('\n')
    heads = []  # (line_idx, level, num)
    for i, ln in enumerate(lines):
        m = re.match(r'^(#{2,4})\s+(\d+(?:\.\d+)?)\b', ln)
        if m:
            heads.append((i, len(m.group(1)), m.group(2)))
    chunks = []
    seen = set()
    for num in numbers:
        if num in seen:
            continue
        seen.add(num)
        hi = next((k for k, (_, _, n) in enumerate(heads) if n == num), None)
        if hi is None:
            continue
        start, lvl, _ = heads[hi]
        end = len(lines)
        for (i2, lvl2, _) in heads[hi + 1:]:
            if lvl2 <= lvl:
                end = i2
                break
        chunks.append('\n'.join(lines[start:end]).rstrip())
    return '\n\n'.join(chunks)


def _section_numbers_by_heading(design_content: str, keywords: list) -> list:
    """扫 §2/§3/§4… 标题，返回标题含任一 keyword 的章节号（如「数据绑定」→ '2.4'）。

    按内容定位、不认固定号——章号在模板演进中会漂（数据绑定曾是 2.3、现为 2.4），
    硬编码号会在重编号后抽错节。
    """
    nums = []
    for ln in design_content.split('\n'):
        m = re.match(r'^#{2,4}\s+(\d+(?:\.\d+)?)\s+(.*)', ln)
        if m and any(kw in m.group(2) for kw in keywords):
            nums.append(m.group(1))
    return nums


def build_trd_section(design_content: str, task_entry: str, exclude_sections=frozenset()) -> str:
    """摘录改写 subagent 所需的 TRD 章节并组装成注入块。

    基线含「数据绑定 + 组件接入」，外加 task「实现上下文」以 `§x.x`/`TRD§x.x` 引用到的章节（如 §4.3）。
    `exclude_sections`：剔除的章节号集——**组件规格投影已按组件聚合携带的类型（变体/数据绑定/交互/边界）
    由调用方传入剔除，避免 trd_block 整章全表与投影的本组件行重复**（见 build_prompt）。投影缺席时传空集、
    全量注入作兜底。章节按标题内容定位（兼容章号漂移）。
    """
    refs = set(re.findall(r'§\s*(\d+\.\d+)', task_entry))
    baseline = set(_section_numbers_by_heading(design_content, ['数据绑定', '组件接入']))
    numbers = sorted((baseline | refs) - set(exclude_sections),
                     key=lambda s: [int(x) for x in s.split('.')])
    body = extract_sections(design_content, numbers)
    return body


# ---- 组件规格投影（三层变体 + §6 边界，按 rootNodeId 聚合成 per-component 视图） ----

def _cells(line: str) -> list:
    """拆一行 markdown 表格为 cell 列表（去首尾 `|` 与空白）。"""
    return [c.strip() for c in line.strip().strip('|').split('|')]


def _is_separator(cells: list) -> bool:
    """`|---|:--|` 这类分隔行。"""
    return bool(cells) and set(''.join(cells)) <= set('-: ')


def _norm_node(cell: str) -> str:
    """从 cell 里抽 `\\d+:\\d+`（容忍反引号/多余文字）。"""
    m = re.search(r'(\d+:\d+)', cell or '')
    return m.group(1) if m else ''


def _classify_anchored_table(headers_lower: str) -> str:
    """按表头特征列判定三层变体表 / §6 边界表类型（不依赖章节号，列序/章号漂移仍稳）。"""
    if 'countfrom' in headers_lower:
        return 'layout'      # 层① 数量→布局
    if 'statedim' in headers_lower:
        return 'state'       # 层② 状态→生命周期
    if '数据种类' in headers_lower:
        return 'content'     # 层③ 内容→渲染
    if '边界' in headers_lower or '兜底' in headers_lower:
        return 'boundary'    # §6 边界
    return 'other'


def iter_tables(design_content: str):
    """逐个 yield (headers, rows)：扫描 design.md 所有 markdown 表格（rows 已剔表头/分隔行）。"""
    lines = design_content.split('\n')
    i, n = 0, len(lines)
    while i < n:
        if lines[i].strip().startswith('|'):
            headers = _cells(lines[i])
            rows, j = [], i + 1
            while j < n and lines[j].strip().startswith('|'):
                rc = _cells(lines[j])
                if not _is_separator(rc):
                    rows.append(rc)
                j += 1
            yield headers, rows
            i = j
            continue
        i += 1


def parse_anchored_tables(design_content: str) -> list:
    """扫描 design.md，收集所有含 `rootNodeId` 列的 markdown 表（三层变体表 + §6 边界表）。

    返回 [{type, headers, root_idx, rows}]，rows 为 cell 列表的列表（已剔表头/分隔行）。
    """
    tables = []
    for headers, rows in iter_tables(design_content):
        root_idx = next((k for k, c in enumerate(headers) if 'rootnodeid' in c.lower()), None)
        if root_idx is not None:
            tables.append({'type': _classify_anchored_table(' '.join(headers).lower()),
                           'headers': headers, 'root_idx': root_idx, 'rows': rows})
    return tables


# ---- nodeId → 所属组件 rootNodeId 归属索引（让无 rootNodeId 列的绑定/交互表也能聚到组件） ----

def build_owner_index(design_content: str):
    """从 §2.1 对照表建 {nodeId: rootNodeId} 与 {组件名: rootNodeId}。

    对照表 heading `#### 组件：<name> (rootNodeId: X)` 定当前 owner，其下表格每行的 nodeId
    列（含跨状态差异小表）都归该 owner。这是把数据绑定表 / 交互表按 nodeId 聚到组件的锚——
    两表本身无 rootNodeId 列，靠此归属而非双写。
    """
    node2root, name2root = {}, {}
    cur_root = None
    for ln in design_content.split('\n'):
        m = re.match(r'^#{2,4}\s*组件[：:]\s*(.+?)\s*[（(]\s*rootNodeId:\s*`?(\d+:\d+)`?\s*[)）]',
                     ln, re.IGNORECASE)
        if m:
            cur_root = m.group(2)
            name2root[m.group(1).strip()] = cur_root
            node2root.setdefault(cur_root, cur_root)
            continue
        if re.match(r'^#{1,4}\s', ln):  # 任何其它标题 → 离开当前组件块，停止归属
            cur_root = None
            continue
        if cur_root and ln.strip().startswith('|'):
            cells = _cells(ln)
            if _is_separator(cells):
                continue
            for c in cells:
                nid = _norm_node(c)
                if nid:
                    node2root.setdefault(nid, cur_root)
    return node2root, name2root


def _collect_owned_rows(design_content: str, root: str, node2root: dict, owner_names: set,
                        header_pred) -> tuple:
    """扫首张匹配 header_pred 的表，收归属本组件的行（组件名列命中 或 行内任一 nodeId 归属 root）。

    返回 (headers, rows, drop_idx)；drop_idx 指恒同值的「组件」列（渲染时丢弃），无则 -1。
    """
    for headers, rows in iter_tables(design_content):
        if not header_pred(headers):
            continue
        comp_idx = next((k for k, h in enumerate(headers) if '组件' in h), None)
        mine = []
        for r in rows:
            owned = comp_idx is not None and comp_idx < len(r) and r[comp_idx].strip() in owner_names
            if not owned:
                owned = any(node2root.get(_norm_node(c)) == root for c in r if _norm_node(c))
            if owned:
                mine.append(r)
        if mine:
            # 仅纯「组件」列（本组件恒同值）渲染时丢弃；「组件/元素」等合并列含元素信息，保留
            drop = comp_idx if (comp_idx is not None and headers[comp_idx].strip() == '组件') else -1
            return headers, mine, drop
    return None


def _render_table(headers: list, rows: list, drop_idx: int) -> str:
    """重渲染 markdown 表，丢掉 drop_idx 列（rootNodeId 过滤后对每行同值、冗余）。"""
    keep = [k for k in range(len(headers)) if k != drop_idx]
    pick = lambda cells: [cells[k] if k < len(cells) else '' for k in keep]
    out = ['| ' + ' | '.join(pick(headers)) + ' |',
           '| ' + ' | '.join(['---'] * len(keep)) + ' |']
    out += ['| ' + ' | '.join(pick(r)) + ' |' for r in rows]
    return '\n'.join(out)


def build_component_projection(design_content: str, root_node_id: str, table_content: str = None) -> str:
    """按 rootNodeId 从三层变体表 + §6 边界 + 数据绑定 + 交互聚合本组件的行，产 co-located 投影块。

    变体/边界表带 rootNodeId 列直接过滤；数据绑定/交互表无 rootNodeId 列，靠 build_owner_index
    的 nodeId 归属索引聚到组件。这是「组件规格投影」——让一个组件的数量/状态/内容/绑定/交互/边界
    故事聚在一处，跨章矛盾单视角可见（如状态变体声明「已领」态 vs 交互 R1「本券不可领」同框）。
    段落按表头内容识别、不认章号，故老稿（数据绑定在 §2.3）与新稿（§2.4）均适用。

    `table_content`：§2.1 对照表源（组件 heading + nodeId 行）。对照表已移出 design.md 正文、
    落 `delivery/<task>/schema/component-table.md` 后由调用方读入传进来；为 None 时回退用
    design_content（兼容对照表仍内联在 §2.1 的老稿）。变体/绑定/交互表始终取自 design_content。
    """
    tables = parse_anchored_tables(design_content)
    node2root, name2root = build_owner_index(table_content or design_content)
    owner_names = {n for n, r in name2root.items() if r == root_node_id}

    def anchored(kind: str) -> str:
        """带 rootNodeId 列的表：取该类型首表、过滤本组件行、丢 rootNodeId 列渲染。"""
        for t in tables:
            if t['type'] != kind:
                continue
            ri = t['root_idx']
            mine = [r for r in t['rows'] if ri < len(r) and _norm_node(r[ri]) == root_node_id]
            if mine:
                return _render_table(t['headers'], mine, ri)
        return '（无）'

    def owned(header_pred) -> str:
        """无 rootNodeId 列的表（绑定/交互）：按 nodeId 归属 / 组件名聚本组件行。"""
        hit = _collect_owned_rows(design_content, root_node_id, node2root, owner_names, header_pred)
        return _render_table(*hit) if hit else '（无）'

    def _is_binding_header(h):
        joined = ' '.join(h)
        return ('绑定字段' in joined or '字段' in joined) and ('nodeid' in joined.lower() or '元素' in joined)

    def _is_interaction_header(h):
        joined = ' '.join(h)
        return '触发' in joined and '行为' in joined

    return '\n\n'.join([
        f"#### 数量布局（层①）\n{anchored('layout')}",
        f"#### 状态变体（层②）\n{anchored('state')}",
        f"#### 内容渲染（层③）\n{anchored('content')}",
        f"#### 数据绑定（Props / 消费字段）\n{owned(_is_binding_header)}",
        f"#### 交互规则（§4.2）\n{owned(_is_interaction_header)}",
        f"#### 边界与健壮性（§6）\n{anchored('boundary')}",
    ])


def parse_root_node_id(title: str) -> str:
    """从 task 标题 `... (nodeId: 45:21) ...` 抽 rootNodeId（兼容 rootNodeId: 写法）。"""
    m = re.search(r'\(\s*(?:root)?nodeId:\s*`?(\d+:\d+)`?\s*\)', title, re.IGNORECASE)
    return m.group(1) if m else ''


def extract_tech_stack(design_content: str) -> str:
    """从 TRD 第 1 章提取技术栈描述。

    两种写法都要支持：
    - 键值对：`技术栈：React 18 + TypeScript`
    - 小节标题：`### 1.1 技术栈` + 下方列表/表格（本 schema 模板即此形态）

    取不到就返回 ''，由调用方退化成"请自行从 design.md 第 1 章确认"，
    **不返回表格行或标题残片**——那会污染 prompt 首句（曾出现
    "你正在为一个 | 维度 | 选型 | 说明 | 项目改写 UI 组件"）。
    """
    def clean(text: str) -> str:
        text = re.sub(r'[*`]', '', text).strip(' -：:')
        # 表格行、纯标记残片一律弃用
        if not text or text.startswith('|') or text.count('|') >= 2 or len(text) < 3:
            return ''
        return text

    # ① 键值对形式
    for pattern in [
        r'(?:技术栈|技术选型|Tech Stack)[：:][ \t]*([^\n]+)',
        r'(?:项目类型|项目技术)[：:][ \t]*([^\n]+)',
        r'(?:框架|Framework)[：:][ \t]*([^\n]+)',
    ]:
        for match in re.finditer(pattern, design_content, re.IGNORECASE):
            value = clean(match.group(1))
            if value:
                return value

    # ② 小节标题形式：取标题下首个有效内容行（跳过空行/表格/子标题）
    lines = design_content.split('\n')
    for i, line in enumerate(lines):
        if not re.match(r'^#{2,4}\s', line):
            continue
        if not re.search(r'技术栈|技术选型|Tech Stack', line, re.IGNORECASE):
            continue
        for follow in lines[i + 1:i + 8]:
            if not follow.strip() or follow.startswith('#'):
                continue
            value = clean(follow)
            if not value:
                continue
            # 该位置的内容语义不确定（可能是散文导语、表格、列表）。只认「像技术栈」的：
            # 含技术名或版本号；否则宁可返回空、让调用方提示自行确认，
            # 也不要把"下表列出各维度选型"这类导语当技术栈塞进 prompt 首句。
            looks_like_stack = re.search(
                r'\d+\.\d|React|Vue|Angular|Svelte|Next|Nuxt|Taro|TypeScript|JavaScript|'
                r'小程序|uni-?app|Flutter|Webpack|Vite', value, re.IGNORECASE)
            return value if looks_like_stack else ''
        break

    return ''


def extract_task_entry(tasks_content: str, component_id: str) -> str:
    """Extract a task entry by its ID (e.g. '5.1') including all nested sub-steps and subagent context."""
    pattern = rf'^- \[[ x]\] {re.escape(component_id)}\s'
    lines = tasks_content.split('\n')
    start_idx = -1
    for i, line in enumerate(lines):
        if re.match(pattern, line):
            start_idx = i
            break

    if start_idx == -1:
        print(f"ERROR: task entry '{component_id}' not found in tasks.md", file=sys.stderr)
        sys.exit(1)

    entry_lines = [lines[start_idx]]
    for i in range(start_idx + 1, len(lines)):
        line = lines[i]
        if re.match(r'^- \[[ x]\] \d+\.\d+', line) or re.match(r'^##', line):
            break
        if line.strip() == '':
            if i + 1 < len(lines) and (re.match(r'^- \[[ x]\] \d+\.\d+', lines[i + 1]) or re.match(r'^##', lines[i + 1])):
                break
            entry_lines.append(line)
        else:
            entry_lines.append(line)

    return '\n'.join(entry_lines).strip()


def scope_task_for_rewrite(task_entry: str) -> str:
    """把主 agent 视角的完整 task 块裁成改写 subagent 视角。

    保留标题行 + 「subagent 上下文（改写输入）」块（含实现上下文 Props/数据绑定/事件、
    变体及其 export_image、children）——这些正是改写 subagent 的输入；剥掉主 agent 已执行的
    编排步骤（prepare/get_node_data/list-masters/normalize/build-*/spawn Agent），避免误导
    改写 subagent 去重跑 relay-schema 工具链或嵌套 spawn。无「subagent 上下文」块时只留标题。
    """
    lines = task_entry.split('\n')
    title = lines[0].strip() if lines else ''
    ctx_idx = next((i for i, ln in enumerate(lines) if re.search(r'subagent 上下文', ln)), -1)
    if ctx_idx == -1:
        return title
    return '\n'.join([title] + lines[ctx_idx:]).rstrip()


def build_prompt(task: str, component_id: str, static_file: str, skeleton_file: str) -> str:
    # 1. Read ui-protocol.md and extract rewrite rules
    protocol_path = os.path.join(SCRIPT_DIR, 'ui-protocol.md')
    protocol_content = read_file(protocol_path)
    rewrite_rules = extract_rewrite_rules(protocol_content)

    # 2. Read context.md for design params
    context_path = f'delivery/{task}/context.md'
    context_content = read_file(context_path)
    params = extract_design_params(context_content)

    # 3. Read tasks.md and extract the component entry
    tasks_path = f'openspec/changes/{task}/tasks.md'
    tasks_content = read_file(tasks_path)
    raw_entry = extract_task_entry(tasks_content, component_id)
    task_entry = scope_task_for_rewrite(raw_entry)
    root_node_id = parse_root_node_id(raw_entry.split('\n', 1)[0])

    # 4. design.md path (TRD)
    design_path = f'openspec/changes/{task}/design.md'
    design_content = read_file(design_path)
    tech_stack = extract_tech_stack(design_content)

    # 4b. 组件规格投影：按 rootNodeId 把三层变体 + 数据绑定 + 交互 + §6 边界聚合成 co-located 视图
    #     对照表（组件归属源）从 schema 层读入；软读取回退 design.md（兼容对照表仍内联 §2.1 的老稿）
    table_content = load_component_table(task)
    projection = build_component_projection(design_content, root_node_id, table_content or None) if root_node_id else ''
    if projection:
        projection_block = (
            "## 组件规格投影（本组件 rootNodeId 聚合视图）\n"
            "> 本组件在 TRD 里散落各章的行（数量①/状态②/内容③/数据绑定/交互/§6 边界）已聚合到此。\n"
            "> **实现条件渲染/数据绑定/交互/边界兜底以此为准**——co-located 视图，勿再去翻整章大表找自己的行。\n\n"
            f"{projection}"
        )
    else:
        projection_block = ''

    # 投影已按组件聚合携带 变体/数据绑定/交互/边界 → trd_block 剔除这些整章全表，避免与投影重复；
    # trd_block 只留投影没有的（组件接入 + 状态视图矩阵 + 补充规则 + 接口等）。投影缺席则不剔、全量兜底。
    exclude = (set(_section_numbers_by_heading(
        design_content, ['组件变体矩阵', '数据绑定', '交互清单', '数据边界兜底'])) if projection else frozenset())
    trd_sections = build_trd_section(design_content, task_entry, exclude_sections=exclude)

    # 5. Assemble prompt
    tech_intro = f"你正在为一个 {tech_stack} 项目改写 UI 组件。" if tech_stack else "你正在改写 UI 组件。请从 design.md 第 1 章确认项目技术栈后再编码。"

    if trd_sections.strip():
        trd_block = (
            "## 相关 TRD 章节（已摘录，无需再读 design.md）\n"
            "> 组件接入 + 状态视图矩阵 + 补充规则 + 接口等**投影没有的**整节。**变体/数据绑定/交互/边界见上方「组件规格投影」的本组件聚合行**，此处不再重复。\n"
            f"> **直接用这里的内容实现**；仅当 task 引用了下方未摘录的章节时，才 Read `{design_path}` 补读。\n\n"
            f"{trd_sections}"
        )
    else:
        trd_block = (
            "## 设计文档（TRD）\n"
            f"- 路径: {design_path}\n"
            "- 你的任务里「实现上下文」的 Props / 数据绑定 / 业务规则字段多以 `{TRD§x.x}` 形式引用，"
            "指向本 TRD 的对应章节。**遇到此类引用，必须先 Read 该文件对应章节拿到真实内容，"
            "再据此实现——不得凭引用字面或经验假设 Props 字段、绑定路径、状态分支。**"
        )

    prompt = f"""{tech_intro}

## 静态稿（relay 图层生成）
- 文件路径: {static_file}
- 请先 Read 该文件获取静态稿代码作为改写基线

## 注入参数
- CDN 倍率: {params['cdn_scale']}（仅用于 export_image 的 scale 参数）
- 骨架文件: {skeleton_file}
  - Read 该文件，搜索 `<ComponentName />` 定位本组件在骨架中的位置
  - 读取包裹本组件的父容器的样式声明，编写组件时需考虑父容器样式对当前组件根元素的影响，避免与之冲突或冗余

{trd_block}

{projection_block}

## 保留静态稿的图片渲染
静态稿已按渲染判定生成图片（切图 + 正式 CDN URL）。改写时**保留这些图片的结构与 URL，禁止改成文本/CSS**；你只加数据绑定，不重判渲染方式。

## 你的任务
> 静态稿已由上一阶段生成，你只需 Read 静态稿文件 + Edit 原地改写；**禁止重跑 relay-schema 工具链 / get_node_data / normalize，也不要再 spawn 任何 subagent**（那些是主 agent 已完成的编排）。变体差异图的 `export_image` 例外——task 标注了才执行。
{task_entry}

## 输出要求
基于静态稿原地改写为功能完整的业务组件（直接 Edit 静态稿文件，不新建文件）。

---

{rewrite_rules}"""

    return prompt


def _slug(name: str) -> str:
    """组件名 → 文件名安全片段（保留字母数字/中文/-_，其余转下划线，供投影文件名带组件名用）。"""
    s = re.sub(r'[^\w一-鿿-]+', '_', name.strip()).strip('_')
    return s or 'component'


def _write_under_root(rel_path: str, content: str) -> str:
    full = rel_path if os.path.isabs(rel_path) else os.path.join(PROJECT_ROOT, rel_path)
    os.makedirs(os.path.dirname(full), exist_ok=True)
    with open(full, 'w', encoding='utf-8') as f:
        f.write(content)
    return rel_path


def emit_projections(task: str):
    """TRD 出口评审视图：按 §2.1 对照表逐组件装配「组件规格投影」，落 review 产物。

    与 rewrite 模式共用 build_component_projection（同一装配器两处用）。组件清单来自
    §2.1 对照表 heading `### 组件：<name> (rootNodeId: X)`——对照表已移出 design.md 正文、
    落 `delivery/<task>/schema/component-table.md`；软读取回退 design.md（兼容老稿）。
    变体/绑定/交互/边界表始终取自 design.md。
    """
    design_path = f'openspec/changes/{task}/design.md'
    design_content = read_file(design_path)
    table_content = load_component_table(task)
    comps = re.findall(
        r'^#{2,4}\s*组件[：:]\s*(.+?)\s*[（(]\s*rootNodeId:\s*`?(\d+:\d+)`?\s*[)）]',
        table_content or design_content, re.M)
    if not comps:
        print(json.dumps({
            'warning': '未找到「### 组件：<name> (rootNodeId: X)」对照表标题，无投影可生成'
                       '（查 delivery/<task>/schema/component-table.md 或 design.md §2.1）',
            'design': design_path,
        }, ensure_ascii=False, indent=2))
        return

    written = []
    for name, rnid in comps:
        proj = build_component_projection(design_content, rnid, table_content or None)
        content = (
            f"# 组件规格投影：{name.strip()} (rootNodeId: {rnid})\n\n"
            "> 生成物（build-ui-component-prompt.py --emit-projections）；主题章仍是 source of truth，勿双写。\n"
            "> 供评审「单视角自洽核对」：本组件 数量/状态/内容/绑定/交互/边界 是否自洽——尤其\n"
            "> **状态变体②声明的态 vs 交互§4.2 的行为**是否矛盾（如声明「已领」态却在交互里「本券不可领」），\n"
            "> 每态有归宿、每绑定字段有接口支撑、四类边界有兜底。\n\n"
            f"{proj}\n"
        )
        rel = f'delivery/{task}/review/component-projections/{_slug(name)}_{rnid.replace(":", "_")}.md'
        _write_under_root(rel, content)
        written.append(rel)

    print(json.dumps({
        'emitted': len(written),
        'files': written,
        'next': '人工/reviewer 逐份核对组件故事自洽性（warning 级，不阻断定稿；升 blocking 归闸门一）',
    }, ensure_ascii=False, indent=2))


def main():
    parser = argparse.ArgumentParser(description='Build UI subagent prompt (rewrite) / emit 组件规格投影 (review)')
    parser.add_argument('--task', required=True, help='Task name (e.g. uplink-jump)')
    parser.add_argument('--emit-projections', action='store_true',
                        help='TRD 出口评审视图模式：按 §2.1 对照表逐组件装配组件规格投影，落 '
                             'delivery/<task>/review/component-projections/（只需 --task）')
    parser.add_argument('--component-id', help='Task entry ID (e.g. "5.1")（rewrite 模式必填）')
    parser.add_argument('--static-file',
                        help='静态稿文件路径（rewrite 模式必填，如 "src/components/CouponCard/index.tsx"）')
    parser.add_argument('--skeleton-file',
                        help='骨架 JSX 文件路径（rewrite 模式必填，如 "src/pages/index.tsx"）')
    parser.add_argument('--emit-to', help='prompt 写盘路径（默认 delivery/<task>/workspace/prompts/<id>.rewrite.md）')
    parser.add_argument('--stdout', action='store_true', help='强制把完整 prompt 打到 stdout（调试；默认落盘只回路径）')

    args = parser.parse_args()

    if args.emit_projections:
        emit_projections(args.task)
        return

    missing = [f'--{n.replace("_", "-")}' for n in ('component_id', 'static_file', 'skeleton_file')
               if not getattr(args, n)]
    if missing:
        parser.error('rewrite 模式需要 ' + ' / '.join(missing))

    prompt = build_prompt(args.task, args.component_id, args.static_file, args.skeleton_file)
    emit_prompt(prompt, args.task, args.component_id, 'rewrite', args.emit_to, args.stdout)


if __name__ == '__main__':
    main()
