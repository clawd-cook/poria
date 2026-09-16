export interface CapabilityMetadata {
  id: string;
  name: string;
  description: string;
  version: string;
}

export interface IChannel<TInput = unknown, TOutput = unknown> {
  readonly metadata: CapabilityMetadata;
  execute(input: TInput, context: ChannelContext): Promise<TOutput>;
}

export interface ChannelContext {
  credentials: unknown;
  pipelineId?: string;
}
