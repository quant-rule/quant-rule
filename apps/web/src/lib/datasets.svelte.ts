import { call, errorText, uploadDataset } from "./api";
import type { DatasetSummary, QualityReport } from "./types";

/** What the Datasets screen knows. */
export interface Datasets {
  items: DatasetSummary[];
  loading: boolean;
  uploading: boolean;
  error: string | null;
  lastQuality: QualityReport | null;
}

let current = $state<Datasets>({
  items: [],
  loading: false,
  uploading: false,
  error: null,
  lastQuality: null,
});

/** Read the current datasets state. */
export function datasets(): Datasets {
  return current;
}

/** Load the profile's datasets. A failure shows the code and drops stale rows. */
export async function load(): Promise<void> {
  current = { ...current, loading: true, error: null, lastQuality: null };
  try {
    const items = await call<DatasetSummary[]>("/catalog/datasets");
    current = { ...current, items, loading: false };
  } catch (error) {
    current = { ...current, items: [], loading: false, error: errorText(error) };
  }
}

/** Upload one CSV, then reload the list so the server's record is what shows. */
export async function upload(file: File, tolerant = false): Promise<void> {
  current = { ...current, uploading: true, error: null, lastQuality: null };
  try {
    const record = await uploadDataset(file, { tolerant });
    current = { ...current, uploading: false };
    await load();
    current = { ...current, lastQuality: record.quality };
  } catch (error) {
    current = { ...current, uploading: false, error: errorText(error) };
  }
}
