import type { ISkill, SkillContext, SkillInput, SkillOutput, CapabilityMetadata } from "@poria/core";
import { isFixtureMode } from "../fixture.js";

const metadata: CapabilityMetadata = {
  id: "skill:gen-code",
  name: "GenCode",
  description: "Agent-driven code generation with OutputGuard validation",
  version: "0.1.0",
};

const FIXTURE_OUTPUT: SkillOutput = {
  output: {
    changedFiles: ["src/index.ts"],
    totalDiffLines: 50,
    agentSessionId: "session-fixture-001",
  },
};

export class GenCodeSkill implements ISkill {
  readonly metadata = metadata;

  async execute(_input: SkillInput, _context: SkillContext): Promise<SkillOutput> {
    if (isFixtureMode()) return FIXTURE_OUTPUT;
    throw new Error("GenCodeSkill: real implementation not yet available");
  }
}
