import { describe, it, expect, vi, beforeEach } from "vitest";
import type { RepoConfig } from "@poria/core";

// Mock node:fs at module level (ESM-compatible)
vi.mock("node:fs", () => ({
  existsSync: vi.fn().mockReturnValue(true),
}));

// Mock the terminal exec function
vi.mock("../../terminal/index.js", () => {
  return {
    exec: vi.fn().mockResolvedValue({ code: 0, stdout: "", stderr: "" }),
    TimeoutError: class TimeoutError extends Error {
      override readonly name = "TimeoutError" as const;
      constructor(
        public readonly command: string,
        public readonly timeoutMs: number,
      ) {
        super(`Command timed out after ${timeoutMs}ms`);
      }
    },
  };
});

import { existsSync } from "node:fs";
import { WorktreeResource } from "../index.js";
import { exec } from "../../terminal/index.js";

const mockExec = vi.mocked(exec);
const mockExistsSync = vi.mocked(existsSync);

function makeRepo(overrides: Partial<RepoConfig> = {}): RepoConfig {
  return {
    name: "my-repo",
    gitUrl: "git@coding.jd.com:group/my-repo.git",
    branch: "feature_TEST123",
    baseBranch: "master",
    gitlabProjectPath: "group/my-repo",
    ...overrides,
  };
}

describe("WorktreeResource", () => {
  let resource: WorktreeResource;
  const repo = makeRepo();
  const pipelineId = "pl-20260916-abc12345";

  beforeEach(() => {
    vi.clearAllMocks();
    mockExistsSync.mockReturnValue(true);
    resource = new WorktreeResource("/test/workspace/projects");
  });

  describe("metadata", () => {
    it("has correct metadata", () => {
      expect(resource.metadata.id).toBe("resource:worktree");
      expect(resource.metadata.name).toBe("Git Worktree");
      expect(resource.metadata.version).toBe("0.1.0");
    });
  });

  describe("path()", () => {
    it("returns correct worktree path format", () => {
      const result = resource.path(repo, pipelineId);
      expect(result).toContain(pipelineId);
      expect(result).toContain(repo.name);
      expect(result).toMatch(/\/test\/workspace\/projects\/pl-20260916-abc12345\/my-repo$/);
    });
  });

  describe("create()", () => {
    it("calls git worktree add with correct arguments", async () => {
      const result = await resource.create(repo, pipelineId, "master");

      expect(mockExec).toHaveBeenCalledTimes(1);
      const call = mockExec.mock.calls[0]![0]!;
      expect(call.command).toContain("git worktree add");
      expect(call.command).toContain(repo.branch);
      expect(call.command).toContain("master");
      expect(call.timeoutMs).toBe(120_000);

      expect(result.worktreePath).toContain(pipelineId);
      expect(result.branch).toBe(repo.branch);
    });
  });

  describe("cleanDirtyState()", () => {
    it("calls git checkout . and git clean -fd", async () => {
      mockExistsSync.mockReturnValue(true);

      await resource.cleanDirtyState(repo, pipelineId);

      expect(mockExec).toHaveBeenCalledTimes(2);

      const checkoutCall = mockExec.mock.calls[0]![0]!;
      expect(checkoutCall.command).toBe("git checkout .");
      expect(checkoutCall.timeoutMs).toBe(120_000);

      const cleanCall = mockExec.mock.calls[1]![0]!;
      expect(cleanCall.command).toBe("git clean -fd");
      expect(cleanCall.timeoutMs).toBe(120_000);
    });

    it("skips cleaning when worktree path does not exist", async () => {
      mockExistsSync.mockReturnValue(false);

      await resource.cleanDirtyState(repo, pipelineId);

      expect(mockExec).not.toHaveBeenCalled();
    });
  });

  describe("remove()", () => {
    it("calls git worktree remove --force", async () => {
      mockExistsSync.mockReturnValue(true);

      await resource.remove(repo, pipelineId);

      expect(mockExec).toHaveBeenCalledTimes(1);
      const call = mockExec.mock.calls[0]![0]!;
      expect(call.command).toContain("git worktree remove --force");
      expect(call.timeoutMs).toBe(120_000);
    });

    it("skips removal when path does not exist", async () => {
      mockExistsSync.mockReturnValue(false);

      await resource.remove(repo, pipelineId);

      expect(mockExec).not.toHaveBeenCalled();
    });
  });

  describe("execute() (IResource interface)", () => {
    it("delegates to create()", async () => {
      const result = await resource.execute(
        { repo, pipelineId, baseBranch: "master" },
        {},
      );

      expect(result.worktreePath).toContain(pipelineId);
      expect(result.branch).toBe(repo.branch);
      expect(mockExec).toHaveBeenCalledTimes(1);
    });
  });
});
