// @poria/infrastructure — barrel export

// Store
export { initDatabase } from "./store/schema.js";
export { SqlitePipelineStore } from "./store/pipeline-repo.js";
export { EventStore } from "./store/event-store.js";
export { AuditStore } from "./store/audit-store.js";
export type { AuditAction, AuditEntry } from "./store/audit-store.js";
export { PipelineQueue, WorkerLock } from "./store/queue.js";
export { PipelineRecovery } from "./store/recovery.js";
export type { IWorktreeCleaner, IHumanLoopNotifier, RecoveryResult } from "./store/recovery.js";
export { EventArchiver, EventReplayService } from "./store/archiver.js";
export type { ArchiveResult } from "./store/archiver.js";
export { DatabaseBackup } from "./store/backup.js";

// Auth
export {
  getCredentials,
  saveCredentials,
  getStatus,
  getUserRoot,
  getAuthFilePath,
  redactCookie,
  authHeaders,
  parseUsernameFromCookie,
  assertSafeAuthRoot,
  logout,
} from "./auth/credentials.js";
export type { JacpCredentials, AuthStatus } from "./auth/credentials.js";
export { readBrowserCookies } from "./auth/browser-cookie.js";
export type { BrowserCookie } from "./auth/browser-cookie.js";
export { CredentialGuard, AuthExpiredDuringPipelineError } from "./auth/credential-guard.js";
export type { ApiProbe } from "./auth/credential-guard.js";

// Config
export { loadConfig } from "./config/index.js";
export type { PoriaConfig } from "./config/index.js";

// Logger
export { createLogger, JsonLogger } from "./logger/index.js";
export type { ILogger, LogLevel, LogContext } from "./logger/index.js";

// Plugin Loader
export { PluginLoader } from "./plugin-loader/index.js";
export type { IPluginLoader } from "./plugin-loader/index.js";

// Metrics
export { InMemoryMetricsCollector } from "./metrics/collector.js";
export type { IMetricsCollector, MetricEntry, MetricsSnapshot } from "./metrics/collector.js";
export { StdoutReporter } from "./metrics/reporters/stdout.js";
export type { IMetricsReporter } from "./metrics/reporters/stdout.js";
export { FileReporter } from "./metrics/reporters/file.js";
