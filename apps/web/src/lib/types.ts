/** A `sha256:<64 hex>` content digest. */
export type Digest = string;

export interface ServiceHealth {
  name: string;
  state: "healthy" | "degraded" | "down";
  detail: string | null;
}

export interface HealthReport {
  v: string;
  state: "healthy" | "degraded" | "down";
  services: ServiceHealth[];
  queue_depth: number;
  workers: number;
  image_digest: Digest | null;
}

export interface VersionInfo {
  product: string;
  api: string;
  image_digest: Digest | null;
  contracts: [string, string][];
}

/** One dataset in a list. */
export interface DatasetSummary {
  data_version: Digest;
  name: string;
  candles: number;
  first_bar: number;
  last_bar: number;
  bytes: number;
}

/** A dataset record, including where its bytes live. */
export interface DatasetRecord {
  summary: DatasetSummary;
  object_key: string;
  sha256: Digest;
  created_at: string;
}

/** One profile in a list. */
export interface ProfileSummary {
  profile_id: string;
  name: string;
  datasets: number;
  runs: number;
  simulations: number;
}
