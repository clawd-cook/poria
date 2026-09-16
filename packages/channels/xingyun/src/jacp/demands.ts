import {
  asOptionalNumber,
  asOptionalText,
  jacpFetch,
  type JacpCredentials,
} from "./client.js";

export type UserVO = {
  erp?: string;
  name?: string;
  orgId?: string;
  orgName?: string;
};

export type DemandDetail = {
  id: number;
  demandCode: string;
  name: string;
  status?: number;
  demandDesc?: string;
  demandDescLink?: string;
  projectId?: number;
  processor?: UserVO;
  proposer?: UserVO;
  receiver?: UserVO;
  extendedFields?: Record<string, unknown>;
};

export type DemandActionResult = {
  demandId?: number;
  demandStatusCode?: number;
  taskOwner?: string;
  taskId?: number;
  taskName?: string;
};

function asUser(value: unknown): UserVO | undefined {
  if (!value || typeof value !== "object") return undefined;
  const item = value as Record<string, unknown>;
  const user: UserVO = {
    erp: item.erp == null ? undefined : String(item.erp),
    name: item.name == null ? undefined : String(item.name),
    orgId: item.orgId == null ? undefined : String(item.orgId),
    orgName: item.orgName == null ? undefined : String(item.orgName),
  };
  if (!user.erp && !user.name) return undefined;
  return user;
}

function asDemandDetail(value: unknown): DemandDetail {
  if (!value || typeof value !== "object") {
    throw new Error("需求详情为空");
  }
  const item = value as Record<string, unknown>;
  const id = Number(item.id);
  if (!Number.isFinite(id)) {
    throw new Error("需求详情缺少 ID");
  }
  const demandCode = String(item.demandCode ?? "").trim();
  const name =
    String(item.name ?? "").trim() || demandCode || `#${id}`;
  const extended =
    item.extendedFields &&
    typeof item.extendedFields === "object" &&
    !Array.isArray(item.extendedFields)
      ? (item.extendedFields as Record<string, unknown>)
      : undefined;
  return {
    id,
    demandCode,
    name,
    status: asOptionalNumber(item.status),
    demandDesc: item.demandDesc == null ? undefined : String(item.demandDesc),
    demandDescLink: asOptionalText(item.demandDescLink),
    projectId: asOptionalNumber(item.projectId),
    processor: asUser(item.processor),
    proposer: asUser(item.proposer),
    receiver: asUser(item.receiver),
    extendedFields: extended,
  };
}

export async function getDemandById(
  credentials: JacpCredentials,
  demandId: number,
): Promise<DemandDetail> {
  const data = await jacpFetch<unknown>(
    credentials,
    `/openapi/v3/demands/${demandId}`,
    { method: "GET" },
    "需求详情查询失败",
  );
  return asDemandDetail(data);
}

async function postDemandAction(
  credentials: JacpCredentials,
  path: string,
  body: Record<string, unknown>,
  errorLabel: string,
): Promise<DemandActionResult> {
  const demandId = asOptionalNumber(body.demandId);
  const data = await jacpFetch<unknown>(
    credentials,
    path,
    {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    },
    errorLabel,
    { allowEmpty: true },
  );
  if (!data || typeof data !== "object") {
    return { demandId };
  }
  const item = data as Record<string, unknown>;
  return {
    demandId: asOptionalNumber(item.demandId) ?? demandId,
    demandStatusCode: asOptionalNumber(item.demandStatusCode),
    taskOwner: asOptionalText(item.taskOwner),
    taskId: asOptionalNumber(item.taskId),
    taskName: asOptionalText(item.taskName),
  };
}

export function communicateDemand(
  credentials: JacpCredentials,
  demandId: number,
): Promise<DemandActionResult> {
  return postDemandAction(
    credentials,
    "/openapi/v3/demands/actions/communicate",
    { demandId },
    "沟通需求失败",
  );
}

export function acceptDemand(
  credentials: JacpCredentials,
  demandId: number,
): Promise<DemandActionResult> {
  return postDemandAction(
    credentials,
    "/openapi/v3/demands/actions/accept",
    { demandId },
    "受理需求失败",
  );
}
