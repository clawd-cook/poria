import fs from "node:fs";
import os from "node:os";
import path from "node:path";

// ── Types (replaces @dj-lib/poria-plugin-sdk JacpCredentials) ──

export interface JacpCredentials {
  username: string;
  cookie: string;
}

export interface AuthStatus {
  loggedIn: boolean;
  username?: string;
}

interface StoredAuth extends JacpCredentials {
  updatedAt: string;
}

// ── Constants ──

export const USER_DIR_NAME = ".poria";
export const AUTH_FILE_NAME = "auth.json";
export const ERP_COOKIE_NAME = "erp_erp";

// ── Path helpers ──

export function getUserRoot(home = os.homedir()): string {
  return path.join(home, USER_DIR_NAME);
}

export function getAuthFilePath(userRoot = getUserRoot()): string {
  return path.join(userRoot, AUTH_FILE_NAME);
}

// ── Cookie helpers ──

/** Redact cookie for safe logging. */
export function redactCookie(cookie: string): string {
  if (!cookie) return "";
  if (cookie.length <= 12) return "***";
  return `${cookie.slice(0, 4)}...${cookie.slice(-4)} (${cookie.length} chars)`;
}

export function authHeaders(credentials: JacpCredentials): Record<string, string> {
  return { Cookie: credentials.cookie };
}

export function parseUsernameFromCookie(cookie: string): string | undefined {
  const parts = cookie.split(/;\s*/);
  for (const part of parts) {
    const eq = part.indexOf("=");
    if (eq <= 0) continue;
    const name = part.slice(0, eq).trim();
    const value = part.slice(eq + 1).trim();
    if (name === ERP_COOKIE_NAME && value) {
      return value;
    }
  }
  return undefined;
}

// ── File I/O ──

function writeFileAtomic(filePath: string, data: string, mode = 0o600): void {
  const dir = path.dirname(filePath);
  fs.mkdirSync(dir, { recursive: true, mode: 0o700 });
  const tmp = path.join(dir, `.${path.basename(filePath)}.${process.pid}.tmp`);
  try {
    fs.writeFileSync(tmp, data, { mode });
    fs.renameSync(tmp, filePath);
    fs.chmodSync(filePath, mode);
  } catch (error) {
    try {
      fs.unlinkSync(tmp);
    } catch {
      // ignore
    }
    throw error;
  }
}

// ── Credential CRUD ──

/**
 * Read credentials from ~/.poria/auth.json.
 * Returns undefined if file does not exist or is invalid.
 */
export function getCredentials(userRoot = getUserRoot()): JacpCredentials | undefined {
  const filePath = getAuthFilePath(userRoot);
  if (!fs.existsSync(filePath)) return undefined;
  try {
    const raw = fs.readFileSync(filePath, "utf-8");
    const parsed = JSON.parse(raw) as StoredAuth;
    if (!parsed?.username || !parsed?.cookie) return undefined;
    return { username: parsed.username, cookie: parsed.cookie };
  } catch {
    return undefined;
  }
}

/**
 * Check login status.
 */
export function getStatus(userRoot = getUserRoot()): AuthStatus {
  const creds = getCredentials(userRoot);
  if (!creds) return { loggedIn: false };
  return { loggedIn: true, username: creds.username };
}

/**
 * Persist credentials to ~/.poria/auth.json with atomic write.
 */
export function saveCredentials(
  credentials: JacpCredentials,
  userRoot = getUserRoot(),
  options?: { home?: string },
): void {
  if (!credentials.username?.trim() || !credentials.cookie?.trim()) {
    throw new Error("username and cookie are required");
  }
  assertSafeAuthRoot(userRoot, options?.home);
  const payload: StoredAuth = {
    username: credentials.username.trim(),
    cookie: credentials.cookie.trim(),
    updatedAt: new Date().toISOString(),
  };
  writeFileAtomic(getAuthFilePath(userRoot), JSON.stringify(payload, null, 2));
}

/**
 * Auth may only live under the user home `.poria/` (or a non-`.poria` test root).
 * Never write credentials into a project workflow `.poria/` directory.
 */
export function assertSafeAuthRoot(
  userRoot: string,
  home = os.homedir(),
): void {
  const resolved = path.resolve(userRoot);
  const allowed = path.resolve(getUserRoot(home));
  if (resolved === allowed) return;

  const marker = `${path.sep}${USER_DIR_NAME}`;
  const underProjectPoria =
    resolved.endsWith(marker) || resolved.includes(`${marker}${path.sep}`);
  if (underProjectPoria) {
    throw new Error(
      "Refusing to store auth credentials under a project .poria/ directory; use ~/.poria/ only",
    );
  }
}

/**
 * Remove stored credentials.
 */
export function logout(userRoot = getUserRoot()): void {
  const filePath = getAuthFilePath(userRoot);
  if (fs.existsSync(filePath)) {
    fs.unlinkSync(filePath);
  }
}
