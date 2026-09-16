import { describe, it, expect } from "vitest";
import { classifyRisk } from "../risk-classifier.js";

describe("classifyRisk", () => {
  it("returns critical for CI/deploy files", () => {
    expect(classifyRisk([".github/workflows/ci.yml"], 10)).toBe("critical");
    expect(classifyRisk([".gitlab-ci.yml"], 10)).toBe("critical");
    expect(classifyRisk(["Dockerfile"], 10)).toBe("critical");
    expect(classifyRisk(["deploy/k8s.yaml"], 10)).toBe("critical");
    expect(classifyRisk(["infra/main.tf"], 10)).toBe("critical");
  });

  it("returns high for config files with >100 diffLines", () => {
    expect(classifyRisk(["package.json"], 101)).toBe("high");
    expect(classifyRisk([".env.production"], 200)).toBe("high");
    expect(classifyRisk(["tsconfig.json"], 150)).toBe("high");
  });

  it("returns medium for config files with ≤100 diffLines", () => {
    expect(classifyRisk(["package.json"], 50)).toBe("medium");
    expect(classifyRisk(["vite.config.ts"], 100)).toBe("medium");
  });

  it("returns high for >500 diffLines even without config", () => {
    expect(classifyRisk(["src/app.ts"], 501)).toBe("high");
  });

  it("returns medium for 201–500 diffLines without config", () => {
    expect(classifyRisk(["src/app.ts"], 201)).toBe("medium");
    expect(classifyRisk(["src/app.ts"], 500)).toBe("medium");
  });

  it("returns low for normal files with ≤200 diffLines", () => {
    expect(classifyRisk(["src/app.ts"], 200)).toBe("low");
    expect(classifyRisk(["src/utils.ts", "src/index.ts"], 50)).toBe("low");
  });

  it("critical takes precedence over config", () => {
    expect(classifyRisk([".github/workflows/ci.yml", "package.json"], 10)).toBe("critical");
  });
});
