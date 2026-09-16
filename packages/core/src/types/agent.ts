export interface AgentTaskInput {
  prompt: string;
  worktreePath: string;
  systemPrompt?: string;
  model?: string;
  maxBudgetUsd?: number;
  maxTurns?: number;
  timeoutMs?: number;
  extraTools?: string[];
  onProgress?: (msg: unknown) => Promise<void>;
}

export interface AgentTaskResult {
  success: boolean;
  result?: string;
  error?: string;
  sessionId?: string;
  costUsd?: number;
  messages: unknown[];
}
