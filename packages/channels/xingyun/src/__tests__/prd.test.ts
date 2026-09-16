import { describe, it, expect } from "vitest";
import { resolvePrdFromAttachments, PrdResolveError } from "../prd.js";
import type { CardAttachment } from "../jacp/cards.js";

function attachment(overrides: Partial<CardAttachment> = {}): CardAttachment {
  return {
    tagName: overrides.tagName ?? "附件",
    name: overrides.name ?? "doc",
    url: overrides.url ?? "https://example.com/page",
    ...overrides,
  };
}

describe("resolvePrdFromAttachments", () => {
  it("returns the single JoySpace attachment", () => {
    const attachments = [
      attachment({ name: "PRD", url: "https://joyspace.jd.com/pages/abc" }),
    ];
    const result = resolvePrdFromAttachments(attachments);
    expect(result.url).toBe("https://joyspace.jd.com/pages/abc");
    expect(result.attachment.name).toBe("PRD");
  });

  it("throws NO_PRD when no JoySpace links", () => {
    const attachments = [
      attachment({ name: "design", url: "https://example.com/page" }),
    ];
    expect(() => resolvePrdFromAttachments(attachments)).toThrow(PrdResolveError);
    try {
      resolvePrdFromAttachments(attachments);
    } catch (e) {
      expect((e as PrdResolveError).code).toBe("NO_PRD");
    }
  });

  it("throws NO_PRD with empty array", () => {
    expect(() => resolvePrdFromAttachments([])).toThrow(PrdResolveError);
  });

  it("prefers attachment named PRD when multiple JoySpace links exist", () => {
    const attachments = [
      attachment({ name: "Design Doc", url: "https://joyspace.jd.com/pages/design" }),
      attachment({ name: "PRD 需求文档", url: "https://joyspace.jd.com/pages/prd" }),
    ];
    const result = resolvePrdFromAttachments(attachments);
    expect(result.url).toBe("https://joyspace.jd.com/pages/prd");
  });

  it("throws AMBIGUOUS_PRD when multiple JoySpace links with PRD-like names exist", () => {
    const attachments = [
      attachment({ name: "PRD v1", tagName: "PRD", url: "https://joyspace.jd.com/pages/v1" }),
      attachment({ name: "PRD v2", tagName: "PRD", url: "https://joyspace.jd.com/pages/v2" }),
    ];
    expect(() => resolvePrdFromAttachments(attachments)).toThrow(PrdResolveError);
    try {
      resolvePrdFromAttachments(attachments);
    } catch (e) {
      expect((e as PrdResolveError).code).toBe("AMBIGUOUS_PRD");
      expect((e as PrdResolveError).candidates.length).toBe(2);
    }
  });

  it("filters non-JoySpace URLs", () => {
    const attachments = [
      attachment({ name: "ext doc", url: "https://other.jd.com/pages/abc" }),
      attachment({ name: "PRD", url: "https://joyspace.jd.com/pages/real-prd" }),
    ];
    const result = resolvePrdFromAttachments(attachments);
    expect(result.url).toBe("https://joyspace.jd.com/pages/real-prd");
  });
});
