import {
  asOptionalNumber,
  asOptionalText,
  jacpFetch,
  type JacpCredentials,
} from "./client.js";

export type CardAttachment = {
  id?: number;
  tagId?: number;
  tagName: string;
  name: string;
  url: string;
};

export async function getCardByCode(
  credentials: JacpCredentials,
  cardCode: string,
): Promise<unknown> {
  const code = cardCode.trim();
  if (!code) throw new Error("缺少卡片编码");
  return jacpFetch<unknown>(
    credentials,
    `/openapi/v3/cards/code/${encodeURIComponent(code)}`,
    { method: "GET" },
    "空间卡片详情查询失败",
  );
}

export async function getCardById(
  credentials: JacpCredentials,
  cardId: number,
): Promise<unknown> {
  if (!Number.isFinite(cardId) || cardId <= 0) {
    throw new Error("缺少卡片 ID");
  }
  return jacpFetch<unknown>(
    credentials,
    `/openapi/v3/cards/${encodeURIComponent(String(cardId))}`,
    { method: "GET" },
    "空间卡片详情查询失败",
  );
}

export function asCardSpaceId(value: unknown): number | undefined {
  if (!value || typeof value !== "object") return undefined;
  const item = value as Record<string, unknown>;
  const nested = item.space;
  if (nested && typeof nested === "object") {
    const id = asOptionalNumber((nested as { id?: unknown }).id);
    if (id != null) return id;
  }
  return asOptionalNumber(item.spaceId);
}

export function asCardCode(value: unknown): string | undefined {
  if (!value || typeof value !== "object") return undefined;
  return asOptionalText((value as { code?: unknown }).code);
}

export function asCardAttachments(value: unknown): CardAttachment[] {
  if (!value || typeof value !== "object") return [];
  const raw = (value as { attachments?: unknown }).attachments;
  if (!Array.isArray(raw)) return [];
  return raw
    .map((item) => asCardAttachment(item))
    .filter((item): item is CardAttachment => Boolean(item));
}

function asCardAttachment(value: unknown): CardAttachment | undefined {
  if (!value || typeof value !== "object") return undefined;
  const item = value as {
    id?: unknown;
    tagId?: unknown;
    tagName?: unknown;
    name?: unknown;
    url?: unknown;
  };
  const name = asOptionalText(item.name);
  const url = asOptionalText(item.url);
  if (!name || !url) return undefined;
  return {
    id: asOptionalNumber(item.id),
    tagId: asOptionalNumber(item.tagId),
    tagName: asOptionalText(item.tagName) || "附件",
    name,
    url,
  };
}
