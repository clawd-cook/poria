export const AUTH_EXPIRED_ISSUE_CLASS = "auth_expired";

export function isAuthExpiredIssue(issueClass: string | null | undefined): boolean {
  if (!issueClass) {
    return false;
  }
  const normalized = issueClass.toLowerCase().replace(/[^a-z]/g, "");
  return normalized === "authexpired" || normalized.includes("authexpired");
}

export function isAuthExpiredMessage(message: string): boolean {
  const lower = message.toLowerCase();
  return (
    message.includes("请先登录") ||
    message.includes("登录已过期") ||
    message.includes("请重新登录") ||
    lower.includes("auth expired") ||
    lower.includes("authexpired") ||
    lower.includes("authrequired") ||
    lower.includes("cookie expired") ||
    lower.includes("sso cookie") ||
    lower.includes("http 401")
  );
}
