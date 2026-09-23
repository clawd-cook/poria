#!/usr/bin/env python3
"""扫描 MyBatis-Plus Wrapper 的字符串字段名用法。

preset 模板：仅适用于使用 MyBatis-Plus 的项目（探测 pom 依赖
com.baomidou:mybatis-plus*）。适用则原样落地为
.workflow/scripts/lint/lint-mybatis-wrapper.py，无需填参。

检查目标：
  1. 直接使用 QueryWrapper / UpdateWrapper（非 Lambda 版本）
  2. 使用 Wrappers.query() / Wrappers.update() 这类返回非 Lambda Wrapper 的静态方法
  3. Wrapper 条件方法（eq/ne/in/select/set/orderByDesc/...）以字符串字面量作为列名
  4. 使用 setSql(...) 注入更新 SQL 片段

为什么需要扫描：
  - 字符串列名无法被 IDE/编译器跟踪，重命名字段、梳理业务依赖时极易遗漏
  - 使用 LambdaQueryWrapper/LambdaUpdateWrapper + 方法引用（Entity::getField）可获得类型安全
  - setSql(...) 会把更新表达式变成裸 SQL 片段，字段变更和安全审查都难以追踪

用法：
  # 扫描当前脚本所在项目
  python3 .workflow/scripts/lint/lint-mybatis-wrapper.py

  # 扫描指定根目录（便于跨子项目复用）
  python3 .workflow/scripts/lint/lint-mybatis-wrapper.py --root /path/to/project

  # JSON 输出（便于接入其他工具）
  python3 .workflow/scripts/lint/lint-mybatis-wrapper.py --format json

  # 只看某个严重级别
  python3 .workflow/scripts/lint/lint-mybatis-wrapper.py --severity error

  # 使用 @LintIgnore 注解跳过个别声明（与 lint-quality 约定一致）
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Iterable, List, Optional, Tuple


# 按 .workflow/ 标记向上找项目根，而非写死层级——脚本落盘深度以后再挪也不会断。
_HERE = Path(__file__).resolve()
DEFAULT_ROOT = next((p for p in _HERE.parents if (p / ".workflow").is_dir()), _HERE.parents[3])

MP_IMPORT_HINT = "com.baomidou.mybatisplus"

# 方法名：MyBatis-Plus Wrapper 中以列名作为首个（或 boolean 条件之后的）参数的典型方法
# 取并集时区分两类：
#   - 强特征方法：名字在 MP 之外几乎不会出现 → 规则触发时置信度高
#   - 通用方法：名字在其他 API 中也常见（如 eq / in / set / select）→ 仅在文件导入了 MP 时才触发
STRONG_METHODS = (
    "likeLeft",
    "likeRight",
    "notLike",
    "notIn",
    "notBetween",
    "isNotNull",
    "isNull",
    "orderByAsc",
    "orderByDesc",
    "groupBy",
    "having",
    "allEq",
    "inSql",
    "notInSql",
    "ne",
    "gt",
    "ge",
    "lt",
    "le",
)

GENERIC_METHODS = (
    "eq",
    "in",
    "between",
    "like",
    "select",
    "set",
)

ALL_METHODS = STRONG_METHODS + GENERIC_METHODS


def _methods_regex(methods: Tuple[str, ...]) -> str:
    # 按长度倒序，避免 like 覆盖 likeLeft
    return "|".join(sorted(methods, key=len, reverse=True))


# 匹配：.method( [可选条件参数,] "列名" [, 或 )]
# 允许在列名前有一个条件表达式参数（MP 常见的 boolean 条件重载，如 .eq(flag, "col", val)）
STRONG_STRING_COLUMN_RE = re.compile(
    r"\.(?P<method>" + _methods_regex(STRONG_METHODS) + r")\s*\(\s*"
    r'(?:[^",()]{1,80},\s*)?'
    r'"(?P<column>[^"\n]+)"\s*[,)]'
)

GENERIC_STRING_COLUMN_RE = re.compile(
    r"\.(?P<method>" + _methods_regex(GENERIC_METHODS) + r")\s*\(\s*"
    r'(?:[^",()]{1,80},\s*)?'
    r'"(?P<column>[^"\n]+)"\s*[,)]'
)

# 非 Lambda Wrapper 实例化：QueryWrapper<> / UpdateWrapper<>
# 用负向预查避开 LambdaQueryWrapper/LambdaUpdateWrapper
NON_LAMBDA_WRAPPER_RE = re.compile(
    r"(?<![A-Za-z])(?<!Lambda)(?P<type>QueryWrapper|UpdateWrapper)\s*<"
)

# Wrappers.query() / Wrappers.update()（这些返回非 Lambda Wrapper）
WRAPPERS_NON_LAMBDA_RE = re.compile(
    r"\bWrappers\s*\.\s*(?P<kind>query|update)\s*\("
)

# LambdaUpdateWrapper#setSql(...) 仍然绕过字段方法引用，项目内禁止使用
SET_SQL_RE = re.compile(r"\.setSql\s*\(")

LINT_IGNORE_RE = re.compile(r"@\s*LintIgnore\b")


# ---------------- 数据结构 ----------------


@dataclass
class JavaFile:
    path: Path
    rel_path: Path
    text: str
    uses_mybatis_plus: bool
    ignore_entire_file: bool
    ignored_ranges: List[Tuple[int, int]]


@dataclass
class Finding:
    severity: str
    code: str
    path: str
    line: int
    detail: str
    why: str
    how: str
    snippet: str


# ---------------- 文件加载与注释剥离 ----------------


def iter_java_files(root: Path, excludes: Iterable[str]) -> Iterable[Path]:
    exclude_parts = {p.strip() for p in excludes if p.strip()}
    for path in root.rglob("*.java"):
        rel_parts = set(path.relative_to(root).parts)
        if rel_parts & exclude_parts:
            continue
        # 跳过 Maven 构建产物
        if "target" in path.parts or "generated-sources" in path.parts:
            continue
        yield path


def strip_line_comment(line: str) -> str:
    in_string = False
    escaped = False
    for index, char in enumerate(line):
        if escaped:
            escaped = False
            continue
        if char == "\\" and in_string:
            escaped = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if not in_string and line[index : index + 2] == "//":
            return line[:index]
    return line


def code_lines(text: str) -> Iterable[Tuple[int, str]]:
    """按行产出已剥离 // 和 /* */ 注释后的代码。"""
    in_block_comment = False
    for line_no, line in enumerate(text.splitlines(), start=1):
        code = line
        if in_block_comment:
            if "*/" not in code:
                continue
            code = code.split("*/", 1)[1]
            in_block_comment = False

        while "/*" in code:
            before, after = code.split("/*", 1)
            if "*/" in after:
                code = before + after.split("*/", 1)[1]
            else:
                code = before
                in_block_comment = True
                break

        code = strip_line_comment(code)
        if code.strip():
            yield line_no, code


def is_type_declaration(line: str) -> bool:
    stripped = line.strip()
    return (
        stripped.startswith(("class ", "interface ", "enum ", "record "))
        or " class " in stripped
        or " interface " in stripped
        or " enum " in stripped
    )


def detect_lint_ignore(text: str) -> Tuple[bool, List[Tuple[int, int]]]:
    logical_lines = list(code_lines(text))
    ignored_ranges: List[Tuple[int, int]] = []
    for i, (line_no, line) in enumerate(logical_lines):
        if not LINT_IGNORE_RE.search(line):
            continue
        if is_type_declaration(line):
            return True, []

        j = i + 1
        while j < len(logical_lines) and logical_lines[j][1].strip().startswith("@"):
            j += 1
        if j >= len(logical_lines):
            continue

        decl_line_no, decl_line = logical_lines[j]
        if is_type_declaration(decl_line):
            return True, []

        end_line_no = _find_member_end_line(logical_lines, j)
        ignored_ranges.append((decl_line_no, end_line_no))
    return False, ignored_ranges


def _find_member_end_line(
    logical_lines: List[Tuple[int, str]], start_idx: int
) -> int:
    brace_depth = 0
    seen_open_brace = False
    for idx in range(start_idx, len(logical_lines)):
        line_no, line = logical_lines[idx]
        open_count = line.count("{")
        close_count = line.count("}")
        if open_count > 0:
            seen_open_brace = True
        brace_depth += open_count
        brace_depth -= close_count
        if seen_open_brace and brace_depth <= 0:
            return line_no
        if not seen_open_brace and line.rstrip().endswith(";"):
            return line_no
    return logical_lines[start_idx][0]


def is_ignored_line(line_no: int, ranges: List[Tuple[int, int]]) -> bool:
    return any(start <= line_no <= end for start, end in ranges)


def load_java_files(root: Path, excludes: Iterable[str]) -> List[JavaFile]:
    files: List[JavaFile] = []
    for path in iter_java_files(root, excludes):
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        ignore_entire_file, ignored_ranges = detect_lint_ignore(text)
        files.append(
            JavaFile(
                path=path,
                rel_path=path.relative_to(root),
                text=text,
                uses_mybatis_plus=MP_IMPORT_HINT in text,
                ignore_entire_file=ignore_entire_file,
                ignored_ranges=ignored_ranges,
            )
        )
    return files


# ---------------- 规则匹配 ----------------


def _make_finding(
    severity: str,
    code: str,
    java_file: JavaFile,
    line_no: int,
    line: str,
    detail: str,
    why: str,
    how: str,
) -> Finding:
    return Finding(
        severity=severity,
        code=code,
        path=str(java_file.rel_path),
        line=line_no,
        detail=detail,
        why=why,
        how=how,
        snippet=line.strip()[:200],
    )


def scan_file(java_file: JavaFile) -> List[Finding]:
    if java_file.ignore_entire_file:
        return []
    # 非 MP 文件直接跳过，避免通用方法名（eq/in/set/select）误报
    if not java_file.uses_mybatis_plus:
        return []

    findings: List[Finding] = []
    for line_no, line in code_lines(java_file.text):
        if is_ignored_line(line_no, java_file.ignored_ranges):
            continue

        m = NON_LAMBDA_WRAPPER_RE.search(line)
        if m:
            findings.append(
                _make_finding(
                    "error",
                    "non-lambda-wrapper",
                    java_file,
                    line_no,
                    line,
                    f"第 {line_no} 行：使用了非 Lambda 版本的 {m.group('type')}",
                    "非 Lambda Wrapper 的列名是字符串，IDE/编译器无法追踪字段引用，重命名或梳理业务时极易遗漏。",
                    "改用 LambdaQueryWrapper / LambdaUpdateWrapper，或 Wrappers.lambdaQuery() / Wrappers.lambdaUpdate()，以 Entity::getField 方法引用替代字符串。",
                )
            )

        m = WRAPPERS_NON_LAMBDA_RE.search(line)
        if m:
            findings.append(
                _make_finding(
                    "error",
                    "wrappers-non-lambda-factory",
                    java_file,
                    line_no,
                    line,
                    f"第 {line_no} 行：Wrappers.{m.group('kind')}() 返回的是字符串列名 Wrapper",
                    "Wrappers.query()/update() 返回的是 QueryWrapper/UpdateWrapper，仍旧依赖字符串字段名。",
                    f"改为 Wrappers.lambda{'Query' if m.group('kind') == 'query' else 'Update'}() 并使用方法引用。",
                )
            )

        if SET_SQL_RE.search(line):
            findings.append(
                _make_finding(
                    "error",
                    "wrapper-set-sql",
                    java_file,
                    line_no,
                    line,
                    f"第 {line_no} 行：使用了 Wrapper#setSql(...) 拼接更新 SQL 片段",
                    "setSql(...) 绕过 Lambda 字段引用，字段重命名、逻辑删除、多租户条件和 SQL 安全审查都难以被工具追踪。",
                    "简单赋值改用 LambdaUpdateWrapper#set(Entity::getField, value)；自增、CASE WHEN、函数表达式等复杂更新下沉到 Mapper XML 的语义化方法。",
                )
            )

        for match in STRONG_STRING_COLUMN_RE.finditer(line):
            if _has_method_reference_before_column(line, match):
                continue
            findings.append(
                _make_finding(
                    "error",
                    "wrapper-string-column",
                    java_file,
                    line_no,
                    line,
                    f"第 {line_no} 行：.{match.group('method')}(...) 使用字符串列名 \"{match.group('column')}\"",
                    "字符串列名绕过类型系统，重命名字段时静默失效；跨模块追踪引用也无法靠 IDE。",
                    f"改用 Lambda Wrapper + 方法引用，例如 .{match.group('method')}(Entity::getXxx, ...)。",
                )
            )

        for match in GENERIC_STRING_COLUMN_RE.finditer(line):
            if _has_method_reference_before_column(line, match):
                continue
            # 通用方法名需要额外确认是在 wrapper 链上：
            # 简单启发式 → 本行或上一逻辑行中包含 Wrapper/wrapper/Wrappers，或以 . 起头表示延续链式调用
            if _looks_like_wrapper_chain(java_file.text, line_no, line):
                findings.append(
                    _make_finding(
                        "warning",
                        "wrapper-string-column",
                        java_file,
                        line_no,
                        line,
                        f"第 {line_no} 行：.{match.group('method')}(...) 疑似在 Wrapper 上使用字符串列名 \"{match.group('column')}\"",
                        "字符串列名绕过类型系统，重命名字段时静默失效；跨模块追踪引用也无法靠 IDE。",
                        f"若确为 MP Wrapper，改为 Lambda Wrapper + 方法引用；若是其他 API 可加 @LintIgnore 忽略。",
                    )
                )

    return findings


def _has_method_reference_before_column(line: str, match: re.Match) -> bool:
    return "::" in line[match.start("method") : match.start("column")]


def _looks_like_wrapper_chain(text: str, line_no: int, line: str) -> bool:
    stripped = line.lstrip()
    if stripped.startswith("."):
        return True
    # 回看最近若干行，看看是否涉及 Wrapper 变量 / 构造
    all_lines = text.splitlines()
    start = max(0, line_no - 6)
    window = "\n".join(all_lines[start : line_no])
    if re.search(r"\b[Ww]rapper\b|\bWrappers\s*\.", window):
        return True
    return False


# ---------------- 输出 ----------------


def print_text(findings: List[Finding], scanned: int) -> None:
    for f in findings:
        marker = "ERROR" if f.severity == "error" else "WARN"
        print(f"[{marker}] {f.code}: {f.path}:{f.line}")
        print(f"  WHAT: {f.detail}")
        print(f"  CODE: {f.snippet}")
        print(f"  WHY:  {f.why}")
        print(f"  HOW:  {f.how}")
        print()
    error_count = sum(1 for f in findings if f.severity == "error")
    warn_count = sum(1 for f in findings if f.severity == "warning")
    print(
        f"lint-mybatis-wrapper summary: scanned {scanned} Java files, "
        f"{error_count} error(s), {warn_count} warning(s)."
    )


def print_json(findings: List[Finding], scanned: int) -> None:
    payload = {
        "scanned_files": scanned,
        "findings": [asdict(f) for f in findings],
        "summary": {
            "error": sum(1 for f in findings if f.severity == "error"),
            "warning": sum(1 for f in findings if f.severity == "warning"),
        },
    }
    print(json.dumps(payload, ensure_ascii=False, indent=2))


# ---------------- 入口 ----------------


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT, help="扫描根目录（默认：脚本所在项目根）")
    parser.add_argument("--exclude", action="append", default=[], help="跳过的目录名，可多次指定")
    parser.add_argument("--format", choices=("text", "json"), default="text", help="输出格式")
    parser.add_argument(
        "--severity",
        choices=("all", "error", "warning"),
        default="all",
        help="只显示指定严重级别",
    )
    parser.add_argument(
        "--fail-on",
        choices=("never", "error", "any"),
        default="error",
        help="何时以非零退出码退出（默认：存在 error 时失败）",
    )
    args = parser.parse_args(argv)

    root: Path = args.root.resolve()
    if not root.exists():
        print(f"root not found: {root}", file=sys.stderr)
        return 2

    files = load_java_files(root, args.exclude)
    if not files:
        print(
            f"lint-mybatis-wrapper: 在 {root} 下未扫到任何 Java 源文件。\n"
            "闸门拒绝静默通过：请检查 --root 或脚本所在层级与项目源码布局。",
            file=sys.stderr,
        )
        return 2
    findings: List[Finding] = []
    for jf in files:
        findings.extend(scan_file(jf))

    if args.severity != "all":
        findings = [f for f in findings if f.severity == args.severity]

    findings.sort(key=lambda f: (f.path, f.line, f.code))

    if args.format == "json":
        print_json(findings, len(files))
    else:
        print_text(findings, len(files))

    has_error = any(f.severity == "error" for f in findings)
    if args.fail_on == "any" and findings:
        return 1
    if args.fail_on == "error" and has_error:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
