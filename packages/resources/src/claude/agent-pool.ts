import type { AgentTaskInput, AgentTaskResult, StageEnum } from "@poria/core";

// ---------- Stage Agent Configuration ----------

export interface StageAgentConfig {
  allowedTools: string[];
  maxBudgetUsd: number;
  maxTurns: number;
  timeoutMs: number;
}

export const STAGE_AGENT_CONFIG: Record<string, StageAgentConfig> = {
  review_prd: {
    allowedTools: ["Read", "Grep"],
    maxBudgetUsd: 1,
    maxTurns: 10,
    timeoutMs: 5 * 60_000, // 5 minutes
  },
  design: {
    allowedTools: ["Read", "Edit", "Grep"],
    maxBudgetUsd: 3,
    maxTurns: 20,
    timeoutMs: 10 * 60_000, // 10 minutes
  },
  dev: {
    allowedTools: ["Read", "Edit", "Bash", "Glob", "Grep"],
    maxBudgetUsd: 10,
    maxTurns: 100,
    timeoutMs: 30 * 60_000, // 30 minutes
  },
  cr: {
    allowedTools: ["Read", "Bash", "Grep"],
    maxBudgetUsd: 5,
    maxTurns: 30,
    timeoutMs: 15 * 60_000, // 15 minutes
  },
} as const satisfies Partial<Record<StageEnum, StageAgentConfig>>;

// ---------- SDK Interface (mock-friendly) ----------

/**
 * Represents the Agent SDK startup result.
 * The real SDK is @anthropic-ai/claude-agent-sdk — we define a thin interface
 * so the pool can work with either the real SDK or a mock.
 */
export interface IAgentSDK {
  query(
    prompt: string,
    options: AgentQueryOptions,
  ): AsyncIterable<SDKMessage> & { sessionId?: string };
}

export interface AgentQueryOptions {
  cwd?: string;
  maxBudgetUsd?: number;
  maxTurns?: number;
  abortController?: AbortController;
  model?: string;
  appendSystemPrompt?: string;
  allowedTools?: string[];
}

export interface SDKMessage {
  type: string;
  message?: {
    content?: unknown;
  };
  result?: string;
  total_cost_usd?: number;
}

// ---------- Constants ----------

const DEFAULT_TIMEOUT_MS = 30 * 60_000; // 30 minutes
const IDLE_TIMEOUT_MS = 5 * 60_000; // 5 minutes no messages -> kill
const IDLE_CHECK_INTERVAL_MS = 30_000; // Check idle every 30s

// ---------- Claude Agent Pool ----------

export class ClaudeAgentPool {
  private sdk: IAgentSDK | null = null;

  /**
   * Initialize the agent pool.
   * Accepts an IAgentSDK instance (real SDK or mock).
   * In production, the caller loads the real SDK via dynamic import and passes it.
   */
  async initialize(sdk: IAgentSDK): Promise<void> {
    this.sdk = sdk;
  }

  /**
   * Dispatch a single Agent task.
   *
   * - Total timeout via AbortController (default 30min).
   * - Idle timeout: 5min with no messages -> abort (Design F-08).
   * - Idle checker interval cleaned in finally (Design section 16 F-08).
   * - Progress callback for each assistant message.
   * - Returns sessionId for resume support.
   */
  async dispatch(input: AgentTaskInput): Promise<AgentTaskResult> {
    if (!this.sdk) {
      throw new Error("AgentPool not initialized — call initialize() first");
    }

    const totalTimeoutMs = input.timeoutMs ?? DEFAULT_TIMEOUT_MS;
    const controller = new AbortController();
    const totalTimer = setTimeout(() => controller.abort(), totalTimeoutMs);

    // Idle timeout tracking
    let lastMessageAt = Date.now();
    const idleChecker = setInterval(() => {
      if (Date.now() - lastMessageAt > IDLE_TIMEOUT_MS) {
        controller.abort();
      }
    }, IDLE_CHECK_INTERVAL_MS);

    const messages: SDKMessage[] = [];
    let sessionId: string | undefined;

    try {
      const queryIterable = this.sdk.query(input.prompt, {
        cwd: input.worktreePath,
        maxBudgetUsd: input.maxBudgetUsd ?? 5.0,
        maxTurns: input.maxTurns ?? 50,
        abortController: controller,
        model: input.model ?? "sonnet",
        appendSystemPrompt: input.systemPrompt,
        allowedTools: [
          ...(input.extraTools ?? []),
        ],
      });

      // Capture sessionId from the iterable if available
      sessionId = queryIterable.sessionId;

      for await (const msg of queryIterable) {
        lastMessageAt = Date.now();
        messages.push(msg);

        // Report progress for assistant messages
        if (msg.type === "assistant" && msg.message?.content && input.onProgress) {
          await input.onProgress(msg);
        }
      }

      // Extract final result from last result-type message
      const resultMsg = findLast(messages, (m) => m.type === "result");
      return {
        success: true,
        result: resultMsg?.result ?? "",
        sessionId,
        costUsd: resultMsg?.total_cost_usd,
        messages,
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error),
        sessionId,
        messages,
      };
    } finally {
      clearTimeout(totalTimer);
      clearInterval(idleChecker);
    }
  }
}

// ---------- Helpers ----------

/** Array.findLast polyfill for older runtimes (though Node 24 has it). */
function findLast<T>(arr: T[], predicate: (item: T) => boolean): T | undefined {
  for (let i = arr.length - 1; i >= 0; i--) {
    const item = arr[i];
    if (item !== undefined && predicate(item)) {
      return item;
    }
  }
  return undefined;
}
