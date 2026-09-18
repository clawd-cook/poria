export function isQualityGateIssue(issueClass: string | null | undefined): boolean {
  if (!issueClass) {
    return false;
  }
  const normalized = issueClass.toLowerCase().replace(/[^a-z_]/g, "");
  return (
    normalized === "cibuild" ||
    normalized === "ci_build" ||
    normalized === "testcoverage" ||
    normalized === "test_coverage" ||
    normalized === "infrafailure" ||
    normalized === "testfailure"
  );
}

export function isCoverageGate(issueClass: string, message: string): boolean {
  const lower = message.toLowerCase();
  const normalized = issueClass.toLowerCase();
  return (
    normalized.includes("test_coverage") ||
    normalized.includes("testcoverage") ||
    lower.includes("test_coverage") ||
    lower.includes("coverage missing") ||
    message.includes("覆盖率")
  );
}

export function isSecurityScanGate(issueClass: string, message: string): boolean {
  const lower = message.toLowerCase();
  const normalized = issueClass.toLowerCase().replace(/[^a-z]/g, "");
  return (
    (normalized.includes("securityviolation") &&
      (lower.includes("security_scan") ||
        message.includes("安全扫描") ||
        lower.includes("audit"))) ||
    lower.includes("security_scan")
  );
}

export function isQualityGateBlock(
  issueClass: string | null | undefined,
  message: string,
): boolean {
  const lower = message.toLowerCase();
  return (
    isQualityGateIssue(issueClass) ||
    isSecurityScanGate(issueClass ?? "", message) ||
    lower.includes("ci_build") ||
    lower.includes("coverage missing") ||
    lower.includes("test_coverage") ||
    lower.includes("security_scan") ||
    message.includes("CI 构建") ||
    message.includes("测试覆盖率")
  );
}

export function qualityGateTitle(issueClass: string, message: string): string {
  if (isSecurityScanGate(issueClass, message)) {
    return "安全扫描未通过，无法进入下一阶段";
  }
  if (isCoverageGate(issueClass, message)) {
    return "覆盖率报告缺失或未达标，不能当作合并准入";
  }
  return "CI 未通过或尚无结果，不能当作合并准入";
}
