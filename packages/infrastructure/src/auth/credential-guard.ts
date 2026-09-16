import type { JacpCredentials } from "./credentials.js";
import { saveCredentials, parseUsernameFromCookie } from "./credentials.js";
import { readBrowserCookies } from "./browser-cookie.js";

/**
 * Thrown when cookie has expired and auto-refresh fails.
 * Pipeline should enter BLOCKED state and notify user to re-login.
 */
export class AuthExpiredDuringPipelineError extends Error {
  constructor(message?: string) {
    super(
      message ??
        "Cookie has expired and auto-refresh failed. Please run `poria auth login` to re-authenticate.",
    );
    this.name = "AuthExpiredDuringPipelineError";
  }
}

/**
 * Function signature for the API probe used to check credential validity.
 * Injected for testability.
 */
export type ApiProbe = (credentials: JacpCredentials) => Promise<void>;

/**
 * Guards credential validity before each pipeline stage.
 *
 * Flow:
 * 1. Probe a lightweight API endpoint with current credentials
 * 2. If probe fails with auth error -> attempt browser cookie refresh
 * 3. If refresh also fails -> throw AuthExpiredDuringPipelineError
 */
export class CredentialGuard {
  constructor(
    private readonly probe: ApiProbe,
    private readonly userRoot?: string,
  ) {}

  /**
   * Ensure credentials are valid. Returns the (possibly refreshed) credentials.
   * Throws AuthExpiredDuringPipelineError if validation and refresh both fail.
   */
  async ensureValid(credentials: JacpCredentials): Promise<JacpCredentials> {
    try {
      await this.probe(credentials);
      return credentials;
    } catch (probeError) {
      if (!isAuthExpired(probeError)) {
        throw probeError;
      }

      // Cookie expired - attempt auto-refresh from browser
      try {
        const cookie = await readBrowserCookies();
        const username = parseUsernameFromCookie(cookie) ?? credentials.username;
        const refreshed: JacpCredentials = { username, cookie };
        saveCredentials(refreshed, this.userRoot);
        // Verify the refreshed credentials work
        await this.probe(refreshed);
        return refreshed;
      } catch {
        throw new AuthExpiredDuringPipelineError();
      }
    }
  }
}

function isAuthExpired(error: unknown): boolean {
  if (error instanceof Error) {
    const msg = error.message.toLowerCase();
    return (
      msg.includes("401") ||
      msg.includes("unauthorized") ||
      msg.includes("auth") ||
      msg.includes("cookie") ||
      msg.includes("expired") ||
      msg.includes("login")
    );
  }
  return false;
}
