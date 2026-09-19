import { issueClassKey } from "./issueClass";
import { isOutputGuardBlock } from "./outputGuard";
import { isP0UnansweredMessage, isRequirementAmbiguousIssue } from "./prdReview";
import { isQualityGateBlock } from "./qualityGates";
import { isTrdUnconfirmedIssue, isTrdUnconfirmedMessage } from "./trd";
import type { PipelineStatus } from "./types";

/** Board lanes for HITL. Not a new pipeline status — derived from existing issue class. */
export type HitlLane = "blocked" | "confirm" | "review";

export function isPrdInvalidIssue(issueClass: string | null | undefined): boolean {
  return issueClassKey(issueClass) === "prd_invalid";
}

export function hitlLane(input: {
  issueClass?: string | null;
  issueDetail?: string | null;
  status: PipelineStatus | "unstarted";
}): HitlLane | null {
  const issueClass = input.issueClass ?? "";
  const issueDetail = input.issueDetail ?? "";
  if (input.status === "waiting_merge" || issueClassKey(issueClass) === "waiting_merge") {
    return "review";
  }
  if (input.status !== "blocked") {
    return null;
  }
  if (
    isRequirementAmbiguousIssue(issueClass) ||
    isP0UnansweredMessage(issueDetail) ||
    isTrdUnconfirmedIssue(issueClass) ||
    isTrdUnconfirmedMessage(issueDetail) ||
    isPrdInvalidIssue(issueClass)
  ) {
    return "confirm";
  }
  if (
    isOutputGuardBlock(issueClass, issueDetail) ||
    isQualityGateBlock(issueClass, issueDetail) ||
    issueClassKey(issueClass) === "low_cr_score" ||
    issueClassKey(issueClass) === "diff_too_large"
  ) {
    return "review";
  }
  return "blocked";
}

export function pipelineHitlLane(pipeline: {
  issue_class: string | null;
  issue_detail: string | null;
  status: PipelineStatus;
}): HitlLane | null {
  return hitlLane({
    issueClass: pipeline.issue_class,
    issueDetail: pipeline.issue_detail,
    status: pipeline.status,
  });
}
