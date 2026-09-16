import type { CapabilityMetadata } from "./channel.js";

export interface ICommand<TArgs = unknown, TResult = unknown> {
  readonly metadata: CapabilityMetadata;
  execute(args: TArgs, context: CommandContext): Promise<TResult>;
}

export interface CommandContext {
  operator: string;
}
