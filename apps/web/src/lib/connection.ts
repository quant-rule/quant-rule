import { ApiError } from "./api";
import type { HealthReport, VersionInfo } from "./types";

/** The API contract this client was built against. */
export const EXPECTED_API = "quantrule.api/v1";

/** What the client knows about the gateway. */
export type ConnectionState =
  | "unknown"
  | "connected"
  | "degraded"
  | "mismatch"
  | "unreachable"
  /** The gateway refused the call for want of a bearer token. */
  | "unauthorized";

/** The verdict for one gateway answer. */
export interface Classification {
  state: ConnectionState;
  error: string | null;
}

/**
 * The state for one failed call.
 *
 * Only a `401` means the token is missing or wrong. Any other failure is the
 * gateway being unreachable or refusing for a reason the screen reports itself.
 */
export function classifyFailure(error: unknown): "unauthorized" | "unreachable" {
  return error instanceof ApiError && error.status === 401
    ? "unauthorized"
    : "unreachable";
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
