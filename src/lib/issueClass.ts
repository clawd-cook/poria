export const ISSUE_CLASS_LABELS: Record<string, string> = {
  agent_timeout: "Agent 超时",
  auth_expired: "登录已过期",
  awaiting_advance: "阶段完成待确认",
  ci_build: "CI 未通过",
  compilation_error: "编译失败",
  diff_too_large: "变更过大",
  infra_failure: "基础设施失败",
  llm_rate_limit: "模型限流",
  low_cr_score: "CR 评分不足",
  merge_conflict: "合并冲突",
  out_of_scope_change: "超出允许范围",
  permission_denied: "Git 权限不足",
  prd_invalid: "需求链接或 PRD 无效",
  requirement_ambiguous: "需求不清晰",
  security_violation: "安全扫描未通过",
  test_coverage: "覆盖率未达标",
  test_failure: "测试失败",
  trd_unconfirmed: "TRD 未确认",
  unknown: "未分类异常",
  waiting_merge: "待合并确认",
};

export function issueClassKey(issueClass: string | null | undefined): string {
  const raw = (issueClass ?? "").trim();
  if (!raw) {
    return "";
  }
  if (raw.includes("_")) {
    return raw.toLowerCase();
  }
  return raw.replaceAll(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
}

export function issueClassLabel(issueClass: string | null | undefined): string {
  const key = issueClassKey(issueClass);
  if (!key) {
    return ISSUE_CLASS_LABELS.unknown;
  }
  return ISSUE_CLASS_LABELS[key] ?? issueClass ?? ISSUE_CLASS_LABELS.unknown;
}

export function issueClassTitle(issueClass: string, message: string): string | null {
  const key = issueClassKey(issueClass);
  switch (key) {
    case "compilation_error":
      return "编译失败，将按策略自动重试";
    case "test_failure":
      return "测试失败，将按策略自动重试";
    case "agent_timeout":
      return "Agent 已超时中止，可重试当前阶段";
    case "llm_rate_limit":
      return "模型限流，等待后自动重试";
    case "low_cr_score":
      return "CR 评分不足，已回退开发或需要人工确认";
    case "prd_invalid":
      return "需求链接或 PRD 无效，需要产品处理";
    case "permission_denied":
      return "Git 权限不足，需要运维协助";
    case "merge_conflict":
      return "存在合并冲突，需要开发处理";
    case "infra_failure":
    case "ci_build":
      return "基础设施或 CI 失败，需要运维协助";
    case "waiting_merge":
      return "MR 待审查人确认后合入（不会自动点合并）";
    case "awaiting_advance":
      return "阶段已完成，请确认后继续下一节点";
    default:
      if (message.includes("链接无效") || message.toLowerCase().includes("invalid url")) {
        return "需求链接无效，需要产品处理";
      }
      return null;
  }
}
