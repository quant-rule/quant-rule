import { call, errorText, setToken } from "./api";
import {
  classify,
  classifyFailure,
  type ConnectionState,
  EXPECTED_API,
} from "./connection";
import { rememberToken, storedToken } from "./token";
import { clearProfileScope } from "./profileScope.svelte";
import type { HealthReport, VersionInfo } from "./types";

export { EXPECTED_API };
export type { ConnectionState };

export interface Connection {
  state: ConnectionState;
  version: VersionInfo | null;
  health: HealthReport | null;
  error: string | null;
  /** Whether this browser holds a token, so the screen can offer to forget it. */
  tokenStored: boolean;
}

let current = $state<Connection>({
  state: "unknown",
  version: null,
  health: null,
  error: null,
  tokenStored: false,
});

/** Read the current connection. */
export function connection(): Connection {
  return current;
}

/**
 * Apply the token stored in this browser, then ask the gateway what it is.
 *
 * Run once from the app mount before any screen loads data, so every request
 * already carries the stored token.
 */
export async function initialise(): Promise<void> {
  setToken(storedToken());
  await refresh();
}

/**
 * Store a token in this browser and try it.
 *
 * The input is never placed in a URL, a header label, or a rendered message;
 * only the request header carries it. Returns whether the gateway answered
 * without a `401` or a transport failure, so the screen can keep the field for
 * correction when the token was not accepted.
 */
export async function useToken(value: string): Promise<boolean> {
  const token = rememberToken(value);
  setToken(token);
  await refresh();
  return (
    token !== null &&
    current.state !== "unauthorized" &&
    current.state !== "unreachable" &&
    current.state !== "unknown"
  );
}

/** Forget the stored token and re-check the gateway without one. */
export async function forgetToken(): Promise<void> {
  rememberToken(null);
  setToken(null);
  await refresh();
}

/**
 * Prove the bearer token is accepted by calling one gated route.
 *
 * A `401` is the answer the connection screen owns. Any other failure is not an
 * authorization verdict: `/version` and `/health` already describe whether the
 * gateway itself is reachable, so it is left to the screen that owns the route.
 */
async function provesAuthorization(): Promise<void> {
  try {
    await call<unknown>("/profiles");
  } catch (error) {
    if (classifyFailure(error) === "unauthorized") {
      throw error;
    }
  }
}

/**
 * Ask the gateway what it is.
 *
 * A failed call is a state, not an exception: the screen shows `unreachable`
 * and no stale data. A different API version is `mismatch`, not `connected`.
 * A `401` is `unauthorized`: the connected identity is cleared, and the screen
 * asks for the token instead of showing a stale badge.
 */
export async function refresh(): Promise<void> {
  try {
    const [version, health] = await Promise.all([
      call<VersionInfo>("/version"),
      call<HealthReport>("/health"),
    ]);
    // The gateway leaves /version and /health open even when a token is
    // required, so only a gated route proves the bearer token was accepted.
    await provesAuthorization();
    const verdict = classify(version, health);
    if (verdict.state === "mismatch") {
      // A contract the client was not built against is not trusted data.
      clearProfileScope();
    }
    current = {
      state: verdict.state,
      version,
      health,
      error: verdict.error,
      tokenStored: storedToken() !== null,
    };
  } catch (error) {
    if (classifyFailure(error) === "unauthorized") {
      clearProfileScope();
      current = {
        state: "unauthorized",
        version: null,
        health: null,
        error: `This gateway requires a bearer token. ${errorText(error)}`,
        tokenStored: storedToken() !== null,
      };
      return;
    }
    clearProfileScope();
    current = {
      state: "unreachable",
      version: null,
      health: null,
      error: error instanceof Error ? error.message : String(error),
      tokenStored: storedToken() !== null,
    };
  }
}
