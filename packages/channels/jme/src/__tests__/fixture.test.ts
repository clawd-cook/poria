import { describe, it, expect, beforeEach } from "vitest";
import {
  sendFixture,
  readRepliesFixture,
  ensureGatewayAliveFixture,
  getFixtureSentMessages,
  clearFixtureSentMessages,
} from "../fixture.js";

describe("JME fixture mode", () => {
  beforeEach(() => {
    clearFixtureSentMessages();
  });

  it("sendFixture records message and returns confirmation", async () => {
    const result = await sendFixture("user1", "Hello world");
    expect(result).toContain("fixture");
    expect(result).toContain("user1");

    const messages = getFixtureSentMessages();
    expect(messages.length).toBe(1);
    expect(messages[0]!.target).toBe("user1");
    expect(messages[0]!.message).toBe("Hello world");
  });

  it("sendFixture accumulates messages", async () => {
    await sendFixture("user1", "msg1");
    await sendFixture("user2", "msg2");
    expect(getFixtureSentMessages().length).toBe(2);
  });

  it("readRepliesFixture returns empty array", async () => {
    const replies = await readRepliesFixture("chat1", new Date());
    expect(replies).toEqual([]);
  });

  it("ensureGatewayAliveFixture returns true", async () => {
    const alive = await ensureGatewayAliveFixture();
    expect(alive).toBe(true);
  });

  it("clearFixtureSentMessages clears history", async () => {
    await sendFixture("user1", "msg1");
    expect(getFixtureSentMessages().length).toBe(1);
    clearFixtureSentMessages();
    expect(getFixtureSentMessages().length).toBe(0);
  });
});
