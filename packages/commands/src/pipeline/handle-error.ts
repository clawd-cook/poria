import type { Pipeline, Stage } from "@poria/core";
import { IssueClass, ISSUE_POLICIES } from "@poria/core";
import { ExceptionClassifier } from "./exception-classifier.js";

export interface IHumanLoop {
  notify(pipeline: Pipeline, stage: Stage, issueClass: string): Promise<void>;
}

export interface HandleErrorResult {
  issueClass: IssueClass;
  action: "retry" | "blocked" | "failed";
}

export async function handleStageError(
  pipeline: Pipeline,
  stage: Stage,
  error: unknown,
  humanLoop?: IHumanLoop,
): Promise<HandleErrorResult> {
  const issueClass = ExceptionClassifier.classify(error);
  const policy = ISSUE_POLICIES[issueClass];
  const message = error instanceof Error ? error.message : String(error);

  if (policy.autoRetry > 0 && stage.retryCount < policy.autoRetry) {
    stage.status = "failed";
    stage.retryCount++;
    stage.issue = { class: issueClass, message, retryable: true };
    return { issueClass, action: "retry" };
  }

  if (policy.notifyRoles.length > 0) {
    stage.status = "blocked";
    stage.issue = { class: issueClass, message, retryable: false };
    pipeline.status = "blocked";
    if (humanLoop) await humanLoop.notify(pipeline, stage, issueClass);
    return { issueClass, action: "blocked" };
  }

  stage.status = "failed";
  stage.issue = { class: issueClass, message, retryable: false };
  pipeline.status = "failed";
  return { issueClass, action: "failed" };
}
