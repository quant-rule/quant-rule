import { ApiError, call, uploadDataset } from "./api";
import type { DatasetSummary } from "./types";

/** What the Datasets screen knows. */
export interface Datasets {
  items: DatasetSummary[];
  loading: boolean;
  uploading: boolean;
  error: string | null;
}

let current = $state<Datasets>({
  items: [],
  loading: false,
  uploading: false,
  error: null,
});

/** Read the current datasets state. */
export function datasets(): Datasets {
  return current;
}

function describe(error: unknown): string {
  if (error instanceof ApiError) {
    return `${error.code}: ${error.message}`;
  }
  return error instanceof Error ? error.message : String(error);
}

/** Load the profile's datasets. A failure shows the code and drops stale rows. */
export async function load(): Promise<void> {
  current = { ...current, loading: true, error: null };
  try {
    const items = await call<DatasetSummary[]>("/catalog/datasets");
    current = { ...current, items, loading: false };
  } catch (error) {
    current = { ...current, items: [], loading: false, error: describe(error) };
  }
}

/** Upload one CSV, then reload the list so the server's record is what shows. */
export async function upload(file: File): Promise<void> {
  current = { ...current, uploading: true, error: null };
  try {
    await uploadDataset(file);
    current = { ...current, uploading: false };
    await load();
  } catch (error) {
    current = { ...current, uploading: false, error: describe(error) };
  }
}
