import { describe, it, expect } from "vitest";
import { projectIdFromGitUrl, getMrStatusFixture, findMrFixture } from "../mergeRequest.js";

describe("projectIdFromGitUrl", () => {
  it("extracts project path from SSH URL", () => {
    expect(projectIdFromGitUrl("git@coding.jd.com:group/repo.git")).toBe("group/repo");
  });

  it("extracts project path from HTTPS URL", () => {
    expect(projectIdFromGitUrl("https://coding.jd.com/group/repo.git")).toBe("group/repo");
  });

  it("returns undefined for empty input", () => {
    expect(projectIdFromGitUrl("")).toBeUndefined();
  });
});

describe("getMrStatusFixture", () => {
  it("returns 'opened' in fixture mode", () => {
    expect(getMrStatusFixture("group/repo", 1)).toBe("opened");
  });
});

describe("findMrFixture", () => {
  it("returns null in fixture mode (no existing MR)", () => {
    expect(
      findMrFixture({
        projectPath: "group/repo",
        sourceBranch: "feature_test",
        targetBranch: "master",
      }),
    ).toBeNull();
  });
});
