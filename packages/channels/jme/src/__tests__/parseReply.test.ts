import { describe, it, expect } from "vitest";
import { parseReply } from "../reply-parser.js";

describe("parseReply", () => {
  it("returns 'resume' for Chinese '修复'", () => {
    expect(parseReply("修复")).toBe("resume");
  });

  it("returns 'resume' for English 'fix'", () => {
    expect(parseReply("fix")).toBe("resume");
  });

  it("returns 'resume' for 'Fix' (case insensitive)", () => {
    expect(parseReply("Fix")).toBe("resume");
  });

  it("returns 'resume' for '已修复'", () => {
    expect(parseReply("已修复了")).toBe("resume");
  });

  it("returns 'resume' for '重试'", () => {
    expect(parseReply("请重试")).toBe("resume");
  });

  it("returns 'resume' for 'retry'", () => {
    expect(parseReply("please retry")).toBe("resume");
  });

  it("returns 'skip' for '跳过'", () => {
    expect(parseReply("跳过")).toBe("skip");
  });

  it("returns 'skip' for 'skip'", () => {
    expect(parseReply("skip this")).toBe("skip");
  });

  it("returns 'skip' for '忽略'", () => {
    expect(parseReply("忽略")).toBe("skip");
  });

  it("returns 'cancel' for '取消'", () => {
    expect(parseReply("取消")).toBe("cancel");
  });

  it("returns 'cancel' for 'cancel'", () => {
    expect(parseReply("cancel")).toBe("cancel");
  });

  it("returns 'cancel' for '终止'", () => {
    expect(parseReply("终止流水线")).toBe("cancel");
  });

  it("returns 'unknown' for unrecognized text", () => {
    expect(parseReply("I need more time")).toBe("unknown");
  });

  it("returns 'unknown' for empty input", () => {
    expect(parseReply("")).toBe("unknown");
  });

  it("returns 'unknown' for whitespace only", () => {
    expect(parseReply("   ")).toBe("unknown");
  });

  it("prioritizes cancel over resume when both appear", () => {
    // "取消" should win over "修复" since cancel is checked first
    expect(parseReply("取消修复")).toBe("cancel");
  });
});
