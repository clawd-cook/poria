import type { JacpCredentials } from "../types.js";

export type { JacpCredentials };

export const AUTH_EXPIRED_MESSAGE =
  "登录态已失效，请重新登录（poria auth login）";

type JacpEnvelope<T> = {
  code?: number;
  message?: string;
  msg?: string;
  data?: T;
};

export const DEFAULT_JACP_BASE_URL = "http://api-gateway.jd.com/jacp";
export const DEFAULT_JACP_APP_ID = "joycode";

export function asOptionalNumber(value: unknown): number | undefined {
  if (value == null || value === "") return undefined;
  const num = Number(value);
  return Number.isFinite(num) ? num : undefined;
}

export function asOptionalText(value: unknown): string | undefined {
  if (value == null) return undefined;
  const text = String(value).trim();
  return text ? text : undefined;
}

export function asUnknownList(value: unknown): unknown[] {
  if (Array.isArray(value)) return value;
  if (
    value &&
    typeof value === "object" &&
    Array.isArray((value as { records?: unknown }).records)
  ) {
    return (value as { records: unknown[] }).records;
  }
  return [];
}

function formatFetchError(error: unknown): string {
  if (
    error &&
    typeof error === "object" &&
    (error as { name?: unknown }).name === "AbortError"
  ) {
    return "请求超时";
  }
  return error instanceof Error ? error.message : String(error);
}

const RETRYABLE_FETCH =
  /UND_ERR_CONNECT_TIMEOUT|UND_ERR_SOCKET|ECONNRESET|ECONNREFUSED|ETIMEDOUT|ENOTFOUND|EAI_AGAIN|EPIPE|ECONNABORTED/i;

export async function fetchWithRetry(
  input: string | URL,
  init?: RequestInit,
  retries = 2,
): Promise<Response> {
  let lastError: unknown;
  for (let attempt = 0; attempt <= retries; attempt += 1) {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 20_000);
    try {
      return await fetch(input, { ...init, signal: controller.signal });
    } catch (error) {
      lastError = error;
      const retryable =
        attempt < retries &&
        !(
          error &&
          typeof error === "object" &&
          (error as { name?: unknown }).name === "AbortError"
        ) &&
        RETRYABLE_FETCH.test(formatFetchError(error));
      if (!retryable) throw error;
      await new Promise((resolve) => setTimeout(resolve, 400 * (attempt + 1)));
    } finally {
      clearTimeout(timer);
    }
  }
  throw lastError instanceof Error
    ? lastError
    : new Error(formatFetchError(lastError));
}

function throwAuthExpired(): never {
  throw new Error(AUTH_EXPIRED_MESSAGE);
}

function throwIfAuthExpiredPayload(code: unknown, message?: unknown): void {
  const num = Number(code);
  const text = String(message ?? "");
  if (num === 304 || num === 401 || text.includes("登录态已失效")) {
    throwAuthExpired();
  }
}

export async function fetchJsonEnvelope<T>(
  credentials: JacpCredentials,
  url: string,
  init: RequestInit,
  errorLabel: string,
  options?: { allowEmpty?: boolean; extraHeaders?: Record<string, string> },
): Promise<T> {
  let response: Response;
  try {
    response = await fetchWithRetry(url, {
      ...init,
      headers: {
        Cookie: credentials.cookie,
        optErp: credentials.username,
        ...(options?.extraHeaders ?? {}),
        ...(init.headers ?? {}),
      },
    });
  } catch (error) {
    throw new Error(`${errorLabel}: ${formatFetchError(error)}`);
  }

  if (response.status === 401) throwAuthExpired();
  if (!response.ok) {
    throw new Error(`${errorLabel}: HTTP ${response.status}`);
  }

  const envelope = (await response.json()) as JacpEnvelope<T>;
  if (envelope.code !== 200) {
    throwIfAuthExpiredPayload(envelope.code, envelope.message || envelope.msg);
    throw new Error(
      envelope.message ||
        envelope.msg ||
        `${errorLabel}: code ${String(envelope.code)}`,
    );
  }
  if (envelope.data == null) {
    if (options?.allowEmpty) return undefined as T;
    throw new Error(`${errorLabel}: empty response`);
  }
  return envelope.data;
}

export async function jacpFetch<T>(
  credentials: JacpCredentials,
  path: string,
  init: RequestInit,
  errorLabel: string,
  options?: { allowEmpty?: boolean },
): Promise<T> {
  const base =
    process.env.PORIA_JACP_BASE_URL?.trim().replace(/\/+$/, "") ||
    DEFAULT_JACP_BASE_URL;
  const appId = process.env.PORIA_JACP_APP_ID?.trim() || DEFAULT_JACP_APP_ID;
  return fetchJsonEnvelope(credentials, `${base}${path}`, init, errorLabel, {
    allowEmpty: options?.allowEmpty,
    extraHeaders: { appId, optErp: credentials.username },
  });
}
