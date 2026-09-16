/**
 * Fuzzy reply parser for human-loop responses via JME.
 * Design A9: use fuzzy matching since JoyClaw agent uses an internal LLM
 * with limited understanding capability.
 */

export type ReplyAction = "resume" | "skip" | "cancel" | "unknown";

const RESUME_PATTERNS = [
  /修复/,
  /fix/i,
  /resume/i,
  /已修复/,
  /已解决/,
  /重试/,
  /retry/i,
];

const SKIP_PATTERNS = [
  /跳过/,
  /skip/i,
  /忽略/,
  /ignore/i,
];

const CANCEL_PATTERNS = [
  /取消/,
  /cancel/i,
  /终止/,
  /abort/i,
  /stop/i,
];

/**
 * Parse a human reply text into an action.
 * Returns "unknown" if no pattern matches — the system should record
 * the message and continue waiting.
 */
export function parseReply(text: string): ReplyAction {
  const trimmed = text.trim();
  if (!trimmed) return "unknown";

  for (const pattern of CANCEL_PATTERNS) {
    if (pattern.test(trimmed)) return "cancel";
  }
  for (const SKIP_PATTERN of SKIP_PATTERNS) {
    if (SKIP_PATTERN.test(trimmed)) return "skip";
  }
  for (const pattern of RESUME_PATTERNS) {
    if (pattern.test(trimmed)) return "resume";
  }

  return "unknown";
}
