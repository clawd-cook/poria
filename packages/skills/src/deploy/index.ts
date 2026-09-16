import type { ISkill, SkillContext, SkillInput, SkillOutput, CapabilityMetadata } from "@poria/core";
import { isFixtureMode } from "../fixture.js";

const metadata: CapabilityMetadata = {
  id: "skill:deploy",
  name: "Deploy",
  description: "Build, push, and create idempotent merge request",
  version: "0.1.0",
};

const FIXTURE_OUTPUT: SkillOutput = {
  output: {
    mrUrl: "https://coding.jd.com/group/repo/-/merge_requests/1",
    mrIid: 1,
    repo: "main",
  },
};

export class DeploySkill implements ISkill {
  readonly metadata = metadata;

  async execute(_input: SkillInput, _context: SkillContext): Promise<SkillOutput> {
    if (isFixtureMode()) return FIXTURE_OUTPUT;
    throw new Error("DeploySkill: real implementation not yet available");
  }
}
