import { call } from "./api";
import { classify, type ConnectionState, EXPECTED_API } from "./connection";
import type { HealthReport, VersionInfo } from "./types";

export { EXPECTED_API };
export type { ConnectionState };

export interface Connection {
  state: ConnectionState;
  version: VersionInfo | null;
  health: HealthReport | null;
  error: string | null;
}

let current = $state<Connection>({
  state: "unknown",
  version: null,
  health: null,
  error: null,
});

/** Read the current connection. */
export function connection(): Connection {
  return current;
}

/**
 * Ask the gateway what it is.
 *
 * A failed call is a state, not an exception: the screen shows `unreachable`
 * and no stale data. A different API version is `mismatch`, not `connected`.
 */
export async function refresh(): Promise<void> {
  try {
    const [version, health] = await Promise.all([
      call<VersionInfo>("/version"),
      call<HealthReport>("/health"),
    ]);
    const verdict = classify(version, health);
    current = {
      state: verdict.state,
      version,
      health,
      error: verdict.error,
    };
  } catch (error) {
    current = {
      state: "unreachable",
      version: null,
      health: null,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}
