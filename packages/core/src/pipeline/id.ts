import { nanoid } from "nanoid";

export function createPipelineId(): string {
  const date = new Date().toISOString().slice(0, 10).replace(/-/g, "");
  return `pl-${date}-${nanoid(8)}`;
}
