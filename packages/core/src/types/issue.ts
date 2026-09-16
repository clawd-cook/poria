export enum IssueClass {
  COMPILATION_ERROR = "compilation_error",
  TEST_FAILURE = "test_failure",
  AGENT_TIMEOUT = "agent_timeout",
  LLM_RATE_LIMIT = "llm_rate_limit",
  REQUIREMENT_AMBIG = "requirement_ambiguous",
  PRD_INVALID = "prd_invalid",
  MERGE_CONFLICT = "merge_conflict",
  LOW_CR_SCORE = "low_cr_score",
  DIFF_TOO_LARGE = "diff_too_large",
  PERMISSION_DENIED = "permission_denied",
  INFRA_FAILURE = "infra_failure",
  SECURITY_VIOLATION = "security_violation",
  OUT_OF_SCOPE = "out_of_scope_change",
  AUTH_EXPIRED = "auth_expired",
  UNKNOWN = "unknown",
}

export interface IssuePolicy {
  autoRetry: number;
  notifyRoles: string[];
  escalateAt: string | null;
  retryDelay?: string;
  note?: string;
}

export const ISSUE_POLICIES: Record<IssueClass, IssuePolicy> = {
  [IssueClass.COMPILATION_ERROR]: { autoRetry: 3, notifyRoles: ["developer"], escalateAt: "2h" },
  [IssueClass.TEST_FAILURE]: { autoRetry: 3, notifyRoles: ["developer"], escalateAt: "2h" },
  [IssueClass.AGENT_TIMEOUT]: { autoRetry: 1, notifyRoles: ["developer"], escalateAt: "1h" },
  [IssueClass.LLM_RATE_LIMIT]: { autoRetry: 5, notifyRoles: [], escalateAt: null, retryDelay: "5m" },
  [IssueClass.REQUIREMENT_AMBIG]: { autoRetry: 0, notifyRoles: ["product"], escalateAt: "4h" },
  [IssueClass.PRD_INVALID]: { autoRetry: 0, notifyRoles: ["product"], escalateAt: "4h" },
  [IssueClass.MERGE_CONFLICT]: { autoRetry: 0, notifyRoles: ["developer"], escalateAt: "2h" },
  [IssueClass.LOW_CR_SCORE]: { autoRetry: 1, notifyRoles: ["developer"], escalateAt: "4h" },
  [IssueClass.DIFF_TOO_LARGE]: { autoRetry: 0, notifyRoles: ["developer"], escalateAt: "4h" },
  [IssueClass.PERMISSION_DENIED]: { autoRetry: 0, notifyRoles: ["ops"], escalateAt: "1h" },
  [IssueClass.INFRA_FAILURE]: { autoRetry: 0, notifyRoles: ["ops"], escalateAt: "1h" },
  [IssueClass.SECURITY_VIOLATION]: { autoRetry: 0, notifyRoles: ["developer", "security"], escalateAt: "1h" },
  [IssueClass.OUT_OF_SCOPE]: { autoRetry: 0, notifyRoles: ["developer"], escalateAt: "2h" },
  [IssueClass.AUTH_EXPIRED]: { autoRetry: 1, notifyRoles: ["developer"], escalateAt: "1h", note: "自动尝试刷新浏览器 cookie" },
  [IssueClass.UNKNOWN]: { autoRetry: 0, notifyRoles: ["developer"], escalateAt: "2h" },
};
