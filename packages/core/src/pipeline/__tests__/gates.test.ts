import { describe, it, expect } from "vitest";
import { evaluate, crScoreMeetsThreshold, DEFAULT_GATES } from "../gates.js";
import type { StageResult } from "../gates.js";
import type { GateRule } from "../../types/gate.js";

describe("crScoreMeetsThreshold", () => {
  it("returns true when actual >= threshold", () => {
    expect(crScoreMeetsThreshold("A+", "B+")).toBe(true);
    expect(crScoreMeetsThreshold("A", "B+")).toBe(true);
    expect(crScoreMeetsThreshold("A-", "B+")).toBe(true);
    expect(crScoreMeetsThreshold("B+", "B+")).toBe(true);
  });

  it("returns false when actual < threshold", () => {
    expect(crScoreMeetsThreshold("B", "B+")).toBe(false);
    expect(crScoreMeetsThreshold("C", "B+")).toBe(false);
    expect(crScoreMeetsThreshold("F", "B+")).toBe(false);
  });

  it("returns false for invalid grades", () => {
    expect(crScoreMeetsThreshold("X", "B+")).toBe(false);
    expect(crScoreMeetsThreshold("B+", "Z")).toBe(false);
    expect(crScoreMeetsThreshold("", "B+")).toBe(false);
  });

  it("handles boundary cases correctly (numeric, not lexicographic)", () => {
    // "A" < "B" lexicographically but A should be higher than B
    expect(crScoreMeetsThreshold("A", "B")).toBe(true);
    expect(crScoreMeetsThreshold("B", "A")).toBe(false);
  });
});

describe("evaluate — stage_exit phase", () => {
  it("only evaluates rules with gatePhase stage_exit", () => {
    const result: StageResult = { crScore: "A", securityPass: true };
    const evaluation = evaluate(result, DEFAULT_GATES, "stage_exit");
    const evaluatedIds = evaluation.details.map(d => d.ruleId);
    expect(evaluatedIds).toContain("cr_score");
    expect(evaluatedIds).toContain("security_scan");
    expect(evaluatedIds).not.toContain("ci_build");
    expect(evaluatedIds).not.toContain("test_coverage");
    expect(evaluatedIds).not.toContain("diff_size");
    expect(evaluatedIds).not.toContain("merge_conflict");
  });

  it("allPass is true when CR score meets threshold", () => {
    const result: StageResult = { crScore: "A", securityPass: true };
    const evaluation = evaluate(result, DEFAULT_GATES, "stage_exit");
    expect(evaluation.allPass).toBe(true);
    expect(evaluation.blockingFailures).toHaveLength(0);
  });

  it("CR score below threshold lands in blockingFailures (regress counts as blocking)", () => {
    const result: StageResult = { crScore: "C", securityPass: true };
    const evaluation = evaluate(result, DEFAULT_GATES, "stage_exit");
    expect(evaluation.allPass).toBe(false);
    expect(evaluation.blockingFailures).toHaveLength(1);
    expect(evaluation.blockingFailures[0]!.ruleId).toBe("cr_score");
  });

  it("failing cr_score rule in DEFAULT_GATES has regressTo: dev", () => {
    const crRule = DEFAULT_GATES.find(r => r.id === "cr_score")!;
    expect(crRule.onFail).toBe("regress");
    expect(crRule.regressTo).toBe("dev");
  });
});

describe("evaluate — deploy phase", () => {
  const passingDeploy: StageResult = {
    ciBuildPass: true,
    testCoverage: 90,
    diffLines: 100,
    hasConflict: false,
  };

  it("only evaluates rules with gatePhase deploy", () => {
    const evaluation = evaluate(passingDeploy, DEFAULT_GATES, "deploy");
    const evaluatedIds = evaluation.details.map(d => d.ruleId);
    expect(evaluatedIds).toContain("ci_build");
    expect(evaluatedIds).toContain("test_coverage");
    expect(evaluatedIds).toContain("diff_size");
    expect(evaluatedIds).toContain("merge_conflict");
    expect(evaluatedIds).not.toContain("cr_score");
    expect(evaluatedIds).not.toContain("security_scan");
  });

  it("allPass true when all deploy gates pass", () => {
    const evaluation = evaluate(passingDeploy, DEFAULT_GATES, "deploy");
    expect(evaluation.allPass).toBe(true);
    expect(evaluation.blockingFailures).toHaveLength(0);
    expect(evaluation.warnFailures).toHaveLength(0);
  });

  it("ciBuildPass false → blocking failure", () => {
    const result: StageResult = { ...passingDeploy, ciBuildPass: false };
    const evaluation = evaluate(result, DEFAULT_GATES, "deploy");
    expect(evaluation.allPass).toBe(false);
    expect(evaluation.blockingFailures.some(f => f.ruleId === "ci_build")).toBe(true);
  });

  it("diffLines > 500 → warn failure (allPass still true)", () => {
    const result: StageResult = { ...passingDeploy, diffLines: 600 };
    const evaluation = evaluate(result, DEFAULT_GATES, "deploy");
    expect(evaluation.allPass).toBe(true);
    expect(evaluation.warnFailures).toHaveLength(1);
    expect(evaluation.warnFailures[0]!.ruleId).toBe("diff_size");
  });

  it("testCoverage below threshold → blocking failure", () => {
    const result: StageResult = { ...passingDeploy, testCoverage: 60 };
    const evaluation = evaluate(result, DEFAULT_GATES, "deploy");
    expect(evaluation.allPass).toBe(false);
    expect(evaluation.blockingFailures.some(f => f.ruleId === "test_coverage")).toBe(true);
  });

  it("multiple failures at once", () => {
    const result: StageResult = {
      ciBuildPass: false,
      testCoverage: 50,
      diffLines: 800,
      hasConflict: true,
    };
    const evaluation = evaluate(result, DEFAULT_GATES, "deploy");
    expect(evaluation.allPass).toBe(false);
    expect(evaluation.blockingFailures.length).toBeGreaterThanOrEqual(3);
    expect(evaluation.warnFailures).toHaveLength(1);
  });
});

describe("evaluate — disabled rules", () => {
  it("skips disabled rules entirely", () => {
    const rules: GateRule[] = DEFAULT_GATES.map(r =>
      r.id === "ci_build" ? { ...r, enabled: false } : r,
    );
    const result: StageResult = { ciBuildPass: false, testCoverage: 90, diffLines: 10, hasConflict: false };
    const evaluation = evaluate(result, rules, "deploy");
    expect(evaluation.details.some(d => d.ruleId === "ci_build")).toBe(false);
    expect(evaluation.allPass).toBe(true);
  });
});
