/**
 * Parse Xingyun demand URLs like:
 * http://xingyun.jd.com/demands/view/<code>/-1?demandId=<id>
 */
export function parseXingyunDemandUrl(raw: string): {
  demandId: number;
  demandCode?: string;
  url: string;
} {
  const text = raw.trim();
  if (!text) {
    throw new Error(
      "Missing Xingyun demand URL. Expected: http://xingyun.jd.com/demands/view/<code>/-1?demandId=<id>",
    );
  }
  let parsed: URL;
  try {
    parsed = new URL(text);
  } catch {
    throw new Error(
      `Invalid Xingyun demand URL: ${text}. Expected: http://xingyun.jd.com/demands/view/<code>/-1?demandId=<id>`,
    );
  }

  const host = parsed.hostname.toLowerCase();
  if (!/(?:^|\.)xingyun\.jd\.com$/i.test(host) && !host.includes("xingyun")) {
    throw new Error(
      `Not a Xingyun demand host (${parsed.hostname}). Expected xingyun.jd.com`,
    );
  }

  const demandIdRaw =
    parsed.searchParams.get("demandId") || parsed.searchParams.get("id");
  const demandId = Number(demandIdRaw);
  if (!Number.isFinite(demandId) || demandId <= 0) {
    throw new Error(
      "Xingyun URL missing demandId query param. Example: ...?demandId=12345",
    );
  }

  let demandCode: string | undefined;
  const pathMatch =
    /\/demands\/view\/([^/]+)/i.exec(parsed.pathname) ||
    /\/demands\/(?:view\/)?([^/]+)/i.exec(parsed.pathname);
  if (pathMatch?.[1] && !/^-?\d+$/.test(pathMatch[1])) {
    demandCode = decodeURIComponent(pathMatch[1]).trim() || undefined;
  }

  return { demandId, demandCode, url: text };
}

/** feature_<code> or feature_demand_<id>; illegal chars replaced by underscore */
export function featureBranchName(
  demandCode: string | undefined,
  demandId: number,
): string {
  const raw = demandCode?.trim()
    ? `feature_${demandCode.trim()}`
    : `feature_demand_${demandId}`;
  return raw.replace(/[^A-Za-z0-9._-]+/g, "_").replace(/_+/g, "_");
}

export function featureSlug(
  demandCode: string | undefined,
  demandId: number,
): string {
  const code = demandCode?.trim() || `demand-${demandId}`;
  const slug = code
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
  return `feat-${slug || demandId}`;
}
