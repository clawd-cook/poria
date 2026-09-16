import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Pipeline, Stage } from "@poria/core";
import { DEFAULT_GATES } from "@poria/core";
import { PipelineRollback } from "../rollback.js";
import type { IRollbackDeps } from "../rollback.js";

function makePipeline(overrides?: Partial<Pipeline>): Pipeline {
  return {
    id: "pl-test",
    demandId: 1,
    demandCode: "D-001",
    status: "running",
    rawLink: "",
    operator: "user1",
    hasRegressed: false,
    config: { gates: DEFAULT_GATES, trdScope: [], repos: [] },
    stages: [
      {
        pipelineId: "pl-test",
        name: "workspace",
        status: "completed",
        retryCount: 0,
        maxRetries: 3,
        output: { worktreePath: "/tmp/worktree", branch: "feat/test" },
        rollback: {
          stageIndex: 3,
          commands: [
            { type: "remove_worktree", params: { path: "/tmp/worktree" } },
            { type: "delete_branch", params: { branch: "feat/test" } },
          ],
        },
      },
      {
        pipelineId: "pl-test",
        name: "deploy",
        status: "completed",
        retryCount: 0,
        maxRetries: 3,
        output: { mrUrls: ["https://coding.jd.com/group/project/-/merge_requests/42"] },
        rollback: {
          stageIndex: 6,
          commands: [
            { type: "close_mr", params: { mrUrl: "https://coding.jd.com/group/project/-/merge_requests/42" } },
          ],
        },
      },
    ] as Stage[],
    repos: [{ name: "main-repo", gitUrl: "", branch: "feat/test", baseBranch: "master", gitlabProjectPath: "group/project" }],
    createdAt: new Date(),
    updatedAt: new Date(),
    ...overrides,
  };
}

describe("PipelineRollback", () => {
  let deps: IRollbackDeps;
  let pipeline: Pipeline;

  beforeEach(() => {
    pipeline = makePipeline();
    deps = {
      store: {
        load: vi.fn().mockResolvedValue(pipeline),
        saveStageTx: vi.fn(),
      },
      terminal: {
        exec: vi.fn().mockResolvedValue({ stdout: "" }),
      },
      codingChannel: {
        getMrStatus: vi.fn().mockResolvedValue("opened"),
        closeMr: vi.fn().mockResolvedValue(undefined),
        createMergeRequest: vi.fn().mockResolvedValue({ url: "https://coding.jd.com/g/p/-/merge_requests/99" }),
      },
      jme: {
        send: vi.fn().mockResolvedValue(undefined),
      },
      logger: {
        warn: vi.fn(),
      },
      fileExists: vi.fn().mockReturnValue(true),
    };
  });

  it("executes unmerged rollback in reverse stage order", async () => {
    const rollback = new PipelineRollback(deps);
    await rollback.execute("pl-test");

    // Deploy stage is last → its commands execute first (reverse order)
    expect(deps.codingChannel.closeMr).toHaveBeenCalledWith("group/project", 42);
    // Then workspace stage commands
    expect(deps.terminal.exec).toHaveBeenCalledWith(
      expect.objectContaining({ command: expect.stringContaining("worktree remove") }),
    );
    expect(deps.terminal.exec).toHaveBeenCalledWith(
      expect.objectContaining({ command: expect.stringContaining("git push origin --delete feat/test") }),
    );
    expect(deps.store.saveStageTx).toHaveBeenCalled();
  });

  it("skips close_mr for already closed MR", async () => {
    (deps.codingChannel.getMrStatus as ReturnType<typeof vi.fn>).mockResolvedValue("closed");

    const rollback = new PipelineRollback(deps);
    await rollback.execute("pl-test");

    expect(deps.codingChannel.closeMr).not.toHaveBeenCalled();
  });

  it("skips remove_worktree when path does not exist", async () => {
    (deps.fileExists as ReturnType<typeof vi.fn>).mockReturnValue(false);

    const rollback = new PipelineRollback(deps);
    await rollback.execute("pl-test");

    const worktreeExecCalls = (deps.terminal.exec as ReturnType<typeof vi.fn>).mock.calls.filter(
      (call: unknown[]) => {
        const arg = call[0] as { command: string };
        return arg.command.includes("worktree");
      },
    );
    expect(worktreeExecCalls).toHaveLength(0);
  });

  it("continues rollback when a single command fails", async () => {
    (deps.codingChannel.closeMr as ReturnType<typeof vi.fn>).mockRejectedValue(new Error("network error"));

    const rollback = new PipelineRollback(deps);
    await rollback.execute("pl-test");

    // Should still attempt workspace cleanup despite closeMr failure
    expect(deps.terminal.exec).toHaveBeenCalled();
    expect(deps.logger.warn).toHaveBeenCalled();
    expect(deps.store.saveStageTx).toHaveBeenCalled();
  });

  it("executes merged rollback with revert MR", async () => {
    // Add branch info to deploy output for merged rollback
    const deployStage = pipeline.stages.find(s => s.name === "deploy")!;
    deployStage.output = {
      ...deployStage.output,
      branch: "feat/test",
      worktreePath: "/tmp/worktree",
    };
    (deps.codingChannel.getMrStatus as ReturnType<typeof vi.fn>).mockResolvedValue("merged");

    const rollback = new PipelineRollback(deps);
    await rollback.execute("pl-test");

    expect(deps.terminal.exec).toHaveBeenCalledWith(
      expect.objectContaining({ command: expect.stringContaining("git revert") }),
    );
    expect(deps.codingChannel.createMergeRequest).toHaveBeenCalled();
    expect(deps.jme.send).toHaveBeenCalled();
    expect(deps.store.saveStageTx).toHaveBeenCalled();
  });
});
