import { existsSync } from "node:fs";
import { join, resolve } from "node:path";
import type { IResource, CapabilityMetadata, ResourceContext, RepoConfig } from "@poria/core";
import { exec, TimeoutError } from "../terminal/index.js";

// Re-export TimeoutError for convenience
export { TimeoutError };

// ---------- Types ----------

export interface WorktreeCreateInput {
  repo: RepoConfig;
  pipelineId: string;
  baseBranch: string;
}

export interface WorktreeCreateResult {
  worktreePath: string;
  branch: string;
}

// ---------- Constants ----------

/** Default workspace root where worktrees are created */
const DEFAULT_WORKSPACE_ROOT = "workspace/projects";

const GIT_MEDIUM_TIMEOUT = 120_000; // 2 minutes for git operations

// ---------- Worktree Resource ----------

export class WorktreeResource implements IResource<WorktreeCreateInput, WorktreeCreateResult> {
  readonly metadata: CapabilityMetadata = {
    id: "resource:worktree",
    name: "Git Worktree",
    description: "Git worktree lifecycle management: create, clean, remove",
    version: "0.1.0",
  };

  private readonly workspaceRoot: string;

  constructor(workspaceRoot?: string) {
    this.workspaceRoot = workspaceRoot ?? DEFAULT_WORKSPACE_ROOT;
  }

  async execute(input: WorktreeCreateInput, _context: ResourceContext): Promise<WorktreeCreateResult> {
    return this.create(input.repo, input.pipelineId, input.baseBranch);
  }

  /**
   * Return the conventional worktree path for a repo within a pipeline.
   * Format: `{workspaceRoot}/{pipelineId}/{repoName}`
   */
  path(repo: RepoConfig, pipelineId: string): string {
    return resolve(join(this.workspaceRoot, pipelineId, repo.name));
  }

  /**
   * Create a new git worktree for a repo branch.
   * Runs `git worktree add <path> -b <branch> <baseBranch>` from the repo's git root.
   */
  async create(
    repo: RepoConfig,
    pipelineId: string,
    baseBranch: string,
  ): Promise<WorktreeCreateResult> {
    const worktreePath = this.path(repo, pipelineId);
    const branch = repo.branch;

    // git worktree add creates the directory and checks out the new branch
    await exec({
      command: `git worktree add "${worktreePath}" -b "${branch}" "${baseBranch}"`,
      cwd: this.resolveRepoRoot(repo),
      timeoutMs: GIT_MEDIUM_TIMEOUT,
    });

    return { worktreePath, branch };
  }

  /**
   * Clean dirty state in a worktree (Recovery scenario, Design F7).
   * Discards uncommitted changes: `git checkout .` + `git clean -fd`.
   */
  async cleanDirtyState(repo: RepoConfig, pipelineId: string): Promise<void> {
    const worktreePath = this.path(repo, pipelineId);

    if (!existsSync(worktreePath)) {
      return; // Nothing to clean if worktree does not exist
    }

    // Discard tracked file changes
    await exec({
      command: "git checkout .",
      cwd: worktreePath,
      timeoutMs: GIT_MEDIUM_TIMEOUT,
    });

    // Remove untracked files and directories
    await exec({
      command: "git clean -fd",
      cwd: worktreePath,
      timeoutMs: GIT_MEDIUM_TIMEOUT,
    });
  }

  /**
   * Remove a worktree. Checks existence first, uses --force to handle locked worktrees.
   */
  async remove(repo: RepoConfig, pipelineId: string): Promise<void> {
    const worktreePath = this.path(repo, pipelineId);

    if (!existsSync(worktreePath)) {
      return; // Already removed
    }

    await exec({
      command: `git worktree remove --force "${worktreePath}"`,
      cwd: this.resolveRepoRoot(repo),
      timeoutMs: GIT_MEDIUM_TIMEOUT,
    });
  }

  /**
   * Resolve the root directory of a repo.
   * For worktree operations, the command must run from the main repo checkout.
   * Uses the repo gitUrl as a hint — in practice the caller provides cwd.
   */
  private resolveRepoRoot(_repo: RepoConfig): string {
    // In the current architecture, repo.gitUrl contains the git remote URL.
    // The actual local path where the repo is cloned is managed by the workspace skill.
    // For worktree operations we rely on cwd being set to the repo root.
    // Return a relative path — the caller (workspace skill) is responsible for
    // providing the correct working directory context.
    return ".";
  }
}
