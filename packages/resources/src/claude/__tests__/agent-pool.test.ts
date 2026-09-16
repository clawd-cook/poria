import { describe, it, expect, vi, beforeEach } from "vitest";
import { ClaudeAgentPool } from "../agent-pool.js";
import type { IAgentSDK, SDKMessage, AgentQueryOptions } from "../agent-pool.js";

// Helper: create an async iterable from an array of messages
function createMockQueryIterable(
  messages: SDKMessage[],
  options?: { sessionId?: string },
): AsyncIterable<SDKMessage> & { sessionId?: string } {
  return {
    sessionId: options?.sessionId,
    async *[Symbol.asyncIterator]() {
      for (const msg of messages) {
        yield msg;
      }
    },
  };
}

// Helper: create a mock SDK
function createMockSDK(
  messages: SDKMessage[],
  options?: { sessionId?: string },
): IAgentSDK {
  return {
    query: vi.fn().mockReturnValue(
      createMockQueryIterable(messages, options),
    ),
  };
}

describe("ClaudeAgentPool", () => {
  let pool: ClaudeAgentPool;

  beforeEach(() => {
    pool = new ClaudeAgentPool();
  });

  describe("initialization", () => {
    it("throws when dispatch is called before initialize", async () => {
      await expect(
        pool.dispatch({
          prompt: "test",
          worktreePath: "/tmp",
        }),
      ).rejects.toThrow("AgentPool not initialized");
    });

    it("initializes successfully with a mock SDK", async () => {
      const sdk = createMockSDK([]);
      await pool.initialize(sdk);

      // Should not throw after initialization
      const result = await pool.dispatch({
        prompt: "test",
        worktreePath: "/tmp",
      });
      expect(result.success).toBe(true);
    });
  });

  describe("dispatch", () => {
    it("returns sessionId from query iterable", async () => {
      const sdk = createMockSDK(
        [{ type: "result", result: "done", total_cost_usd: 0.5 }],
        { sessionId: "session-abc123" },
      );
      await pool.initialize(sdk);

      const result = await pool.dispatch({
        prompt: "do something",
        worktreePath: "/tmp/work",
      });

      expect(result.success).toBe(true);
      expect(result.sessionId).toBe("session-abc123");
    });

    it("returns result and cost from last result message", async () => {
      const messages: SDKMessage[] = [
        { type: "assistant", message: { content: "thinking..." } },
        { type: "result", result: "final answer", total_cost_usd: 1.23 },
      ];
      const sdk = createMockSDK(messages);
      await pool.initialize(sdk);

      const result = await pool.dispatch({
        prompt: "test",
        worktreePath: "/tmp",
      });

      expect(result.success).toBe(true);
      expect(result.result).toBe("final answer");
      expect(result.costUsd).toBe(1.23);
      expect(result.messages).toHaveLength(2);
    });

    it("calls onProgress for assistant messages", async () => {
      const onProgress = vi.fn().mockResolvedValue(undefined);
      const messages: SDKMessage[] = [
        { type: "assistant", message: { content: "step 1" } },
        { type: "tool_use" },
        { type: "assistant", message: { content: "step 2" } },
        { type: "result", result: "done" },
      ];
      const sdk = createMockSDK(messages);
      await pool.initialize(sdk);

      await pool.dispatch({
        prompt: "test",
        worktreePath: "/tmp",
        onProgress,
      });

      // Only assistant messages with content trigger onProgress
      expect(onProgress).toHaveBeenCalledTimes(2);
    });

    it("returns error result on total timeout", async () => {
      // Create a SDK whose async generator respects the AbortController
      const sdk: IAgentSDK = {
        query: vi.fn().mockImplementation((_prompt: string, opts: AgentQueryOptions) => {
          const controller = opts.abortController;
          return {
            sessionId: "sess-timeout",
            async *[Symbol.asyncIterator]() {
              yield { type: "assistant", message: { content: "starting..." } } as SDKMessage;
              // Wait until abort signal fires
              await new Promise<void>((_resolve, reject) => {
                if (controller?.signal.aborted) {
                  reject(new Error("Aborted"));
                  return;
                }
                const onAbort = () => {
                  reject(new Error("Aborted"));
                };
                controller?.signal.addEventListener("abort", onAbort, { once: true });
              });
            },
          };
        }),
      };
      await pool.initialize(sdk);

      const result = await pool.dispatch({
        prompt: "test",
        worktreePath: "/tmp",
        timeoutMs: 100, // Very short timeout for testing
      });

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
      expect(result.sessionId).toBe("sess-timeout");
      // Should have captured the first message before timeout
      expect(result.messages.length).toBeGreaterThanOrEqual(1);
    });

    it("returns error result when SDK query throws during iteration", async () => {
      const sdk: IAgentSDK = {
        query: vi.fn().mockImplementation((_prompt: string, _opts: AgentQueryOptions) => ({
          sessionId: "sess-err",
          async *[Symbol.asyncIterator]() {
            yield { type: "assistant", message: { content: "hello" } } as SDKMessage;
            throw new Error("Agent exploded");
          },
        })),
      };
      await pool.initialize(sdk);

      const result = await pool.dispatch({
        prompt: "test",
        worktreePath: "/tmp",
      });

      expect(result.success).toBe(false);
      expect(result.error).toBe("Agent exploded");
      expect(result.sessionId).toBe("sess-err");
    });

    it("handles SDK throwing synchronously", async () => {
      const sdk: IAgentSDK = {
        query: vi.fn().mockImplementation(() => {
          throw new Error("SDK crash");
        }),
      };
      await pool.initialize(sdk);

      const result = await pool.dispatch({
        prompt: "test",
        worktreePath: "/tmp",
      });

      expect(result.success).toBe(false);
      expect(result.error).toBe("SDK crash");
    });

    it("collects all messages even on success", async () => {
      const messages: SDKMessage[] = [
        { type: "assistant", message: { content: "a" } },
        { type: "tool_use" },
        { type: "tool_result" },
        { type: "assistant", message: { content: "b" } },
        { type: "result", result: "final" },
      ];
      const sdk = createMockSDK(messages);
      await pool.initialize(sdk);

      const result = await pool.dispatch({
        prompt: "test",
        worktreePath: "/tmp",
      });

      expect(result.messages).toHaveLength(5);
    });

    it("passes correct options to SDK query", async () => {
      const sdk = createMockSDK([{ type: "result", result: "ok" }]);
      await pool.initialize(sdk);

      await pool.dispatch({
        prompt: "write code",
        worktreePath: "/work/tree",
        systemPrompt: "You are an expert.",
        model: "opus",
        maxBudgetUsd: 7.5,
        maxTurns: 25,
        extraTools: ["Bash", "Glob"],
      });

      const queryFn = vi.mocked(sdk.query);
      expect(queryFn).toHaveBeenCalledTimes(1);
      expect(queryFn.mock.calls[0]![0]).toBe("write code");
      const opts = queryFn.mock.calls[0]![1]!;
      expect(opts.cwd).toBe("/work/tree");
      expect(opts.appendSystemPrompt).toBe("You are an expert.");
      expect(opts.model).toBe("opus");
      expect(opts.maxBudgetUsd).toBe(7.5);
      expect(opts.maxTurns).toBe(25);
      expect(opts.allowedTools).toContain("Bash");
      expect(opts.allowedTools).toContain("Glob");
    });

    it("returns empty result when no messages are produced", async () => {
      const sdk = createMockSDK([]);
      await pool.initialize(sdk);

      const result = await pool.dispatch({
        prompt: "test",
        worktreePath: "/tmp",
      });

      expect(result.success).toBe(true);
      expect(result.result).toBe("");
      expect(result.messages).toHaveLength(0);
    });
  });
});
