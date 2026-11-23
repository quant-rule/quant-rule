/**
 * The client's retry budget and reconnect state.
 *
 * A retry repeats a GET from this browser; it is not a separate server
 * operation and it never changes run identity. Only a GET read opts in, and the
 * policy is fixed — three attempts, a 250 ms base delay, a factor of four and a
 * 4 s cap — with no jitter and no random source. A mutation and a streaming
 * upload never retry.
 */
import { get, writable, type Readable } from "svelte/store";

/** Total attempts, including the first. */
export const RETRY_ATTEMPTS = 3;
/** The delay after the first failure. */
export const RETRY_BASE_DELAY_MS = 250;
/** The factor applied to each subsequent delay. */
export const RETRY_FACTOR = 4;
/** The largest delay between two attempts. */
export const RETRY_MAX_DELAY_MS = 4_000;

/** What the client knows about its transport right now. */
export interface TransportState {
  /** Whether a read is inside the retry budget. */
  reconnecting: boolean;
  /** The last stable transport message, or null after a success. */
  lastError: string | null;
}

const state = writable<TransportState>({ reconnecting: false, lastError: null });

/** The reconnect state the banner reads. Writes go through the helpers below. */
export const transport: Readable<TransportState> = { subscribe: state.subscribe };

/**
 * The delay after `failedAttempt`, or null when the budget is spent.
 *
 * The schedule is a pure function of the attempt count, so a test asserts it
 * without waiting.
 */
export function retryDelayMs(failedAttempt: number): number | null {
  if (
    !Number.isInteger(failedAttempt) ||
    failedAttempt < 1 ||
    failedAttempt >= RETRY_ATTEMPTS
  ) {
    return null;
  }
  return Math.min(
    RETRY_BASE_DELAY_MS * RETRY_FACTOR ** (failedAttempt - 1),
    RETRY_MAX_DELAY_MS,
  );
}

/** Whether a status may improve when the same GET repeats. */
export function retryableStatus(status: number): boolean {
  return [408, 429, 502, 503, 504].includes(status);
}

/** Only GET is retryable; every mutation and upload is single-attempt. */
export function retryableMethod(method: string | undefined): boolean {
  return (method ?? "GET").toUpperCase() === "GET";
}

/** The message shown while the next attempt waits. */
export function retryMessage(attempt: number, status: number | null): string {
  if (status === null) {
    return `The gateway did not answer on attempt ${attempt} of ${RETRY_ATTEMPTS}.`;
  }
  return `The gateway answered HTTP ${status} on attempt ${attempt} of ${RETRY_ATTEMPTS}.`;
}

/** The message shown after the last attempt failed. */
export function finalMessage(attempt: number, status: number | null): string {
  const noun = attempt === 1 ? "attempt" : "attempts";
  if (status === null) {
    return `The gateway did not answer after ${attempt} ${noun}.`;
  }
  return `The gateway answered HTTP ${status} after ${attempt} ${noun}.`;
}

/** Record that a read is waiting for its next attempt. */
export function beginRetry(message: string): void {
  state.set({ reconnecting: true, lastError: message });
}

/** Record that the budget is spent; keep the stable last message. */
export function endRetry(message: string): void {
  state.set({ reconnecting: false, lastError: message });
}

/** Clear the state after a successful read. */
export function clearRetry(): void {
  const current = get(state);
  if (current.reconnecting || current.lastError !== null) {
    state.set({ reconnecting: false, lastError: null });
  }
}

/** Drop the state between tests. */
export function resetTransport(): void {
  state.set({ reconnecting: false, lastError: null });
}
