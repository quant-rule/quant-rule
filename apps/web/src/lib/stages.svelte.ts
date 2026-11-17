import { call, callVoid, errorText } from "./api";
import { portfolioView } from "./portfolio";
import {
  downloadStageArtifact,
  explainDecision,
  stageDecisions,
  stageEventView,
  stagePath,
} from "./stage";
import type {
  PortfolioView,
  StageDecision,
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
  explain: StageDecision | null;
  explainError: string | null;
  explainLoading: boolean;
  explainBar: number;
  decisions: StageDecision[];
  decisionsError: string | null;
  decisionsLoading: boolean;
  portfolio: PortfolioView | null;
  portfolioError: string | null;
  portfolioLoading: boolean;
}

const PORTFOLIO_FILES = [
  "execution/orders.csv",
  "execution/fills.csv",
  "execution/rejections.csv",
  "execution/summary.txt",
  "portfolio/trades.csv",
  "portfolio/risk_events.csv",
  "portfolio/metrics.json",
  "portfolio/summary.txt",
];

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
  explain: null,
  explainError: null,
  explainLoading: false,
  explainBar: 0,
  decisions: [],
  decisionsError: null,
  decisionsLoading: false,
  portfolio: null,
  portfolioError: null,
  portfolioLoading: false,
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
    explain: null,
    explainError: null,
    explainLoading: false,
    explainBar: 0,
    decisions: [],
    decisionsError: null,
    decisionsLoading: false,
    portfolio: null,
    portfolioError: null,
    portfolioLoading: false,
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

/** Submit one strategy stage, then reload the list. */
export async function submitStrategy(
  modelArtifactId: string,
  configId: string,
): Promise<StageSubmitResponse | null> {
  const at = generation;
  current = { ...current, error: null };
  try {
    const response = await call<StageSubmitResponse>("/stages/strategy/runs", {
      method: "POST",
      body: JSON.stringify({
        kind: "strategy",
        config_id: configId,
        input: {
          kind: "model_artifact",
          model_artifact_id: modelArtifactId,
        },
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

/** Submit one portfolio stage, then reload the list. */
export async function submitPortfolio(
  decisionsArtifactId: string,
  configId: string,
): Promise<StageSubmitResponse | null> {
  const at = generation;
  current = { ...current, error: null };
  try {
    const response = await call<StageSubmitResponse>("/stages/portfolio/runs", {
      method: "POST",
      body: JSON.stringify({
        kind: "portfolio",
        config_id: configId,
        input: {
          kind: "decisions_artifact",
          decisions_artifact_id: decisionsArtifactId,
        },
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
  current = {
    ...current,
    detailLoading: true,
    error: null,
    decisions: [],
    decisionsError: null,
    decisionsLoading: false,
    portfolio: null,
    portfolioError: null,
    portfolioLoading: false,
  };
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
      explain: null,
      explainError: null,
      decisions: [],
      decisionsError: null,
      decisionsLoading: false,
    };
    if (detail.kind === "strategy") {
      await loadDecisions();
    } else if (detail.kind === "portfolio") {
      await loadPortfolio();
    }
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
    explain: null,
    explainError: null,
    decisions: [],
    decisionsError: null,
    decisionsLoading: false,
    portfolio: null,
    portfolioError: null,
    portfolioLoading: false,
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
    explain: null,
    explainError: null,
    decisions: [],
    decisionsError: null,
    decisionsLoading: false,
    portfolio: null,
    portfolioError: null,
    portfolioLoading: false,
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
      explain: null,
      explainError: null,
      decisions: [],
      decisionsError: null,
      decisionsLoading: false,
      portfolio: null,
      portfolioError: null,
      portfolioLoading: false,
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

/** Read the published execution and portfolio bundles for the open stage. */
export async function loadPortfolio(): Promise<void> {
  const at = generation;
  const detail = current.detail;
  current = {
    ...current,
    portfolioLoading: true,
    portfolioError: null,
    portfolio: null,
  };
  if (detail === null || detail.kind !== "portfolio" || detail.artifact === null) {
    current = { ...current, portfolioLoading: false };
    return;
  }
  const texts: Record<string, string> = {};
  try {
    for (const name of PORTFOLIO_FILES) {
      const file = detail.artifact.files.find((artifact) => artifact.name === name);
      if (file === undefined) {
        throw new Error(`the stage artifact has no ${name}`);
      }
      const bytes = await downloadStageArtifact(detail.stage_run_id, file);
      if (at !== generation) {
        return;
      }
      texts[name] = new TextDecoder().decode(bytes);
    }
    current = {
      ...current,
      portfolio: portfolioView(texts),
      portfolioLoading: false,
    };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      portfolio: null,
      portfolioLoading: false,
      portfolioError: errorText(error),
    };
  }
}

/** Read the published decision table for the open stage. */
export async function loadDecisions(): Promise<void> {
  const at = generation;
  const detail = current.detail;
  current = { ...current, decisionsLoading: true, decisionsError: null };
  if (detail === null || detail.kind !== "strategy" || detail.artifact === null) {
    current = { ...current, decisionsLoading: false, decisions: [] };
    return;
  }
  const decisionsFile = detail.artifact.files.find(
    (file) => file.name === "decisions.ndjson",
  );
  if (decisionsFile === undefined) {
    current = { ...current, decisions: [], decisionsLoading: false };
    return;
  }
  try {
    const bytes = await downloadStageArtifact(detail.stage_run_id, decisionsFile);
    if (at !== generation) {
      return;
    }
    const decisions = stageDecisions(new TextDecoder().decode(bytes));
    current = { ...current, decisions, decisionsLoading: false };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      decisions: [],
      decisionsLoading: false,
      decisionsError: errorText(error),
    };
  }
}

/** Read one published strategy decision without recomputing it. */
export async function explain(stageRunId: string, bar: number): Promise<void> {
  const at = generation;
  const detail = current.detail;
  current = {
    ...current,
    explainLoading: true,
    explainError: null,
    explainBar: bar,
    explain: null,
  };
  if (detail === null || detail.artifact === null) {
    current = {
      ...current,
      explainLoading: false,
      explainError: "The stage has not produced an artifact.",
    };
    return;
  }
  const decisionsFile = detail.artifact.files.find(
    (file) => file.name === "decisions.ndjson",
  );
  if (decisionsFile === undefined) {
    current = {
      ...current,
      explainLoading: false,
      explainError: "The stage artifact has no decisions.ndjson.",
    };
    return;
  }
  try {
    const bytes = await downloadStageArtifact(stageRunId, decisionsFile);
    if (at !== generation) {
      return;
    }
    const text = new TextDecoder().decode(bytes);
    const decision = explainDecision(stageDecisions(text), bar);
    current = { ...current, explain: decision, explainLoading: false };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      explain: null,
      explainLoading: false,
      explainError: errorText(error),
    };
  }
}
