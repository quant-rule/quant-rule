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

/** Read the current configs state. */
export function configs(): Configs {
  return current;
}

/** Read the server's config catalog. */
export async function load(): Promise<void> {
  current = { ...current, loading: current.items.length === 0, error: null };
  try {
    const items = await call<ConfigSummary[]>("/catalog/configs");
    current = { ...current, items, loading: false, error: null };
  } catch (error) {
    current = { ...current, items: [], loading: false, error: errorText(error) };
  }
}

/** Submit a run for one valid config and dataset. */
export async function submit(configId: string, dataVersion: Digest): Promise<void> {
  current = { ...current, submitting: configId, error: null, lastRun: null };
  try {
    const response = await call<SubmitRunResponse>("/runs", {
      method: "POST",
      body: JSON.stringify({ data_version: dataVersion, config_id: configId }),
    });
    current = { ...current, submitting: null, lastRun: response.run_id };
  } catch (error) {
    current = { ...current, submitting: null, error: errorText(error) };
  }
}
