import type { MetricsSnapshot } from "../collector.js";

export interface IMetricsReporter {
  report(snapshot: MetricsSnapshot): void;
}

/**
 * Stdout reporter: writes metrics snapshot as formatted text to stdout.
 */
export class StdoutReporter implements IMetricsReporter {
  report(snapshot: MetricsSnapshot): void {
    const lines: string[] = [`--- Metrics Snapshot ${snapshot.timestamp} ---`];

    for (const metric of snapshot.metrics) {
      const labelStr =
        Object.keys(metric.labels).length > 0
          ? ` {${Object.entries(metric.labels)
              .map(([k, v]) => `${k}="${v}"`)
              .join(", ")}}`
          : "";

      if (metric.type === "counter") {
        lines.push(`  ${metric.name}${labelStr} = ${metric.value}`);
      } else {
        const count = metric.observations?.length ?? 0;
        lines.push(
          `  ${metric.name}${labelStr} avg=${metric.value.toFixed(3)} count=${count}`,
        );
      }
    }

    lines.push("---");
    process.stdout.write(lines.join("\n") + "\n");
  }
}
