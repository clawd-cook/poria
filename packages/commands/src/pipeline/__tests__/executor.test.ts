import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Pipeline, Stage, ISkill, SkillContext, SkillInput } from "@poria/core";
import { STAGE_ORDER, DEFAULT_GATES } from "@poria/core";
import { PipelineExecutor } from "../executor.js";
import type { IPipelineStore, ISkillLoader, ICredentialGuard } from "../executor.js";
import type { IHumanLoop } from "../handle-error.js";

function makeStages(): Stage[] {
  return STAGE_ORDER.map(name => ({
    pipelineId: "pl-test",
    name,
    status: "pending" as const,
    retryCount: 0,
    maxRetries: 3,
  }));
}

function makePipeline(overrides?: Partial<Pipeline>): Pipeline {
  return {
    id: "pl-test",
    demandId: 1,
    demandCode: "D-001",
    status: "created",
    rawLink: "",
    operator: "user1",
    hasRegressed: false,
    config: { gates: DEFAULT_GATES, trdScope: [], repos: [] },
    stages: makeStages(),
    repos: [{ name: "main-repo", gitUrl: "", branch: "feat/test", baseBranch: "master", gitlabProjectPath: "group/project" }],
    createdAt: new Date(),
    updatedAt: new Date(),
    ...overrides,
  };
}

function mockSkill(output: Record<string, unknown> = {}): ISkill {
  return {
    metadata: { id: "test", name: "test", description: "test", version: "1.0" },
    execute: vi.fn().mockResolvedValue({ output }),
  };
}

function mockStore(pipeline: Pipeline): IPipelineStore {
  return {
    load: vi.fn().mockResolvedValue(pipeline),
    saveStageTx: vi.fn(),
  };
}

describe("PipelineExecutor", () => {
  let pipeline: Pipeline;
  let store: IPipelineStore;
  let skillLoader: ISkillLoader;
  let credentialGuard: ICredentialGuard;
  let humanLoop: IHumanLoop;

  beforeEach(() => {
    pipeline = makePipeline();
    store = mockStore(pipeline);
    humanLoop = { notify: vi.fn().mockResolvedValue(undefined) };
    credentialGuard = { ensureValid: vi.fn().mockResolvedValue({ token: "test" }) };
  });

  it("drives 7 stages to waiting_merge with passing CR and deploy gates", async () => {
    const crSkill = mockSkill({ crScore: "A", securityPass: true, findings: [] });
    const deploySkill = mockSkill({
      mrUrls: ["https://coding.jd.com/group/project/-/merge_requests/1"],
      ciBuildPass: true,
      testCoverage: 90,
      diffLines: 100,
      hasConflict: false,
    });
    const genericSkill = mockSkill({});

    skillLoader = {
      load: vi.fn().mockImplementation(async (id: string) => {
        if (id === "skill:code-review") return crSkill;
        if (id === "skill:deploy") return deploySkill;
        return genericSkill;
      }),
    };

    const executor = new PipelineExecutor(store, skillLoader, humanLoop, credentialGuard);
    await executor.run("pl-test");

    expect(pipeline.status).toBe("waiting_merge");

    const allStagesComplete = pipeline.stages.every(s => s.status === "completed");
    expect(allStagesComplete).toBe(true);
  });

  it("regresses from cr to dev when cr score is below threshold", async () => {
    const crSkill = mockSkill({ crScore: "C", securityPass: true, findings: ["finding1"] });
    const goodCrSkill = mockSkill({ crScore: "A", securityPass: true, findings: [] });
    const deploySkill = mockSkill({
      mrUrls: ["https://coding.jd.com/group/project/-/merge_requests/1"],
      ciBuildPass: true,
      testCoverage: 90,
      diffLines: 100,
      hasConflict: false,
    });
    const genericSkill = mockSkill({});

    let crCallCount = 0;
    skillLoader = {
      load: vi.fn().mockImplementation(async (id: string) => {
        if (id === "skill:code-review") {
          crCallCount++;
          return crCallCount === 1 ? crSkill : goodCrSkill;
        }
        if (id === "skill:deploy") return deploySkill;
        return genericSkill;
      }),
    };

    const executor = new PipelineExecutor(store, skillLoader, humanLoop, credentialGuard);
    await executor.run("pl-test");

    expect(pipeline.hasRegressed).toBe(true);
    expect(pipeline.status).toBe("waiting_merge");
  });

  it("blocks pipeline when cr score fails twice", async () => {
    const crSkill = mockSkill({ crScore: "C", securityPass: true, findings: ["finding1"] });
    const genericSkill = mockSkill({});

    skillLoader = {
      load: vi.fn().mockImplementation(async (id: string) => {
        if (id === "skill:code-review") return crSkill;
        return genericSkill;
      }),
    };

    const executor = new PipelineExecutor(store, skillLoader, humanLoop, credentialGuard);
    await executor.run("pl-test");

    expect(pipeline.hasRegressed).toBe(true);
    expect(pipeline.status).toBe("blocked");
  });

  it("handles skill error with retry", async () => {
    let devCallCount = 0;
    const failingSkill: ISkill = {
      metadata: { id: "test", name: "test", description: "test", version: "1.0" },
      execute: vi.fn().mockImplementation(async (_input: SkillInput, _ctx: SkillContext) => {
        devCallCount++;
        if (devCallCount <= 2) throw new Error("compilation failed");
        return { output: {} };
      }),
    };
    const crSkill = mockSkill({ crScore: "A", securityPass: true });
    const deploySkill = mockSkill({
      mrUrls: ["https://coding.jd.com/g/p/-/merge_requests/1"],
      ciBuildPass: true,
      testCoverage: 90,
      diffLines: 100,
      hasConflict: false,
    });

    skillLoader = {
      load: vi.fn().mockImplementation(async (id: string) => {
        if (id === "skill:gen-code") return failingSkill;
        if (id === "skill:code-review") return crSkill;
        if (id === "skill:deploy") return deploySkill;
        return mockSkill({});
      }),
    };

    const executor = new PipelineExecutor(store, skillLoader, humanLoop, credentialGuard);
    await executor.run("pl-test");

    const devStage = pipeline.stages.find(s => s.name === "dev")!;
    expect(devStage.status).toBe("completed");
    expect(devStage.retryCount).toBe(2);
    expect(pipeline.status).toBe("waiting_merge");
  });

  it("fails pipeline when retries exhausted", async () => {
    const failingSkill: ISkill = {
      metadata: { id: "test", name: "test", description: "test", version: "1.0" },
      execute: vi.fn().mockRejectedValue(new Error("compilation failed")),
    };

    skillLoader = {
      load: vi.fn().mockImplementation(async (id: string) => {
        if (id === "skill:gen-code") return failingSkill;
        return mockSkill({});
      }),
    };

    const executor = new PipelineExecutor(store, skillLoader, humanLoop, credentialGuard);
    await executor.run("pl-test");

    const devStage = pipeline.stages.find(s => s.name === "dev")!;
    expect(devStage.retryCount).toBe(3);
    // After 3 retries (compilation_error autoRetry=3), executor's own check
    // finds retryCount(3) >= maxRetries(3) and transitions pipeline to failed
    expect(pipeline.status).toBe("failed");
  });

  it("blocks on deploy gate blocking failure", async () => {
    const crSkill = mockSkill({ crScore: "A", securityPass: true });
    const deploySkill = mockSkill({
      mrUrls: ["https://coding.jd.com/g/p/-/merge_requests/1"],
      ciBuildPass: false,
      testCoverage: 50,
      diffLines: 100,
      hasConflict: false,
    });

    skillLoader = {
      load: vi.fn().mockImplementation(async (id: string) => {
        if (id === "skill:code-review") return crSkill;
        if (id === "skill:deploy") return deploySkill;
        return mockSkill({});
      }),
    };

    const executor = new PipelineExecutor(store, skillLoader, humanLoop, credentialGuard);
    await executor.run("pl-test");

    expect(pipeline.status).toBe("blocked");
  });

  it("appends warn failures to deploy output without blocking", async () => {
    const crSkill = mockSkill({ crScore: "A", securityPass: true });
    const deploySkill = mockSkill({
      mrUrls: ["https://coding.jd.com/g/p/-/merge_requests/1"],
      ciBuildPass: true,
      testCoverage: 90,
      diffLines: 600,
      hasConflict: false,
    });

    skillLoader = {
      load: vi.fn().mockImplementation(async (id: string) => {
        if (id === "skill:code-review") return crSkill;
        if (id === "skill:deploy") return deploySkill;
        return mockSkill({});
      }),
    };

    const executor = new PipelineExecutor(store, skillLoader, humanLoop, credentialGuard);
    await executor.run("pl-test");

    expect(pipeline.status).toBe("waiting_merge");
    const deployStage = pipeline.stages.find(s => s.name === "deploy")!;
    expect(deployStage.output?.["gateWarnings"]).toBeDefined();
  });
});
