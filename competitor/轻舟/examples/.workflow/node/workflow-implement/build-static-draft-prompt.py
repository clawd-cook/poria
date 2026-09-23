#!/usr/bin/env python3
"""
Static-draft Subagent Prompt Builder

静态稿 subagent 的 prompt 组装脚本。
主 agent spawn 静态稿 subagent 前必须先调本脚本，将 stdout 原样传给 Agent()。

自动注入：静态稿 subagent 规则（从 ui-protocol.md 提取）+ CDN 倍率 + 精简 schema 路径
+ 产出目录 + tech stack + task 条目的 subagent 上下文。
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


def extract_static_draft_rules(protocol_content: str) -> str:
    """Extract content from '## 静态稿 subagent 规则' heading to the next '## ' heading.

    以整行标题为锚（re.M + 行尾），避免误命中「subagent 编排」段里反引号包裹的
    `## 静态稿 subagent 规则` 提及（它出现在真正标题之前）。
    """
    marker = '## 静态稿 subagent 规则'
    m = re.search(r'^' + re.escape(marker) + r'\s*$', protocol_content, re.M)
    if not m:
        print("ERROR: '## 静态稿 subagent 规则' section not found in ui-protocol.md", file=sys.stderr)
        sys.exit(1)
    rest = protocol_content[m.end():]
    next_h2 = re.search(r'\n## ', rest)
    section = rest[:next_h2.start()] if next_h2 else rest
    return (marker + section).strip()


def extract_design_params(context_content: str) -> dict:
    result = {'cdn_scale': '2', 'design_id': ''}
    cdn_match = re.search(r'CDN\s*(?:图片导出)?倍率[：:]\s*(\d+)', context_content)
    if cdn_match:
        result['cdn_scale'] = cdn_match.group(1)
    # 从 relay 设计稿链接解析 designId（get_node_data / export_image 必传）
    url_match = re.search(r'https?://relay\.jd\.com/file/design\?[^\s\)]+', context_content)
    if url_match:
        id_match = re.search(r'[?&]id=([^&\s]+)', url_match.group(0))
        if id_match:
            result['design_id'] = id_match.group(1)
    return result


def extract_tech_stack(design_content: str) -> str:
    """轻量提取 TRD 第 1 章技术栈；取不到返回 ''（调用方退化成"自行确认"）。"""
    for pattern in [
        r'(?:技术栈|技术选型|Tech Stack)[：:][ \t]*([^\n]+)',
        r'(?:框架|Framework)[：:][ \t]*([^\n]+)',
    ]:
        m = re.search(pattern, design_content, re.IGNORECASE)
        if m:
            value = re.sub(r'[*`]', '', m.group(1)).strip(' -：:')
            if value and not value.startswith('|') and len(value) >= 3:
                return value
    return ''


def extract_task_entry(tasks_content: str, component_id: str) -> str:
    """Extract a task entry by its ID (e.g. '5.1') including nested sub-steps and subagent context."""
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


def scope_task_for_static(task_entry: str) -> str:
    """把主 agent 视角的完整 task 块裁成静态稿 subagent 视角。

    静态稿只做「schema→纯视觉还原」，需要的组件标识（nodeId/名）在标题行即可；
    其余子行要么是主 agent 已执行的编排步骤（prepare/get_node_data/list-masters/
    normalize/build-*/spawn Agent），要么是「subagent 上下文（改写输入）」的 Props/
    数据绑定/事件（那是改写阶段的输入，静态稿不碰）——一律剥掉，避免误导静态稿
    subagent 去重跑 MCP、嵌套 spawn 或提前做数据绑定。
    """
    return task_entry.split('\n', 1)[0].strip()


def build_prompt(task: str, component_id: str, schema_file: str, static_file: str) -> str:
    protocol_content = read_file(os.path.join(SCRIPT_DIR, 'ui-protocol.md'))
    rules = extract_static_draft_rules(protocol_content)

    context_content = read_file(f'delivery/{task}/context.md')
    params = extract_design_params(context_content)

    tasks_content = read_file(f'openspec/changes/{task}/tasks.md')
    task_entry = scope_task_for_static(extract_task_entry(tasks_content, component_id))

    design_content = read_file(f'openspec/changes/{task}/design.md')
    tech_stack = extract_tech_stack(design_content)

    tech_intro = (f"你正在为一个 {tech_stack} 项目生成 UI 静态稿。"
                  if tech_stack else
                  "你正在生成 UI 静态稿。请从 design.md 第 1 章确认项目技术栈后再编码。")

    prompt = f"""{tech_intro}

## 精简 schema（relay 图层规范化产物）
- 文件路径: {schema_file}
- 请先 Read 该文件，它是本静态稿的**唯一样式与布局来源**

## 产出目录
- {static_file}
- 按技术栈写入组件文件（tsx/jsx/vue）+ 样式文件（scss/less）

## 注入参数
- designId（zero-design fileKey，get_node_data / export_image 必传）: {params['design_id'] or '（未解析到，见 context.md「UI 参数」段设计稿链接的 id）'}
- CDN 图片导出倍率: {params['cdn_scale']}（仅用于 export_image 的 scale 参数）

## 你的任务
{task_entry}

## 输出要求
基于 schema 生成静态稿（纯视觉还原，无 Props/数据/交互），写入产出目录。

---

{rules}"""
    return prompt


def main():
    parser = argparse.ArgumentParser(description='Build static-draft subagent prompt')
    parser.add_argument('--task', required=True, help='Task name (e.g. uplink-jump)')
    parser.add_argument('--component-id', required=True, help='Task entry ID (e.g. "5.1")')
    parser.add_argument('--schema-file', required=True, help='Path to normalized schema.json')
    parser.add_argument('--static-file', required=True,
                        help='Output dir for the static draft (e.g. "src/components/CouponCard")')
    parser.add_argument('--emit-to', help='prompt 写盘路径（默认 delivery/<task>/workspace/prompts/<id>.static.md）')
    parser.add_argument('--stdout', action='store_true', help='强制把完整 prompt 打到 stdout（调试；默认落盘只回路径）')

    args = parser.parse_args()
    prompt = build_prompt(args.task, args.component_id, args.schema_file, args.static_file)
    emit_prompt(prompt, args.task, args.component_id, 'static', args.emit_to, args.stdout)


if __name__ == '__main__':
    main()
