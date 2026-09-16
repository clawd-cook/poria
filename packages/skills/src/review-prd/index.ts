import type { ISkill, SkillContext, SkillInput, SkillOutput, CapabilityMetadata } from "@poria/core";
import { isFixtureMode } from "../fixture.js";

const metadata: CapabilityMetadata = {
  id: "skill:review-prd",
  name: "ReviewPRD",
  description: "Analyze PRD and produce PRD_REVIEW.md with P0/P1/P2 questions",
  version: "0.1.0",
};

const FIXTURE_OUTPUT: SkillOutput = {
  output: {
    reviewPath: "/tmp/poria-fixture/project/source/PRD_REVIEW.md",
    p0Answered: true,
    p1Answered: false,
    p2Answered: false,
  },
};

export class ReviewPrdSkill implements ISkill {
  readonly metadata = metadata;

  async execute(_input: SkillInput, _context: SkillContext): Promise<SkillOutput> {
    if (isFixtureMode()) return FIXTURE_OUTPUT;
    throw new Error("ReviewPrdSkill: real implementation not yet available");
  }
}
