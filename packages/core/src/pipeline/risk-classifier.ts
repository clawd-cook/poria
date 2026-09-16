export type RiskLevel = "low" | "medium" | "high" | "critical";

const CONFIG_PATTERNS = [
  /^\.env/,
  /package\.json$/,
  /tsconfig.*\.json$/,
  /pnpm-workspace\.yaml$/,
  /\.eslintrc/,
  /vite\.config/,
  /next\.config/,
  /webpack\.config/,
];

const CRITICAL_PATTERNS = [
  /^\.github\//,
  /^\.gitlab-ci/,
  /Dockerfile/,
  /docker-compose/,
  /^deploy\//,
  /^infra\//,
];

export function classifyRisk(changedFiles: string[], diffLines: number): RiskLevel {
  const hasCritical = changedFiles.some(f => CRITICAL_PATTERNS.some(p => p.test(f)));
  if (hasCritical) return "critical";

  const hasConfig = changedFiles.some(f => CONFIG_PATTERNS.some(p => p.test(f)));
  if (hasConfig && diffLines > 100) return "high";
  if (hasConfig) return "medium";

  if (diffLines > 500) return "high";
  if (diffLines > 200) return "medium";

  return "low";
}
