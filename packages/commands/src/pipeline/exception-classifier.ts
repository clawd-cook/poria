import { IssueClass } from "@poria/core";

export class ExceptionClassifier {
  static classify(error: unknown): IssueClass {
    const msg = error instanceof Error ? error.message : String(error);

    if (msg.includes("compilation") || msg.includes("build failed") || msg.includes("tsc"))
      return IssueClass.COMPILATION_ERROR;
    if (msg.includes("test fail") || msg.includes("vitest") || msg.includes("jest"))
      return IssueClass.TEST_FAILURE;
    if (msg.includes("timeout") || msg.includes("AGENT_TIMEOUT"))
      return IssueClass.AGENT_TIMEOUT;
    if (msg.includes("rate limit") || msg.includes("429") || msg.includes("overloaded"))
      return IssueClass.LLM_RATE_LIMIT;
    if (msg.includes("ambiguous") || msg.includes("P0 unanswered"))
      return IssueClass.REQUIREMENT_AMBIG;
    if (msg.includes("PRD") && (msg.includes("empty") || msg.includes("invalid") || msg.includes("export failed")))
      return IssueClass.PRD_INVALID;
    if (msg.includes("merge conflict") || msg.includes("CONFLICT"))
      return IssueClass.MERGE_CONFLICT;
    if (msg.includes("cr score") || msg.includes("LOW_CR_SCORE"))
      return IssueClass.LOW_CR_SCORE;
    if (msg.includes("diff too large") || msg.includes("DIFF_TOO_LARGE"))
      return IssueClass.DIFF_TOO_LARGE;
    if (msg.includes("permission denied") || msg.includes("403") || msg.includes("forbidden"))
      return IssueClass.PERMISSION_DENIED;
    if (msg.includes("ECONNREFUSED") || msg.includes("ENOTFOUND") || msg.includes("infra"))
      return IssueClass.INFRA_FAILURE;
    if (msg.includes("security") || msg.includes("malicious") || msg.includes("blocked_dependency"))
      return IssueClass.SECURITY_VIOLATION;
    if (msg.includes("out_of_scope") || msg.includes("OutputGuardError"))
      return IssueClass.OUT_OF_SCOPE;
    if (msg.includes("auth expired") || msg.includes("cookie expired") || msg.includes("AuthExpired"))
      return IssueClass.AUTH_EXPIRED;

    return IssueClass.UNKNOWN;
  }
}
