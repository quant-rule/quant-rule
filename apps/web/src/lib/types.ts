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

/** The lifecycle state of a run. */
export type RunState = "queued" | "running" | "succeeded" | "failed" | "cancelled";

/** One run in a list. */
export interface RunSummary {
  run_id: Digest;
  status: RunState;
  config_id: string;
  data_version: Digest;
  submitted_at: string;
  elapsed_ms: number | null;
}

/** Where a configuration came from. */
export type ConfigOrigin = "repository" | "upload";

/** One configuration in a list. */
export interface ConfigSummary {
  config_id: string;
  name: string;
  origin: ConfigOrigin;
  valid: boolean;
  message: string | null;
  config_hash: Digest | null;
}

/** The response to a run submission. */
export interface SubmitRunResponse {
  run_id: Digest;
  status: RunState;
  created: boolean;
}

/** One data-plane stage. */
export type StageName = "data" | "model" | "backtest" | "evidence";

/** The state of one stage. */
export type StageStatus = "pending" | "running" | "done" | "failed" | "skipped";

export interface StageProgress {
  name: StageName;
  state: StageStatus;
  elapsed_ms: number | null;
}

export interface Provenance {
  data_version: Digest;
  config_hash: Digest;
  model_version: string;
  image_digest: Digest;
}

export interface ArtifactRef {
  name: string;
  bytes: number;
  sha256: Digest;
}

/** One run in detail. */
export interface RunDetail {
  run_id: Digest;
  status: RunState;
  owner: string;
  stages: StageProgress[];
  provenance: Provenance;
  artifacts: ArtifactRef[];
  digest: Digest | null;
  error: string | null;
}
