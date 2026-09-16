import type { IChannel, CapabilityMetadata, ChannelContext } from "@poria/core";
import type { JacpCredentials } from "./types.js";
import type { Logger } from "./types.js";
import {
  exportJoySpaceMarkdown,
  type ExportJoySpaceInput,
  type ExportJoySpaceResult,
} from "./export.js";

export interface JoySpaceChannelInput {
  action: "exportToMarkdown";
  url: string;
  outputDir?: string;
  outputName?: string;
  tenantCode?: string;
  logger?: Logger;
}

export interface JoySpaceChannelOutput {
  action: "exportToMarkdown";
  result: ExportJoySpaceResult;
}

const metadata: CapabilityMetadata = {
  id: "channel:joyspace",
  name: "JoySpace",
  description: "JoySpace document export to Markdown",
  version: "0.1.0",
};

class JoySpaceChannelImpl implements IChannel<JoySpaceChannelInput, JoySpaceChannelOutput> {
  readonly metadata = metadata;

  async execute(input: JoySpaceChannelInput, context: ChannelContext): Promise<JoySpaceChannelOutput> {
    if (input.action !== "exportToMarkdown") {
      throw new Error(`Unknown action: ${input.action}`);
    }

    const credentials = context.credentials as JacpCredentials | null | undefined;
    const exportInput: ExportJoySpaceInput = {
      url: input.url,
      outputDir: input.outputDir,
      outputName: input.outputName,
      tenantCode: input.tenantCode,
      credentials: credentials?.cookie ? credentials : undefined,
      logger: input.logger,
    };

    const result = await exportJoySpaceMarkdown(exportInput);
    return { action: "exportToMarkdown", result };
  }
}

export function createJoySpaceChannel(): JoySpaceChannelImpl {
  return new JoySpaceChannelImpl();
}

// Re-exports
export {
  exportJoySpaceMarkdown,
  isJoySpaceFixtureMode,
} from "./export.js";
export type { ExportJoySpaceInput, ExportJoySpaceResult } from "./export.js";
export type { JacpCredentials, Logger } from "./types.js";
