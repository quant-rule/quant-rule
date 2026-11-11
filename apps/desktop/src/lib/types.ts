export interface DatasetRef {
  name: string;
  path: string;
}

export interface WorkspaceView {
  root: string;
  name: string;
  datasets: DatasetRef[];
  runs: string[];
}

export interface DatasetSummaryView {
  name: string;
  path: string;
  candles: number;
  first_open_time: number | null;
  last_open_time: number | null;
}

export interface RunSummaryView {
  name: string;
  output_root: string;
  config_hash: string;
  model_id: string | null;
  execution_id: string | null;
  portfolio_id: string | null;
  instrument: string;
  data_version: string;
  data_source: string;
  data_timeframes: string[];
  model_timeframes: string[];
  execution_fill_timeframe: string;
  initial_cash: number;
  leverage: number;
  max_open_positions: number;
}

export interface TimeframeView {
  timeframe_secs: number;
  bars: number;
  events: number;
  levels: number;
}

export interface ModelRunView {
  run: RunSummaryView;
  timeframes: TimeframeView[];
  events: number;
}

export interface RecordedRunView {
  workspace_id: number;
  dataset_id: number;
  config_id: number;
  run_id: number;
}

export interface MemoryStatusView {
  schema: string;
  version: number;
  path: string;
}

export interface WorkspaceRecordView {
  id: number;
  root: string;
  name: string;
  created_at: number;
}

export interface RunRecordView {
  id: number;
  workspace_id: number;
  config_id: number | null;
  run_key: string;
  data_version: string;
  config_hash: string;
  status: string;
  started_at: number;
  finished_at: number | null;
  summary_json: string;
}

export interface MetricView {
  id: number;
  run_id: number;
  name: string;
  kind: string;
  value: number | string | null;
}

export interface ClaimView {
  id: number;
  run_id: number;
  claim_type: string;
  statement: string;
  status: string;
  created_at: number;
}

export interface NoteView {
  id: number;
  workspace_id: number;
  run_id: number | null;
  body: string;
  created_at: number;
  updated_at: number;
}
