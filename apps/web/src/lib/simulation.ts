/**
 * The simulation transport seam.
 *
 * Every simulation call goes through this module, so the chart component never
 * knows a URL and the day the `simulation` service ships only this file needs
 * the real shape confirmed. The paths and bodies follow
 * `docs/architecture/http-api.md` section 6; the series response is a local
 * view type until DIST-102 fixes its exact envelope.
 */

import { call, callVoid } from "./api";
import type { ChartSeries, MarkDraft } from "./chart/types";

/** The response to a stored mark. */
export interface MarkAck {
  mark_id: string;
}

function base(simulationId: string): string {
  return `/simulation/runs/${encodeURIComponent(simulationId)}`;
}

/** The series path for one chart window. */
export function seriesPath(simulationId: string, chart: number): string {
  return `${base(simulationId)}/windows/${chart}/series`;
}

/** The marks path for one simulation run. */
export function marksPath(simulationId: string): string {
  return `${base(simulationId)}/marks`;
}

/** Read one window's series. */
export async function fetchSeries(
  simulationId: string,
  chart: number,
): Promise<ChartSeries> {
  return call<ChartSeries>(seriesPath(simulationId, chart));
}

/** Store one human mark and return its server-assigned id. */
export async function postMark(
  simulationId: string,
  draft: MarkDraft,
): Promise<MarkAck> {
  const ack = await call<MarkAck>(marksPath(simulationId), {
    method: "POST",
    body: JSON.stringify(draft),
  });
  if (typeof ack.mark_id !== "string" || ack.mark_id.length === 0) {
    throw new Error("the server accepted the mark without a mark id");
  }
  return ack;
}

/** Delete one mark; a 204 is the success shape and carries no JSON. */
export async function deleteMark(
  simulationId: string,
  markId: string,
): Promise<void> {
  await callVoid(`${marksPath(simulationId)}/${encodeURIComponent(markId)}`, {
    method: "DELETE",
  });
}
