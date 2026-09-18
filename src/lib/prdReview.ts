export const REQUIREMENT_AMBIGUOUS_ISSUE_CLASS = "requirement_ambiguous";

export function isRequirementAmbiguousIssue(issueClass: string | null | undefined): boolean {
  if (!issueClass) {
    return false;
  }
  const normalized = issueClass.toLowerCase().replace(/[^a-z]/g, "");
  return normalized === "requirementambiguous" || normalized.includes("requirementambiguous");
}

export function isP0UnansweredMessage(message: string): boolean {
  const lower = message.toLowerCase();
  return message.includes("P0 unanswered") || lower.includes("p0 unanswered");
}
