import {
  asOptionalNumber,
  asOptionalText,
  jacpFetch,
  type JacpCredentials,
} from "./client.js";

export async function getSpaceById(
  credentials: JacpCredentials,
  spaceId: number,
): Promise<unknown> {
  return jacpFetch<unknown>(
    credentials,
    `/openapi/v3/spaces/${encodeURIComponent(String(spaceId))}`,
    { method: "GET" },
    "空间详情查询失败",
  );
}

export function asSpaceKey(value: unknown): string | undefined {
  if (!value || typeof value !== "object") return undefined;
  return asOptionalText((value as { key?: unknown }).key);
}

export function asSpaceId(value: unknown): number | undefined {
  if (!value || typeof value !== "object") return undefined;
  return asOptionalNumber((value as { id?: unknown }).id);
}
