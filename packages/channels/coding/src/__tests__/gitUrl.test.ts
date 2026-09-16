import { describe, it, expect } from "vitest";
import {
  normalizeGitUrl,
  repoNameFromGitUrl,
  repoSearchPathFromGitUrl,
  sameGitUrl,
} from "../gitUrl.js";

describe("normalizeGitUrl", () => {
  it("converts SSH to HTTPS lowercase", () => {
    expect(normalizeGitUrl("git@coding.jd.com:poria/demo.git")).toBe(
      "https://coding.jd.com/poria/demo",
    );
  });

  it("normalizes HTTPS URL", () => {
    expect(normalizeGitUrl("https://Coding.JD.COM/Poria/Demo.GIT/")).toBe(
      "https://coding.jd.com/poria/demo",
    );
  });
});

describe("repoNameFromGitUrl", () => {
  it("extracts repo name from SSH URL", () => {
    expect(repoNameFromGitUrl("git@coding.jd.com:poria/demo.git")).toBe("demo");
  });

  it("extracts repo name from HTTPS URL", () => {
    expect(repoNameFromGitUrl("https://coding.jd.com/group/sub/repo.git")).toBe("repo");
  });

  it("returns undefined for empty input", () => {
    expect(repoNameFromGitUrl("")).toBeUndefined();
  });
});

describe("repoSearchPathFromGitUrl", () => {
  it("extracts full path from SSH URL", () => {
    expect(repoSearchPathFromGitUrl("git@coding.jd.com:poria/demo.git")).toBe("poria/demo");
  });

  it("extracts full path from SSH with ssh:// scheme", () => {
    expect(repoSearchPathFromGitUrl("ssh://git@coding.jd.com/poria/demo.git")).toBe("poria/demo");
  });
});

describe("sameGitUrl", () => {
  it("considers SSH and HTTPS equivalent", () => {
    expect(
      sameGitUrl("git@coding.jd.com:poria/demo.git", "https://coding.jd.com/poria/demo"),
    ).toBe(true);
  });

  it("is case insensitive", () => {
    expect(
      sameGitUrl("git@CODING.JD.COM:Poria/Demo.git", "git@coding.jd.com:poria/demo.git"),
    ).toBe(true);
  });

  it("returns false for empty values", () => {
    expect(sameGitUrl("", "git@coding.jd.com:poria/demo.git")).toBe(false);
    expect(sameGitUrl(undefined, undefined)).toBe(false);
  });
});
