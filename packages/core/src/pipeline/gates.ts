import type { GateRule, GateResult, GateEvaluation, GatePhase } from "../types/gate.js";

const CR_GRADE_ORDER: Record<string, number> = {
  "A+": 10, "A": 9, "A-": 8,
  "B+": 7,  "B": 6, "B-": 5,
  "C+": 4,  "C": 3, "C-": 2,
  "D": 1,   "F": 0,
};

export function crScoreMeetsThreshold(actual: string, threshold: string): boolean {
  const actualNum = CR_GRADE_ORDER[actual];
  const thresholdNum = CR_GRADE_ORDER[threshold];
  if (actualNum === undefined || thresholdNum === undefined) return false;
  return actualNum >= thresholdNum;
}

export const DEFAULT_GATES: GateRule[] = [
  { id: "ci_build",       name: "CI 构建",   enabled: true, threshold: null, onFail: "block",   gatePhase: "deploy" },
  { id: "test_coverage",  name: "测试覆盖率", enabled: true, threshold: 80,   onFail: "block",   gatePhase: "deploy" },
  { id: "security_scan",  name: "安全扫描",   enabled: true, threshold: null, onFail: "block",   gatePhase: "stage_exit" },
  { id: "diff_size",      name: "变更量",     enabled: true, threshold: 500,  onFail: "warn",    gatePhase: "deploy" },
  { id: "merge_conflict", name: "合并冲突",   enabled: true, threshold: null, onFail: "block",   gatePhase: "deploy" },
  { id: "cr_score",       name: "CR 评分",   enabled: true, threshold: "B+", onFail: "regress", gatePhase: "stage_exit", regressTo: "dev" },
];

export interface StageResult {
  crScore?: string;
  testCoverage?: number;
  ciBuildPass?: boolean;
  securityPass?: boolean;
  diffLines?: number;
  hasConflict?: boolean;
  [key: string]: unknown;
}

function evaluateOne(rule: GateRule, result: StageResult): GateResult {
  switch (rule.id) {
    case "ci_build":
      return gateResult(rule, result.ciBuildPass === true, result.ciBuildPass, rule.threshold);
    case "test_coverage":
      return gateResult(rule, (result.testCoverage ?? 0) >= (rule.threshold as number), result.testCoverage, rule.threshold);
    case "security_scan":
      return gateResult(rule, result.securityPass === true, result.securityPass, rule.threshold);
    case "diff_size":
      return gateResult(rule, (result.diffLines ?? 0) <= (rule.threshold as number), result.diffLines, rule.threshold);
    case "merge_conflict":
      return gateResult(rule, result.hasConflict !== true, result.hasConflict, rule.threshold);
    case "cr_score":
      return gateResult(
        rule,
        crScoreMeetsThreshold(result.crScore ?? "", rule.threshold as string),
        result.crScore,
        rule.threshold,
      );
    default:
      return gateResult(rule, true, null, rule.threshold);
  }
}

function gateResult(rule: GateRule, pass: boolean, actual: unknown, threshold: unknown): GateResult {
  return {
    ruleId: rule.id,
    pass,
    actual,
    threshold,
    message: pass ? `${rule.name}: 通过` : `${rule.name}: 未通过 (actual=${String(actual)}, threshold=${String(threshold)})`,
  };
}

function findRule(ruleId: string, rules: GateRule[]): GateRule | undefined {
  return rules.find(r => r.id === ruleId);
}

export function evaluate(result: StageResult, rules: GateRule[], phase: GatePhase): GateEvaluation {
  const applicable = rules.filter(r => r.enabled && r.gatePhase === phase);
  const details = applicable.map(rule => evaluateOne(rule, result));

  const blockingFailures = details.filter(r => {
    const rule = findRule(r.ruleId, rules);
    return !r.pass && rule && (rule.onFail === "block" || rule.onFail === "regress");
  });

  const warnFailures = details.filter(r => {
    const rule = findRule(r.ruleId, rules);
    return !r.pass && rule?.onFail === "warn";
  });

  return {
    allPass: blockingFailures.length === 0,
    details,
    blockingFailures,
    warnFailures,
  };
}
