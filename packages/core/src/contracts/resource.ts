import type { CapabilityMetadata } from "./channel.js";

export interface IResource<TInput = unknown, TOutput = unknown> {
  readonly metadata: CapabilityMetadata;
  execute(input: TInput, context: ResourceContext): Promise<TOutput>;
}

export interface ResourceContext {
  pipelineId?: string;
  workdir?: string;
}
