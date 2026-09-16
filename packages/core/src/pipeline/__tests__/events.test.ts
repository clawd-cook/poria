import { describe, it, expect } from "vitest";
import {
  pipelineCreatedEvent,
  pipelineStartedEvent,
  pipelineCompletedEvent,
  pipelineFailedEvent,
  pipelineCancelledEvent,
  pipelineWaitingMergeEvent,
  stageStartedEvent,
  stageCompletedEvent,
  stageFailedEvent,
  stageBlockedEvent,
  stageResumedEvent,
  stageRegressedEvent,
  agentDispatchedEvent,
  agentProgressEvent,
  agentCompletedEvent,
  agentFailedEvent,
  gateEvaluatedEvent,
  gateRegressTriggeredEvent,
  humanAssistRequestedEvent,
  humanAssistReceivedEvent,
  humanAssistEscalatedEvent,
  credentialRefreshedEvent,
  credentialExpiredEvent,
  gitCommitEvent,
  gitPushEvent,
  mrCreatedEvent,
  mrMergedEvent,
  worktreeCreatedEvent,
  worktreeCleanedEvent,
  rollbackExecutedEvent,
} from "../events.js";
import { IssueClass } from "../../types/issue.js";
import type { DemandMetadata } from "../../types/demand.js";

const PID = "pl-20260916-test1234";

const demandRef: DemandMetadata = {
  demandId: 123,
  demandCode: "TEST01",
  name: "test demand",
  prdUrl: "https://example.com/prd",
  attachments: [],
  rawLink: "https://xingyun.jd.com/demands/view/TEST01/-1?demandId=123",
};

describe("event factory functions", () => {
  it("creates all 27 event types with correct kind", () => {
    const events = [
      pipelineCreatedEvent(PID, demandRef),
      pipelineStartedEvent(PID),
      pipelineCompletedEvent(PID),
      pipelineFailedEvent(PID, "timeout"),
      pipelineCancelledEvent(PID, "user1"),
      pipelineWaitingMergeEvent(PID, ["https://mr/1"]),
      stageStartedEvent(PID, "init"),
      stageCompletedEvent(PID, "init", { prdPath: "/tmp/prd.md" }),
      stageFailedEvent(PID, "dev", "compile error", 1),
      stageBlockedEvent(PID, "cr", IssueClass.LOW_CR_SCORE),
      stageResumedEvent(PID, "cr", "user fixed"),
      stageRegressedEvent(PID, "cr", "dev", "CR score too low"),
      agentDispatchedEvent(PID, "dev", "sess-001"),
      agentProgressEvent(PID, "dev", "writing code..."),
      agentCompletedEvent(PID, "dev", 2.5),
      agentFailedEvent(PID, "dev", "timeout"),
      gateEvaluatedEvent(PID, "deploy", []),
      gateRegressTriggeredEvent(PID, "cr_score", "cr", "dev"),
      humanAssistRequestedEvent(PID, IssueClass.MERGE_CONFLICT, "dev1"),
      humanAssistReceivedEvent(PID, "resume", "fixed"),
      humanAssistEscalatedEvent(PID, 2),
      credentialRefreshedEvent(PID),
      credentialExpiredEvent(PID, "dev"),
      gitCommitEvent(PID, "repo-a", "abc123"),
      gitPushEvent(PID, "repo-a", "feature_TEST01"),
      mrCreatedEvent(PID, "repo-a", "https://mr/1", 42),
      mrMergedEvent(PID, "repo-a", "https://mr/1"),
      worktreeCreatedEvent(PID, "repo-a", "/tmp/wt"),
      worktreeCleanedEvent(PID, "repo-a", "recovery"),
      rollbackExecutedEvent(PID, "delete_branch", "feature_TEST01"),
    ];

    expect(events).toHaveLength(30);

    const kinds = new Set(events.map(e => e.kind));
    expect(kinds.size).toBe(30);

    for (const event of events) {
      expect(event.pipelineId).toBe(PID);
      expect(event.timestamp).toBeInstanceOf(Date);
    }
  });

  it("pipeline_created includes demandRef", () => {
    const e = pipelineCreatedEvent(PID, demandRef);
    expect(e.kind).toBe("pipeline_created");
    expect(e.demandRef.demandCode).toBe("TEST01");
  });

  it("stage_failed includes retry count", () => {
    const e = stageFailedEvent(PID, "dev", "compile error", 3);
    expect(e.kind).toBe("stage_failed");
    expect(e.retryCount).toBe(3);
  });

  it("mr_created includes iid", () => {
    const e = mrCreatedEvent(PID, "repo-a", "https://mr/1", 42);
    expect(e.kind).toBe("mr_created");
    expect(e.iid).toBe(42);
  });
});
