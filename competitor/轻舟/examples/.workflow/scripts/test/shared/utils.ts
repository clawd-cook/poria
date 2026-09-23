export function deepMerge<T extends Record<string, any>>(a: T, b: Record<string, any>): T {
  const out: any = Array.isArray(a) ? [...a] : { ...a };
  for (const k of Object.keys(b)) {
    const av = (a as any)[k];
    const bv = b[k];
    if (
      bv &&
      typeof bv === "object" &&
      !Array.isArray(bv) &&
      av &&
      typeof av === "object" &&
      !Array.isArray(av)
    ) {
      out[k] = deepMerge(av, bv);
    } else {
      out[k] = bv;
    }
  }
  return out;
}
