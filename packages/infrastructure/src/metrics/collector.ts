/**
 * Metrics collector interface matching Design SS9.
 * Implementations record counters and histograms for pipeline observability.
 */

export interface IMetricsCollector {
  // Pipeline dimension
  incrementCounter(name: string, labels?: Record<string, string>): void;
  recordHistogram(name: string, value: number, labels?: Record<string, string>): void;

  // Convenience methods
  recordPipelineCreated(): void;
  recordPipelineCompleted(durationMs: number): void;
  recordPipelineFailed(): void;
  recordStageComplete(stageName: string, durationMs: number): void;
  recordStageRetry(stageName: string): void;
  recordStageFailure(stageName: string, issueClass: string): void;
  recordAgentExecution(stageName: string, durationMs: number): void;
  recordOutputGuardViolation(): void;
  recordLlmTokens(stageName: string, model: string, tokens: number): void;
  recordLlmCost(stageName: string, model: string, cost: number): void;
  recordHumanLoop(): void;
  recordHumanLoopResponse(responseMs: number): void;
  recordHumanLoopEscalation(): void;
  recordGatePass(gateId: string): void;
  recordGateFail(gateId: string): void;

  // Snapshot for reporters
  snapshot(): MetricsSnapshot;
}

export interface MetricEntry {
  name: string;
  type: "counter" | "histogram";
  labels: Record<string, string>;
  value: number;
  /** For histograms: all observed values */
  observations?: number[];
}

export interface MetricsSnapshot {
  timestamp: string;
  metrics: MetricEntry[];
}

/**
 * In-memory metrics collector. Stores counters and histograms
 * for later export via reporters.
 */
export class InMemoryMetricsCollector implements IMetricsCollector {
  private counters = new Map<string, number>();
  private histograms = new Map<string, number[]>();

  private counterKey(name: string, labels?: Record<string, string>): string {
    if (!labels || Object.keys(labels).length === 0) return name;
    const sorted = Object.entries(labels).sort(([a], [b]) => a.localeCompare(b));
    return `${name}{${sorted.map(([k, v]) => `${k}="${v}"`).join(",")}}`;
  }

  incrementCounter(name: string, labels?: Record<string, string>): void {
    const key = this.counterKey(name, labels);
    this.counters.set(key, (this.counters.get(key) ?? 0) + 1);
  }

  recordHistogram(name: string, value: number, labels?: Record<string, string>): void {
    const key = this.counterKey(name, labels);
    const existing = this.histograms.get(key) ?? [];
    existing.push(value);
    this.histograms.set(key, existing);
  }

  recordPipelineCreated(): void {
    this.incrementCounter("pipeline_total");
  }

  recordPipelineCompleted(durationMs: number): void {
    this.incrementCounter("pipeline_completed");
    this.recordHistogram("pipeline_duration_seconds", durationMs / 1000);
  }

  recordPipelineFailed(): void {
    this.incrementCounter("pipeline_failed");
  }

  recordStageComplete(stageName: string, durationMs: number): void {
    this.recordHistogram("stage_duration_seconds", durationMs / 1000, { stage: stageName });
  }

  recordStageRetry(stageName: string): void {
    this.incrementCounter("stage_retry_total", { stage: stageName });
  }

  recordStageFailure(stageName: string, issueClass: string): void {
    this.incrementCounter("stage_failure_total", { stage: stageName, issue_class: issueClass });
  }

  recordAgentExecution(stageName: string, durationMs: number): void {
    this.recordHistogram("agent_execution_seconds", durationMs / 1000, { stage: stageName });
  }

  recordOutputGuardViolation(): void {
    this.incrementCounter("agent_output_guard_violations");
  }

  recordLlmTokens(stageName: string, model: string, tokens: number): void {
    this.incrementCounter("llm_tokens_total", { stage: stageName, model });
    const key = this.counterKey("llm_tokens_total", { stage: stageName, model });
    // Add the full count, not just +1
    this.counters.set(key, (this.counters.get(key) ?? 0) + tokens - 1);
  }

  recordLlmCost(stageName: string, model: string, cost: number): void {
    this.incrementCounter("llm_cost_total", { stage: stageName, model });
    const key = this.counterKey("llm_cost_total", { stage: stageName, model });
    this.counters.set(key, (this.counters.get(key) ?? 0) + cost - 1);
  }

  recordHumanLoop(): void {
    this.incrementCounter("human_loop_total");
  }

  recordHumanLoopResponse(responseMs: number): void {
    this.recordHistogram("human_loop_response_seconds", responseMs / 1000);
  }

  recordHumanLoopEscalation(): void {
    this.incrementCounter("human_loop_escalation_total");
  }

  recordGatePass(gateId: string): void {
    this.incrementCounter("gate_pass_total", { gate: gateId });
  }

  recordGateFail(gateId: string): void {
    this.incrementCounter("gate_fail_total", { gate: gateId });
  }

  snapshot(): MetricsSnapshot {
    const metrics: MetricEntry[] = [];

    for (const [key, value] of this.counters) {
      const { name, labels } = parseKey(key);
      metrics.push({ name, type: "counter", labels, value });
    }

    for (const [key, observations] of this.histograms) {
      const { name, labels } = parseKey(key);
      const sum = observations.reduce((a, b) => a + b, 0);
      metrics.push({
        name,
        type: "histogram",
        labels,
        value: observations.length > 0 ? sum / observations.length : 0,
        observations: [...observations],
      });
    }

    return { timestamp: new Date().toISOString(), metrics };
  }
}

function parseKey(key: string): { name: string; labels: Record<string, string> } {
  const braceIdx = key.indexOf("{");
  if (braceIdx === -1) return { name: key, labels: {} };

  const name = key.slice(0, braceIdx);
  const labelStr = key.slice(braceIdx + 1, -1);
  const labels: Record<string, string> = {};
  for (const pair of labelStr.split(",")) {
    const eqIdx = pair.indexOf("=");
    if (eqIdx > 0) {
      const k = pair.slice(0, eqIdx);
      const v = pair.slice(eqIdx + 2, -1); // strip quotes
      labels[k] = v;
    }
  }
  return { name, labels };
}
