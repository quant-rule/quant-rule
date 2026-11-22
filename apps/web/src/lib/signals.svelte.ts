import { ApiError, errorText } from "./api";
import {
  createSignal,
  fetchSignal,
  listSignals,
  type ArtifactRecord,
  type SignalRequest,
  type SignalResponse,
} from "./signals";

/** What the Signals screen knows. */
export interface Signals {
  items: ArtifactRecord[];
  loading: boolean;
  error: string | null;
  selected: string | null;
  detail: ArtifactRecord | null;
  detailLoading: boolean;
  creating: boolean;
  created: SignalResponse | null;
  unavailable: boolean;
}

const EMPTY: Signals = {
  items: [],
  loading: false,
  error: null,
  selected: null,
  detail: null,
  detailLoading: false,
  creating: false,
  created: null,
  unavailable: false,
};

let current = $state<Signals>({ ...EMPTY });

/** A counter that makes an in-flight read from a previous profile stale. */
let generation = 0;

/** Drop every row and the open detail; the next read belongs to the active profile. */
export function reset(): void {
  generation += 1;
  current = { ...EMPTY };
}

/** Read the current signals state. */
export function signals(): Signals {
  return current;
}

/** Read the profile's signal metadata. */
export async function load(): Promise<void> {
  const at = generation;
  current = { ...current, loading: current.items.length === 0, error: null };
  try {
    const items = await listSignals();
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

/** Read one signal record. */
export async function select(signalId: string): Promise<void> {
  const at = generation;
  current = { ...current, selected: signalId, detailLoading: true, error: null };
  try {
    const detail = await fetchSignal(signalId);
    if (at !== generation || current.selected !== signalId) {
      return;
    }
    current = { ...current, detail, detailLoading: false };
  } catch (error) {
    if (at !== generation || current.selected !== signalId) {
      return;
    }
    current = { ...current, detail: null, detailLoading: false, error: errorText(error) };
  }
}

/**
 * Ask the strategy service for one signal.
 *
 * A deployment without the strategy service answers 503. That is a recorded
 * deployment gap, not a malformed request, so the screen disables create and
 * states it; every other failure keeps its stable code.
 */
export async function create(request: SignalRequest): Promise<void> {
  const at = generation;
  current = { ...current, creating: true, created: null, error: null };
  try {
    const created = await createSignal(request);
    if (at !== generation) {
      return;
    }
    current = {
      ...current,
      creating: false,
      created,
      detail: created.artifact,
      selected: created.artifact.summary.artifact_id,
    };
    await load();
  } catch (error) {
    if (at !== generation) {
      return;
    }
    if (
      error instanceof ApiError &&
      (error.status === 503 || error.code === "service.unavailable")
    ) {
      current = {
        ...current,
        creating: false,
        unavailable: true,
        error:
          "signal create is unavailable: this deployment does not run the strategy service (503 service.unavailable)",
      };
      return;
    }
    current = { ...current, creating: false, error: errorText(error) };
  }
}
