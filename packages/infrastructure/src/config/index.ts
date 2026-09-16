import fs from "node:fs";
import path from "node:path";

/**
 * Poria global configuration.
 * Covers gate thresholds, timeouts, retry counts, and queue polling.
 */
export interface PoriaConfig {
  /** Gate thresholds */
  gates: {
    crScoreThreshold: string;
    testCoverageThreshold: number;
    diffSizeThreshold: number;
  };

  /** Timeout tiers in milliseconds */
  timeouts: {
    gitShort: number;
    gitMedium: number;
    gitLong: number;
    build: number;
    agent: number;
    agentIdle: number;
  };

  /** Retry configuration */
  retry: {
    maxStageRetries: number;
    queuePollIntervalMs: number;
    mrPollIntervalMs: number;
    mrTimeoutMs: number;
  };

  /** Paths */
  paths: {
    dbPath: string;
    archiveDir: string;
    backupDir: string;
    logDir: string;
  };
}

const DEFAULT_CONFIG: PoriaConfig = {
  gates: {
    crScoreThreshold: "B+",
    testCoverageThreshold: 80,
    diffSizeThreshold: 500,
  },
  timeouts: {
    gitShort: 30_000,
    gitMedium: 120_000,
    gitLong: 300_000,
    build: 600_000,
    agent: 1_800_000,
    agentIdle: 300_000,
  },
  retry: {
    maxStageRetries: 3,
    queuePollIntervalMs: 5_000,
    mrPollIntervalMs: 60_000,
    mrTimeoutMs: 24 * 3600_000,
  },
  paths: {
    dbPath: "workspace/db/poria.db",
    archiveDir: "workspace/archive",
    backupDir: "workspace/db/backup",
    logDir: "workspace/logs",
  },
};

/**
 * Load configuration with overrides from file and environment variables.
 */
export function loadConfig(overrides?: Partial<PoriaConfig>): PoriaConfig {
  let fileConfig: Partial<PoriaConfig> = {};

  // Try loading from poria.config.json in cwd
  const configPath = path.resolve("poria.config.json");
  if (fs.existsSync(configPath)) {
    try {
      const raw = fs.readFileSync(configPath, "utf-8");
      fileConfig = JSON.parse(raw) as Partial<PoriaConfig>;
    } catch {
      // Ignore invalid config file, use defaults
    }
  }

  // Environment variable overrides
  const envOverrides: Partial<PoriaConfig> = {};
  const envGates = readEnvGates();
  if (Object.keys(envGates).length > 0) {
    envOverrides.gates = { ...DEFAULT_CONFIG.gates, ...envGates };
  }

  return deepMerge(DEFAULT_CONFIG, fileConfig, envOverrides, overrides ?? {});
}

function readEnvGates(): Partial<PoriaConfig["gates"]> {
  const result: Partial<PoriaConfig["gates"]> = {};
  const cr = process.env["PORIA_CR_SCORE_THRESHOLD"];
  if (cr) result.crScoreThreshold = cr;
  const cov = process.env["PORIA_TEST_COVERAGE_THRESHOLD"];
  if (cov) result.testCoverageThreshold = Number(cov);
  const diff = process.env["PORIA_DIFF_SIZE_THRESHOLD"];
  if (diff) result.diffSizeThreshold = Number(diff);
  return result;
}

function deepMerge(...sources: Partial<PoriaConfig>[]): PoriaConfig {
  const result = structuredClone(DEFAULT_CONFIG);
  for (const source of sources) {
    if (source.gates) Object.assign(result.gates, source.gates);
    if (source.timeouts) Object.assign(result.timeouts, source.timeouts);
    if (source.retry) Object.assign(result.retry, source.retry);
    if (source.paths) Object.assign(result.paths, source.paths);
  }
  return result;
}
