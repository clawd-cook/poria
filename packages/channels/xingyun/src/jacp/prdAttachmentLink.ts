function asOptionalNumber(value: unknown): number | undefined {
  if (value == null || value === "") return undefined;
  const num = Number(value);
  return Number.isFinite(num) ? num : undefined;
}

const GENERIC_SEGMENTS = new Set([
  "pages",
  "page",
  "docs",
  "doc",
  "view",
  "views",
  "demand",
  "demands",
  "card",
  "cards",
  "mine",
  "p_view",
  "open",
  "team",
  "index",
  "detail",
]);

function pathDemandId(pathname: string): number | undefined {
  const match = /\/demands\/(?:view\/(?:card\/)?)?(\d+)/i.exec(pathname);
  return asOptionalNumber(match?.[1]);
}

function distinctivePathSegment(pathname: string): string | undefined {
  const segments = pathname.split("/").filter(Boolean);
  for (let i = segments.length - 1; i >= 0; i -= 1) {
    const segment = decodeURIComponent(segments[i] ?? "").trim();
    if (
      !segment ||
      GENERIC_SEGMENTS.has(segment.toLowerCase()) ||
      /^\d+$/.test(segment)
    ) {
      continue;
    }
    if (segment.length >= 6) return segment;
  }
  return undefined;
}

export function parsePrdLink(raw: string): {
  url: string;
  demandId?: number;
  fingerprint: string;
} {
  const urlText = raw.trim();
  if (!urlText) throw new Error("缺少 PRD 链接");
  let parsed: URL;
  try {
    parsed = new URL(urlText);
  } catch {
    throw new Error("不是合法的 PRD 链接");
  }

  const params = parsed.searchParams;
  const demandId =
    asOptionalNumber(params.get("demandId")) ??
    asOptionalNumber(params.get("id")) ??
    pathDemandId(parsed.pathname);

  const fingerprint =
    params.get("pageId")?.trim() ||
    params.get("docId")?.trim() ||
    params.get("page_id")?.trim() ||
    distinctivePathSegment(parsed.pathname) ||
    `${parsed.hostname}${parsed.pathname}`.replace(/\/+$/, "");

  return { url: urlText, demandId, fingerprint };
}

const JOYSPACE_HOST = /(?:^|\.)joyspace\.jd\.com$/i;

export function isJoySpacePrdLink(raw: string): boolean {
  try {
    const host = new URL(parsePrdLink(raw).url).hostname;
    return JOYSPACE_HOST.test(host);
  } catch {
    return false;
  }
}

/** Prefer attachments whose name/tag mentions PRD / 需求 / prd. */
export function preferPrdNamed(
  candidates: Array<{ tagName?: string; name?: string; url?: string }>,
): Array<{ tagName?: string; name?: string; url?: string }> {
  const preferred = candidates.filter(
    (item) =>
      /prd|需求/i.test(item.tagName ?? "") ||
      /prd|需求/i.test(item.name ?? ""),
  );
  return preferred.length > 0 ? preferred : candidates;
}
