import { execFile } from "node:child_process";
import { promisify } from "node:util";
import type { JacpCredentials, MrStatus, MrInfo, FindMrQuery, CreateMrResult } from "./types.js";
import { repoSearchPathFromGitUrl } from "./gitUrl.js";

const execFileAsync = promisify(execFile);

const DEFAULT_CODING_BASE = "https://coding.jd.com";

function getCodingBase(): string {
  return (
    process.env.PORIA_CODING_BASE_URL?.trim().replace(/\/+$/, "") ||
    DEFAULT_CODING_BASE
  );
}

export function projectIdFromGitUrl(gitUrl: string): string | undefined {
  return repoSearchPathFromGitUrl(gitUrl);
}

async function git(cwd: string, args: string[]): Promise<string> {
  const { stdout } = await execFileAsync("git", args, {
    cwd,
    timeout: 15_000,
    windowsHide: true,
  });
  return stdout.trim();
}

export async function detectCurrentBranch(
  cwd = process.cwd(),
): Promise<string | undefined> {
  try {
    const branch = await git(cwd, ["branch", "--show-current"]);
    return branch || undefined;
  } catch {
    return undefined;
  }
}

export async function detectDefaultTargetBranch(
  cwd = process.cwd(),
): Promise<string | undefined> {
  try {
    const ref = await git(cwd, ["symbolic-ref", "refs/remotes/origin/HEAD"]);
    const match = /refs\/remotes\/origin\/(.+)$/.exec(ref);
    return match?.[1]?.trim() || undefined;
  } catch {
    return undefined;
  }
}

export async function createMergeRequestLive(
  credentials: JacpCredentials,
  input: {
    projectId: string;
    title: string;
    description?: string;
    sourceBranch: string;
    targetBranch: string;
  },
): Promise<CreateMrResult> {
  const base = getCodingBase();
  const projectId = encodeURIComponent(input.projectId);
  const response = await fetch(
    `${base}/api/v4/projects/${projectId}/merge_requests`,
    {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Cookie: credentials.cookie,
      },
      body: JSON.stringify({
        source_branch: input.sourceBranch,
        target_branch: input.targetBranch,
        title: input.title,
        description: input.description ?? "",
      }),
    },
  );

  if (response.status === 401) {
    throw new Error("登录态已失效，请重新登录（poria auth login）");
  }
  if (!response.ok) {
    const body = await response.text();
    throw new Error(
      `Create merge request failed: HTTP ${response.status} ${body.slice(0, 400)}`,
    );
  }

  const data = (await response.json()) as {
    web_url?: string;
    iid?: number;
  };
  const url = data.web_url?.trim();
  if (!url) {
    throw new Error("Create merge request failed: empty web_url");
  }
  return {
    url,
    iid: data.iid,
    sourceBranch: input.sourceBranch,
    targetBranch: input.targetBranch,
  };
}

/**
 * Get MR status from GitLab API (Design A11).
 * GET /api/v4/projects/{encoded_path}/merge_requests/{iid}
 */
export async function getMrStatusLive(
  credentials: JacpCredentials,
  projectPath: string,
  iid: number,
): Promise<MrStatus> {
  const base = getCodingBase();
  const encodedPath = encodeURIComponent(projectPath);
  const response = await fetch(
    `${base}/api/v4/projects/${encodedPath}/merge_requests/${iid}`,
    {
      method: "GET",
      headers: {
        Cookie: credentials.cookie,
      },
    },
  );

  if (response.status === 401) {
    throw new Error("登录态已失效，请重新登录（poria auth login）");
  }
  if (response.status === 404) {
    throw new Error(`MR not found: project=${projectPath} iid=${iid}`);
  }
  if (!response.ok) {
    throw new Error(`Get MR status failed: HTTP ${response.status}`);
  }

  const data = (await response.json()) as { state?: string };
  const state = data.state?.trim();
  if (
    state === "opened" ||
    state === "closed" ||
    state === "merged" ||
    state === "locked"
  ) {
    return state;
  }
  throw new Error(`Unknown MR state: ${state ?? "undefined"}`);
}

/**
 * Find an existing MR matching the query (Design F-03 idempotent MR creation).
 * GET /api/v4/projects/{id}/merge_requests?source_branch=...&target_branch=...&state=opened
 */
export async function findMrLive(
  credentials: JacpCredentials,
  query: FindMrQuery,
): Promise<MrInfo | null> {
  const base = getCodingBase();
  const encodedPath = encodeURIComponent(query.projectPath);
  const params = new URLSearchParams({
    source_branch: query.sourceBranch,
    target_branch: query.targetBranch,
    state: query.state ?? "opened",
  });
  const response = await fetch(
    `${base}/api/v4/projects/${encodedPath}/merge_requests?${params.toString()}`,
    {
      method: "GET",
      headers: {
        Cookie: credentials.cookie,
      },
    },
  );

  if (response.status === 401) {
    throw new Error("登录态已失效，请重新登录（poria auth login）");
  }
  if (!response.ok) {
    throw new Error(`Find MR failed: HTTP ${response.status}`);
  }

  const data = (await response.json()) as Array<{
    web_url?: string;
    iid?: number;
    source_branch?: string;
    target_branch?: string;
    state?: string;
  }>;

  if (!Array.isArray(data) || data.length === 0) {
    return null;
  }

  const first = data[0]!;
  const url = first.web_url?.trim();
  if (!url || first.iid == null) return null;

  return {
    url,
    iid: first.iid,
    sourceBranch: first.source_branch ?? query.sourceBranch,
    targetBranch: first.target_branch ?? query.targetBranch,
    state: (first.state as MrStatus | undefined) ?? "opened",
  };
}

/** Fixture for getMrStatus. */
export function getMrStatusFixture(
  _projectPath: string,
  _iid: number,
): MrStatus {
  return "opened";
}

/** Fixture for findMr. */
export function findMrFixture(query: FindMrQuery): MrInfo | null {
  // In fixture mode, no existing MR found (allows creation)
  void query;
  return null;
}
