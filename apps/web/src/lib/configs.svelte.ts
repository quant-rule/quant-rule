import { call, errorText } from "./api";
import type { ConfigSummary, Digest, SubmitRunResponse } from "./types";

/** What the Configs screen knows. */
export interface Configs {
  items: ConfigSummary[];
  loading: boolean;
  error: string | null;
  submitting: string | null;
  lastRun: Digest | null;
}

let current = $state<Configs>({
  items: [],
  loading: false,
  error: null,
  submitting: null,
  lastRun: null,
});

/** A counter that makes an in-flight read from a previous profile stale. */
let generation = 0;

/** Drop every row; the next read belongs to the active profile. */
export function reset(): void {
  generation += 1;
  current = {
    items: [],
    loading: false,
    error: null,
    submitting: null,
    lastRun: null,
  };
}

/** Read the current configs state. */
export function configs(): Configs {
  return current;
}

/** Read the server's config catalog. */
export async function load(): Promise<void> {
  const at = generation;
  current = { ...current, loading: current.items.length === 0, error: null };
  try {
    const items = await call<ConfigSummary[]>("/catalog/configs");
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

/** Submit a run for one valid config and dataset. */
export async function submit(configId: string, dataVersion: Digest): Promise<void> {
  const at = generation;
  current = { ...current, submitting: configId, error: null, lastRun: null };
  try {
    const response = await call<SubmitRunResponse>("/runs", {
      method: "POST",
      body: JSON.stringify({ data_version: dataVersion, config_id: configId }),
    });
    if (at !== generation) {
      return;
    }
    current = { ...current, submitting: null, lastRun: response.run_id };
  } catch (error) {
    if (at !== generation) {
      return;
    }
    current = { ...current, submitting: null, error: errorText(error) };
  }
}
