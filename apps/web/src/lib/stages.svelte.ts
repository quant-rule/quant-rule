import { call, callVoid, errorText } from "./api";
import {
  downloadStageArtifact,
  stageEventView,
  stagePath,
} from "./stage";
import type {
  StageEventView,
  StageRunDetail,
  StageRunSummary,
  StageSubmitResponse,
} from "./types";

/** What the Workbench Model tab knows. */
export interface Stages {
  items: StageRunSummary[];
  loading: boolean;
  error: string | null;
  selected: string | null;
  detail: StageRunDetail | null;
  detailLoading: boolean;
  events: StageEventView | null;
  eventsLoading: boolean;
  eventsError: string | null;
  eventsTimeframe: string;
}

let current = $state<Stages>({
  items: [],
  loading: false,
  error: null,
  selected: null,
  detail: null,
  detailLoading: false,
  events: null,
  eventsLoading: false,
  eventsError: null,
  eventsTimeframe: "5m",
});

/** A counter that makes an in-flight read from a previous profile stale. */
let generation = 0;

/** Drop every stage row and the open detail. */
export function reset(): void {
  generation += 1;
  current = {
    items: [],
    loading: false,
    error: null,
    selected: null,
    detail: null,
    detailLoading: false,
    events: null,
    eventsLoading: false,
    eventsError: null,
    eventsTimeframe: "5m",
  };
}

/** Read the current stage state. */
export function stages(): Stages {
  return current;
}

/** Read the profile's stage list. */
export async function load(): Promise<void> {
  const at = generation;
  const first = current.items.length === 0;
  current = { ...current, loading: first, error: null };
  try {
    const items = await call<StageRunSummary[]>("/stages/runs?limit=100");
    if (at !== generation) {
      return;
    }
    current = { ...current, items, loading: false, error: null };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, items: [], loading: false, error: errorText(error) };
  }
}

/** Submit one model stage, then reload the list. */
export async function submit(
  dataVersion: string,
  configId: string,
): Promise<StageSubmitResponse | null> {
  const at = generation;
  current = { ...current, error: null };
  try {
    const response = await call<StageSubmitResponse>("/stages/model/runs", {
      method: "POST",
      body: JSON.stringify({
        kind: "model",
        config_id: configId,
        input: { kind: "data_version", data_version: dataVersion },
      }),
    });
    if (at !== generation) {
      return null;
    }
    await load();
    if (at !== generation) {
      return null;
    }
    return response;
  } catch (error) {
    if (at !== generation) {
      return null;
    }
    current = { ...current, error: errorText(error) };
    return null;
  }
}

/** Read one stage detail. */
export async function loadDetail(stageRunId: string): Promise<void> {
  const at = generation;
  current = { ...current, detailLoading: true, error: null };
  try {
    const detail = await call<StageRunDetail>(stagePath(stageRunId));
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      detail,
      detailLoading: false,
      error: null,
      events: null,
      eventsError: null,
    };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      detail: null,
      detailLoading: false,
      error: errorText(error),
    };
  }
}

/** Open one stage. */
export async function select(stageRunId: string): Promise<void> {
  current = {
    ...current,
    selected: stageRunId,
    detail: null,
    detailLoading: true,
    events: null,
    eventsError: null,
  };
  await loadDetail(stageRunId);
}

/** Return to the table and drop the open detail. */
export function clearSelection(): void {
  current = {
    ...current,
    selected: null,
    detail: null,
    detailLoading: false,
    events: null,
    eventsError: null,
  };
}

/** Cancel a queued or running stage, then read the server's state. */
export async function cancel(stageRunId: string): Promise<void> {
  const at = generation;
  try {
    await call(`${stagePath(stageRunId)}/cancel`, { method: "POST" });
    if (at !== generation) {
      return;
    }
    await load();
    if (current.selected === stageRunId) {
      await loadDetail(stageRunId);
    }
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, error: errorText(error) };
  }
}

/** Delete one stage and return to the list. */
export async function remove(stageRunId: string): Promise<void> {
  const at = generation;
  try {
    await callVoid(stagePath(stageRunId), { method: "DELETE" });
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      selected: null,
      detail: null,
      detailLoading: false,
      events: null,
      eventsError: null,
    };
    await load();
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, error: errorText(error) };
  }
}

/** Read and render the selected model stage's event batch and coverage. */
export async function loadEvents(timeframe: string): Promise<void> {
  const at = generation;
  const detail = current.detail;
  current = {
    ...current,
    eventsLoading: true,
    eventsError: null,
    eventsTimeframe: timeframe,
  };
  if (detail === null || detail.artifact === null) {
    current = {
      ...current,
      eventsLoading: false,
      eventsError: "The stage has not produced an artifact.",
    };
    return;
  }
  const eventsFile = detail.artifact.files.find((file) => file.name === "events.csv");
  const reportFile = detail.artifact.files.find((file) => file.name === "report.json");
  if (eventsFile === undefined) {
    current = {
      ...current,
      eventsLoading: false,
      eventsError: "The stage artifact has no events.csv.",
    };
    return;
  }
  try {
    const eventsBytes = await downloadStageArtifact(detail.stage_run_id, eventsFile);
    const reportBytes =
      reportFile === undefined
        ? null
        : await downloadStageArtifact(detail.stage_run_id, reportFile);
    if (at !== generation) {
      return;
    }
    const text = new TextDecoder().decode(eventsBytes);
    const report = reportBytes === null ? null : new TextDecoder().decode(reportBytes);
    current = {
      ...current,
      events: stageEventView(detail.stage_run_id, timeframe, text, report),
      eventsLoading: false,
    };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      events: null,
      eventsLoading: false,
      eventsError: errorText(error),
    };
  }
}
