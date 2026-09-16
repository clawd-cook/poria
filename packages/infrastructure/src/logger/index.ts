export type LogLevel = "debug" | "info" | "warn" | "error";

export interface LogContext {
  pipelineId?: string;
  stageName?: string;
  [key: string]: unknown;
}

export interface ILogger {
  debug(message: string, context?: LogContext): void;
  info(message: string, context?: LogContext): void;
  warn(message: string, context?: LogContext): void;
  error(message: string, context?: LogContext): void;
  child(defaultContext: LogContext): ILogger;
}

const LEVEL_ORDER: Record<LogLevel, number> = {
  debug: 0,
  info: 1,
  warn: 2,
  error: 3,
};

/**
 * JSON-structured logger that outputs one JSON object per line.
 */
export class JsonLogger implements ILogger {
  private readonly minLevel: number;
  private readonly defaultContext: LogContext;
  private readonly writer: (line: string) => void;

  constructor(
    options: {
      level?: LogLevel;
      module?: string;
      context?: LogContext;
      writer?: (line: string) => void;
    } = {},
  ) {
    this.minLevel = LEVEL_ORDER[options.level ?? "info"];
    this.defaultContext = {
      module: options.module,
      ...options.context,
    };
    this.writer = options.writer ?? ((line: string) => process.stderr.write(line + "\n"));
  }

  debug(message: string, context?: LogContext): void {
    this.log("debug", message, context);
  }

  info(message: string, context?: LogContext): void {
    this.log("info", message, context);
  }

  warn(message: string, context?: LogContext): void {
    this.log("warn", message, context);
  }

  error(message: string, context?: LogContext): void {
    this.log("error", message, context);
  }

  child(defaultContext: LogContext): ILogger {
    return new JsonLogger({
      level: Object.entries(LEVEL_ORDER).find(
        ([, v]) => v === this.minLevel,
      )?.[0] as LogLevel,
      context: { ...this.defaultContext, ...defaultContext },
      writer: this.writer,
    });
  }

  private log(level: LogLevel, message: string, context?: LogContext): void {
    if (LEVEL_ORDER[level] < this.minLevel) return;

    const entry = {
      timestamp: new Date().toISOString(),
      level,
      message,
      ...this.defaultContext,
      ...context,
    };

    this.writer(JSON.stringify(entry));
  }
}

/**
 * Create a logger instance.
 */
export function createLogger(options?: {
  level?: LogLevel;
  module?: string;
  context?: LogContext;
  writer?: (line: string) => void;
}): ILogger {
  return new JsonLogger(options);
}
