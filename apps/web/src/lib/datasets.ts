import { call } from "./api";
import type { DatasetRecord } from "./types";

/** The one-dataset path; the version is an opaque content identifier. */
export function datasetPath(dataVersion: string): string {
  return `/catalog/datasets/${encodeURIComponent(dataVersion)}`;
}

/** Read one dataset record, including its object identity and quality. */
export function datasetRecord(dataVersion: string): Promise<DatasetRecord> {
  return call<DatasetRecord>(datasetPath(dataVersion));
}
