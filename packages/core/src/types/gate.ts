import type { StageEnum } from "./pipeline.js";

export type GatePhase = "stage_exit" | "deploy";
export type GateOnFail = "block" | "warn" | "regress";

export interface GateRule {
  id: string;
  name: string;
  enabled: boolean;
  threshold: unknown;
  onFail: GateOnFail;
  gatePhase: GatePhase;
  regressTo?: StageEnum;
}

export interface GateResult {
  ruleId: string;
  pass: boolean;
  actual: unknown;
  threshold: unknown;
  message: string;
}

export interface GateEvaluation {
  allPass: boolean;
  details: GateResult[];
  blockingFailures: GateResult[];
  warnFailures: GateResult[];
}
