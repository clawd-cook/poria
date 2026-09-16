import type { IChannel, CapabilityMetadata, ChannelContext } from "@poria/core";
import {
  runAgent,
  parseAgentOutput,
  ensureGatewayAlive,
  type JoyClawBridgeConfig,
} from "./joyclaw-bridge.js";
import {
  isJmeFixtureMode,
  sendFixture,
  readRepliesFixture,
  ensureGatewayAliveFixture,
} from "./fixture.js";

export interface JmeChannelInput {
  action: "send" | "readReplies" | "ensureGatewayAlive";
  send?: { target: string; message: string; timeoutSec?: number };
  readReplies?: { chatName: string; since: Date | string };
  bridgeConfig?: JoyClawBridgeConfig;
}

export type JmeChannelOutput =
  | { action: "send"; result: string }
  | { action: "readReplies"; replies: string[] }
  | { action: "ensureGatewayAlive"; alive: boolean };

const metadata: CapabilityMetadata = {
  id: "channel:jme",
  name: "JME",
  description: "JingME messaging via JoyClaw bridge",
  version: "0.1.0",
};

class JmeChannelImpl implements IChannel<JmeChannelInput, JmeChannelOutput> {
  readonly metadata = metadata;

  async execute(input: JmeChannelInput, _context: ChannelContext): Promise<JmeChannelOutput> {
    switch (input.action) {
      case "send":
        return this.send(input);
      case "readReplies":
        return this.readReplies(input);
      case "ensureGatewayAlive":
        return this.checkGateway(input);
      default:
        throw new Error(`Unknown action: ${input.action}`);
    }
  }

  private async send(input: JmeChannelInput): Promise<JmeChannelOutput> {
    const { target, message, timeoutSec } = input.send!;

    if (isJmeFixtureMode()) {
      const result = await sendFixture(target, message);
      return { action: "send", result };
    }

    const agentMessage = `向 ${target} 发送京ME消息: ${message}`;
    const stdout = await runAgent(agentMessage, timeoutSec ?? 120, input.bridgeConfig);
    const result = parseAgentOutput(stdout);
    return { action: "send", result };
  }

  private async readReplies(input: JmeChannelInput): Promise<JmeChannelOutput> {
    const { chatName, since } = input.readReplies!;
    const sinceDate = typeof since === "string" ? new Date(since) : since;

    if (isJmeFixtureMode()) {
      const replies = await readRepliesFixture(chatName, sinceDate);
      return { action: "readReplies", replies };
    }

    const agentMessage = `查看 ${chatName} 聊天中 ${sinceDate.toISOString()} 之后的消息`;
    const stdout = await runAgent(agentMessage, 300, input.bridgeConfig);
    const result = parseAgentOutput(stdout);
    // Split result into individual messages (best effort)
    const replies = result
      .split("\n")
      .map((line) => line.trim())
      .filter(Boolean);
    return { action: "readReplies", replies };
  }

  private async checkGateway(input: JmeChannelInput): Promise<JmeChannelOutput> {
    if (isJmeFixtureMode()) {
      const alive = await ensureGatewayAliveFixture();
      return { action: "ensureGatewayAlive", alive };
    }
    const alive = await ensureGatewayAlive(input.bridgeConfig);
    return { action: "ensureGatewayAlive", alive };
  }
}

export function createJmeChannel(): JmeChannelImpl {
  return new JmeChannelImpl();
}

// Re-exports
export { parseReply } from "./reply-parser.js";
export type { ReplyAction } from "./reply-parser.js";
export {
  isJmeFixtureMode,
  sendFixture,
  readRepliesFixture,
  ensureGatewayAliveFixture,
  getFixtureSentMessages,
  clearFixtureSentMessages,
} from "./fixture.js";
export {
  runAgent,
  parseAgentOutput,
  ensureGatewayAlive,
} from "./joyclaw-bridge.js";
export type { JoyClawBridgeConfig } from "./joyclaw-bridge.js";
