import type { StageEnum } from "@poria/core";

export const STAGE_SKILL_MAP: Record<StageEnum, string> = {
  init: "skill:init",
  review_prd: "skill:review-prd",
  design: "skill:gen-trd",
  workspace: "skill:workspace",
  dev: "skill:gen-code",
  cr: "skill:code-review",
  deploy: "skill:deploy",
};
