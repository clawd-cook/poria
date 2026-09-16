import type { ISkill, SkillContext, SkillInput, SkillOutput, CapabilityMetadata } from "@poria/core";
import { isFixtureMode } from "../fixture.js";

const metadata: CapabilityMetadata = {
  id: "skill:gen-trd",
  name: "GenTRD",
  description: "Generate TRD.md and extract allowed modification scope",
  version: "0.1.0",
};

const FIXTURE_OUTPUT: SkillOutput = {
  output: {
    trdPath: "/tmp/poria-fixture/project/source/TRD.md",
    trdScope: ["src/**/*.ts", "package.json"],
  },
};

export class GenTrdSkill implements ISkill {
  readonly metadata = metadata;

  async execute(_input: SkillInput, _context: SkillContext): Promise<SkillOutput> {
    if (isFixtureMode()) return FIXTURE_OUTPUT;
    throw new Error("GenTrdSkill: real implementation not yet available");
  }
}
