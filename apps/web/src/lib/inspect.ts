/**
 * The read-only inspection calls.
 *
 * Each function describes one gateway inspection route and returns the
 * server's numbers unchanged. The panels decide how to show them; nothing here
 * derives a reading or maps a code to a word.
 */

import { call } from "./api";
import type {
  DatasetSummary,
  Digest,
  InspectEvents,
  RunDigest,
  TimeframeInspection,
} from "./types";

/** Counts and coverage for one stored dataset. */
export function inspectCandles(dataVersion: Digest): Promise<DatasetSummary> {
  return call<DatasetSummary>(
    `/inspect/candles?data_version=${encodeURIComponent(dataVersion)}`,
  );
}

/** The timeframe ladder, counted from the stored bytes. */
export function inspectTimeframes(dataVersion: Digest): Promise<TimeframeInspection> {
  return call<TimeframeInspection>(
    `/inspect/timeframes?data_version=${encodeURIComponent(dataVersion)}`,
  );
}

/** The event batch for one dataset and timeframe. */
export function inspectEvents(
  dataVersion: Digest,
  timeframe: string,
): Promise<InspectEvents> {
  return call<InspectEvents>(
    `/inspect/events?data_version=${encodeURIComponent(dataVersion)}&timeframe=${encodeURIComponent(timeframe)}`,
  );
}

/** The recorded canonical digest of one run. */
export function verifyRun(runId: Digest): Promise<RunDigest> {
  return call<RunDigest>(`/runs/${encodeURIComponent(runId)}/digest`);
}
