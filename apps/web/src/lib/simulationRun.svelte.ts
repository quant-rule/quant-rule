/**
 * The simulation run detail screen.
 *
 * One server resource per screen: the run, its windows, the drawn series, and
 * the marks. A request counter keeps an in-flight read from a previous window
 * from overwriting the current one. The actions re-read the run so the screen
 * shows what the server recorded.
 */

import { errorText } from "./api";
import type { ChartMark, ChartSeries } from "./chart/types";
import {
  fetchMarks,
  fetchRun,
  fetchSeries,
  postCheck,
  postDecision,
  postOracle,
  selectWindows,
  type SimulationCheck,
  type SimulationDecisionKind,
  type SimulationRun,
  type SimulationWindow,
} from "./simulation";

/** What the simulation run detail screen knows. */
export interface SimulationRunView {
  run: SimulationRun | null;
  runLoading: boolean;
  runError: string | null;
  windows: SimulationWindow[];
  windowsLoading: boolean;
  windowsError: string | null;
  chart: number | null;
  series: ChartSeries | null;
  seriesLoading: boolean;
  seriesError: string | null;
  marks: ChartMark[];
  marksLoading: boolean;
  marksError: string | null;
  oracleBusy: boolean;
  checkBusy: boolean;
  checkResult: SimulationCheck | null;
  checkError: string | null;
  decisionBusy: boolean;
  decisionError: string | null;
  actionError: string | null;
  actionStatus: string | null;
}

function initial(): SimulationRunView {
  return {
    run: null,
    runLoading: true,
    runError: null,
    windows: [],
    windowsLoading: false,
    windowsError: null,
    chart: null,
    series: null,
    seriesLoading: false,
    seriesError: null,
    marks: [],
    marksLoading: false,
    marksError: null,
    oracleBusy: false,
    checkBusy: false,
    checkResult: null,
    checkError: null,
    decisionBusy: false,
    decisionError: null,
    actionError: null,
    actionStatus: null,
  };
}

let current = $state<SimulationRunView>(initial());

/** A counter that makes an in-flight window read stale. */
let windowRequest = 0;

/** A counter that makes an in-flight marks read stale. */
let marksRequest = 0;

/** Read the current run state. */
export function simulationRun(): SimulationRunView {
  return current;
}

/** Drop every row; the next read belongs to the open run. */
export function reset(): void {
  windowRequest += 1;
  marksRequest += 1;
  current = initial();
}

/** A decided run refuses every action. */
export function locked(): boolean {
  return current.run?.status === "decided" || current.run?.decision != null;
}

/** Replace the mark list after the chart appended or removed one. */
export function setMarks(marks: ChartMark[]): void {
  current = { ...current, marks };
}

/** Read the run, then its windows and marks. */
export async function openRun(simId: string): Promise<void> {
  reset();
  try {
    const run = await fetchRun(simId);
    current = { ...current, run };
  } catch (error) {
    current = { ...current, runLoading: false, runError: errorText(error) };
    return;
  }
  current = { ...current, runLoading: false };
  await Promise.all([loadWindows(simId), loadMarks(simId)]);
}

/** Read the selected windows and draw the preferred one. */
export async function loadWindows(simId: string): Promise<void> {
  current = { ...current, windowsLoading: true, windowsError: null };
  try {
    const found = await selectWindows(simId);
    current = { ...current, windows: found };
    if (found.length === 0) {
      current = { ...current, chart: null, series: null };
      return;
    }
    const chart = current.chart;
    const preferred =
      chart !== null && found.some((window) => window.index === chart)
        ? chart
        : found[0].index;
    await openWindow(simId, preferred);
  } catch (error) {
    current = { ...current, windowsError: errorText(error) };
  } finally {
    current = { ...current, windowsLoading: false };
  }
}

/** Draw one window; a slower earlier request cannot overwrite it. */
export async function openWindow(simId: string, index: number): Promise<void> {
  const request = (windowRequest += 1);
  current = {
    ...current,
    chart: index,
    series: null,
    seriesError: null,
    seriesLoading: true,
  };
  try {
    const next = await fetchSeries(simId, index);
    if (request === windowRequest) {
      current = { ...current, series: next };
    }
  } catch (error) {
    if (request === windowRequest) {
      current = { ...current, seriesError: errorText(error) };
    }
  } finally {
    if (request === windowRequest) {
      current = { ...current, seriesLoading: false };
    }
  }
}

/** Read the marks; a slower earlier request cannot overwrite them. */
export async function loadMarks(simId: string): Promise<void> {
  const request = (marksRequest += 1);
  current = { ...current, marksLoading: true, marksError: null };
  try {
    const next = await fetchMarks(simId);
    if (request === marksRequest) {
      current = { ...current, marks: next };
    }
  } catch (error) {
    if (request === marksRequest) {
      current = { ...current, marks: [], marksError: errorText(error) };
    }
  } finally {
    if (request === marksRequest) {
      current = { ...current, marksLoading: false };
    }
  }
}

/** Re-read the run after an action, keeping the confirmed state on a failure. */
export async function refreshRun(simId: string): Promise<void> {
  try {
    const run = await fetchRun(simId);
    current = { ...current, run };
  } catch {
    // The action already succeeded against the server; keep the confirmed run.
  }
}

/** Run the oracle, then re-read the marks and the run. */
export async function runOracle(simId: string): Promise<void> {
  if (current.oracleBusy || locked()) {
    return;
  }
  current = {
    ...current,
    oracleBusy: true,
    actionError: null,
    actionStatus: null,
  };
  try {
    await postOracle(simId);
    await Promise.all([loadMarks(simId), refreshRun(simId)]);
    current = { ...current, actionStatus: "Oracle marks recorded." };
  } catch (error) {
    current = { ...current, actionError: errorText(error) };
  } finally {
    current = { ...current, oracleBusy: false };
  }
}

/** Run the check on the unmarked window, then re-read the run. */
export async function runCheck(simId: string): Promise<void> {
  if (current.checkBusy || locked()) {
    return;
  }
  current = {
    ...current,
    checkBusy: true,
    checkResult: null,
    checkError: null,
    actionError: null,
    actionStatus: null,
  };
  try {
    const result = await postCheck(simId);
    current = { ...current, checkResult: result };
    await refreshRun(simId);
    current = {
      ...current,
      actionStatus: result.passed ? "The check passed." : "The check failed.",
    };
  } catch (error) {
    current = { ...current, checkError: errorText(error) };
  } finally {
    current = { ...current, checkBusy: false };
  }
}

/** Record one decision; returns whether the server accepted it. */
export async function submitDecision(
  simId: string,
  kind: SimulationDecisionKind,
  note: string,
): Promise<boolean> {
  if (current.decisionBusy || locked()) {
    return false;
  }
  const text = note.trim();
  if (text === "") {
    current = { ...current, decisionError: "A decision needs a note." };
    return false;
  }
  current = {
    ...current,
    decisionBusy: true,
    decisionError: null,
    actionError: null,
    actionStatus: null,
  };
  try {
    const record = await postDecision(simId, kind, text);
    const run = current.run;
    current = {
      ...current,
      run: run === null ? null : { ...run, status: "decided", decision: record },
      actionStatus: "Decision recorded. Marking is closed.",
    };
    return true;
  } catch (error) {
    current = { ...current, decisionError: errorText(error) };
    return false;
  } finally {
    current = { ...current, decisionBusy: false };
  }
}
