import type {
  Pipeline, Stage, StageEnum, SkillOutput,
  RollbackInstruction, ISkill, SkillContext,
  PipelineEvent, StageResult,
} from "@poria/core";
import {
  STAGE_ORDER,
  transitionPipeline, transitionStage,
  pipelineStartedEvent, pipelineFailedEvent, pipelineWaitingMergeEvent,
  stageStartedEvent, stageCompletedEvent, stageFailedEvent, stageBlockedEvent,
  stageRegressedEvent, gateEvaluatedEvent, gateRegressTriggeredEvent,
  evaluateGates,
} from "@poria/core";
import { handleStageError } from "./handle-error.js";
import type { IHumanLoop } from "./handle-error.js";

export interface IPipelineStore {
  load(pipelineId: string): Promise<Pipeline>;
  saveStageTx(stage: Stage | null, pipeline: Pipeline, events: PipelineEvent[]): void;
}

export interface ISkillLoader {
  load(skillId: string): Promise<ISkill>;
}

export interface ICredentialGuard {
  ensureValid(): Promise<unknown>;
}

export interface IMultiRepoOrchestrator {
  execute(
    pipeline: Pipeline,
    stage: Stage,
    skill: ISkill,
    credentials: unknown,
  ): Promise<SkillOutput>;
}

const STAGE_SKILL_MAP: Record<StageEnum, string> = {
  init: "skill:init",
  review_prd: "skill:review-prd",
  design: "skill:gen-trd",
  workspace: "skill:workspace",
  dev: "skill:gen-code",
  cr: "skill:code-review",
  deploy: "skill:deploy",
};

const MULTI_REPO_STAGES: ReadonlySet<StageEnum> = new Set(["dev", "cr", "deploy"]);

function findCurrentStage(pipeline: Pipeline): Stage | null {
  return pipeline.stages.find(s => s.status !== "completed" && s.status !== "skipped") ?? null;
}

function buildRollbackInstructions(stage: Stage, _pipeline: Pipeline): RollbackInstruction | undefined {
  const stageIndex = STAGE_ORDER.indexOf(stage.name);
  const commands: RollbackInstruction["commands"] = [];

  if (stage.name === "workspace") {
    const output = stage.output as Record<string, string> | undefined;
    if (output?.["worktreePath"]) {
      commands.push({ type: "remove_worktree", params: { path: String(output["worktreePath"]) } });
    }
    if (output?.["branch"]) {
      commands.push({ type: "delete_branch", params: { branch: String(output["branch"]) } });
    }
  }

  if (stage.name === "deploy") {
    const output = stage.output as Record<string, unknown> | undefined;
    const mrUrls = output?.["mrUrls"] as string[] | undefined;
    if (mrUrls) {
      for (const url of mrUrls) {
        commands.push({ type: "close_mr", params: { mrUrl: url } });
      }
    }
  }

  if (commands.length === 0) return undefined;
  return { stageIndex, commands };
}

export class PipelineExecutor {
  constructor(
    private readonly store: IPipelineStore,
    private readonly skillLoader: ISkillLoader,
    private readonly humanLoop: IHumanLoop | undefined,
    private readonly credentialGuard: ICredentialGuard,
    private readonly multiRepoOrchestrator?: IMultiRepoOrchestrator,
  ) {}

  async run(pipelineId: string): Promise<void> {
    const pipeline = await this.store.load(pipelineId);

    if (pipeline.status === "created") {
      transitionPipeline(pipeline, "running");
      this.store.saveStageTx(null, pipeline, [pipelineStartedEvent(pipeline.id)]);
    }

    while (pipeline.status === "running") {
      const stage = findCurrentStage(pipeline);
      if (!stage) {
        transitionPipeline(pipeline, "waiting_merge");
        const mrUrls = this.collectMrUrls(pipeline);
        this.store.saveStageTx(null, pipeline, [pipelineWaitingMergeEvent(pipeline.id, mrUrls)]);
        return;
      }

      if (stage.status === "failed" && stage.retryCount >= stage.maxRetries) {
        transitionPipeline(pipeline, "failed");
        this.store.saveStageTx(stage, pipeline, [
          pipelineFailedEvent(pipeline.id, `Stage ${stage.name} exhausted retries`),
        ]);
        return;
      }

      try {
        const credentials = await this.credentialGuard.ensureValid();

        transitionStage(stage, "running");
        stage.startedAt = new Date();
        this.store.saveStageTx(stage, pipeline, [stageStartedEvent(pipeline.id, stage.name)]);

        const skillId = STAGE_SKILL_MAP[stage.name];
        const skill = await this.skillLoader.load(skillId);

        const isMultiRepo = pipeline.repos.length > 1 && MULTI_REPO_STAGES.has(stage.name);
        let result: SkillOutput;

        if (isMultiRepo && this.multiRepoOrchestrator) {
          result = await this.multiRepoOrchestrator.execute(pipeline, stage, skill, credentials);
        } else {
          const ctx: SkillContext = {
            pipelineId: pipeline.id,
            workdir: String(stage.input?.["workdir"] ?? ""),
            credentials,
          };
          result = await skill.execute({ stage, pipeline }, ctx);
        }

        stage.output = result.output;
        stage.completedAt = new Date();
        const rollback = buildRollbackInstructions(stage, pipeline);
        if (rollback) stage.rollback = rollback;

        const events: PipelineEvent[] = [
          stageCompletedEvent(pipeline.id, stage.name, result.output),
        ];

        // CR gate evaluation
        if (stage.name === "cr") {
          const gateAction = this.handleCrGate(pipeline, stage, events);
          if (gateAction === "regress") {
            this.store.saveStageTx(stage, pipeline, events);
            continue;
          }
          if (gateAction === "blocked") {
            this.store.saveStageTx(stage, pipeline, events);
            return;
          }
        }

        // Deploy gate evaluation
        if (stage.name === "deploy") {
          this.handleDeployGate(pipeline, stage, events);
          if (stage.status === "blocked") {
            this.store.saveStageTx(stage, pipeline, events);
            return;
          }
        }

        transitionStage(stage, "completed");
        this.store.saveStageTx(stage, pipeline, events);

        // Deploy complete → waiting_merge
        if (stage.name === "deploy") {
          transitionPipeline(pipeline, "waiting_merge");
          const mrUrls = this.collectMrUrls(pipeline);
          this.store.saveStageTx(null, pipeline, [pipelineWaitingMergeEvent(pipeline.id, mrUrls)]);
          return;
        }
      } catch (error) {
        const result = await handleStageError(pipeline, stage, error, this.humanLoop);
        const events: PipelineEvent[] = [];

        if (result.action === "retry") {
          events.push(stageFailedEvent(pipeline.id, stage.name, String(error), stage.retryCount));
        } else if (result.action === "blocked") {
          events.push(stageBlockedEvent(pipeline.id, stage.name, result.issueClass));
        } else {
          events.push(pipelineFailedEvent(pipeline.id, String(error)));
        }

        this.store.saveStageTx(stage, pipeline, events);

        if (result.action === "blocked" || result.action === "failed") return;
        // retry → continue loop
      }
    }
  }

  private handleCrGate(
    pipeline: Pipeline,
    stage: Stage,
    events: PipelineEvent[],
  ): "pass" | "regress" | "blocked" {
    const crResult: StageResult = {
      crScore: stage.output?.["crScore"] as string | undefined,
      securityPass: stage.output?.["securityPass"] as boolean | undefined,
    };
    const evaluation = evaluateGates(crResult, pipeline.config.gates, "stage_exit");
    events.push(gateEvaluatedEvent(pipeline.id, stage.name, evaluation.details));

    if (evaluation.allPass) return "pass";

    const regressRule = pipeline.config.gates.find(
      r => r.enabled && r.onFail === "regress" && !evaluation.details.find(d => d.ruleId === r.id)?.pass,
    );

    if (regressRule) {
      if (!pipeline.hasRegressed) {
        pipeline.hasRegressed = true;
        const devStage = pipeline.stages.find(s => s.name === "dev");
        const crStage = pipeline.stages.find(s => s.name === "cr");
        if (devStage) {
          devStage.status = "pending";
          devStage.input = {
            ...devStage.input,
            crFeedback: {
              crScore: crResult.crScore,
              findings: stage.output?.["findings"],
            },
          };
        }
        if (crStage) crStage.status = "pending";
        events.push(
          stageRegressedEvent(pipeline.id, "cr", regressRule.regressTo ?? "dev", `CR score: ${String(crResult.crScore)}`),
          gateRegressTriggeredEvent(pipeline.id, regressRule.id, "cr", regressRule.regressTo ?? "dev"),
        );
        return "regress";
      }

      // Already regressed once, block
      stage.status = "blocked";
      pipeline.status = "blocked";
      events.push(stageBlockedEvent(pipeline.id, stage.name, "low_cr_score" as never));
      return "blocked";
    }

    // Non-regress blocking failure
    stage.status = "blocked";
    pipeline.status = "blocked";
    return "blocked";
  }

  private handleDeployGate(
    pipeline: Pipeline,
    stage: Stage,
    events: PipelineEvent[],
  ): void {
    const deployResult: StageResult = {
      ciBuildPass: stage.output?.["ciBuildPass"] as boolean | undefined,
      testCoverage: stage.output?.["testCoverage"] as number | undefined,
      diffLines: stage.output?.["diffLines"] as number | undefined,
      hasConflict: stage.output?.["hasConflict"] as boolean | undefined,
    };
    const evaluation = evaluateGates(deployResult, pipeline.config.gates, "deploy");
    events.push(gateEvaluatedEvent(pipeline.id, stage.name, evaluation.details));

    if (evaluation.blockingFailures.length > 0) {
      stage.status = "blocked";
      pipeline.status = "blocked";
      return;
    }

    if (evaluation.warnFailures.length > 0) {
      stage.output = {
        ...stage.output,
        gateWarnings: evaluation.warnFailures.map(f => f.message),
      };
    }
  }

  private collectMrUrls(pipeline: Pipeline): string[] {
    const deployStage = pipeline.stages.find(s => s.name === "deploy");
    const mrUrls = deployStage?.output?.["mrUrls"] as string[] | undefined;
    return mrUrls ?? [];
  }
}
