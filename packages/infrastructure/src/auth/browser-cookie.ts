/**
 * Browser cookie extraction.
 * Migrated from @dj-lib/poria-auth, adapted to remove @dj-lib/poria-plugin-sdk dependency.
 */

export type BrowserCookie = {
  name?: string;
  value?: string;
  domain?: string;
  path?: string;
  secure?: boolean;
  expires?: number;
};

export const DEFAULT_COOKIE_DOMAINS = ["jd.com", "coding.jd.com"] as const;

function domainMatch(cookieDomain: string, host: string): boolean {
  const d = (cookieDomain || "").replace(/^\./, "").toLowerCase();
  const h = (host || "").toLowerCase();
  if (!d || !h) return false;
  if (d === h) return true;
  if (h.endsWith(`.${d}`)) return true;
  return false;
}

function registeredDomain(host: string): string {
  const parts = host.split(".");
  return parts.slice(-2).join(".") || host;
}

function filterAndDedup(
  cookies: BrowserCookie[],
  hosts: string[],
): BrowserCookie[] {
  const matched = cookies.filter((c) =>
    hosts.some((host) => domainMatch(c.domain || "", host)),
  );
  const best = new Map<string, BrowserCookie>();
  for (const c of matched) {
    const key = c.name || "";
    const cur = best.get(key);
    if (!cur) {
      best.set(key, c);
      continue;
    }
    const cd = (c.domain || "").length;
    const ud = (cur.domain || "").length;
    const moreSpecific =
      cd !== ud ? cd > ud : (c.path || "").length > (cur.path || "").length;
    if (moreSpecific) best.set(key, c);
  }
  return Array.from(best.values());
}

function cookiesToHeader(cookies: BrowserCookie[]): string {
  return cookies
    .filter((c) => c.name && c.value != null)
    .map((c) => `${c.name}=${c.value}`)
    .join("; ");
}

async function loadRookie(): Promise<Record<string, unknown>> {
  try {
    // Dynamic import: @rookie-rs/api is an optional dependency
    // @ts-expect-error — optional peer dependency, may not be installed
    return (await import("@rookie-rs/api")) as Record<string, unknown>;
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(
      `Failed to load @rookie-rs/api (${message}). Install it or use --cookie fallback.`,
    );
  }
}

/**
 * Read browser cookies for the given domains using @rookie-rs/api.
 * This is a dynamic import so the dependency is optional.
 */
export async function readBrowserCookies(
  domains: string[] = [...DEFAULT_COOKIE_DOMAINS],
  browser = "chrome",
): Promise<string> {
  const rookie = await loadRookie();
  const readDomains = [
    ...new Set(domains.map((d) => registeredDomain(d.replace(/^\./, "")))),
  ];

  let raw: unknown;
  if (browser === "any" && typeof rookie["load"] === "function") {
    raw = await (rookie["load"] as (d: string[]) => Promise<unknown>)(readDomains);
  } else if (typeof rookie[browser] === "function") {
    raw = await (
      rookie[browser] as (d: string[]) => Promise<unknown>
    )(readDomains);
  } else if (typeof rookie["load"] === "function") {
    raw = await (
      rookie["load"] as (
        d: string[],
        opts?: { browser: string },
      ) => Promise<unknown>
    )(readDomains, { browser });
  } else {
    throw new Error(`@rookie-rs/api does not support browser: ${browser}`);
  }

  if (!Array.isArray(raw)) {
    throw new Error("Unexpected cookie payload from @rookie-rs/api");
  }

  const filtered = filterAndDedup(
    raw as BrowserCookie[],
    domains.map((d) => d.replace(/^\./, "").toLowerCase()),
  );
  const header = cookiesToHeader(filtered);
  if (!header) {
    throw new Error(
      `No login cookies found for ${domains.join(", ")}. ` +
        `Log in via browser (macOS may prompt for Keychain access), then retry.`,
    );
  }
  return header;
}
