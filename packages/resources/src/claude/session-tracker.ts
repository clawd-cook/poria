/**
 * Session Tracker — maps (pipelineId, stageName) to Agent session IDs.
 *
 * P1 implementation: in-memory Map.
 * The actual persistence is handled by the stages table `agent_session_id` column
 * in infra-persistence. This tracker serves as the calling layer's thin wrapper
 * so that the agent pool and pipeline executor have a consistent API.
 */
export class SessionTracker {
  private sessions = new Map<string, string>();

  /** Record an Agent session ID for a pipeline stage. */
  record(pipelineId: string, stageName: string, sessionId: string): void {
    const key = SessionTracker.key(pipelineId, stageName);
    this.sessions.set(key, sessionId);
  }

  /** Retrieve a previously recorded session ID, or undefined if not found. */
  get(pipelineId: string, stageName: string): string | undefined {
    const key = SessionTracker.key(pipelineId, stageName);
    return this.sessions.get(key);
  }

  /** Remove a session record. */
  remove(pipelineId: string, stageName: string): boolean {
    const key = SessionTracker.key(pipelineId, stageName);
    return this.sessions.delete(key);
  }

  /** Clear all tracked sessions. */
  clear(): void {
    this.sessions.clear();
  }

  /** Number of tracked sessions. */
  get size(): number {
    return this.sessions.size;
  }

  private static key(pipelineId: string, stageName: string): string {
    return `${pipelineId}::${stageName}`;
  }
}
