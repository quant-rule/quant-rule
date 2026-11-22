/** A `sha256:<64 hex>` content digest. */
export type Digest = string;

export interface ServiceHealth {
  name: string;
  state: "healthy" | "degraded" | "down";
  detail: string | null;
}

export interface HealthReport {
  v: string;
  /** The service that produced the report; `/ready` uses it to reject a mismatch. */
  service: string;
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

/** One completed startup retention cleanup. */
export interface CleanupReport {
  completed_utc: string;
  stages_removed: number;
  objects_removed: number;
  error: string | null;
}

/** The evidence service's retention and cleanup snapshot. */
export interface OperationsReport {
  stage_retention_days: number;
  stage_store: boolean;
  last_cleanup: CleanupReport | null;
}

/** One dataset in a list. */
export interface DatasetSummary {
  data_version: Digest;
  name: string;
  candles: number;
  first_bar: number;
  last_bar: number;
  bytes: number;
  archived_at?: string;
}

/** One row skipped by a tolerant ingest. */
export interface SkippedRow {
  line: number;
  reason: "field_count" | "invalid" | "misaligned" | "duplicate" | "out_of_order";
}

/** The quality report of a tolerant ingest. */
export interface QualityReport {
  rows_read: number;
  accepted: number;
  rejected: number;
  duplicates: number;
  out_of_order: number;
  invalid: number;
  skipped: SkippedRow[];
}

/** A dataset record, including where its bytes live. */
export interface DatasetRecord {
  summary: DatasetSummary;
  object_key: string;
  sha256: Digest;
  quality: QualityReport | null;
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
  archived_at?: string;
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
  model_artifact_id: Digest | null;
  provenance: Provenance;
  artifacts: ArtifactRef[];
  digest: Digest | null;
  error: string | null;
}

/** The role one cited object plays in a run. */
export type ReferenceKind =
  | "model_artifact"
  | "strategy_pack"
  | "signal_artifact"
  | "portfolio_policy"
  | "execution_model";

/** The pack metadata a lineage record keeps as provenance. */
export interface PackProvenance {
  kind: "strategy" | "portfolio" | "execution";
  name: string;
  version: string;
  owner: string;
  state: "draft" | "validated" | "frozen" | "retired";
}

/** One object a run cited, with pack provenance when it names a pack. */
export interface RunArtifactReference {
  kind: ReferenceKind;
  id: Digest;
  hash: Digest;
  pack?: PackProvenance;
}

/** The research artifact references a run recorded. */
export interface RunLineage {
  run_id: Digest;
  references: RunArtifactReference[];
  recorded_at: string;
}

/** One timeframe's bars and coverage in an inspection. */
export interface TimeframeBars {
  timeframe: string;
  bars: number;
  partial_buckets: number;
  first_bar: number | null;
  last_bar: number | null;
}

/** A dataset's timeframe ladder, counted from its stored bytes. */
export interface TimeframeInspection {
  data_version: Digest;
  timeframes: TimeframeBars[];
}

/** One published event record, as the gateway serializes it. */
export interface InspectEvent {
  event_code?: number;
  event_type?: number;
  event_subtype?: number;
  type?: number;
  source?: number;
  occurred_bar?: number;
  available_bar?: number;
  bar?: number;
  direction?: number;
  [key: string]: unknown;
}

/** The event batch for one dataset and timeframe. */
export interface InspectEvents {
  data_version: Digest;
  timeframe: string;
  events: InspectEvent[];
}

/** The recorded canonical digest of one run. */
export interface RunDigest {
  run_id: Digest;
  digest: Digest;
}

/** Which engine a diagnostic stage runs. */
export type StageKind = "model" | "strategy" | "portfolio";

/** The lifecycle of a diagnostic stage. */
export type StageState = "queued" | "running" | "succeeded" | "failed" | "cancelled";

/** One input a stage consumes. */
export type StageInput =
  | { kind: "data_version"; data_version: Digest }
  | { kind: "model_artifact"; model_artifact_id: Digest }
  | { kind: "decisions_artifact"; decisions_artifact_id: Digest };

/** The provenance of one stage run. */
export interface StageProvenance {
  data_version: Digest;
  config_hash: Digest;
  model_version: string;
  image_digest: Digest;
}

/** A content-addressed stage artifact bundle. */
export interface StageArtifact {
  artifact_id: Digest;
  files: ArtifactRef[];
}

/** One diagnostic stage in a list. */
export interface StageRunSummary {
  stage_run_id: Digest;
  kind: StageKind;
  state: StageState;
  config_id: string;
  submitted_at: string;
  artifact_id: Digest | null;
  evidence: false;
}

/** One diagnostic stage in detail. */
export interface StageRunDetail {
  stage_run_id: Digest;
  kind: StageKind;
  state: StageState;
  owner: string;
  config_id: string;
  input: StageInput;
  provenance: StageProvenance;
  artifact: StageArtifact | null;
  submitted_at: string;
  error: string | null;
  attempts: number;
  evidence: false;
}

/** The response to a stage submission. */
export interface StageSubmitResponse {
  stage_run_id: Digest;
  state: StageState;
  created: boolean;
}

/** One event row from a model stage's events.csv. */
export type StageEventRow = Record<string, string>;

/** The filtered event batch and report coverage of one model stage. */
export interface StageEventView {
  stage_run_id: Digest;
  timeframe: string;
  columns: string[];
  events: StageEventRow[];
  coverage: StageCoverage[];
}

/** One published strategy decision record. */
export interface StageDecision {
  schema: string;
  timeframe_secs: number;
  bar_index: number;
  open_time: number;
  available_bar_index: number;
  position_before: "flat" | "long" | "short";
  position_after: "flat" | "long" | "short";
  decision: "hold" | "enter" | "enter_bracket" | "exit";
  side: "long" | "short" | null;
  reason: string | null;
  intent: unknown;
  atr14: number | null;
  [key: string]: unknown;
}

/** One published portfolio metrics record. */
export interface PortfolioMetrics {
  schema: string;
  initial_cash: number;
  final_equity: number;
  net_pnl: number;
  execution_fees: number;
  commission: number;
  funding_paid: number;
  funding_received: number;
  max_drawdown: number;
  exposure: number | null;
  halted: boolean;
  [key: string]: unknown;
}

/** One comma-separated published artifact. */
export interface PortfolioTable {
  columns: string[];
  rows: string[][];
}

/** One line of a published summary. */
export interface PortfolioSummaryEntry {
  label: string;
  value: string;
}

/** The published execution and portfolio view. */
export interface PortfolioView {
  orders: PortfolioTable;
  fills: PortfolioTable;
  rejections: PortfolioTable;
  trades: PortfolioTable;
  executionSummary: PortfolioSummaryEntry[];
  riskEvents: PortfolioTable;
  metrics: PortfolioMetrics;
  summary: PortfolioSummaryEntry[];
}

/** One coverage row from a model stage's report.json. */
export interface StageCoverage {
  timeframe_secs: number;
  bars: number | null;
  expected_bars: number | null;
  coverage_share: number | null;
}
