import type { RunState, RunSummary } from "./types";

/** Whether a run still expects to change. */
export function isActive(status: RunState): boolean {
  return status === "queued" || status === "running";
}

/** The next poll delay: fast while a run is active, slow otherwise. */
export function pollDelay(items: RunSummary[]): number {
  return items.some((item) => isActive(item.status)) ? 2_000 : 30_000;
}

/** A compact run or data id for a table cell. */
export function shortId(value: string): string {
  return value.length > 18 ? `${value.slice(0, 15)}…` : value;
}

/** A duration from the control plane, or an em dash when unfinished. */
export function elapsed(milliseconds: number | null): string {
  if (milliseconds === null) {
    return "—";
  }
  const seconds = milliseconds / 1_000;
  return seconds < 10 ? `${seconds.toFixed(1)}s` : `${Math.round(seconds)}s`;
}

/** An RFC 3339 instant as a UTC table value. */
export function submitted(iso: string): string {
  return iso.replace("T", " ").replace(".000Z", "Z");
}
