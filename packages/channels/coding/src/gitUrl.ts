/** Git URL helpers adapted from h2o-plugin gitContext (no VS Code). */

export function normalizeGitUrl(gitUrl: string): string {
  return gitUrl
    .trim()
    .replace(/^git@([^:]+):/i, (_match, host: string) => `https://${host}/`)
    .replace(/\/+$/, "")
    .replace(/\.git$/i, "")
    .replace(/\/+$/, "")
    .toLowerCase();
}

export function repoNameFromGitUrl(gitUrl: string): string | undefined {
  const cleaned = gitUrl.trim().replace(/\.git$/i, "");
  const pathPart = cleaned
    .replace(/^git@[^:]+:/, "")
    .replace(/^https?:\/\/[^/]+\//i, "");
  const name = pathPart.split("/").filter(Boolean).pop();
  return name || undefined;
}

export function repoSearchPathFromGitUrl(gitUrl: string): string | undefined {
  const cleaned = gitUrl.trim().replace(/\.git$/i, "");
  const pathPart = cleaned
    .replace(/^git@[^:]+:/i, "")
    .replace(/^ssh:\/\/git@[^/]+\//i, "")
    .replace(/^https?:\/\/[^/]+\//i, "")
    .replace(/^\/+/, "");
  return pathPart || undefined;
}

export function sameGitUrl(left?: string, right?: string): boolean {
  if (!left || !right) return false;
  return normalizeGitUrl(left) === normalizeGitUrl(right);
}

export async function getOriginGitUrl(
  cwd = process.cwd(),
): Promise<string | undefined> {
  const { execFile } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const execFileAsync = promisify(execFile);
  try {
    const { stdout } = await execFileAsync(
      "git",
      ["remote", "get-url", "origin"],
      { cwd, timeout: 10_000, windowsHide: true },
    );
    const url = stdout.trim();
    return url || undefined;
  } catch {
    return undefined;
  }
}
