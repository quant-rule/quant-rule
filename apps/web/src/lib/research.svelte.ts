/**
 * The research screen's one server-resource store.
 *
 * Submission, comparison, and decision are explicit actions. Nothing here
 * submits or accepts automatically, and a failed read drops stale research
 * instead of presenting it as current.
 */

import { errorText } from "./api";
import {
  compareRun,
  createRun,
  decideRun,
  fetchBaseline,
  fetchRun,
  listBaselines,
  listInvalidations,
  listRuns,
  submitRun,
  type BaselineRecord,
  type CreateImprovement,
  type ImprovementComparison,
  type ImprovementDecision,
  type ImprovementRunDetail,
  type ImprovementRunSummary,
  type ImprovementTier,
  type InvalidationRecord,
} from "./research";

export interface ResearchState {
  items: ImprovementRunSummary[];
  loading: boolean;
  error: string | null;
  selected: string | null;
  detail: ImprovementRunDetail | null;
  comparison: ImprovementComparison | null;
  baselines: BaselineRecord[];
  baseline: BaselineRecord | null;
  invalidations: InvalidationRecord[];
  creating: boolean;
  submitting: boolean;
  deciding: boolean;
}

const initial = (): ResearchState => ({
  items: [],
  loading: false,
  error: null,
  selected: null,
  detail: null,
  comparison: null,
  baselines: [],
  baseline: null,
  invalidations: [],
  creating: false,
  submitting: false,
  deciding: false,
});

let current = $state<ResearchState>(initial());
let generation = 0;

/** Drop the screen; the next read belongs to the active profile. */
export function reset(): void {
  generation += 1;
  current = initial();
}

/** Read the current screen state. */
export function research(): ResearchState {
  return current;
}

/** Read the list and both registries. */
export async function load(): Promise<void> {
  const at = generation;
  current = { ...current, loading: current.items.length === 0, error: null };
  try {
    const [items, baselines, invalidations] = await Promise.all([
      listRuns({ limit: 100 }),
      listBaselines(),
      listInvalidations(),
    ]);
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      items,
      baselines,
      invalidations,
      loading: false,
      error: null,
    };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...initial(), error: errorText(error) };
  }
}

/** Open one run. */
export async function select(improvementId: string): Promise<void> {
  const at = generation;
  current = {
    ...current,
    selected: improvementId,
    detail: null,
    comparison: null,
    error: null,
  };
  try {
    const detail = await fetchRun(improvementId);
    if (at !== generation) {
      return;
    }
    current = { ...current, detail };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, error: errorText(error) };
  }
}

/** Create one pre-registration and select it. */
export async function create(input: CreateImprovement): Promise<boolean> {
  const at = generation;
  current = { ...current, creating: true, error: null };
  try {
    const detail = await createRun(input);
    if (at !== generation) {
      return false;
    }
    current = {
      ...current,
      creating: false,
      selected: detail.improvement_id,
      detail,
      comparison: null,
    };
    await load();
    return true;
  } catch (error) {
    if (at !== generation) {
      return false;
    }
    current = { ...current, creating: false, error: errorText(error) };
    return false;
  }
}

/** Submit the coupled run and read the new record. */
export async function submit(improvementId: string): Promise<void> {
  const at = generation;
  current = { ...current, submitting: true, error: null };
  try {
    const detail = await submitRun(improvementId);
    if (at !== generation) {
      return;
    }
    current = { ...current, submitting: false, detail };
    await load();
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, submitting: false, error: errorText(error) };
  }
}

/** Read the comparison table. */
export async function compare(
  improvementId: string,
  baseline?: string,
): Promise<void> {
  const at = generation;
  current = { ...current, error: null, comparison: null };
  try {
    const comparison = await compareRun(improvementId, baseline);
    if (at !== generation) {
      return;
    }
    current = { ...current, comparison };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, error: errorText(error) };
  }
}

/** Record one explicit human decision and reload the registries. */
export async function decide(
  improvementId: string,
  decision: ImprovementDecision,
  note: string,
): Promise<boolean> {
  const at = generation;
  current = { ...current, deciding: true, error: null };
  try {
    const detail = await decideRun(improvementId, decision, note);
    if (at !== generation) {
      return false;
    }
    current = { ...current, deciding: false, detail };
    await load();
    return true;
  } catch (error) {
    if (at !== generation) {
      return false;
    }
    current = { ...current, deciding: false, error: errorText(error) };
    return false;
  }
}

/** Read one tier baseline and retain it for the detail panel. */
export async function showBaseline(tier: ImprovementTier): Promise<void> {
  const at = generation;
  current = { ...current, error: null, baseline: null };
  try {
    const baseline = await fetchBaseline(tier);
    if (at !== generation) {
      return;
    }
    current = { ...current, baseline };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, error: errorText(error) };
  }
}
