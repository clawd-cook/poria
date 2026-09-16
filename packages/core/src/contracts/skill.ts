import type { CapabilityMetadata } from "./channel.js";
import type { SkillInput, SkillOutput } from "../types/pipeline.js";

export interface ISkill {
  readonly metadata: CapabilityMetadata;
  execute(input: SkillInput, context: SkillContext): Promise<SkillOutput>;
}

export interface SkillContext {
  pipelineId: string;
  workdir: string;
  credentials: unknown;
}
