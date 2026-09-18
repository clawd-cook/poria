export const TRD_UNCONFIRMED_ISSUE_CLASS = "trd_unconfirmed";

export function isTrdUnconfirmedIssue(issueClass: string | null | undefined): boolean {
  if (!issueClass) {
    return false;
  }
  const normalized = issueClass.toLowerCase().replace(/[^a-z]/g, "");
  return normalized === "trdunconfirmed" || normalized.includes("trdunconfirmed");
}

export function isTrdUnconfirmedMessage(message: string): boolean {
  const lower = message.toLowerCase();
  return message.includes("TRD unconfirmed") || lower.includes("trd unconfirmed");
}
