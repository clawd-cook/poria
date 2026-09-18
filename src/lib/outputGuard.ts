export const OUT_OF_SCOPE_ISSUE_CLASS = "out_of_scope_change";
export const SECURITY_VIOLATION_ISSUE_CLASS = "security_violation";

export function isOutOfScopeIssue(issueClass: string | null | undefined): boolean {
  if (!issueClass) {
    return false;
  }
  const normalized = issueClass.toLowerCase().replace(/[^a-z]/g, "");
  return normalized === "outofscopechange" || normalized.includes("outofscope");
}

export function isSecurityViolationIssue(issueClass: string | null | undefined): boolean {
  if (!issueClass) {
    return false;
  }
  const normalized = issueClass.toLowerCase().replace(/[^a-z]/g, "");
  return normalized === "securityviolation" || normalized.includes("securityviolation");
}

export function isOutputGuardMessage(message: string): boolean {
  const lower = message.toLowerCase();
  return (
    message.includes("OutputGuardError") ||
    lower.includes("out_of_scope") ||
    lower.includes("blocked_dependency") ||
    message.includes("不得进入 CR")
  );
}

export function isOutputGuardBlock(
  issueClass: string | null | undefined,
  message: string,
): boolean {
  return (
    isOutOfScopeIssue(issueClass) ||
    (isSecurityViolationIssue(issueClass) && isOutputGuardMessage(message)) ||
    isOutputGuardMessage(message)
  );
}
