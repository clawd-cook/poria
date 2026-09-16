import { describe, it, expect } from "vitest";
import { parseXingyunDemandUrl, featureBranchName, featureSlug } from "../demandUrl.js";

describe("parseXingyunDemandUrl", () => {
  it("parses real Xingyun demand URL format", () => {
    const result = parseXingyunDemandUrl(
      "http://xingyun.jd.com/demands/view/JL3R4IV4/-1?demandId=4840029",
    );
    expect(result.demandId).toBe(4840029);
    expect(result.demandCode).toBe("JL3R4IV4");
    expect(result.url).toBe("http://xingyun.jd.com/demands/view/JL3R4IV4/-1?demandId=4840029");
  });

  it("extracts demandId from 'id' param as fallback", () => {
    const result = parseXingyunDemandUrl(
      "http://xingyun.jd.com/demands/view/ABC123/-1?id=12345",
    );
    expect(result.demandId).toBe(12345);
    expect(result.demandCode).toBe("ABC123");
  });

  it("handles subdomain xingyun URLs", () => {
    const result = parseXingyunDemandUrl(
      "http://test.xingyun.jd.com/demands/view/CODE1/-1?demandId=999",
    );
    expect(result.demandId).toBe(999);
    expect(result.demandCode).toBe("CODE1");
  });

  it("throws on empty input", () => {
    expect(() => parseXingyunDemandUrl("")).toThrow("Missing Xingyun demand URL");
  });

  it("throws on invalid URL", () => {
    expect(() => parseXingyunDemandUrl("not-a-url")).toThrow("Invalid Xingyun demand URL");
  });

  it("throws on non-xingyun host", () => {
    expect(() =>
      parseXingyunDemandUrl("http://example.com/demands/view/X/-1?demandId=1"),
    ).toThrow("Not a Xingyun demand host");
  });

  it("throws when demandId is missing", () => {
    expect(() =>
      parseXingyunDemandUrl("http://xingyun.jd.com/demands/view/CODE1/-1"),
    ).toThrow("missing demandId");
  });

  it("throws when demandId is not a number", () => {
    expect(() =>
      parseXingyunDemandUrl("http://xingyun.jd.com/demands/view/CODE1/-1?demandId=abc"),
    ).toThrow("missing demandId");
  });

  it("omits demandCode when path segment is numeric", () => {
    const result = parseXingyunDemandUrl(
      "http://xingyun.jd.com/demands/view/123/-1?demandId=456",
    );
    expect(result.demandId).toBe(456);
    expect(result.demandCode).toBeUndefined();
  });

  it("trims whitespace from input", () => {
    const result = parseXingyunDemandUrl(
      "  http://xingyun.jd.com/demands/view/CODE1/-1?demandId=100  ",
    );
    expect(result.demandId).toBe(100);
  });
});

describe("featureBranchName", () => {
  it("generates branch from demand code", () => {
    expect(featureBranchName("JL3R4IV4", 4840029)).toBe("feature_JL3R4IV4");
  });

  it("falls back to demand ID when no code", () => {
    expect(featureBranchName(undefined, 4840029)).toBe("feature_demand_4840029");
  });

  it("falls back to demand ID when code is empty", () => {
    expect(featureBranchName("", 4840029)).toBe("feature_demand_4840029");
  });

  it("replaces illegal characters", () => {
    expect(featureBranchName("CODE WITH SPACES", 1)).toBe("feature_CODE_WITH_SPACES");
  });
});

describe("featureSlug", () => {
  it("generates slug from demand code", () => {
    expect(featureSlug("JL3R4IV4", 4840029)).toBe("feat-jl3r4iv4");
  });

  it("falls back to demand ID when no code", () => {
    expect(featureSlug(undefined, 4840029)).toBe("feat-demand-4840029");
  });
});
