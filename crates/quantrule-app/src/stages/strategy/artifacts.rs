//! Shared strategy artifact rendering.
//!
//! The standalone `strategy run` command and the backtest runner both write the
//! same decisions bundle. This module owns the byte shape so the orchestrated
//! run cannot drift from the stage command.

use std::fmt::Write as _;

use quantrule_backtest::side::Side;
use quantrule_backtest::strategy::{Decision, ExitReason};
use serde_json::{Value, json};

use super::StrategyRunError;

/// The identity and configuration a decisions bundle records.
pub struct StrategyArtifactMeta<'a> {
    /// The input candle identity.
    pub data_version: &'a str,
    /// The model configuration identity.
    pub model_config_id: &'a str,
    /// The strategy configuration identity.
    pub strategy_config_id: &'a str,
    /// The requested period start, as written.
    pub period_from: Option<&'a str>,
    /// The requested period end, as written.
    pub period_to: Option<&'a str>,
    /// The optional strategy label.
    pub strategy_id: Option<&'a str>,
    /// The engine name.
    pub engine: &'a str,
    /// The execution timeframe code.
    pub execution_timeframe: &'a str,
    /// The context timeframe codes, ascending.
    pub context_timeframes: Vec<&'a str>,
}

/// The decision counts a summary carries.
#[derive(Debug, Default)]
pub struct DecisionCounts {
    /// Hold decisions.
    pub hold: usize,
    /// Market entry decisions.
    pub enter: usize,
    /// Bracket entry decisions.
    pub enter_bracket: usize,
    /// Exit decisions.
    pub exit: usize,
    /// Long entries.
    pub long: usize,
    /// Short entries.
    pub short: usize,
    /// Entry reason count.
    pub entry_reason: usize,
    /// Bracket-entry reason count.
    pub bracket_entry_reason: usize,
    /// Event-condition exit reason count.
    pub event_condition_reason: usize,
    /// Time-stop exit reason count.
    pub time_stop_reason: usize,
    /// Giveback exit reason count.
    pub giveback_reason: usize,
}

impl DecisionCounts {
    /// Counts one decision.
    pub fn observe(&mut self, decision: Decision) {
        match decision {
            Decision::Hold => self.hold += 1,
            Decision::Enter(entry) => {
                self.enter += 1;
                self.entry_reason += 1;
                match entry.side() {
                    Side::Long => self.long += 1,
                    Side::Short => self.short += 1,
                }
            }
            Decision::EnterBracket(_) => {
                self.enter_bracket += 1;
                self.bracket_entry_reason += 1;
            }
            Decision::Exit(exit) => {
                self.exit += 1;
                match exit.reason {
                    ExitReason::EventCondition => self.event_condition_reason += 1,
                    ExitReason::TimeStop => self.time_stop_reason += 1,
                    ExitReason::Giveback => self.giveback_reason += 1,
                }
            }
        }
    }
}

/// Renders the decisions NDJSON.
///
/// # Errors
///
/// Returns the serializer's error when a record cannot be encoded.
pub fn decisions_ndjson(records: &[Value]) -> Result<String, StrategyRunError> {
    let mut text = String::new();
    for record in records {
        text.push_str(&serde_json::to_string(record).map_err(StrategyRunError::Json)?);
        text.push('\n');
    }
    Ok(text)
}

/// Renders `summary.txt`.
pub fn summary_bytes(
    meta: &StrategyArtifactMeta<'_>,
    bars: usize,
    first_open_time: Option<i64>,
    last_open_time: Option<i64>,
    counts: &DecisionCounts,
) -> String {
    let context = meta.context_timeframes.join(" ");
    let mut out = String::new();
    let _ = writeln!(out, "QuantRule strategy run");
    let _ = writeln!(
        out,
        "strategy_id: {}",
        meta.strategy_id.unwrap_or("(unnamed)")
    );
    let _ = writeln!(out, "engine: {}", meta.engine);
    let _ = writeln!(out, "data_version: {}", meta.data_version);
    let _ = writeln!(out, "model_config_id: {}", meta.model_config_id);
    let _ = writeln!(out, "strategy_config_id: {}", meta.strategy_config_id);
    let _ = writeln!(out, "execution_timeframe: {}", meta.execution_timeframe);
    let _ = writeln!(out, "context_timeframes: {context}");
    let _ = writeln!(out, "bars: {bars}");
    let _ = writeln!(
        out,
        "first_open_time: {}",
        first_open_time.map_or_else(|| "null".to_owned(), quantrule_data::format_utc_timestamp)
    );
    let _ = writeln!(
        out,
        "last_open_time: {}",
        last_open_time.map_or_else(|| "null".to_owned(), quantrule_data::format_utc_timestamp)
    );
    let _ = writeln!(
        out,
        "decisions: hold={} enter={} enter_bracket={} exit={}",
        counts.hold, counts.enter, counts.enter_bracket, counts.exit
    );
    let _ = writeln!(out, "sides: long={} short={}", counts.long, counts.short);
    let _ = writeln!(
        out,
        "reasons: entry={} bracket_entry={} event_condition={} time_stop={} giveback={}",
        counts.entry_reason,
        counts.bracket_entry_reason,
        counts.event_condition_reason,
        counts.time_stop_reason,
        counts.giveback_reason
    );
    out
}

/// Renders `provenance.json`.
///
/// # Errors
///
/// Returns the serializer's error when the value cannot be encoded.
pub fn provenance_bytes(meta: &StrategyArtifactMeta<'_>) -> Result<Vec<u8>, StrategyRunError> {
    let value = json!({
        "schema": "quantrule.stage-provenance/v1",
        "stage": "strategy",
        "artifact": "decisions",
        "data_version": meta.data_version,
        "config_id": meta.strategy_config_id,
        "code_commit": "unknown",
        "source_artifacts": [{
            "stage": "model",
            "artifact": "model",
            "schema": "quantrule.provenance/v1",
            "data_version": meta.data_version,
            "config_id": meta.model_config_id,
        }],
        "seed": Value::Null,
        "period": {
            "from": meta.period_from,
            "to": meta.period_to,
        },
        "artifacts": ["decisions.ndjson", "summary.txt", "provenance.json"],
        "strategy": {
            "id": meta.strategy_id,
            "engine": meta.engine,
            "execution_timeframe": meta.execution_timeframe,
            "context_timeframes": meta.context_timeframes,
        },
    });
    let mut bytes = serde_json::to_vec(&value).map_err(StrategyRunError::Json)?;
    bytes.push(b'\n');
    Ok(bytes)
}
