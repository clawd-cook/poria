/** Board / reuse key: nonempty demand_code, otherwise `id:{demandId}`. */
export function demandTaskKey(demandCode: string | null | undefined, demandId: number): string {
  const code = demandCode?.trim() ?? "";
  return code.length > 0 ? code : `id:${demandId}`;
}
