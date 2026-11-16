/**
 * The simulation transport seam.
 *
 * Every simulation call goes through this module, so a screen never knows a
 * URL and the day the `simulation` service ships only this file needs the real
 * shape confirmed. The paths and bodies follow
 * `docs/architecture/http-api.md` section 6. The response types are local
 * view types until DIST-102 fixes the envelopes; the normalizers accept the
 * two envelope forms the shell already tolerates (a bare array or an object
 * carrying the same records under `windows` or `marks`).
 */

import { call, callVoid } from "./api";
import type {
  ChartMark,
  ChartSeries,
  MarkDraft,
  MarkOrigin,
} from "./chart/types";

/** Where a simulation run is in the research loop. */
export type SimulationStatus =
  | "draft"
  | "marking"
  | "reviewed"
  | "checked"
  | "decided";

/** The decision a run can record; a decision closes marking. */
export type SimulationDecisionKind = "test" | "refine" | "stop";

/** One simulation run in a list. */
export interface SimulationSummary {
  sim_id: string;
  premise: string;
  status: SimulationStatus;
  windows: number;
  marks: number;
  /** The recorded bundle the run reads. */
  bundle: string;
  /** Windows carrying at least one mark. */
  marked_windows: number;
}

/** The fields a researcher supplies to create a run. */
export interface SimulationCreate {
  premise: string;
  bundle: string;
  inputs: string;
  seed: number;
  criteria: string;
}

/** One deterministic window selection, as a half-open bar range. */
export interface SimulationWindow {
  index: number;
  start: number;
  end: number;
}

/** One recorded decision. */
export interface SimulationDecision {
  decision: SimulationDecisionKind;
  note: string;
  decided_at_utc?: string;
}

/** The counts the detail endpoint publishes. */
export interface SimulationCounts {
  windows?: number;
  marked_windows?: number;
  human_marks?: number;
  oracle_marks?: number;
}

/** One simulation run in detail. */
export interface SimulationRun {
  sim_id: string;
  premise: string;
  status: SimulationStatus;
  bundle?: string;
  inputs?: string;
  inputs_digest?: string;
  seed?: number;
  criteria?: string;
  counts?: SimulationCounts;
  decision?: SimulationDecision | null;
  created_at?: string;
}

/** The event study and its verdict; the field names match the check table. */
export interface SimulationCheck {
  events: number;
  mean_net: number;
  hit_rate: number;
  control: number;
  control_mean_net: number;
  control_hit_rate: number;
  positive_half_years: number;
  passed: boolean;
  failures: string[];
}

/** The response to a stored mark. */
export interface MarkAck {
  mark_id: string;
}

/** Whether a create form carries every field the endpoint needs. */
export function canCreate(input: SimulationCreate): boolean {
  return (
    input.premise.trim().length > 0 &&
    input.bundle.trim().length > 0 &&
    input.inputs.trim().length > 0 &&
    input.criteria.trim().length > 0 &&
    Number.isInteger(input.seed) &&
    input.seed >= 0
  );
}

function base(simulationId: string): string {
  return `/simulation/runs/${encodeURIComponent(simulationId)}`;
}

/** The runs list path, with its optional filters. */
export function runsPath(options: {
  status?: SimulationStatus;
  limit?: number;
} = {}): string {
  const query = new URLSearchParams();
  if (options.status !== undefined) {
    query.set("status", options.status);
  }
  if (options.limit !== undefined) {
    query.set("limit", String(options.limit));
  }
  const suffix = query.toString();
  return suffix === "" ? "/simulation/runs" : `/simulation/runs?${suffix}`;
}

/** The detail path for one simulation run. */
export function runPath(simulationId: string): string {
  return base(simulationId);
}

/** The window-selection path for one simulation run. */
export function windowsPath(simulationId: string): string {
  return `${base(simulationId)}/windows`;
}

/** The series path for one chart window. */
export function seriesPath(simulationId: string, chart: number): string {
  return `${base(simulationId)}/windows/${chart}/series`;
}

/** The marks path for one simulation run. */
export function marksPath(simulationId: string): string {
  return `${base(simulationId)}/marks`;
}

/** The oracle path for one simulation run. */
export function oraclePath(simulationId: string): string {
  return `${base(simulationId)}/oracle`;
}

/** The check path for one simulation run. */
export function checkPath(simulationId: string): string {
  return `${base(simulationId)}/check`;
}

/** The decision path for one simulation run. */
export function decisionPath(simulationId: string): string {
  return `${base(simulationId)}/decision`;
}

/** Read the run list; a failure is the caller's to present, never a stale list. */
export async function listRuns(
  options: { status?: SimulationStatus; limit?: number } = {},
): Promise<SimulationSummary[]> {
  return call<SimulationSummary[]>(runsPath(options));
}

/** Create one simulation run and return its server-assigned id. */
export async function createRun(input: SimulationCreate): Promise<string> {
  const ack = await call<{ sim_id?: unknown }>("/simulation/runs", {
    method: "POST",
    body: JSON.stringify(input),
  });
  if (typeof ack.sim_id !== "string" || ack.sim_id.length === 0) {
    throw new Error("the server accepted the simulation without a sim_id");
  }
  return ack.sim_id;
}

/** Read one run's premise, inputs, counts, status, and decision. */
export async function fetchRun(simulationId: string): Promise<SimulationRun> {
  return call<SimulationRun>(runPath(simulationId));
}

type WindowsBody = SimulationWindow[] | { windows?: SimulationWindow[] };

function windowList(body: WindowsBody): SimulationWindow[] {
  const rows = Array.isArray(body) ? body : (body.windows ?? []);
  const windows = rows.map((row) => {
    if (
      !Number.isInteger(row.index) ||
      row.index < 0 ||
      !Number.isInteger(row.start) ||
      !Number.isInteger(row.end) ||
      row.start < 0 ||
      row.end < row.start
    ) {
      throw new Error("the server returned an invalid simulation window");
    }
    return { index: row.index, start: row.start, end: row.end };
  });
  return windows.sort((left, right) => left.index - right.index);
}

/**
 * Ask the server for the deterministic window selection.
 *
 * The endpoint is idempotent for the same seed, so opening a run may call it
 * again to recover the navigator after a reload.
 */
export async function selectWindows(
  simulationId: string,
): Promise<SimulationWindow[]> {
  const body = await call<WindowsBody>(windowsPath(simulationId), {
    method: "POST",
  });
  return windowList(body);
}

/** Read one window's series. */
export async function fetchSeries(
  simulationId: string,
  chart: number,
): Promise<ChartSeries> {
  return call<ChartSeries>(seriesPath(simulationId, chart));
}

type ServerMark = Omit<ChartMark, "chart"> & { chart: number | string };
type MarksBody = ServerMark[] | { marks?: ServerMark[] };

function markList(body: MarksBody): ChartMark[] {
  const rows = Array.isArray(body) ? body : (body.marks ?? []);
  return rows.map((row) => {
    if (typeof row.mark_id !== "string" || row.mark_id.length === 0) {
      throw new Error("the server returned a mark without a mark id");
    }
    const chart = typeof row.chart === "number" ? row.chart : Number(row.chart);
    if (!Number.isInteger(chart) || chart < 0) {
      throw new Error("the server returned a mark with an invalid window");
    }
    return { ...row, chart };
  });
}

/** Read the marks, optionally only one origin. */
export async function fetchMarks(
  simulationId: string,
  origin?: MarkOrigin,
): Promise<ChartMark[]> {
  const query = origin === undefined ? "" : `?origin=${encodeURIComponent(origin)}`;
  const body = await call<MarksBody>(`${marksPath(simulationId)}${query}`);
  return markList(body);
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

/** Ask the server for the deterministic oracle marks. */
export async function postOracle(simulationId: string): Promise<ChartMark[]> {
  const body = await call<MarksBody>(oraclePath(simulationId), {
    method: "POST",
  });
  return markList(body);
}

type CheckBody = SimulationCheck | { check?: SimulationCheck };

/** Run the event-study check and return its table and verdict. */
export async function postCheck(
  simulationId: string,
): Promise<SimulationCheck> {
  const body = await call<CheckBody>(checkPath(simulationId), {
    method: "POST",
  });
  const result = "passed" in body ? body : body.check;
  if (result === undefined || typeof result.passed !== "boolean") {
    throw new Error("the server returned no check verdict");
  }
  return result;
}

type DecisionBody = SimulationDecision | { decision?: SimulationDecision };

/** Record test, refine, or stop, with the reason; the run locks marking. */
export async function postDecision(
  simulationId: string,
  decision: SimulationDecisionKind,
  note: string,
): Promise<SimulationDecision> {
  const body = await call<DecisionBody>(decisionPath(simulationId), {
    method: "POST",
    body: JSON.stringify({ decision, note }),
  });
  const record = "note" in body ? body : body.decision;
  if (
    record === undefined ||
    (record.decision !== "test" &&
      record.decision !== "refine" &&
      record.decision !== "stop")
  ) {
    throw new Error("the server returned no decision record");
  }
  return record;
}
