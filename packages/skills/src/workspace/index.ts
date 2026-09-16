import type { ISkill, SkillContext, SkillInput, SkillOutput, CapabilityMetadata } from "@poria/core";
import { isFixtureMode } from "../fixture.js";

const metadata: CapabilityMetadata = {
  id: "skill:workspace",
  name: "Workspace",
  description: "Create git worktree and bind Xingyun branch",
  version: "0.1.0",
};

const FIXTURE_OUTPUT: SkillOutput = {
  output: {
    repos: [
      {
        name: "main",
        branch: "feature_TEST",
        baseBranch: "master",
        changeId: "chg-fixture-001",
        worktreePath: "/tmp/poria-fixture/worktree",
        gitlabProjectPath: "group/repo",
      },
    ],
  },
};

export class WorkspaceSkill implements ISkill {
  readonly metadata = metadata;

  async execute(_input: SkillInput, _context: SkillContext): Promise<SkillOutput> {
    if (isFixtureMode()) return FIXTURE_OUTPUT;
    throw new Error("WorkspaceSkill: real implementation not yet available");
  }
}
