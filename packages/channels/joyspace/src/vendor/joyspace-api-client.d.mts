export function buildCookieHeader(input: {
  cookies?: Record<string, string> | null;
  cookieHeader?: string;
}): string;

export function buildDefaultAuthOptions(): { tenantCode: string };

export function createJoySpaceApiContext(overrides?: {
  tenantCode?: string;
  cookieHeader?: string;
  cookies?: Record<string, string>;
  cookieSource?: string;
  pythonCommand?: string;
}): Promise<{
  authMode: string;
  cookieSource: string;
  tenantCode: string;
  teamHeaderId: string;
  cookieHeader: string;
  options: Record<string, unknown>;
}>;

export function downloadDiagramXml(linkUrl: string): Promise<string>;

export function extractPageIdFromUrl(pageUrl: string): string;

export function fetchDiagramDetail(input: {
  diagramId: string;
  pageId: string;
  cookieHeader: string;
  teamHeaderId: string;
}): Promise<Record<string, unknown>>;

export function fetchPageBasic(input: {
  pageId: string;
  cookieHeader: string;
  teamHeaderId: string;
}): Promise<Record<string, unknown>>;

export function fetchPageContent(input: {
  pageId: string;
  cookieHeader: string;
  teamHeaderId: string;
}): Promise<{ content?: unknown[] } & Record<string, unknown>>;

export function loadJdCookiesFromBrowser(options?: {
  pythonCommand?: string;
}): Promise<{
  cookies: Record<string, string>;
  source: string;
  error: string;
}>;

export function normalizeBrowserCookiePayload(payload: unknown): {
  cookies: Record<string, string>;
  source: string;
  error: string;
};

export function normalizeCookieMap(
  cookies: unknown,
): Record<string, string>;

export function requestJoySpaceJson(input: {
  method: string;
  url: string;
  cookieHeader: string;
  teamHeaderId: string;
  body?: unknown;
}): Promise<unknown>;

export function resolveAuth(options?: {
  cookieHeader?: string;
  cookies?: Record<string, string>;
  cookieSource?: string;
  pythonCommand?: string;
}): Promise<{
  mode: string;
  cookieSource: string;
  cookies: Record<string, string> | null;
  cookieHeader?: string;
}>;
