//! Serializable views returned by the desktop commands.

use quantrule_app::memory::{
    ClaimRecord, MetricRecord, MetricValue, NoteRecord, RunRecord, WorkspaceRecord,
};
use quantrule_app::service::{ModelRunSummary, RunSummary, TimeframeModelSummary};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct DatasetRefView {
    pub(super) name: String,
    pub(super) path: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceView {
    pub(super) root: String,
    pub(super) name: String,
    pub(super) datasets: Vec<DatasetRefView>,
    pub(super) runs: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DatasetSummaryView {
    pub(super) name: String,
    pub(super) path: String,
    pub(super) candles: usize,
    pub(super) first_open_time: Option<i64>,
    pub(super) last_open_time: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct RunSummaryView {
    pub(super) name: String,
    pub(super) output_root: String,
    pub(super) config_hash: String,
    pub(super) model_id: Option<String>,
    pub(super) execution_id: Option<String>,
    pub(super) portfolio_id: Option<String>,
    pub(super) instrument: String,
    pub(super) data_version: String,
    pub(super) data_source: String,
    pub(super) data_timeframes: Vec<String>,
    pub(super) model_timeframes: Vec<String>,
    pub(super) execution_fill_timeframe: String,
    pub(super) initial_cash: f64,
    pub(super) leverage: f64,
    pub(super) max_open_positions: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimeframeView {
    pub(super) timeframe_secs: u64,
    pub(super) bars: usize,
    pub(super) events: usize,
    pub(super) levels: usize,
}

#[derive(Debug, Deserialize)]
pub struct ModelSummaryView {
    pub(super) timeframes: Vec<TimeframeView>,
    pub(super) events: usize,
}

impl From<ModelSummaryView> for ModelRunSummary {
    fn from(view: ModelSummaryView) -> Self {
        Self {
            timeframes: view
                .timeframes
                .into_iter()
                .map(|timeframe| TimeframeModelSummary {
                    timeframe_secs: timeframe.timeframe_secs,
                    bars: timeframe.bars,
                    events: timeframe.events,
                    levels: timeframe.levels,
                })
                .collect(),
            events: view.events,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ModelRunView {
    pub(super) run: RunSummaryView,
    pub(super) timeframes: Vec<TimeframeView>,
    pub(super) events: usize,
}

#[derive(Debug, Serialize)]
pub struct MemoryStatusView {
    pub(super) schema: String,
    pub(super) version: i64,
    pub(super) path: String,
}

#[derive(Debug, Serialize)]
pub struct WorkspaceRecordView {
    pub(super) id: i64,
    pub(super) root: String,
    pub(super) name: String,
    pub(super) created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct RunRecordView {
    pub(super) id: i64,
    pub(super) workspace_id: i64,
    pub(super) config_id: Option<i64>,
    pub(super) run_key: String,
    pub(super) data_version: String,
    pub(super) config_hash: String,
    pub(super) status: String,
    pub(super) started_at: i64,
    pub(super) finished_at: Option<i64>,
    pub(super) summary_json: String,
}

#[derive(Debug, Serialize)]
pub struct MetricView {
    pub(super) id: i64,
    pub(super) run_id: i64,
    pub(super) name: String,
    pub(super) kind: String,
    pub(super) value: Value,
}

#[derive(Debug, Serialize)]
pub struct ClaimView {
    pub(super) id: i64,
    pub(super) run_id: i64,
    pub(super) claim_type: String,
    pub(super) statement: String,
    pub(super) status: String,
    pub(super) created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct NoteView {
    pub(super) id: i64,
    pub(super) workspace_id: i64,
    pub(super) run_id: Option<i64>,
    pub(super) body: String,
    pub(super) created_at: i64,
    pub(super) updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct RecordedRunView {
    pub(super) workspace_id: i64,
    pub(super) dataset_id: i64,
    pub(super) config_id: i64,
    pub(super) run_id: i64,
}

impl From<RunSummary> for RunSummaryView {
    fn from(summary: RunSummary) -> Self {
        Self {
            name: summary.name,
            output_root: summary.output_root.to_string_lossy().into_owned(),
            config_hash: summary.config_hash,
            model_id: summary.model_id,
            execution_id: summary.execution_id,
            portfolio_id: summary.portfolio_id,
            instrument: summary.instrument,
            data_version: summary.data_version,
            data_source: summary.data_source.to_string_lossy().into_owned(),
            data_timeframes: summary.data_timeframes,
            model_timeframes: summary.model_timeframes,
            execution_fill_timeframe: summary.execution_fill_timeframe,
            initial_cash: summary.initial_cash,
            leverage: summary.leverage,
            max_open_positions: summary.max_open_positions,
        }
    }
}

impl From<WorkspaceRecord> for WorkspaceRecordView {
    fn from(record: WorkspaceRecord) -> Self {
        Self {
            id: record.id,
            root: record.root,
            name: record.name,
            created_at: record.created_at,
        }
    }
}

impl From<RunRecord> for RunRecordView {
    fn from(record: RunRecord) -> Self {
        Self {
            id: record.id,
            workspace_id: record.workspace_id,
            config_id: record.config_id,
            run_key: record.run_key,
            data_version: record.data_version,
            config_hash: record.config_hash,
            status: record.status,
            started_at: record.started_at,
            finished_at: record.finished_at,
            summary_json: record.summary_json,
        }
    }
}

impl From<ClaimRecord> for ClaimView {
    fn from(record: ClaimRecord) -> Self {
        Self {
            id: record.id,
            run_id: record.run_id,
            claim_type: record.claim_type,
            statement: record.statement,
            status: record.status,
            created_at: record.created_at,
        }
    }
}

impl From<NoteRecord> for NoteView {
    fn from(record: NoteRecord) -> Self {
        Self {
            id: record.id,
            workspace_id: record.workspace_id,
            run_id: record.run_id,
            body: record.body,
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

pub(super) fn metric_view(record: MetricRecord) -> MetricView {
    let (kind, value) = match record.value {
        MetricValue::Real(number) => ("real".to_owned(), Value::from(number)),
        MetricValue::Integer(number) => ("integer".to_owned(), Value::from(number)),
        MetricValue::Text(text) => ("text".to_owned(), Value::String(text)),
        MetricValue::Null => ("null".to_owned(), Value::Null),
    };
    MetricView {
        id: record.id,
        run_id: record.run_id,
        name: record.name,
        kind,
        value,
    }
}
