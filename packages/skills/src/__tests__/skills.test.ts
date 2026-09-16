import { describe, it, expect, beforeAll, afterAll } from "vitest";
import type { ISkill, SkillInput, SkillContext } from "@poria/core";
import { STAGE_ORDER } from "@poria/core";
import { InitSkill } from "../init/index.js";
import { ReviewPrdSkill } from "../review-prd/index.js";
import { GenTrdSkill } from "../gen-trd/index.js";
import { WorkspaceSkill } from "../workspace/index.js";
import { GenCodeSkill } from "../gen-code/index.js";
import { CodeReviewSkill } from "../code-review/index.js";
import { DeploySkill } from "../deploy/index.js";
import { HumanLoopCoordinator, parseHumanReply } from "../human-loop/index.js";
import { STAGE_SKILL_MAP } from "../stage-skill-map.js";

const stubInput: SkillInput = {
  stage: {
    pipelineId: "pipe-001",
    name: "init",
    status: "running",
    retryCount: 0,
    maxRetries: 3,
  },
  pipeline: {
    id: "pipe-001",
    demandId: 1,
    demandCode: "TEST",
    status: "running",
    rawLink: "http://fixture",
    operator: "tester",
    hasRegressed: false,
    config: { gates: [], trdScope: [], repos: [] },
    stages: [],
    repos: [],
    createdAt: new Date(),
    updatedAt: new Date(),
  },
};

const stubContext: SkillContext = {
  pipelineId: "pipe-001",
  workdir: "/tmp/poria-fixture",
  credentials: null,
};

describe("Skills fixture mode", () => {
  let originalEnv: string | undefined;

  beforeAll(() => {
    originalEnv = process.env.PORIA_PIPELINE_FIXTURE;
    process.env.PORIA_PIPELINE_FIXTURE = "1";
  });

  afterAll(() => {
    if (originalEnv === undefined) {
      delete process.env.PORIA_PIPELINE_FIXTURE;
    } else {
      process.env.PORIA_PIPELINE_FIXTURE = originalEnv;
    }
  });

  const skills: Array<{ name: string; skill: ISkill; expectedKey: string }> = [
    { name: "InitSkill", skill: new InitSkill(), expectedKey: "projectDir" },
    { name: "ReviewPrdSkill", skill: new ReviewPrdSkill(), expectedKey: "reviewPath" },
    { name: "GenTrdSkill", skill: new GenTrdSkill(), expectedKey: "trdPath" },
    { name: "WorkspaceSkill", skill: new WorkspaceSkill(), expectedKey: "repos" },
    { name: "GenCodeSkill", skill: new GenCodeSkill(), expectedKey: "changedFiles" },
    { name: "CodeReviewSkill", skill: new CodeReviewSkill(), expectedKey: "crReportPath" },
    { name: "DeploySkill", skill: new DeploySkill(), expectedKey: "mrUrl" },
  ];

  for (const { name, skill, expectedKey } of skills) {
    it(`${name} returns fixture output with '${expectedKey}'`, async () => {
      const result = await skill.execute(stubInput, stubContext);
      expect(result.output).toBeDefined();
      expect(result.output).toHaveProperty(expectedKey);
    });

    it(`${name} has correct metadata.id prefix`, () => {
      expect(skill.metadata.id).toMatch(/^skill:/);
    });
  }

  it("InitSkill fixture contains demandMetadata", async () => {
    const result = await new InitSkill().execute(stubInput, stubContext);
    const meta = result.output.demandMetadata as Record<string, unknown>;
    expect(meta.demandId).toBe(1);
    expect(meta.demandCode).toBe("TEST");
  });

  it("CodeReviewSkill fixture crScore is A", async () => {
    const result = await new CodeReviewSkill().execute(stubInput, stubContext);
    expect(result.output.crScore).toBe("A");
  });

  it("DeploySkill fixture has mrUrl and mrIid", async () => {
    const result = await new DeploySkill().execute(stubInput, stubContext);
    expect(result.output.mrUrl).toContain("merge_requests");
    expect(result.output.mrIid).toBe(1);
  });
});

describe("Skills without fixture mode throw", () => {
  let originalEnv: string | undefined;

  beforeAll(() => {
    originalEnv = process.env.PORIA_PIPELINE_FIXTURE;
    delete process.env.PORIA_PIPELINE_FIXTURE;
  });

  afterAll(() => {
    if (originalEnv !== undefined) {
      process.env.PORIA_PIPELINE_FIXTURE = originalEnv;
    }
  });

  it("InitSkill throws when not in fixture mode", async () => {
    await expect(new InitSkill().execute(stubInput, stubContext)).rejects.toThrow(
      "real implementation not yet available",
    );
  });
});

describe("HumanLoopCoordinator", () => {
  let originalEnv: string | undefined;

  beforeAll(() => {
    originalEnv = process.env.PORIA_PIPELINE_FIXTURE;
    process.env.PORIA_PIPELINE_FIXTURE = "1";
  });

  afterAll(() => {
    if (originalEnv === undefined) {
      delete process.env.PORIA_PIPELINE_FIXTURE;
    } else {
      process.env.PORIA_PIPELINE_FIXTURE = originalEnv;
    }
  });

  it("notify is a no-op in fixture mode", async () => {
    const coord = new HumanLoopCoordinator();
    await expect(
      coord.notify(stubInput.pipeline, stubInput.stage, "unknown"),
    ).resolves.toBeUndefined();
  });

  it("renotify is a no-op in fixture mode", async () => {
    const coord = new HumanLoopCoordinator();
    await expect(
      coord.renotify(stubInput.pipeline, stubInput.stage),
    ).resolves.toBeUndefined();
  });

  it("escalate is a no-op in fixture mode", async () => {
    const coord = new HumanLoopCoordinator();
    await expect(coord.escalate(stubInput.pipeline, "test")).resolves.toBeUndefined();
  });

  it("pollReply returns null in fixture mode", async () => {
    const coord = new HumanLoopCoordinator();
    const reply = await coord.pollReply(stubInput.pipeline);
    expect(reply).toBeNull();
  });
});

describe("parseHumanReply", () => {
  it("returns resume for '修复'", () => {
    const r = parseHumanReply("修复");
    expect(r).not.toBeNull();
    expect(r!.action).toBe("resume");
  });

  it("returns resume for 'fix'", () => {
    const r = parseHumanReply("fix");
    expect(r).not.toBeNull();
    expect(r!.action).toBe("resume");
  });

  it("returns resume for '已解决'", () => {
    const r = parseHumanReply("已解决了");
    expect(r).not.toBeNull();
    expect(r!.action).toBe("resume");
  });

  it("returns resume for '重试'", () => {
    const r = parseHumanReply("请重试");
    expect(r).not.toBeNull();
    expect(r!.action).toBe("resume");
  });

  it("returns skip for '跳过'", () => {
    const r = parseHumanReply("跳过");
    expect(r).not.toBeNull();
    expect(r!.action).toBe("skip");
  });

  it("returns skip for 'skip'", () => {
    const r = parseHumanReply("skip this");
    expect(r).not.toBeNull();
    expect(r!.action).toBe("skip");
  });

  it("returns cancel for '取消'", () => {
    const r = parseHumanReply("取消");
    expect(r).not.toBeNull();
    expect(r!.action).toBe("cancel");
  });

  it("returns cancel for 'cancel'", () => {
    const r = parseHumanReply("cancel");
    expect(r).not.toBeNull();
    expect(r!.action).toBe("cancel");
  });

  it("returns cancel for '终止'", () => {
    const r = parseHumanReply("终止流水线");
    expect(r).not.toBeNull();
    expect(r!.action).toBe("cancel");
  });

  it("returns null for empty string", () => {
    expect(parseHumanReply("")).toBeNull();
  });

  it("returns null for whitespace only", () => {
    expect(parseHumanReply("   ")).toBeNull();
  });

  it("returns null for unrecognized text", () => {
    expect(parseHumanReply("I need more time")).toBeNull();
  });

  it("prioritizes cancel over resume", () => {
    const r = parseHumanReply("取消修复");
    expect(r).not.toBeNull();
    expect(r!.action).toBe("cancel");
  });

  it("preserves rawMessage", () => {
    const r = parseHumanReply("  fix  ");
    expect(r).not.toBeNull();
    expect(r!.rawMessage).toBe("fix");
  });
});

describe("STAGE_SKILL_MAP", () => {
  it("has entries for all 7 pipeline stages", () => {
    for (const stage of STAGE_ORDER) {
      expect(STAGE_SKILL_MAP).toHaveProperty(stage);
      expect(typeof STAGE_SKILL_MAP[stage]).toBe("string");
    }
  });

  it("maps init to skill:init", () => {
    expect(STAGE_SKILL_MAP.init).toBe("skill:init");
  });

  it("maps review_prd to skill:review-prd", () => {
    expect(STAGE_SKILL_MAP.review_prd).toBe("skill:review-prd");
  });

  it("maps design to skill:gen-trd", () => {
    expect(STAGE_SKILL_MAP.design).toBe("skill:gen-trd");
  });

  it("maps workspace to skill:workspace", () => {
    expect(STAGE_SKILL_MAP.workspace).toBe("skill:workspace");
  });

  it("maps dev to skill:gen-code", () => {
    expect(STAGE_SKILL_MAP.dev).toBe("skill:gen-code");
  });

  it("maps cr to skill:code-review", () => {
    expect(STAGE_SKILL_MAP.cr).toBe("skill:code-review");
  });

  it("maps deploy to skill:deploy", () => {
    expect(STAGE_SKILL_MAP.deploy).toBe("skill:deploy");
  });

  it("every value starts with skill:", () => {
    for (const val of Object.values(STAGE_SKILL_MAP)) {
      expect(val).toMatch(/^skill:/);
    }
  });
});
