const STORAGE_KEY = "poria:v1:hitlAutoNavigate";

export function loadHitlAutoNavigate(): boolean {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === null) {
      return true;
    }
    return raw !== "0" && raw !== "false";
  } catch {
    return true;
  }
}

export function saveHitlAutoNavigate(enabled: boolean): void {
  try {
    localStorage.setItem(STORAGE_KEY, enabled ? "1" : "0");
  } catch {
    // ignore quota / private mode
  }
}
