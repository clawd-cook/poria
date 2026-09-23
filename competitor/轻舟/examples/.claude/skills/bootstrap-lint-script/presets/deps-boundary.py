#!/usr/bin/env python3
"""分层依赖边界静态检查（preset 模板，落地为 .workflow/scripts/lint/lint-deps.py）。

检查多模块 Java 项目的分层依赖方向：api 不依赖实现层、service 不反向依赖
controller、repository 不反向调用 service 等。检查逻辑通用；模块名、包名是
项目参数，落地时由 /bootstrap-lint-script 填写下方 CONFIG 区。RULES 中的
包结构假设（如 service.repository / service.model.domain）也要按项目实际
分层核对增删，不只是填参。
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Callable, Iterable, List, Optional


# 按 .workflow/ 标记向上找项目根，而非写死层级——脚本落盘深度以后再挪也不会断。
_HERE = Path(__file__).resolve()
ROOT = next((p for p in _HERE.parents if (p / ".workflow").is_dir()), _HERE.parents[3])

# ===== CONFIG：项目参数，存在 FILL-ME 时脚本拒绝运行 =====
BASE_PACKAGE = "FILL-ME"  # 项目基础包名，如 com.example.order
MODULE_PREFIX = "FILL-ME"  # 模块目录前缀，如 order-（扫描 <前缀>* 目录）
API_MODULE = "FILL-ME"  # 契约层模块目录名，如 order-api
CONTROLLER_MODULE = "FILL-ME"  # 接入层模块目录名，如 order-controller
SERVICE_MODULE = "FILL-ME"  # 业务层模块目录名，如 order-service
# ========================================================


@dataclass
class JavaFile:
    path: Path
    rel_path: Path
    module: str
    source_root: Path
    is_test: bool
    package: Optional[str]
    imports: List[str]


@dataclass
class Finding:
    severity: str
    code: str
    path: Path
    detail: str
    why: str
    how: str


@dataclass
class Rule:
    code: str
    severity: str
    applies: Callable[[JavaFile], bool]
    check: Callable[[JavaFile], Iterable[str]]
    why: str
    how: str


IMPORT_RE = re.compile(r"^\s*import\s+(?:static\s+)?([^;]+);", re.MULTILINE)
PACKAGE_RE = re.compile(r"^\s*package\s+([^;]+);", re.MULTILINE)
BLOCK_COMMENT_RE = re.compile(r"/\*.*?\*/", re.DOTALL)
LINE_COMMENT_RE = re.compile(r"^\s*//.*$", re.MULTILINE)
LINT_IGNORE_RE = re.compile(r"@\s*LintIgnore\b")


def strip_java_comments(text: str) -> str:
    without_block = BLOCK_COMMENT_RE.sub("", text)
    return LINE_COMMENT_RE.sub("", without_block)


def split_code_lines(text: str) -> List[str]:
    return text.splitlines()


def next_code_line(lines: List[str], start: int) -> Optional[str]:
    index = start
    while index < len(lines):
        line = lines[index].strip()
        if line:
            return line
        index += 1
    return None


def has_class_level_lint_ignore(text: str) -> bool:
    lines = split_code_lines(text)
    for i, line in enumerate(lines):
        if not LINT_IGNORE_RE.search(line):
            continue
        if any(token in line for token in (" class ", " interface ", " enum ", " record ")):
            return True
        candidate = next_code_line(lines, i + 1)
        if candidate is None:
            continue
        if any(token in candidate for token in (" class ", " interface ", " enum ", " record ")):
            return True
        if candidate.startswith(("class ", "interface ", "enum ", "record ")):
            return True
    return False


def load_java_files() -> List[JavaFile]:
    files: List[JavaFile] = []
    for module_dir in sorted(ROOT.glob(f"{MODULE_PREFIX}*")):
        if not module_dir.is_dir():
            continue
        module = module_dir.name
        for source_root in [module_dir / "src/main/java", module_dir / "src/test/java"]:
            if not source_root.exists():
                continue
            for path in sorted(source_root.rglob("*.java")):
                text = path.read_text(encoding="utf-8")
                effective_text = strip_java_comments(text)
                if not effective_text.strip():
                    continue
                if has_class_level_lint_ignore(effective_text):
                    continue
                package_match = PACKAGE_RE.search(effective_text)
                files.append(
                    JavaFile(
                        path=path,
                        rel_path=path.relative_to(ROOT),
                        module=module,
                        source_root=source_root,
                        is_test="src/test/java" in source_root.as_posix(),
                        package=package_match.group(1) if package_match else None,
                        imports=IMPORT_RE.findall(effective_text),
                    )
                )
    return files


def has_import(java_file: JavaFile, prefixes: Iterable[str]) -> List[str]:
    return [
        imported
        for imported in java_file.imports
        if any(imported == prefix or imported.startswith(prefix + ".") for prefix in prefixes)
    ]


def expected_package(java_file: JavaFile) -> str:
    parent = java_file.path.parent.relative_to(java_file.source_root)
    return ".".join(parent.parts)


def check_package_matches_path(java_file: JavaFile) -> Iterable[str]:
    if java_file.package is None:
        yield "缺少 package 声明"
        return
    expected = expected_package(java_file)
    if java_file.package != expected:
        yield f"package 声明为 `{java_file.package}`，但源码路径期望 `{expected}`"


def import_rule(prefixes: Iterable[str]) -> Callable[[JavaFile], Iterable[str]]:
    def check(java_file: JavaFile) -> Iterable[str]:
        for imported in has_import(java_file, prefixes):
            yield f"禁止的 import: `{imported}`"

    return check


def is_repository_file(java_file: JavaFile) -> bool:
    return (
        java_file.module == SERVICE_MODULE
        and not java_file.is_test
        and java_file.package is not None
        and java_file.package.startswith(f"{BASE_PACKAGE}.service.repository")
    )


RULES = [
    Rule(
        code="package-path-mismatch",
        severity="warning",
        applies=lambda f: True,
        check=check_package_matches_path,
        why="Java package 与目录不一致会影响组件扫描、重构和架构 lint 判断。",
        how="修正 package 声明，或将文件移动到与 package 对应的目录。",
    ),
    Rule(
        code="service-no-controller",
        severity="error",
        applies=lambda f: f.module == SERVICE_MODULE and not f.is_test,
        check=import_rule([f"{BASE_PACKAGE}.controller"]),
        why="业务层反向依赖 Controller 会破坏 service -> controller 的分层边界。",
        how="将 Controller 入参转换留在接入层，Service 只接收 DTO、领域参数或基础类型。",
    ),
    Rule(
        code="api-no-upper-layer",
        severity="error",
        applies=lambda f: f.module == API_MODULE and not f.is_test,
        check=import_rule([f"{BASE_PACKAGE}.service", f"{BASE_PACKAGE}.controller"]),
        why="api 模块是契约层，不能依赖业务实现层或 HTTP 接入层。",
        how="把实现逻辑移到 service/controller，api 中只保留 DTO、枚举、注解等契约对象。",
    ),
    Rule(
        code="repository-no-service",
        severity="error",
        applies=is_repository_file,
        check=import_rule([f"{BASE_PACKAGE}.service.service"]),
        why="Repository/Mapper 是持久化边界，不应反向调用业务服务。",
        how="将业务编排放到 service 层，Repository 只保留数据访问接口。",
    ),
    Rule(
        code="controller-direct-repository",
        severity="warning",
        applies=lambda f: f.module == CONTROLLER_MODULE,
        check=import_rule([f"{BASE_PACKAGE}.service.repository"]),
        why="Controller 直连 Repository 会绕过 Service 编排，分散事务和业务规则。",
        how="新增逻辑请通过 Service；历史命中先作为技术债暴露。",
    ),
    Rule(
        code="controller-domain-coupling",
        severity="warning",
        applies=lambda f: f.module == CONTROLLER_MODULE,
        check=import_rule([f"{BASE_PACKAGE}.service.model.domain"]),
        why="Controller 直接依赖 domain 会放大接入层与领域模型耦合。",
        how="对外入参/出参优先使用 api.dto 或专用 param/result 对象。",
    ),
    Rule(
        code="api-framework-leak",
        severity="warning",
        applies=lambda f: f.module == API_MODULE,
        check=import_rule(["org.springframework", "javax.servlet"]),
        why="契约层依赖 Web 框架会降低复用性，并扩大下游调用方依赖面。",
        how="新增契约优先使用普通 Java 类型；历史兼容命中先作为 warning。",
    ),
]


def collect_findings(java_files: Iterable[JavaFile]) -> List[Finding]:
    findings: List[Finding] = []
    for java_file in java_files:
        for rule in RULES:
            if not rule.applies(java_file):
                continue
            for detail in rule.check(java_file):
                findings.append(
                    Finding(
                        severity=rule.severity,
                        code=rule.code,
                        path=java_file.rel_path,
                        detail=detail,
                        why=rule.why,
                        how=rule.how,
                    )
                )
    return findings


def print_finding(finding: Finding) -> None:
    marker = "ERROR" if finding.severity == "error" else "WARN"
    print(f"[{marker}] {finding.code}: {finding.path}")
    print(f"  WHAT: {finding.detail}")
    print(f"  WHY:  {finding.why}")
    print(f"  HOW:  {finding.how}")
    print()


def main() -> int:
    unfilled = [
        name
        for name, value in (
            ("BASE_PACKAGE", BASE_PACKAGE),
            ("MODULE_PREFIX", MODULE_PREFIX),
            ("API_MODULE", API_MODULE),
            ("CONTROLLER_MODULE", CONTROLLER_MODULE),
            ("SERVICE_MODULE", SERVICE_MODULE),
        )
        if value == "FILL-ME"
    ]
    if unfilled:
        print(
            "lint-deps: CONFIG 区参数未填写: " + ", ".join(unfilled) + "。\n"
            "本脚本是 preset 模板，需由 /bootstrap-lint-script 探参填写后才可运行。",
            file=sys.stderr,
        )
        return 2

    java_files = load_java_files()
    if not java_files:
        print(
            f"lint-deps: 在 {ROOT} 下未扫到任何 Java 源文件（{MODULE_PREFIX}*/src/main|test/java）。\n"
            "闸门拒绝静默通过：请检查 MODULE_PREFIX 与项目模块布局是否一致。",
            file=sys.stderr,
        )
        return 2

    findings = collect_findings(java_files)

    for finding in findings:
        print_finding(finding)

    error_count = sum(1 for finding in findings if finding.severity == "error")
    warning_count = sum(1 for finding in findings if finding.severity == "warning")
    print(
        f"lint-deps summary: scanned {len(java_files)} Java files, "
        f"{error_count} error(s), {warning_count} warning(s)."
    )

    return 1 if error_count else 0


if __name__ == "__main__":
    sys.exit(main())
