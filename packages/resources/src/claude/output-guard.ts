import { minimatch } from "minimatch";

// ---------- Types ----------

export type ViolationSeverity = "block" | "warn";

export type ViolationType =
  | "trd_scope_empty"
  | "out_of_scope"
  | "diff_too_large"
  | "blocked_dependency";

export interface Violation {
  type: ViolationType;
  severity: ViolationSeverity;
  message: string;
  file?: string;
  actual?: number;
  threshold?: number;
  dependency?: string;
}

export interface OutputGuardConfig {
  /** Glob patterns of allowed file paths (from TRD scope) */
  allowedPaths: string[];
  /** Maximum diff lines before triggering a warning (default 500) */
  maxDiffLines: number;
  /** Known malicious/blocked dependency names */
  blockedDependencies: string[];
}

export interface AgentOutput {
  changedFiles: string[];
  totalDiffLines: number;
  addedDependencies: Array<{ name: string }>;
}

export interface GuardResult {
  pass: boolean;
  violations: Violation[];
}

// ---------- Defaults ----------

const DEFAULT_MAX_DIFF_LINES = 500;

// ---------- Output Guard ----------

/**
 * Agent output guard — checks file scope, diff size, and dependency safety.
 *
 * Design reference: Design section 6.1
 *
 * - File scope: Agent changes must match trdScope glob patterns.
 *   Empty trdScope => warn (not block) — more likely TRD extraction failure
 *   than "forbid all files".
 * - Diff size: totalDiffLines > maxDiffLines => warn (needs human review).
 * - Dependency safety: new deps in blockedDependencies list => block.
 *
 * pass = true when no "block" severity violations exist.
 */
export class OutputGuard {
  check(agentOutput: AgentOutput, config: OutputGuardConfig): GuardResult {
    const violations: Violation[] = [];

    // 1. File scope check
    this.checkFileScope(agentOutput.changedFiles, config.allowedPaths, violations);

    // 2. Diff size check
    this.checkDiffSize(agentOutput.totalDiffLines, config.maxDiffLines, violations);

    // 3. Dependency safety check
    this.checkDependencies(agentOutput.addedDependencies, config.blockedDependencies, violations);

    const hasBlockingViolation = violations.some((v) => v.severity === "block");

    return {
      pass: !hasBlockingViolation,
      violations,
    };
  }

  private checkFileScope(
    changedFiles: string[],
    allowedPaths: string[],
    violations: Violation[],
  ): void {
    // trdScope empty fallback: warn, not block (Design section 6.1)
    if (allowedPaths.length === 0) {
      violations.push({
        type: "trd_scope_empty",
        severity: "warn",
        message: "trdScope is empty — file scope guard skipped, all changes allowed",
      });
      return;
    }

    for (const file of changedFiles) {
      const matches = allowedPaths.some((pattern) => minimatch(file, pattern));
      if (!matches) {
        violations.push({
          type: "out_of_scope",
          severity: "block",
          message: `File "${file}" is outside allowed scope`,
          file,
        });
      }
    }
  }

  private checkDiffSize(
    totalDiffLines: number,
    maxDiffLines: number,
    violations: Violation[],
  ): void {
    const threshold = maxDiffLines > 0 ? maxDiffLines : DEFAULT_MAX_DIFF_LINES;

    if (totalDiffLines > threshold) {
      violations.push({
        type: "diff_too_large",
        severity: "warn",
        message: `Diff size (${totalDiffLines} lines) exceeds threshold (${threshold} lines) — needs human review`,
        actual: totalDiffLines,
        threshold,
      });
    }
  }

  private checkDependencies(
    addedDependencies: Array<{ name: string }>,
    blockedDependencies: string[],
    violations: Violation[],
  ): void {
    if (blockedDependencies.length === 0) {
      return;
    }

    const blockedSet = new Set(blockedDependencies);

    for (const dep of addedDependencies) {
      if (blockedSet.has(dep.name)) {
        violations.push({
          type: "blocked_dependency",
          severity: "block",
          message: `Dependency "${dep.name}" is on the blocked list`,
          dependency: dep.name,
        });
      }
    }
  }
}
