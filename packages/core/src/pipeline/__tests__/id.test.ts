import { describe, it, expect } from "vitest";
import { createPipelineId } from "../id.js";

describe("createPipelineId", () => {
  it("matches format pl-YYYYMMDD-xxxxxxxx", () => {
    const id = createPipelineId();
    expect(id).toMatch(/^pl-\d{8}-[a-zA-Z0-9_-]{8}$/);
  });

  it("date portion matches today", () => {
    const id = createPipelineId();
    const today = new Date().toISOString().slice(0, 10).replace(/-/g, "");
    expect(id.slice(3, 11)).toBe(today);
  });

  it("generates unique IDs", () => {
    const a = createPipelineId();
    const b = createPipelineId();
    expect(a).not.toBe(b);
  });
});
