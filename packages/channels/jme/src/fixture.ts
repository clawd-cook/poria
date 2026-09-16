/**
 * Fixture mode for JME channel (PORIA_JME_FIXTURE=1).
 * Provides offline mock implementations for testing.
 */

export function isJmeFixtureMode(): boolean {
  return process.env.PORIA_JME_FIXTURE === "1";
}

const sentMessages: Array<{ target: string; message: string; timestamp: Date }> = [];

export async function sendFixture(target: string, message: string): Promise<string> {
  sentMessages.push({ target, message, timestamp: new Date() });
  return `[fixture] Message sent to ${target}: ${message.slice(0, 100)}`;
}

export async function readRepliesFixture(
  _chatName: string,
  _since: Date,
): Promise<string[]> {
  // In fixture mode, return an empty reply list by default
  return [];
}

export async function ensureGatewayAliveFixture(): Promise<boolean> {
  // Always alive in fixture mode
  return true;
}

/** Get all messages sent in fixture mode (useful for testing). */
export function getFixtureSentMessages(): ReadonlyArray<{
  target: string;
  message: string;
  timestamp: Date;
}> {
  return sentMessages;
}

/** Clear fixture message history (useful for test cleanup). */
export function clearFixtureSentMessages(): void {
  sentMessages.length = 0;
}
