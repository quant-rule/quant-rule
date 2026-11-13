import { call } from "./api";
import type { HealthReport, VersionInfo } from "./types";

/** What the client knows about the gateway. */
export type ConnectionState = "unknown" | "connected" | "degraded" | "unreachable";

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
 * and no stale data.
 */
export async function refresh(): Promise<void> {
  try {
    const [version, health] = await Promise.all([
      call<VersionInfo>("/version"),
      call<HealthReport>("/health"),
    ]);
    current = {
      state: health.state === "healthy" ? "connected" : "degraded",
      version,
      health,
      error: null,
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
