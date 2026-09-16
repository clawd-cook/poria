/**
 * Demand status guard (Design A10).
 * MVP: does not filter by status. Placeholder for future status enforcement.
 */

const KNOWN_ACTIVE_STATUSES = new Set([20, 30]); // communicating, accepted

/**
 * Returns true if the demand is considered active.
 * MVP implementation always returns true (no status filtering).
 */
export function isDemandActive(status: number | undefined): boolean {
  // Status mapping incomplete; do not filter yet.
  // TODO: collect full status enumeration from real API responses.
  void KNOWN_ACTIVE_STATUSES;
  if (status == null) return true;
  return true;
}
