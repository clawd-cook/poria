export const TIMEOUT = {
  GIT_SHORT: 30_000,
  GIT_MEDIUM: 120_000,
  GIT_LONG: 300_000,
  BUILD: 600_000,
  AGENT: 1_800_000,
} as const;

export type TimeoutTier = keyof typeof TIMEOUT;
