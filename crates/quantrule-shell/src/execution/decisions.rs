//! Read a strategy decisions bundle into the execution vocabulary.
//!
//! The reader consumes `decisions.ndjson` and `provenance.json`. It does not
//! read a model bundle, a candle, a fill, cash, equity, or P&L. The decision
//! artifact is the handoff between the strategy and execution stages.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use quantrule_backtest::intent::{BracketIntent, EntryStyle, StopSpec, TargetSpec, TradeIntent};
use quantrule_backtest::side::Side;
use serde_json::Value;

/// The strategy decisions bundle could not be read.
#[derive(Debug)]
pub(crate) enum DecisionsError {
    /// A required artifact could not be read.
    Read {
        /// The artifact path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// A required artifact is not shaped like a decisions bundle.
    Invalid {
        /// The artifact path.
        path: PathBuf,
        /// What is wrong, with the decision's own field names.
        message: String,
    },
}

impl fmt::Display for DecisionsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(
                    formatter,
                    "invalid decisions bundle {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for DecisionsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Invalid { .. } => None,
        }
    }
}

/// One decision as execution reads it.
#[derive(Debug, Clone)]
pub(crate) struct DecisionRecord {
    /// The execution timeframe in seconds.
    pub(crate) timeframe_secs: u64,
    /// The zero-based execution bar.
    pub(crate) bar_index: usize,
    /// The bar's UTC open time.
    pub(crate) open_time: i64,
    /// The decision and its request.
    pub(crate) kind: DecisionKind,
    /// The decision bar's ATR(14), or `None`.
    pub(crate) atr14: Option<f64>,
}

/// The decision kind with its execution request.
#[derive(Debug, Clone, Copy)]
pub(crate) enum DecisionKind {
    /// No order.
    Hold,
    /// Open one position on the next bar.
    Enter(TradeIntent),
    /// Place a two-sided stop-entry bracket.
    EnterBracket(BracketIntent),
    /// Close the open position.
    Exit(Side),
}

/// A complete decisions bundle.
#[derive(Debug, Clone)]
pub(crate) struct DecisionsBundle {
    /// The strategy output's data version.
    pub(crate) data_version: String,
    /// The strategy configuration identity.
    pub(crate) config_id: String,
    /// The requested period start, as written.
    pub(crate) period_from: Option<String>,
    /// The requested period end, as written.
    pub(crate) period_to: Option<String>,
    /// The decisions in ascending bar order.
    pub(crate) decisions: Vec<DecisionRecord>,
}

impl DecisionsBundle {
    /// Loads a decisions bundle from its directory.
    ///
    /// # Errors
    ///
    /// Returns [`DecisionsError`] when a required artifact is missing,
    /// unreadable, or not shaped like a decisions bundle.
    pub(crate) fn load(path: &Path) -> Result<Self, DecisionsError> {
        let (data_version, config_id, period_from, period_to) =
            parse_provenance(&path.join("provenance.json"))?;
        let decisions = parse_decisions(&path.join("decisions.ndjson"))?;
        if decisions.is_empty() {
            return Err(DecisionsError::Invalid {
                path: path.join("decisions.ndjson"),
                message: "the bundle has no decisions".to_owned(),
            });
        }
        for pair in decisions.windows(2) {
            if pair[0].bar_index >= pair[1].bar_index {
                return Err(DecisionsError::Invalid {
                    path: path.join("decisions.ndjson"),
                    message: format!(
                        "decisions are not in ascending bar order: {} then {}",
                        pair[0].bar_index, pair[1].bar_index
                    ),
                });
            }
        }
        Ok(Self {
            data_version,
            config_id,
            period_from,
            period_to,
            decisions,
        })
    }
}

fn parse_provenance(
    path: &Path,
) -> Result<(String, String, Option<String>, Option<String>), DecisionsError> {
    let text = read_text(path)?;
    let value: Value = serde_json::from_str(&text).map_err(|error| DecisionsError::Invalid {
        path: path.to_path_buf(),
        message: format!("not JSON: {error}"),
    })?;
    let object = value.as_object().ok_or_else(|| DecisionsError::Invalid {
        path: path.to_path_buf(),
        message: "the provenance root is not an object".to_owned(),
    })?;
    let schema = object.get("schema").and_then(Value::as_str).unwrap_or("");
    if schema != "quantrule.stage-provenance/v1" {
        return Err(DecisionsError::Invalid {
            path: path.to_path_buf(),
            message: format!("expected schema 'quantrule.stage-provenance/v1', found '{schema}'"),
        });
    }
    let stage = object.get("stage").and_then(Value::as_str).unwrap_or("");
    if stage != "strategy" {
        return Err(DecisionsError::Invalid {
            path: path.to_path_buf(),
            message: format!("expected stage 'strategy', found '{stage}'"),
        });
    }
    let artifact = object.get("artifact").and_then(Value::as_str).unwrap_or("");
    if artifact != "decisions" {
        return Err(DecisionsError::Invalid {
            path: path.to_path_buf(),
            message: format!("expected artifact 'decisions', found '{artifact}'"),
        });
    }
    let data_version = object
        .get("data_version")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| DecisionsError::Invalid {
            path: path.to_path_buf(),
            message: "data_version is missing or empty".to_owned(),
        })?
        .to_owned();
    let config_id = object
        .get("config_id")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| DecisionsError::Invalid {
            path: path.to_path_buf(),
            message: "config_id is missing or empty".to_owned(),
        })?
        .to_owned();
    let period = object.get("period");
    let period_from = period
        .and_then(|period| period.get("from"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    let period_to = period
        .and_then(|period| period.get("to"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    Ok((data_version, config_id, period_from, period_to))
}

fn parse_decisions(path: &Path) -> Result<Vec<DecisionRecord>, DecisionsError> {
    let text = read_text(path)?;
    let mut decisions = Vec::new();
    for (line, text) in text.lines().enumerate() {
        if text.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(text).map_err(|error| DecisionsError::Invalid {
            path: path.to_path_buf(),
            message: format!("line {} is not JSON: {error}", line + 1),
        })?;
        decisions.push(parse_decision(&value, path, line + 1)?);
    }
    Ok(decisions)
}

fn parse_decision(
    value: &Value,
    path: &Path,
    line: usize,
) -> Result<DecisionRecord, DecisionsError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(path, line, "the record is not an object"))?;
    let schema = string(object, "schema", path, line)?;
    if schema != "quantrule.strategy-decision/v1" {
        return Err(invalid(
            path,
            line,
            &format!("expected schema 'quantrule.strategy-decision/v1', found '{schema}'"),
        ));
    }
    let timeframe_secs = u64_field(object, "timeframe_secs", path, line)?;
    let bar_index = usize_field(object, "bar_index", path, line)?;
    let open_time = i64_field(object, "open_time", path, line)?;
    let atr14 = optional_number(object, "atr14", path, line)?;
    let decision = string(object, "decision", path, line)?;
    let kind = match decision {
        "hold" => DecisionKind::Hold,
        "enter" => {
            let side = object
                .get("side")
                .map(|value| parse_side(value, path, line))
                .transpose()?
                .ok_or_else(|| invalid(path, line, "an entry decision needs a side"))?;
            DecisionKind::Enter(parse_trade_intent(
                field(object, "intent", path, line)?,
                side,
                path,
                line,
            )?)
        }
        "enter_bracket" => DecisionKind::EnterBracket(parse_bracket_intent(
            field(object, "intent", path, line)?,
            path,
            line,
        )?),
        "exit" => {
            let side = object
                .get("side")
                .map(|value| parse_side(value, path, line))
                .transpose()?
                .ok_or_else(|| invalid(path, line, "an exit decision needs a side"))?;
            DecisionKind::Exit(side)
        }
        other => {
            return Err(invalid(path, line, &format!("unknown decision '{other}'")));
        }
    };
    Ok(DecisionRecord {
        timeframe_secs,
        bar_index,
        open_time,
        kind,
        atr14,
    })
}

fn parse_trade_intent(
    value: &Value,
    side: Side,
    path: &Path,
    line: usize,
) -> Result<TradeIntent, DecisionsError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(path, line, "the entry intent is not an object"))?;
    let kind = string(object, "kind", path, line)?;
    if kind != "market_next_open" && kind != "limit" {
        return Err(invalid(
            path,
            line,
            &format!("expected a market or limit entry intent, found '{kind}'"),
        ));
    }
    let entry = parse_entry_style(object, path, line)?;
    let stop = parse_stop(field(object, "stop", path, line)?, path, line)?;
    let target = parse_target(field(object, "target", path, line)?, path, line)?;
    let valid_until = optional_usize(object, "valid_until", path, line)?;
    let max_slippage_bps = optional_number(object, "max_slippage_bps", path, line)?;
    Ok(TradeIntent {
        side,
        entry,
        stop,
        target,
        valid_until,
        max_slippage_bps,
    })
}

fn parse_entry_style(
    object: &serde_json::Map<String, Value>,
    path: &Path,
    line: usize,
) -> Result<EntryStyle, DecisionsError> {
    let entry = field(object, "entry", path, line)?;
    let entry = entry
        .as_object()
        .ok_or_else(|| invalid(path, line, "the entry style is not an object"))?;
    let kind = string(entry, "kind", path, line)?;
    match kind {
        "market_next_open" => Ok(EntryStyle::MarketNextOpen),
        "limit" => Ok(EntryStyle::Limit {
            price: number(entry, "price", path, line)?,
            valid_for_bars: u32_field(entry, "valid_for_bars", path, line)?,
        }),
        other => Err(invalid(
            path,
            line,
            &format!("unknown entry style '{other}'"),
        )),
    }
}

fn parse_stop(value: &Value, path: &Path, line: usize) -> Result<StopSpec, DecisionsError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(path, line, "the stop specification is not an object"))?;
    let kind = string(object, "kind", path, line)?;
    match kind {
        "atr_multiple" => Ok(StopSpec::AtrMultiple {
            multiple: number(object, "multiple", path, line)?,
        }),
        "price" => Ok(StopSpec::Price {
            price: number(object, "price", path, line)?,
        }),
        other => Err(invalid(path, line, &format!("unknown stop kind '{other}'"))),
    }
}

fn parse_target(value: &Value, path: &Path, line: usize) -> Result<TargetSpec, DecisionsError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(path, line, "the target specification is not an object"))?;
    let kind = string(object, "kind", path, line)?;
    match kind {
        "reward_to_risk" => Ok(TargetSpec::RewardToRisk {
            multiple: number(object, "multiple", path, line)?,
        }),
        "price" => Ok(TargetSpec::Price {
            price: number(object, "price", path, line)?,
        }),
        other => Err(invalid(
            path,
            line,
            &format!("unknown target kind '{other}'"),
        )),
    }
}

fn parse_bracket_intent(
    value: &Value,
    path: &Path,
    line: usize,
) -> Result<BracketIntent, DecisionsError> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid(path, line, "the bracket intent is not an object"))?;
    let kind = string(object, "kind", path, line)?;
    if kind != "bracket" {
        return Err(invalid(
            path,
            line,
            &format!("expected a bracket intent, found '{kind}'"),
        ));
    }
    Ok(BracketIntent {
        upper_price: number(object, "upper_price", path, line)?,
        lower_price: number(object, "lower_price", path, line)?,
        stop: parse_stop(field(object, "stop", path, line)?, path, line)?,
        target: parse_target(field(object, "target", path, line)?, path, line)?,
        valid_for_bars: u32_field(object, "valid_for_bars", path, line)?,
        max_slippage_bps: optional_number(object, "max_slippage_bps", path, line)?,
    })
}

fn parse_side(value: &Value, path: &Path, line: usize) -> Result<Side, DecisionsError> {
    match value.as_str() {
        Some("long") => Ok(Side::Long),
        Some("short") => Ok(Side::Short),
        _ => Err(invalid(path, line, "expected side 'long' or 'short'")),
    }
}

fn read_text(path: &Path) -> Result<String, DecisionsError> {
    fs::read_to_string(path).map_err(|source| DecisionsError::Read {
        path: path.to_path_buf(),
        source,
    })
}

fn invalid(path: &Path, line: usize, message: &str) -> DecisionsError {
    DecisionsError::Invalid {
        path: path.to_path_buf(),
        message: format!("line {line}: {message}"),
    }
}

fn field<'a>(
    object: &'a serde_json::Map<String, Value>,
    name: &str,
    path: &Path,
    line: usize,
) -> Result<&'a Value, DecisionsError> {
    object
        .get(name)
        .ok_or_else(|| invalid(path, line, &format!("missing field '{name}'")))
}

fn string<'a>(
    object: &'a serde_json::Map<String, Value>,
    name: &str,
    path: &Path,
    line: usize,
) -> Result<&'a str, DecisionsError> {
    field(object, name, path, line)?
        .as_str()
        .ok_or_else(|| invalid(path, line, &format!("'{name}' is not a string")))
}

fn number(
    object: &serde_json::Map<String, Value>,
    name: &str,
    path: &Path,
    line: usize,
) -> Result<f64, DecisionsError> {
    field(object, name, path, line)?
        .as_f64()
        .ok_or_else(|| invalid(path, line, &format!("'{name}' is not a number")))
}

fn optional_number(
    object: &serde_json::Map<String, Value>,
    name: &str,
    path: &Path,
    line: usize,
) -> Result<Option<f64>, DecisionsError> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_f64()
            .map(Some)
            .ok_or_else(|| invalid(path, line, &format!("'{name}' is not a number or null"))),
    }
}

fn u64_field(
    object: &serde_json::Map<String, Value>,
    name: &str,
    path: &Path,
    line: usize,
) -> Result<u64, DecisionsError> {
    field(object, name, path, line)?
        .as_u64()
        .ok_or_else(|| invalid(path, line, &format!("'{name}' is not an unsigned integer")))
}

fn usize_field(
    object: &serde_json::Map<String, Value>,
    name: &str,
    path: &Path,
    line: usize,
) -> Result<usize, DecisionsError> {
    let value = u64_field(object, name, path, line)?;
    usize::try_from(value)
        .map_err(|_| invalid(path, line, &format!("'{name}' does not fit this platform")))
}

fn u32_field(
    object: &serde_json::Map<String, Value>,
    name: &str,
    path: &Path,
    line: usize,
) -> Result<u32, DecisionsError> {
    let value = u64_field(object, name, path, line)?;
    u32::try_from(value).map_err(|_| invalid(path, line, &format!("'{name}' does not fit u32")))
}

fn i64_field(
    object: &serde_json::Map<String, Value>,
    name: &str,
    path: &Path,
    line: usize,
) -> Result<i64, DecisionsError> {
    field(object, name, path, line)?
        .as_i64()
        .ok_or_else(|| invalid(path, line, &format!("'{name}' is not a signed integer")))
}

fn optional_usize(
    object: &serde_json::Map<String, Value>,
    name: &str,
    path: &Path,
    line: usize,
) -> Result<Option<usize>, DecisionsError> {
    match object.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(_) => usize_field(object, name, path, line).map(Some),
    }
}

#[cfg(test)]
mod tests {
    use super::DecisionsBundle;

    #[test]
    fn a_missing_bundle_is_reported() {
        let error = DecisionsBundle::load(std::path::Path::new("target/no-such-decisions"))
            .expect_err("the bundle does not exist");
        assert!(error.to_string().contains("cannot read"));
    }
}
