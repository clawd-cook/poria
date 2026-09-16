import fs from "node:fs";
import path from "node:path";
import type { MetricsSnapshot } from "../collector.js";
import type { IMetricsReporter } from "./stdout.js";

/**
 * File reporter: appends metrics snapshot as a JSON line to a file.
 */
export class FileReporter implements IMetricsReporter {
  constructor(private readonly filePath: string) {}

  report(snapshot: MetricsSnapshot): void {
    const dir = path.dirname(this.filePath);
    fs.mkdirSync(dir, { recursive: true });
    fs.appendFileSync(this.filePath, JSON.stringify(snapshot) + "\n", "utf-8");
  }
}
