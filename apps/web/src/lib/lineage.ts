/**
 * Run lineage transport.
 *
 * A run with no recorded lineage is an empty state, not a broken run. Only the
 * stable `run.lineage_not_found` code becomes null; every other failure is
 * reported, because a substituted or unavailable record must not look empty.
 */

import { ApiError, call } from "./api";
import type { Digest, RunLineage } from "./types";

/** Read one run's recorded references, or null when none were recorded. */
export async function fetchRunLineage(runId: Digest): Promise<RunLineage | null> {
  try {
    return await call<RunLineage>(`/runs/${encodeURIComponent(runId)}/lineage`);
  } catch (error) {
    if (error instanceof ApiError && error.code === "run.lineage_not_found") {
      return null;
    }
    throw error;
  }
}
