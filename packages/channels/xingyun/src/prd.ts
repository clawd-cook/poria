import type { CardAttachment } from "./jacp/cards.js";
import {
  isJoySpacePrdLink,
  preferPrdNamed,
} from "./jacp/prdAttachmentLink.js";

export class PrdResolveError extends Error {
  readonly code: "NO_PRD" | "AMBIGUOUS_PRD";
  readonly candidates: CardAttachment[];

  constructor(
    code: "NO_PRD" | "AMBIGUOUS_PRD",
    message: string,
    candidates: CardAttachment[] = [],
  ) {
    super(message);
    this.name = "PrdResolveError";
    this.code = code;
    this.candidates = candidates;
  }
}

/**
 * PRD selection rules (design.md):
 * 1. JoySpace links only
 * 2. Exactly one -> use it
 * 3. Multiple: prefer name/tag containing PRD/需求/prd; still >1 -> hard fail
 * 4. Zero -> hard fail
 */
export function resolvePrdFromAttachments(
  attachments: CardAttachment[],
): { url: string; attachment: CardAttachment } {
  const joyspace = attachments.filter((item) => isJoySpacePrdLink(item.url));
  if (joyspace.length === 0) {
    throw new PrdResolveError(
      "NO_PRD",
      "No JoySpace PRD attachment found on the demand card. Attach a JoySpace PRD link and retry.",
      attachments,
    );
  }
  if (joyspace.length === 1) {
    return { url: joyspace[0]!.url, attachment: joyspace[0]! };
  }

  const preferred = preferPrdNamed(joyspace) as CardAttachment[];
  if (preferred.length === 1) {
    return { url: preferred[0]!.url, attachment: preferred[0]! };
  }

  const lines = preferred.map(
    (item, index) => `  ${index + 1}. ${item.name} -> ${item.url}`,
  );
  throw new PrdResolveError(
    "AMBIGUOUS_PRD",
    `Multiple JoySpace PRD candidates; resolve manually:\n${lines.join("\n")}`,
    preferred,
  );
}
