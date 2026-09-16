import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

/** Normalize git URL to HTTPS lowercase form for comparison. */
export function normalizeGitUrl(gitUrl: string): string {
  return gitUrl
    .trim()
    .replace(/^git@([^:]+):/i, (_match, host: string) => `https://${host}/`)
    .replace(/\/+$/, "")
    .replace(/\.git$/i, "")
    .replace(/\/+$/, "")
    .toLowerCase();
}

/** Compare two git URLs after normalization. */
export function sameGitUrl(left?: string, right?: string): boolean {
  if (!left || !right) return false;
  return normalizeGitUrl(left) === normalizeGitUrl(right);
}

/** Extract the path portion from a git URL for repo search. */
export function repoSearchPathFromGitUrl(gitUrl: string): string | undefined {
  const cleaned = gitUrl.trim().replace(/\.git$/i, "");
  const pathPart = cleaned
    .replace(/^git@[^:]+:/i, "")
    .replace(/^ssh:\/\/git@[^/]+\//i, "")
    .replace(/^https?:\/\/[^/]+\//i, "")
    .replace(/^\/+/, "");
  return pathPart || undefined;
}

export type LocalGitContext = {
  workspacePath?: string;
  gitUrl?: string;
  repoName?: string;
  currentBranch?: string;
};

async function git(
  cwd: string,
  args: string[],
  timeout = 5000,
): Promise<string> {
  try {
    const { stdout } = await execFileAsync("git", args, {
      cwd,
      timeout,
      windowsHide: true,
      maxBuffer: 10 * 1024 * 1024,
    });
    return stdout.trim();
  } catch (error) {
    const err = error as { stderr?: string; message?: string };
    const detail = (err.stderr || err.message || String(error)).trim();
    throw new Error(detail || "git command failed");
  }
}

export async function getLocalGitContext(
  cwd = process.cwd(),
): Promise<LocalGitContext> {
  try {
    const inside = await git(cwd, ["rev-parse", "--is-inside-work-tree"]);
    if (inside !== "true") return { workspacePath: cwd };
  } catch {
    return { workspacePath: cwd };
  }

  let gitUrl: string | undefined;
  try {
    gitUrl = await git(cwd, ["remote", "get-url", "origin"]);
  } catch {
    try {
      const remotes = (await git(cwd, ["remote"])).split(/\s+/).filter(Boolean);
      if (remotes[0]) {
        gitUrl = await git(cwd, ["remote", "get-url", remotes[0]]);
      }
    } catch {
      gitUrl = undefined;
    }
  }

  let currentBranch: string | undefined;
  try {
    currentBranch = (await git(cwd, ["branch", "--show-current"])) || undefined;
  } catch {
    currentBranch = undefined;
  }

  return {
    workspacePath: cwd,
    gitUrl,
    repoName: gitUrl ? repoSearchPathFromGitUrl(gitUrl) : undefined,
    currentBranch,
  };
}

export async function detectOriginHeadBranch(
  cwd: string,
): Promise<string | undefined> {
  try {
    const ref = await git(cwd, [
      "symbolic-ref",
      "refs/remotes/origin/HEAD",
    ]);
    const match = /refs\/remotes\/origin\/(.+)$/.exec(ref);
    return match?.[1]?.trim() || undefined;
  } catch {
    return undefined;
  }
}

export async function createAndPushBranch(input: {
  cwd?: string;
  expectedGitUrl: string;
  defaultBranch: string;
  newBranch: string;
}): Promise<void> {
  const gitContext = await getLocalGitContext(input.cwd);
  const cwd = gitContext.workspacePath;
  if (!cwd || !gitContext.gitUrl) {
    throw new Error("当前工作区没有 git 仓库，无法创建分支");
  }
  if (!sameGitUrl(gitContext.gitUrl, input.expectedGitUrl)) {
    throw new Error("当前工作区仓库与所选仓库不一致，无法创建并推送分支");
  }
  const defaultBranch = input.defaultBranch.trim() || "master";
  const newBranch = input.newBranch.trim();
  if (!newBranch) throw new Error("缺少新分支名");
  if (!/^[A-Za-z0-9._/-]+$/.test(newBranch)) {
    throw new Error("分支名包含非法字符");
  }

  // Idempotent: if already on the branch and it tracks origin, skip recreate.
  if (gitContext.currentBranch === newBranch) {
    try {
      await git(cwd, ["rev-parse", "--verify", `origin/${newBranch}`], 15_000);
      return;
    } catch {
      // fall through to push
    }
  }

  await git(cwd, ["fetch", "origin"], 120_000);
  try {
    await git(cwd, ["checkout", defaultBranch], 30_000);
  } catch {
    await git(
      cwd,
      ["checkout", "-B", defaultBranch, `origin/${defaultBranch}`],
      30_000,
    );
  }
  try {
    await git(cwd, ["merge", "--ff-only", `origin/${defaultBranch}`], 60_000);
  } catch {
    // already up to date
  }

  try {
    await git(cwd, ["rev-parse", "--verify", newBranch], 5_000);
    await git(cwd, ["checkout", newBranch], 15_000);
  } catch {
    await git(cwd, ["checkout", "-b", newBranch], 15_000);
  }
  await git(cwd, ["push", "-u", "origin", newBranch], 120_000);
}
