#!/usr/bin/env python3
"""
Skeleton Subagent Prompt Builder

骨架层 subagent 的 prompt 组装脚本。
主 agent spawn 骨架 subagent 前必须先调本脚本，将 stdout 原样传给 Agent()。

自动注入：骨架 subagent 规则（从 ui-protocol.md 提取）+ CDN 倍率 + task 条目的 subagent 上下文。
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


def extract_skeleton_rules(protocol_content: str) -> str:
    """Extract content from '## 骨架 subagent 规则' heading to the next '## ' heading.

    以整行标题为锚（re.M + 行尾），避免误命中「subagent 编排」段里反引号包裹的
    `## 骨架 subagent 规则` 提及（它出现在真正标题之前）。
    """
    marker = '## 骨架 subagent 规则'
    m = re.search(r'^' + re.escape(marker) + r'\s*$', protocol_content, re.M)
    if not m:
        print("ERROR: '## 骨架 subagent 规则' section not found in ui-protocol.md", file=sys.stderr)
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


def extract_task_entry(tasks_content: str, skeleton_id: str) -> str:
    """Extract a skeleton task entry by its ID (e.g. '4.1') including all nested content."""
    pattern = rf'^- \[[ x]\] {re.escape(skeleton_id)}\s'
    lines = tasks_content.split('\n')
    start_idx = -1
    for i, line in enumerate(lines):
        if re.match(pattern, line):
            start_idx = i
            break

    if start_idx == -1:
        print(f"ERROR: task entry '{skeleton_id}' not found in tasks.md", file=sys.stderr)
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


def build_prompt(task: str, skeleton_id: str) -> str:
    # 1. Read ui-protocol.md and extract skeleton rules
    protocol_path = os.path.join(SCRIPT_DIR, 'ui-protocol.md')
    protocol_content = read_file(protocol_path)
    skeleton_rules = extract_skeleton_rules(protocol_content)

    # 2. Read context.md for CDN scale
    context_path = f'delivery/{task}/context.md'
    context_content = read_file(context_path)
    params = extract_design_params(context_content)

    # 3. Read tasks.md and extract the skeleton entry
    tasks_path = f'openspec/changes/{task}/tasks.md'
    tasks_content = read_file(tasks_path)
    task_entry = extract_task_entry(tasks_content, skeleton_id)

    # 4. Assemble prompt
    prompt = f"""你正在编写页面骨架（JSX + 样式文件）。

## 注入参数
- CDN 图片导出倍率: {params['cdn_scale']}（用于 export_image 的 scale 参数）

## 你的任务
{task_entry}

---

{skeleton_rules}"""

    return prompt


def main():
    parser = argparse.ArgumentParser(description='Build skeleton subagent prompt')
    parser.add_argument('--task', required=True, help='Task name (e.g. uplink-jump)')
    parser.add_argument('--skeleton-id', required=True, help='Task entry ID (e.g. "4.1")')
    parser.add_argument('--emit-to', help='prompt 写盘路径（默认 delivery/<task>/workspace/prompts/<id>.skeleton.md）')
    parser.add_argument('--stdout', action='store_true', help='强制把完整 prompt 打到 stdout（调试；默认落盘只回路径）')

    args = parser.parse_args()
    prompt = build_prompt(args.task, args.skeleton_id)
    emit_prompt(prompt, args.task, args.skeleton_id, 'skeleton', args.emit_to, args.stdout)


if __name__ == '__main__':
    main()
