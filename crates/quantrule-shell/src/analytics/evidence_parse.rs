//! Parse the recorded trades, candles, decisions, and benchmark inputs.
//!
//! These parsers read artifacts the shell already wrote. They do not rerun a
//! stage, compute a signal, or read a live source.

use std::collections::BTreeMap;
use std::path::PathBuf;

use quantrule_backtest::portfolio::{ExitReason, TradeRecord};
use quantrule_backtest::side::Side;
use quantrule_data::Candle;
use serde_json::Value;

use quantrule_app::artifact::backtest::BundleManifest;

use super::evidence::{
    AnalyticsEvidenceError, DECISION_SCHEMA, DecisionKind, DecisionReason, DecisionRecord,
    DecisionSide, TRADES_HEADER, as_object, invalid, required_number, validate_header,
};

pub(crate) fn parse_decisions(text: &str) -> Result<Vec<DecisionRecord>, AnalyticsEvidenceError> {
    let path = "strategy/decisions.ndjson";
    let mut decisions = Vec::new();
    for (index, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| invalid(path, &format!("line {} is not JSON: {error}", index + 1)))?;
        let object = value
            .as_object()
            .ok_or_else(|| invalid(path, &format!("line {} must be an object", index + 1)))?;
        let schema = object
            .get("schema")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(path, &format!("line {} has no schema", index + 1)))?;
        if schema != DECISION_SCHEMA {
            return Err(AnalyticsEvidenceError::UnsupportedSchema {
                path: PathBuf::from(path),
                expected: DECISION_SCHEMA,
                observed: schema.to_owned(),
            });
        }
        let timeframe_secs = object
            .get("timeframe_secs")
            .and_then(Value::as_u64)
            .filter(|secs| *secs > 0)
            .ok_or_else(|| {
                invalid(
                    path,
                    &format!(
                        "line {} timeframe_secs must be a positive integer",
                        index + 1
                    ),
                )
            })?;
        let decision_code = object
            .get("decision")
            .and_then(Value::as_str)
            .ok_or_else(|| invalid(path, &format!("line {} has no decision", index + 1)))?;
        let decision = DecisionKind::from_code(decision_code).ok_or_else(|| {
            invalid(
                path,
                &format!("line {} has unknown decision '{decision_code}'", index + 1),
            )
        })?;
        let side = match object.get("side") {
            None | Some(Value::Null) => None,
            Some(Value::String(code)) => Some(DecisionSide::from_code(code).ok_or_else(|| {
                invalid(
                    path,
                    &format!("line {} has unknown side '{code}'", index + 1),
                )
            })?),
            Some(_) => {
                return Err(invalid(
                    path,
                    &format!("line {} side must be a string or null", index + 1),
                ));
            }
        };
        let reason = match object.get("reason") {
            None | Some(Value::Null) => None,
            Some(Value::String(code)) => {
                Some(DecisionReason::from_code(code).ok_or_else(|| {
                    invalid(
                        path,
                        &format!("line {} has unknown reason '{code}'", index + 1),
                    )
                })?)
            }
            Some(_) => {
                return Err(invalid(
                    path,
                    &format!("line {} reason must be a string or null", index + 1),
                ));
            }
        };
        decisions.push(DecisionRecord {
            timeframe_secs,
            decision,
            side,
            reason,
        });
    }
    Ok(decisions)
}

pub(crate) fn dominant_decision_timeframe(decisions: &[DecisionRecord]) -> Option<u64> {
    let mut counts: BTreeMap<u64, usize> = BTreeMap::new();
    for decision in decisions {
        *counts.entry(decision.timeframe_secs).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(secs, count)| (*count, std::cmp::Reverse(*secs)))
        .map(|(secs, _)| secs)
}

pub(crate) fn dominant_readings_timeframe(text: &str) -> Option<u64> {
    let mut lines = text.lines();
    let header = lines.next()?;
    let index = header
        .split(',')
        .position(|name| name == "timeframe_secs")?;
    let mut counts: BTreeMap<u64, usize> = BTreeMap::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        let Some(value) = fields
            .get(index)
            .and_then(|value| value.parse::<u64>().ok())
        else {
            continue;
        };
        *counts.entry(value).or_insert(0) += 1;
    }
    counts
        .into_iter()
        .max_by_key(|(secs, count)| (*count, std::cmp::Reverse(*secs)))
        .map(|(secs, _)| secs)
}

pub(crate) fn parse_candles(
    text: &str,
    timeframe_secs: u64,
) -> Result<Vec<Candle>, AnalyticsEvidenceError> {
    let path = "model/readings.csv";
    let mut lines = text.lines();
    let header = lines
        .next()
        .ok_or_else(|| invalid(path, "the readings table is empty"))?;
    let columns: Vec<&str> = header.split(',').collect();
    let index = |name: &str| {
        columns
            .iter()
            .position(|column| *column == name)
            .ok_or_else(|| invalid(path, &format!("the readings table has no {name} column")))
    };
    let timeframe_index = index("timeframe_secs")?;
    let timestamp_index = index("timestamp")?;
    let open_index = index("open")?;
    let high_index = index("high")?;
    let low_index = index("low")?;
    let close_index = index("close")?;
    let volume_index = index("volume")?;
    let max_index = [
        timeframe_index,
        timestamp_index,
        open_index,
        high_index,
        low_index,
        close_index,
        volume_index,
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    let mut candles = Vec::new();
    if timeframe_secs == 0 {
        return Ok(candles);
    }
    for (line_number, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() <= max_index {
            return Err(invalid(
                path,
                &format!(
                    "line {} has fewer than {} columns",
                    line_number + 2,
                    max_index + 1
                ),
            ));
        }
        let row_timeframe = fields[timeframe_index].parse::<u64>().map_err(|_| {
            invalid(
                path,
                &format!("line {} timeframe_secs is not an integer", line_number + 2),
            )
        })?;
        if row_timeframe != timeframe_secs {
            continue;
        }
        let timestamp = fields[timestamp_index].parse::<i64>().map_err(|_| {
            invalid(
                path,
                &format!("line {} timestamp is not an integer", line_number + 2),
            )
        })?;
        let number = |field: &str, value: &str| {
            value.parse::<f64>().map_err(|_| {
                invalid(
                    path,
                    &format!("line {} {field} is not a number", line_number + 2),
                )
            })
        };
        let candle = Candle::new(
            candles.len() as u64,
            timestamp,
            number("open", fields[open_index])?,
            number("high", fields[high_index])?,
            number("low", fields[low_index])?,
            number("close", fields[close_index])?,
            Some(number("volume", fields[volume_index])?),
        )
        .map_err(|error| {
            invalid(
                path,
                &format!("line {} is not a valid candle: {error}", line_number + 2),
            )
        })?;
        candles.push(candle);
    }
    candles.sort_by_key(Candle::timestamp);
    if candles
        .windows(2)
        .any(|window| window[1].timestamp() <= window[0].timestamp())
    {
        return Err(invalid(path, "the candles are not strictly increasing"));
    }
    Ok(candles)
}

pub(crate) fn parse_trades(text: &str) -> Result<Vec<TradeRecord>, AnalyticsEvidenceError> {
    let path = "portfolio/trades.csv";
    validate_header(path, text, TRADES_HEADER)?;
    let mut trades = Vec::new();
    for (line_number, line) in text.lines().skip(1).enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != 17 {
            return Err(invalid(
                path,
                &format!(
                    "line {} has {} columns, expected 17",
                    line_number + 2,
                    fields.len()
                ),
            ));
        }
        let number = |field: &str, value: &str| {
            value.parse::<f64>().map_err(|_| {
                invalid(
                    path,
                    &format!("line {} {field} is not a number", line_number + 2),
                )
            })
        };
        let integer = |field: &str, value: &str| {
            value.parse::<usize>().map_err(|_| {
                invalid(
                    path,
                    &format!("line {} {field} is not an integer", line_number + 2),
                )
            })
        };
        let side = match fields[1] {
            "long" => Side::Long,
            "short" => Side::Short,
            other => {
                return Err(invalid(
                    path,
                    &format!("line {} has unknown side '{other}'", line_number + 2),
                ));
            }
        };
        let exit_reason = match fields[16] {
            "strategy" => ExitReason::StrategyExit,
            "stop" => ExitReason::Stop,
            "target" => ExitReason::Target,
            other => {
                return Err(invalid(
                    path,
                    &format!("line {} has unknown exit reason '{other}'", line_number + 2),
                ));
            }
        };
        trades.push(TradeRecord {
            trade_id: fields[0].parse::<u64>().map_err(|_| {
                invalid(
                    path,
                    &format!("line {} trade_id is not an integer", line_number + 2),
                )
            })?,
            side,
            quantity: number("quantity", fields[2])?,
            entry_bar: integer("entry_bar", fields[3])?,
            entry_time: fields[4].parse::<i64>().map_err(|_| {
                invalid(
                    path,
                    &format!("line {} entry_time is not an integer", line_number + 2),
                )
            })?,
            entry_price: number("entry_price", fields[5])?,
            exit_bar: integer("exit_bar", fields[6])?,
            exit_time: fields[7].parse::<i64>().map_err(|_| {
                invalid(
                    path,
                    &format!("line {} exit_time is not an integer", line_number + 2),
                )
            })?,
            exit_price: number("exit_price", fields[8])?,
            gross_pnl: number("gross_pnl", fields[9])?,
            execution_fees: number("execution_fees", fields[10])?,
            commission: number("commission", fields[11])?,
            funding: number("funding", fields[12])?,
            net_pnl: number("net_pnl", fields[13])?,
            return_fraction: if fields[14].is_empty() || fields[14] == "null" {
                None
            } else {
                Some(number("return_fraction", fields[14])?)
            },
            hold_bars: integer("hold_bars", fields[15])?,
            exit_reason,
        });
    }
    Ok(trades)
}

pub(crate) fn parse_commission_rate(
    bundle: &BundleManifest,
) -> Result<f64, AnalyticsEvidenceError> {
    let path = "portfolio/provenance.json";
    let value = bundle
        .read_json(path)
        .map_err(AnalyticsEvidenceError::Bundle)?;
    let object = as_object(path, &value)?;
    let portfolio = object
        .get("portfolio")
        .and_then(Value::as_object)
        .ok_or_else(|| invalid(path, "portfolio must be an object"))?;
    required_number(path, portfolio, "commission_rate")
}
