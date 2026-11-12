import type { ModelRunView } from "./types";

/**
 * The last model run completed in this UI session.
 *
 * The run panel owns the timing and the computed counts; the memory panel
 * records them. The store keeps the values across navigation so a recorded run
 * has a real duration and the model evidence, instead of two equal
 * `Date.now()` values and a discarded result.
 */
export interface LastRun {
  config: string;
  startedAt: number;
  finishedAt: number | null;
  model: ModelRunView | null;
}

let current = $state<LastRun | null>(null);

export function setLastRun(run: LastRun | null): void {
  current = run;
}

/**
 * Returns the timestamps to record for a run.
 *
 * A matching completed run uses its own start and finish. A config with no
 * completed run uses the record time as the start and a null finish, rather
 * than claiming a zero-duration run.
 */
export function recordTimes(
  config: string,
  now: number,
): { startedAt: number; finishedAt: number | null } {
  if (current !== null && current.config === config) {
    return { startedAt: current.startedAt, finishedAt: current.finishedAt };
  }
  return { startedAt: now, finishedAt: null };
}

/** Returns the completed model for a config, or null when there is none. */
export function lastModel(config: string): ModelRunView | null {
  return current !== null && current.config === config ? current.model : null;
}
