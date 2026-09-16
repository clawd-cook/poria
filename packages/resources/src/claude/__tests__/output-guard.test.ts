import { describe, it, expect } from "vitest";
import { OutputGuard } from "../output-guard.js";
import type { AgentOutput, OutputGuardConfig } from "../output-guard.js";

describe("OutputGuard", () => {
  const guard = new OutputGuard();

  function makeConfig(overrides: Partial<OutputGuardConfig> = {}): OutputGuardConfig {
    return {
      allowedPaths: overrides.allowedPaths ?? ["src/**"],
      maxDiffLines: overrides.maxDiffLines ?? 500,
      blockedDependencies: overrides.blockedDependencies ?? [],
      ...overrides,
    };
  }

  function makeOutput(overrides: Partial<AgentOutput> = {}): AgentOutput {
    return {
      changedFiles: overrides.changedFiles ?? [],
      totalDiffLines: overrides.totalDiffLines ?? 10,
      addedDependencies: overrides.addedDependencies ?? [],
      ...overrides,
    };
  }

  describe("file scope check", () => {
    it("trdScope empty -> warn (not block), pass = true", () => {
      const result = guard.check(
        makeOutput({ changedFiles: ["anything.ts", "anywhere/file.ts"] }),
        makeConfig({ allowedPaths: [] }),
      );

      expect(result.pass).toBe(true);
      expect(result.violations).toHaveLength(1);
      expect(result.violations[0]!.type).toBe("trd_scope_empty");
      expect(result.violations[0]!.severity).toBe("warn");
    });

    it("file within trdScope -> no violation", () => {
      const result = guard.check(
        makeOutput({ changedFiles: ["src/components/App.tsx"] }),
        makeConfig({ allowedPaths: ["src/**"] }),
      );

      expect(result.pass).toBe(true);
      expect(result.violations).toHaveLength(0);
    });

    it("file outside trdScope -> block, pass = false", () => {
      const result = guard.check(
        makeOutput({ changedFiles: ["config/secret.json"] }),
        makeConfig({ allowedPaths: ["src/**"] }),
      );

      expect(result.pass).toBe(false);
      expect(result.violations).toHaveLength(1);
      expect(result.violations[0]!.type).toBe("out_of_scope");
      expect(result.violations[0]!.severity).toBe("block");
      expect(result.violations[0]!.file).toBe("config/secret.json");
    });

    it("multiple files: some in scope, some out -> block for out-of-scope", () => {
      const result = guard.check(
        makeOutput({
          changedFiles: [
            "src/index.ts",
            "src/utils/helper.ts",
            "node_modules/evil/index.js",
          ],
        }),
        makeConfig({ allowedPaths: ["src/**"] }),
      );

      expect(result.pass).toBe(false);
      expect(result.violations).toHaveLength(1);
      expect(result.violations[0]!.file).toBe("node_modules/evil/index.js");
    });

    it("multiple glob patterns work", () => {
      const result = guard.check(
        makeOutput({
          changedFiles: ["src/app.ts", "package.json", "tests/app.test.ts"],
        }),
        makeConfig({ allowedPaths: ["src/**", "package.json", "tests/**"] }),
      );

      expect(result.pass).toBe(true);
      expect(result.violations).toHaveLength(0);
    });
  });

  describe("diff size check", () => {
    it("diff within limit -> no violation", () => {
      const result = guard.check(
        makeOutput({ totalDiffLines: 100 }),
        makeConfig({ maxDiffLines: 500 }),
      );

      expect(result.pass).toBe(true);
      expect(result.violations).toHaveLength(0);
    });

    it("diff exceeds limit -> warn (not block), pass = true", () => {
      const result = guard.check(
        makeOutput({ totalDiffLines: 600 }),
        makeConfig({ maxDiffLines: 500 }),
      );

      expect(result.pass).toBe(true);
      expect(result.violations).toHaveLength(1);
      expect(result.violations[0]!.type).toBe("diff_too_large");
      expect(result.violations[0]!.severity).toBe("warn");
      expect(result.violations[0]!.actual).toBe(600);
      expect(result.violations[0]!.threshold).toBe(500);
    });

    it("diff exactly at limit -> no violation", () => {
      const result = guard.check(
        makeOutput({ totalDiffLines: 500 }),
        makeConfig({ maxDiffLines: 500 }),
      );

      expect(result.pass).toBe(true);
      expect(result.violations).toHaveLength(0);
    });
  });

  describe("dependency safety check", () => {
    it("no blocked deps -> no violation", () => {
      const result = guard.check(
        makeOutput({ addedDependencies: [{ name: "lodash" }] }),
        makeConfig({ blockedDependencies: ["evil-pkg"] }),
      );

      expect(result.pass).toBe(true);
      expect(result.violations).toHaveLength(0);
    });

    it("blocked dependency -> block, pass = false", () => {
      const result = guard.check(
        makeOutput({ addedDependencies: [{ name: "evil-pkg" }] }),
        makeConfig({ blockedDependencies: ["evil-pkg"] }),
      );

      expect(result.pass).toBe(false);
      expect(result.violations).toHaveLength(1);
      expect(result.violations[0]!.type).toBe("blocked_dependency");
      expect(result.violations[0]!.severity).toBe("block");
      expect(result.violations[0]!.dependency).toBe("evil-pkg");
    });

    it("multiple deps, one blocked -> block", () => {
      const result = guard.check(
        makeOutput({
          addedDependencies: [
            { name: "safe-lib" },
            { name: "malware-pkg" },
            { name: "another-safe" },
          ],
        }),
        makeConfig({ blockedDependencies: ["malware-pkg", "evil-pkg"] }),
      );

      expect(result.pass).toBe(false);
      expect(result.violations).toHaveLength(1);
      expect(result.violations[0]!.dependency).toBe("malware-pkg");
    });

    it("empty blockedDependencies list -> no checks", () => {
      const result = guard.check(
        makeOutput({ addedDependencies: [{ name: "anything" }] }),
        makeConfig({ blockedDependencies: [] }),
      );

      expect(result.pass).toBe(true);
      expect(result.violations).toHaveLength(0);
    });
  });

  describe("mixed scenarios", () => {
    it("block + warn: out-of-scope file + large diff -> pass = false", () => {
      const result = guard.check(
        makeOutput({
          changedFiles: ["outside/file.ts"],
          totalDiffLines: 1000,
        }),
        makeConfig({ allowedPaths: ["src/**"], maxDiffLines: 500 }),
      );

      expect(result.pass).toBe(false);
      const types = result.violations.map((v) => v.type);
      expect(types).toContain("out_of_scope");
      expect(types).toContain("diff_too_large");
    });

    it("warn only: empty trdScope + large diff -> pass = true", () => {
      const result = guard.check(
        makeOutput({ totalDiffLines: 1000 }),
        makeConfig({ allowedPaths: [], maxDiffLines: 500 }),
      );

      expect(result.pass).toBe(true);
      expect(result.violations).toHaveLength(2);
      expect(result.violations.every((v) => v.severity === "warn")).toBe(true);
    });

    it("multiple blocks: out-of-scope + blocked dep -> pass = false, 2 violations", () => {
      const result = guard.check(
        makeOutput({
          changedFiles: ["forbidden/file.ts"],
          addedDependencies: [{ name: "evil-pkg" }],
        }),
        makeConfig({
          allowedPaths: ["src/**"],
          blockedDependencies: ["evil-pkg"],
        }),
      );

      expect(result.pass).toBe(false);
      expect(result.violations.filter((v) => v.severity === "block")).toHaveLength(2);
    });

    it("clean output -> pass = true, no violations", () => {
      const result = guard.check(
        makeOutput({
          changedFiles: ["src/app.ts"],
          totalDiffLines: 50,
          addedDependencies: [],
        }),
        makeConfig({
          allowedPaths: ["src/**"],
          maxDiffLines: 500,
          blockedDependencies: ["evil-pkg"],
        }),
      );

      expect(result.pass).toBe(true);
      expect(result.violations).toHaveLength(0);
    });
  });
});
