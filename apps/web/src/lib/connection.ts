import type { HealthReport, VersionInfo } from "./types";

/** The API contract this client was built against. */
export const EXPECTED_API = "quantrule.api/v1";

/** What the client knows about the gateway. */
export type ConnectionState =
  | "unknown"
  | "connected"
  | "degraded"
  | "mismatch"
  | "unreachable";

/** The verdict for one gateway answer. */
export interface Classification {
  state: ConnectionState;
  error: string | null;
}

/**
 * Classify a gateway answer.
 *
 * A version mismatch outranks a degraded service: the shapes may still parse,
 * but the client cannot trust a contract it was not built against.
 */
export function classify(version: VersionInfo, health: HealthReport): Classification {
  if (version.api !== EXPECTED_API) {
    return {
      state: "mismatch",
      error: `The gateway speaks ${version.api}; this client expects ${EXPECTED_API}.`,
    };
  }
  if (health.state !== "healthy") {
    return { state: "degraded", error: null };
  }
  return { state: "connected", error: null };
}
