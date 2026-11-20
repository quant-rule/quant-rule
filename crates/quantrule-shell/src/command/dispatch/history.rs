//! Runtime-history list, archive, and erase handlers.

use std::io::Write;

use quantrule_api::{
    DatasetRecord, DatasetSummary, EraseDatasetResponse, EraseReceipt, ImprovementRunSummary,
    RunSummary, SimulationEraseResponse, SimulationSummary, StageRunSummary,
};

use super::path::with_query;
use crate::client::{Client, encode};
use crate::command::types::{GlobalOptions, HistoryKind};
use crate::error::RunError;
use crate::output;

pub(super) fn history_list(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    kind: HistoryKind,
    include_archived: bool,
    limit: Option<usize>,
) -> Result<(), RunError> {
    match kind {
        HistoryKind::Run => {
            let mut query = Vec::new();
            if include_archived {
                query.push(String::from("include_archived=true"));
            }
            if let Some(limit) = limit {
                query.push(format!("limit={limit}"));
            }
            let path = with_query("/runs", &query);
            let response = client.get(&path)?;
            let rows: Vec<RunSummary> = decode_json!(Vec<RunSummary>, response, &path);
            output::emit(globals.json, json_view!(&rows), stdout, |writer| {
                output::history(writer, kind.as_str(), &rows, &[], &[], &[], &[])
            })
        }
        HistoryKind::Stage => {
            let mut query = Vec::new();
            if let Some(limit) = limit {
                query.push(format!("limit={limit}"));
            }
            let path = with_query("/stages/runs", &query);
            let response = client.get(&path)?;
            let rows: Vec<StageRunSummary> = decode_json!(Vec<StageRunSummary>, response, &path);
            output::emit(globals.json, json_view!(&rows), stdout, |writer| {
                output::history(writer, kind.as_str(), &[], &rows, &[], &[], &[])
            })
        }
        HistoryKind::Research => {
            let mut query = Vec::new();
            if include_archived {
                query.push(String::from("include_archived=true"));
            }
            if let Some(limit) = limit {
                query.push(format!("limit={limit}"));
            }
            let path = with_query("/research/runs", &query);
            let response = client.get(&path)?;
            let rows: Vec<ImprovementRunSummary> =
                decode_json!(Vec<ImprovementRunSummary>, response, &path);
            output::emit(globals.json, json_view!(&rows), stdout, |writer| {
                output::history(writer, kind.as_str(), &[], &[], &rows, &[], &[])
            })
        }
        HistoryKind::Simulation => {
            let mut query = Vec::new();
            if include_archived {
                query.push(String::from("include_archived=true"));
            }
            if let Some(limit) = limit {
                query.push(format!("limit={limit}"));
            }
            let path = with_query("/simulation/runs", &query);
            let response = client.get(&path)?;
            let rows: Vec<SimulationSummary> =
                decode_json!(Vec<SimulationSummary>, response, &path);
            output::emit(globals.json, json_view!(&rows), stdout, |writer| {
                output::history(writer, kind.as_str(), &[], &[], &[], &rows, &[])
            })
        }
        HistoryKind::Dataset => {
            let path = if include_archived {
                "/catalog/datasets?include_archived=true"
            } else {
                "/catalog/datasets"
            };
            let response = client.get(path)?;
            let mut rows: Vec<DatasetSummary> = decode_json!(Vec<DatasetSummary>, response, path);
            if let Some(limit) = limit {
                rows.truncate(limit);
            }
            output::emit(globals.json, json_view!(&rows), stdout, |writer| {
                output::history(writer, kind.as_str(), &[], &[], &[], &[], &rows)
            })
        }
    }
}

pub(super) fn history_archive(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    kind: HistoryKind,
    id: &str,
    dry_run: bool,
) -> Result<(), RunError> {
    if dry_run {
        return output::emit(
            globals.json,
            serde_json::json!({"dry_run": true}),
            stdout,
            |writer| output::dry_run(writer, "archive", kind.as_str(), id),
        );
    }
    let encoded = encode(id);
    match kind {
        HistoryKind::Run => {
            let path = format!("/runs/{encoded}/archive");
            let response = client.post_empty(&path)?;
            let row: RunSummary = decode_json!(RunSummary, response, &path);
            output::emit(globals.json, json_view!(&row), stdout, |writer| {
                output::archived(writer, kind.as_str(), id)
            })
        }
        HistoryKind::Research => {
            let path = format!("/research/runs/{encoded}/archive");
            let response = client.post_empty(&path)?;
            let row: ImprovementRunSummary = decode_json!(ImprovementRunSummary, response, &path);
            output::emit(globals.json, json_view!(&row), stdout, |writer| {
                output::archived(writer, kind.as_str(), id)
            })
        }
        HistoryKind::Simulation => {
            let path = format!("/simulation/runs/{encoded}/archive");
            let response = client.post_empty(&path)?;
            let row: SimulationSummary = decode_json!(SimulationSummary, response, &path);
            output::emit(globals.json, json_view!(&row), stdout, |writer| {
                output::archived(writer, kind.as_str(), id)
            })
        }
        HistoryKind::Dataset => {
            let path = format!("/catalog/datasets/{encoded}/archive");
            let response = client.post_empty(&path)?;
            let row: DatasetRecord = decode_json!(DatasetRecord, response, &path);
            output::emit(globals.json, json_view!(&row), stdout, |writer| {
                output::archived(writer, kind.as_str(), id)
            })
        }
        HistoryKind::Stage => Err(RunError::Usage(String::from(
            "a stage has no archive; use history erase",
        ))),
    }
}

pub(super) fn history_erase(
    client: &Client,
    globals: &GlobalOptions,
    stdout: &mut impl Write,
    kind: HistoryKind,
    id: &str,
    confirm: &str,
    dry_run: bool,
) -> Result<(), RunError> {
    if dry_run {
        return output::emit(
            globals.json,
            serde_json::json!({"dry_run": true}),
            stdout,
            |writer| output::dry_run(writer, "erase", kind.as_str(), id),
        );
    }
    let encoded = encode(id);
    match kind {
        HistoryKind::Run => {
            let path = format!("/runs/{encoded}?confirm={}", encode(confirm));
            let response = client.delete_empty(&path)?;
            let receipt: EraseReceipt = decode_json!(EraseReceipt, response, &path);
            output::emit(globals.json, json_view!(&receipt), stdout, |writer| {
                output::erased(writer, kind.as_str(), id, receipt.bytes)
            })
        }
        HistoryKind::Research => {
            let path = format!("/research/runs/{encoded}?confirm={}", encode(confirm));
            let response = client.delete_empty(&path)?;
            let receipt: EraseReceipt = decode_json!(EraseReceipt, response, &path);
            output::emit(globals.json, json_view!(&receipt), stdout, |writer| {
                output::erased(writer, kind.as_str(), id, receipt.bytes)
            })
        }
        HistoryKind::Simulation => {
            let path = format!("/simulation/runs/{encoded}?confirm={}", encode(confirm));
            let response = client.delete_empty(&path)?;
            let receipt: SimulationEraseResponse =
                decode_json!(SimulationEraseResponse, response, &path);
            output::emit(globals.json, json_view!(&receipt), stdout, |writer| {
                output::erased(writer, kind.as_str(), &receipt.sim_id.0, 0)
            })
        }
        HistoryKind::Dataset => {
            let path = format!("/catalog/datasets/{encoded}?confirm={}", encode(confirm));
            let response = client.delete_empty(&path)?;
            let receipt: EraseDatasetResponse = decode_json!(EraseDatasetResponse, response, &path);
            output::emit(globals.json, json_view!(&receipt), stdout, |writer| {
                output::erased(
                    writer,
                    kind.as_str(),
                    &receipt.data_version.to_string(),
                    receipt.bytes,
                )
            })
        }
        HistoryKind::Stage => {
            let path = format!("/stages/runs/{encoded}");
            let response = client.delete_empty(&path)?;
            let row: StageRunSummary = decode_json!(StageRunSummary, response, &path);
            output::emit(globals.json, json_view!(&row), stdout, |writer| {
                output::erased(writer, kind.as_str(), id, 0)
            })
        }
    }
}
