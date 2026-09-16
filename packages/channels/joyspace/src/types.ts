/**
 * JacpCredentials type — migrated from @dj-lib/poria-plugin-sdk.
 */
export interface JacpCredentials {
  cookie: string;
  username: string;
}

/** Logger interface matching the subset used by joyspace export. */
export interface Logger {
  info?: (...args: unknown[]) => void;
  warn?: (...args: unknown[]) => void;
  error?: (...args: unknown[]) => void;
  debug?: (...args: unknown[]) => void;
}
