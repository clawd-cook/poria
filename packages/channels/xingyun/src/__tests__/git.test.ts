import { describe, it, expect } from "vitest";
import {
  normalizeGitUrl,
  repoSearchPathFromGitUrl,
  sameGitUrl,
} from "../git.js";

describe("normalizeGitUrl", () => {
  it("converts SSH to HTTPS lowercase", () => {
    expect(normalizeGitUrl("git@coding.jd.com:group/repo.git")).toBe(
      "https://coding.jd.com/group/repo",
    );
  });

  it("strips .git suffix and trailing slashes", () => {
    expect(normalizeGitUrl("https://coding.jd.com/group/repo.git/")).toBe(
      "https://coding.jd.com/group/repo",
    );
  });
});

describe("repoSearchPathFromGitUrl", () => {
  it("extracts path from SSH URL", () => {
    expect(repoSearchPathFromGitUrl("git@coding.jd.com:group/repo.git")).toBe(
      "group/repo",
    );
  });

  it("extracts path from HTTPS URL", () => {
    expect(repoSearchPathFromGitUrl("https://coding.jd.com/group/repo.git")).toBe(
      "group/repo",
    );
  });
});

describe("sameGitUrl", () => {
  it("considers SSH and HTTPS equivalent", () => {
    expect(
      sameGitUrl(
        "git@coding.jd.com:group/repo.git",
        "https://coding.jd.com/group/repo",
      ),
    ).toBe(true);
  });

  it("returns false for different repos", () => {
    expect(
      sameGitUrl(
        "git@coding.jd.com:group/repo1.git",
        "git@coding.jd.com:group/repo2.git",
      ),
    ).toBe(false);
  });

  it("returns false for null/undefined", () => {
    expect(sameGitUrl(undefined, "git@coding.jd.com:g/r.git")).toBe(false);
    expect(sameGitUrl("git@coding.jd.com:g/r.git", undefined)).toBe(false);
  });
});
