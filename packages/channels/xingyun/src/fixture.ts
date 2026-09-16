import type { DemandDetail } from "./jacp/demands.js";
import type { CardAttachment } from "./jacp/cards.js";

export function isXingyunFixtureMode(): boolean {
  return process.env.PORIA_XINGYUN_FIXTURE === "1";
}

export interface FixtureDemand extends DemandDetail {
  demandDescLink?: string;
}

export const FIXTURE_DEMAND: FixtureDemand = {
  id: 10001,
  demandCode: "REQ-FIXTURE-001",
  name: "Fixture Xingyun demand",
  status: 20,
  demandDesc: "Offline fixture for PORIA_XINGYUN_FIXTURE=1",
  demandDescLink: "https://joyspace.jd.com/pages/fixture-prd",
  projectId: 42,
};

export const FIXTURE_ATTACHMENTS: CardAttachment[] = [
  {
    id: 1,
    tagName: "PRD",
    name: "PRD 需求文档",
    url: "https://joyspace.jd.com/pages/fixture-prd",
  },
];

export const FIXTURE_BIND = {
  branch: "feature_REQ-FIXTURE-001",
  changeId: "fixture-change-001",
  baseBranch: "master",
};
