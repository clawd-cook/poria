import type { ISkill, SkillContext, SkillInput, SkillOutput, CapabilityMetadata } from "@poria/core";
import { isFixtureMode } from "../fixture.js";

const metadata: CapabilityMetadata = {
  id: "skill:code-review",
  name: "CodeReview",
  description: "Agent-driven code review with security scan",
  version: "0.1.0",
};

const FIXTURE_OUTPUT: SkillOutput = {
  output: {
    crReportPath: "/tmp/poria-fixture/project/CR_REPORT.md",
    crScore: "A",
    findings: [],
    securityScanPassed: true,
  },
};

export class CodeReviewSkill implements ISkill {
  readonly metadata = metadata;

  async execute(_input: SkillInput, _context: SkillContext): Promise<SkillOutput> {
    if (isFixtureMode()) return FIXTURE_OUTPUT;
    throw new Error("CodeReviewSkill: real implementation not yet available");
  }
}
