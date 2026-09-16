import type { ISkill, SkillContext, SkillInput, SkillOutput, CapabilityMetadata } from "@poria/core";
import { isFixtureMode } from "../fixture.js";

const metadata: CapabilityMetadata = {
  id: "skill:init",
  name: "Init",
  description: "Parse demand link and export PRD from JoySpace",
  version: "0.1.0",
};

const FIXTURE_OUTPUT: SkillOutput = {
  output: {
    projectDir: "/tmp/poria-fixture/project",
    prdPath: "/tmp/poria-fixture/project/source/PRD.md",
    prdTitle: "Fixture PRD",
    demandMetadata: {
      demandId: 1,
      demandCode: "TEST",
      name: "Fixture Demand",
      prdUrl: "http://fixture",
      attachments: [],
      rawLink: "http://fixture",
    },
  },
};

export class InitSkill implements ISkill {
  readonly metadata = metadata;

  async execute(_input: SkillInput, _context: SkillContext): Promise<SkillOutput> {
    if (isFixtureMode()) return FIXTURE_OUTPUT;
    throw new Error("InitSkill: real implementation not yet available");
  }
}
