import type { Pipeline, PipelineEvent, RollbackCommand, Stage } from "@poria/core";
import { rollbackExecutedEvent } from "@poria/core";
import type { IPipelineStore } from "./executor.js";

export interface IRollbackDeps {
  store: IPipelineStore;
  terminal: {
    exec(opts: { command: string; cwd?: string; timeoutMs?: number }): Promise<{ stdout: string }>;
  };
  codingChannel: {
    getMrStatus(mrUrl: string): Promise<string>;
    closeMr(projectPath: string, iid: number): Promise<void>;
    createMergeRequest(opts: Record<string, unknown>): Promise<{ url: string }>;
  };
  jme: {
    send(opts: { message: string; target: string }): Promise<void>;
  };
  logger: {
    warn(msg: string): void;
  };
  fileExists: (path: string) => boolean;
}

export class PipelineRollback {
  constructor(private readonly deps: IRollbackDeps) {}

  async execute(pipelineId: string): Promise<void> {
    const pipeline = await this.deps.store.load(pipelineId);
    const deployStage = pipeline.stages.find(s => s.name === "deploy");
    const mrUrls = deployStage?.output?.["mrUrls"] as string[] | undefined;

    const hasMergedMr = await this.checkAnyMerged(mrUrls ?? []);

    if (hasMergedMr) {
      await this.rollbackMerged(pipeline);
    } else {
      await this.rollbackUnmerged(pipeline);
    }
  }

  private async rollbackUnmerged(pipeline: Pipeline): Promise<void> {
    const events: PipelineEvent[] = [];
    const stagesReversed = [...pipeline.stages].reverse();

    for (const stage of stagesReversed) {
      if (!stage.rollback) continue;
      for (const cmd of stage.rollback.commands) {
        await this.executeRollbackCommand(cmd, pipeline.id, events);
      }
    }

    if (events.length > 0) {
      this.deps.store.saveStageTx(null, pipeline, events);
    }
  }

  private async rollbackMerged(pipeline: Pipeline): Promise<void> {
    const events: PipelineEvent[] = [];

    for (const repo of pipeline.repos) {
      try {
        const deployStage = pipeline.stages.find(s => s.name === "deploy");
        const repoOutput = this.getRepoOutput(deployStage, repo.name);
        const branch = repoOutput?.["branch"] as string | undefined;

        if (!branch) {
          this.deps.logger.warn(`No branch found for repo ${repo.name}, skipping revert`);
          continue;
        }

        await this.deps.terminal.exec({
          command: `git revert --no-edit HEAD`,
          cwd: repoOutput?.["worktreePath"] as string | undefined,
          timeoutMs: 120_000,
        });

        await this.deps.terminal.exec({
          command: `git push origin ${branch}`,
          cwd: repoOutput?.["worktreePath"] as string | undefined,
          timeoutMs: 120_000,
        });

        const mrResult = await this.deps.codingChannel.createMergeRequest({
          projectPath: repo.gitlabProjectPath,
          sourceBranch: branch,
          targetBranch: repo.baseBranch,
          title: `Revert: ${pipeline.demandName ?? pipeline.demandCode}`,
        });

        events.push(rollbackExecutedEvent(pipeline.id, "revert_mr", `Revert MR created: ${mrResult.url}`));

        if (pipeline.operator) {
          await this.deps.jme.send({
            message: `[Poria Rollback] Revert MR created for ${repo.name}: ${mrResult.url}`,
            target: pipeline.operator,
          });
        }
      } catch (error) {
        this.deps.logger.warn(`Revert failed for repo ${repo.name}: ${String(error)}`);
        events.push(rollbackExecutedEvent(pipeline.id, "revert_mr_failed", `Repo ${repo.name}: ${String(error)}`));
      }
    }

    if (events.length > 0) {
      this.deps.store.saveStageTx(null, pipeline, events);
    }
  }

  private async executeRollbackCommand(
    cmd: RollbackCommand,
    pipelineId: string,
    events: PipelineEvent[],
  ): Promise<void> {
    try {
      switch (cmd.type) {
        case "close_mr": {
          const mrUrl = cmd.params["mrUrl"] ?? "";
          const status = await this.deps.codingChannel.getMrStatus(mrUrl);
          if (status === "closed" || status === "merged") {
            events.push(rollbackExecutedEvent(pipelineId, "close_mr", `Skipped (already ${status}): ${mrUrl}`));
            return;
          }
          const { projectPath, iid } = parseMrUrl(mrUrl);
          await this.deps.codingChannel.closeMr(projectPath, iid);
          events.push(rollbackExecutedEvent(pipelineId, "close_mr", mrUrl));
          break;
        }
        case "delete_branch": {
          const branch = cmd.params["branch"] ?? "";
          await this.deps.terminal.exec({ command: `git push origin --delete ${branch}`, timeoutMs: 30_000 }).catch(() => {
            /* branch may already be deleted */
          });
          events.push(rollbackExecutedEvent(pipelineId, "delete_branch", branch));
          break;
        }
        case "remove_worktree": {
          const worktreePath = cmd.params["path"] ?? "";
          if (!this.deps.fileExists(worktreePath)) {
            events.push(rollbackExecutedEvent(pipelineId, "remove_worktree", `Skipped (not found): ${worktreePath}`));
            return;
          }
          await this.deps.terminal.exec({ command: `git worktree remove --force ${worktreePath}`, timeoutMs: 30_000 });
          events.push(rollbackExecutedEvent(pipelineId, "remove_worktree", worktreePath));
          break;
        }
        default:
          events.push(rollbackExecutedEvent(pipelineId, cmd.type, "Unknown command type"));
      }
    } catch (error) {
      this.deps.logger.warn(`Rollback command ${cmd.type} failed: ${String(error)}`);
      events.push(rollbackExecutedEvent(pipelineId, `${cmd.type}_failed`, String(error)));
    }
  }

  private async checkAnyMerged(mrUrls: string[]): Promise<boolean> {
    for (const url of mrUrls) {
      const status = await this.deps.codingChannel.getMrStatus(url);
      if (status === "merged") return true;
    }
    return false;
  }

  private getRepoOutput(stage: Stage | undefined, _repoName: string): Record<string, unknown> | undefined {
    if (!stage?.output) return undefined;
    return stage.output as Record<string, unknown>;
  }
}

function parseMrUrl(url: string): { projectPath: string; iid: number } {
  // GitLab MR URLs: https://coding.jd.com/group/project/-/merge_requests/123
  const match = url.match(/\/([^/]+\/[^/]+)\/-\/merge_requests\/(\d+)/);
  if (!match?.[1] || !match[2]) {
    return { projectPath: "", iid: 0 };
  }
  return { projectPath: match[1], iid: parseInt(match[2], 10) };
}
