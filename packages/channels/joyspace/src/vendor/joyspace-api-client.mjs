import { execFile } from "node:child_process";
import { promisify } from "node:util";

const DEFAULT_JOYSPACE_API_BASE = "https://apijoyspace.jd.com";
const DEFAULT_TENANT_CODE = "CN.JD.GROUP";

const TENANT_CONFIG = Object.freeze({
  "CN.JD.GROUP": { teamHeaderId: "00046419", ddAppId: "ee" },
  "TH.JD.GROUP": { teamHeaderId: "00046420", ddAppId: "th.ee" },
  "ID.JD.GROUP": { teamHeaderId: "00046421", ddAppId: "id.ee" },
  "SF.JD.GROUP": { teamHeaderId: "00046422", ddAppId: "sf.ee" },
});
const execFileAsync = promisify(execFile);
const BROWSER_COOKIE3_SCRIPT = String.raw`
import json
import sys

LOADERS = ["chrome", "chromium", "firefox", "edge", "brave", "vivaldi"]

try:
    import browser_cookie3
except Exception as exc:
    print(json.dumps({
        "ok": False,
        "error": "browser_cookie3 import failed: " + str(exc),
        "attempts": [],
    }, ensure_ascii=False))
    sys.exit(0)

def collect(loader_name):
    loader = getattr(browser_cookie3, loader_name, None)
    if not callable(loader):
        return {}, "loader unavailable"
    try:
        cookie_dict = {}
        for cookie in loader(domain_name="jd.com"):
            name = str(getattr(cookie, "name", "") or "")
            value = str(getattr(cookie, "value", "") or "")
            if name and value:
                cookie_dict[name] = value
        return cookie_dict, None
    except Exception as exc:
        return {}, str(exc)

attempts = []

# Match relay-d2c-b: Chrome jd.com cookie jar is the validated first path.
cookies, error = collect("chrome")
attempts.append({"loader": "chrome", "count": len(cookies), "error": error})
if cookies:
    print(json.dumps({
        "ok": True,
        "source": "chrome",
        "cookies": cookies,
        "attempts": attempts,
    }, ensure_ascii=False))
    sys.exit(0)

for loader_name in LOADERS:
    if loader_name == "chrome":
        continue
    cookies, error = collect(loader_name)
    attempts.append({"loader": loader_name, "count": len(cookies), "error": error})
    if cookies:
        print(json.dumps({
            "ok": True,
            "source": loader_name,
            "cookies": cookies,
            "attempts": attempts,
        }, ensure_ascii=False))
        sys.exit(0)

print(json.dumps({
    "ok": False,
    "error": "No jd.com cookies found in supported browsers",
    "attempts": attempts,
}, ensure_ascii=False))
`;

function requireTenantConfig(tenantCode) {
  const config = TENANT_CONFIG[tenantCode];
  if (!config) {
    throw new Error(
      `Unsupported tenantCode "${tenantCode}". Expected one of ${Object.keys(TENANT_CONFIG).join(", ")}`,
    );
  }
  return config;
}

function normalizeCookieMap(cookies) {
  if (!cookies || typeof cookies !== "object") {
    return {};
  }
  return Object.fromEntries(
    Object.entries(cookies)
      .map(([name, value]) => [String(name || "").trim(), String(value || "").trim()])
      .filter(([name, value]) => name && value),
  );
}

function buildCookieHeader({ cookies = null, cookieHeader = "" }) {
  if (cookieHeader) {
    return cookieHeader;
  }
  const cookieMap = normalizeCookieMap(cookies);
  if (Object.keys(cookieMap).length > 0) {
    return Object.entries(cookieMap)
      .map(([name, value]) => `${name}=${value}`)
      .join("; ");
  }
  return "";
}

function formatErrorWithCause(error) {
  const message = error?.message || String(error);
  const cause = error?.cause?.message || error?.cause;
  return cause ? `${message} (${cause})` : message;
}

function normalizeBrowserCookiePayload(payload) {
  if (!payload || typeof payload !== "object" || payload.ok !== true) {
    return {
      cookies: {},
      source: "",
      error: payload?.error || "browser cookie payload not ok",
    };
  }
  const cookies = normalizeCookieMap(payload.cookies);
  if (Object.keys(cookies).length === 0) {
    return {
      cookies: {},
      source: payload.source || "",
      error: "browser cookie payload has no usable cookies",
    };
  }
  return {
    cookies,
    source: String(payload.source || "browser"),
    error: "",
  };
}

async function loadJdCookiesFromBrowser({ pythonCommand = process.env.PYTHON || "python3" } = {}) {
  try {
    const { stdout, stderr } = await execFileAsync(
      pythonCommand,
      ["-c", BROWSER_COOKIE3_SCRIPT],
      {
        timeout: 15_000,
        maxBuffer: 1024 * 1024,
      },
    );
    const parsed = JSON.parse(stdout || "{}");
    const result = normalizeBrowserCookiePayload(parsed);
    if (result.error && stderr) {
      return { ...result, error: `${result.error}; ${stderr.trim()}` };
    }
    return result;
  } catch (error) {
    return {
      cookies: {},
      source: "",
      error: `browser_cookie3 lookup failed: ${formatErrorWithCause(error)}`,
    };
  }
}

function buildDefaultAuthOptions() {
  return {
    tenantCode:
      process.env.JMECHAT_TENANT_CODE ||
      process.env.JMECHAT_tenantCode ||
      DEFAULT_TENANT_CODE,
  };
}

async function resolveAuth(options = {}) {
  // Prefer explicit credentials / cookie header (Poria auth / plugin context).
  const explicitHeader = String(options.cookieHeader || "").trim();
  if (explicitHeader) {
    return {
      mode: "credentials",
      cookieSource: options.cookieSource || "credentials",
      cookies: null,
      cookieHeader: explicitHeader,
    };
  }
  const explicitCookies = normalizeCookieMap(options.cookies);
  if (Object.keys(explicitCookies).length > 0) {
    return {
      mode: "credentials",
      cookieSource: options.cookieSource || "credentials",
      cookies: explicitCookies,
    };
  }

  const browserCookies = await loadJdCookiesFromBrowser({
    pythonCommand: options.pythonCommand,
  });
  if (Object.keys(browserCookies.cookies).length > 0) {
    return {
      mode: "browser",
      cookieSource: browserCookies.source,
      cookies: browserCookies.cookies,
    };
  }

  throw new Error(
    `Unable to resolve JoySpace auth from credentials or browser cookies. ${browserCookies.error || "No jd.com cookies found in supported browsers"}. Run \`poria auth login\`, or install browser_cookie3 and login to joyspace.jd.com / jd.com in Chrome, then retry.`,
  );
}

async function requestJoySpaceJson({ method, url, cookieHeader, teamHeaderId, body }) {
  const response = await fetch(`${DEFAULT_JOYSPACE_API_BASE}${url}`, {
    method,
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
      Cookie: cookieHeader,
      "x-team-id": teamHeaderId,
    },
    body: body ? JSON.stringify(body) : undefined,
  });

  if (!response.ok) {
    throw new Error(`${url} HTTP ${response.status} ${response.statusText}`);
  }

  const json = await response.json();
  // Improved error detection for various error formats used by JoySpace APIs
  if (
    json?.status === "failed" ||
    json?.errCode ||
    (json?.errorCode && json.errorCode !== "0") ||
    (json?.code != null && json.code !== 0 && json.code !== "0")
  ) {
    const errCode = json.errCode || json.errorCode || json.code || "unknown";
    const errMsg =
      json.errMsg ||
      json.errorMsg ||
      json.msg ||
      json.message ||
      json.error ||
      "Unknown API error";
    throw new Error(`JoySpace API error ${errCode}: ${errMsg} (${url})`);
  }
  if (json?.status === "success" || json?.status === "0" || json?.status === 0) {
    return json.data;
  }
  return json.data ?? json;
}

function extractPageIdFromUrl(pageUrl) {
  const match = pageUrl.match(
    /joyspace\.jd\.com\/(?:pages|page|doc|sheets?|table|ppt|board|mind|meeting)\/([A-Za-z0-9_-]+)/i,
  );
  if (!match?.[1]) {
    throw new Error(`Unable to extract JoySpace page id from URL: ${pageUrl}`);
  }
  return match[1];
}

async function createJoySpaceApiContext(overrides = {}) {
  const options = {
    ...buildDefaultAuthOptions(),
  };
  for (const [key, value] of Object.entries(overrides)) {
    if (value !== undefined && value !== "") {
      options[key] = value;
    }
  }
  const auth = await resolveAuth(options);
  const { teamHeaderId } = requireTenantConfig(options.tenantCode);
  return {
    authMode: auth.mode,
    cookieSource: auth.cookieSource || "",
    tenantCode: options.tenantCode,
    teamHeaderId,
    cookieHeader: buildCookieHeader(auth),
    options,
  };
}

async function fetchPageBasic({ pageId, cookieHeader, teamHeaderId }) {
  return requestJoySpaceJson({
    method: "GET",
    url: `/v3/pages/${pageId}/basic?sendRecent=0`,
    cookieHeader,
    teamHeaderId,
  });
}

async function fetchPageContent({ pageId, cookieHeader, teamHeaderId }) {
  return requestJoySpaceJson({
    method: "POST",
    url: "/v1/pages/content",
    cookieHeader,
    teamHeaderId,
    body: { pageId },
  });
}

async function fetchDiagramDetail({ diagramId, pageId, cookieHeader, teamHeaderId }) {
  if (!diagramId) {
    throw new Error("fetchDiagramDetail: diagramId is required");
  }
  if (!pageId) {
    throw new Error("fetchDiagramDetail: pageId is required");
  }
  return requestJoySpaceJson({
    method: "GET",
    url: `/v1/diagram/${encodeURIComponent(diagramId)}/detail?pageId=${encodeURIComponent(pageId)}`,
    cookieHeader,
    teamHeaderId,
  });
}

async function downloadDiagramXml(linkUrl) {
  if (!linkUrl) {
    throw new Error("downloadDiagramXml: linkUrl is required");
  }
  const response = await fetch(linkUrl);
  if (!response.ok) {
    throw new Error(`diagram xml HTTP ${response.status} ${response.statusText}`);
  }
  return response.text();
}

export {
  buildCookieHeader,
  buildDefaultAuthOptions,
  createJoySpaceApiContext,
  downloadDiagramXml,
  extractPageIdFromUrl,
  fetchDiagramDetail,
  fetchPageBasic,
  fetchPageContent,
  loadJdCookiesFromBrowser,
  normalizeBrowserCookiePayload,
  normalizeCookieMap,
  requestJoySpaceJson,
  resolveAuth,
};
