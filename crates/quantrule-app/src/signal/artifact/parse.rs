//! Parsing and closed validation of a signal artifact.
//!
//! The parser walks the JSON value rather than deriving a struct, so every
//! rejection names the file and JSON path. It validates the action, side, intent
//! and reason together: a hold cannot carry a side, an entry needs a market or
//! bracket intent, and a bracket cannot name a side. The parser does not trust
//! the writer, so a hand-edited artifact is rejected at the same boundary.

use quantrule_api::Digest;
use quantrule_backtest::intent::{BracketIntent, EntryStyle, StopSpec, TargetSpec, TradeIntent};
use serde_json::{Map, Value};

use super::{
    SIGNAL_ARTIFACT_SCHEMA, SignalAction, SignalArtifact, SignalIntent, SignalReason, SignalRow,
    SignalSide,
};
use crate::config::read::{self, ROOT};
use crate::config::{ConfigError, parse_timeframe};
use crate::signal::error::SignalError;

impl SignalIntent {
    pub(super) fn from_value(
        file: &str,
        path: &str,
        value: &Value,
        side: Option<SignalSide>,
    ) -> Result<Self, ConfigError> {
        let object = read::object(file, path, value)?;
        let kind = read::string(
            file,
            &read::child(path, "kind"),
            read::field(file, path, object, "kind")?,
        )?;
        match kind {
            "market_next_open" => {
                read::check_fields(
                    file,
                    path,
                    object,
                    &["kind", "stop", "target", "valid_until", "max_slippage_bps"],
                )?;
                Self::trade(file, path, object, EntryStyle::MarketNextOpen, side)
            }
            "limit" => {
                read::check_fields(
                    file,
                    path,
                    object,
                    &[
                        "kind",
                        "price",
                        "valid_for_bars",
                        "stop",
                        "target",
                        "valid_until",
                        "max_slippage_bps",
                    ],
                )?;
                let price = read::number(
                    file,
                    &read::child(path, "price"),
                    read::field(file, path, object, "price")?,
                )?;
                let valid_for_bars = read::u64(
                    file,
                    &read::child(path, "valid_for_bars"),
                    read::field(file, path, object, "valid_for_bars")?,
                )?;
                let valid_for_bars = u32::try_from(valid_for_bars).map_err(|_| {
                    ConfigError::range(
                        file,
                        &read::child(path, "valid_for_bars"),
                        "a positive bar count",
                        &valid_for_bars.to_string(),
                    )
                })?;
                if price <= 0.0 || valid_for_bars == 0 {
                    return Err(ConfigError::range(
                        file,
                        path,
                        "a positive limit price and bar count",
                        kind,
                    ));
                }
                Self::trade(
                    file,
                    path,
                    object,
                    EntryStyle::Limit {
                        price,
                        valid_for_bars,
                    },
                    side,
                )
            }
            "bracket" => {
                read::check_fields(
                    file,
                    path,
                    object,
                    &[
                        "kind",
                        "upper_price",
                        "lower_price",
                        "stop",
                        "target",
                        "valid_for_bars",
                        "max_slippage_bps",
                    ],
                )?;
                let upper_price = read::number(
                    file,
                    &read::child(path, "upper_price"),
                    read::field(file, path, object, "upper_price")?,
                )?;
                let lower_price = read::number(
                    file,
                    &read::child(path, "lower_price"),
                    read::field(file, path, object, "lower_price")?,
                )?;
                let valid_for_bars = read::u64(
                    file,
                    &read::child(path, "valid_for_bars"),
                    read::field(file, path, object, "valid_for_bars")?,
                )?;
                let valid_for_bars = u32::try_from(valid_for_bars).map_err(|_| {
                    ConfigError::range(
                        file,
                        &read::child(path, "valid_for_bars"),
                        "a positive bar count",
                        &valid_for_bars.to_string(),
                    )
                })?;
                let intent = BracketIntent {
                    upper_price,
                    lower_price,
                    stop: stop_from_value(
                        file,
                        &read::child(path, "stop"),
                        read::field(file, path, object, "stop")?,
                    )?,
                    target: target_from_value(
                        file,
                        &read::child(path, "target"),
                        read::field(file, path, object, "target")?,
                    )?,
                    valid_for_bars,
                    max_slippage_bps: optional_non_negative(
                        file,
                        &read::child(path, "max_slippage_bps"),
                        object.get("max_slippage_bps"),
                    )?,
                };
                if !intent.is_valid() {
                    return Err(ConfigError::range(
                        file,
                        path,
                        "a valid two-sided bracket",
                        kind,
                    ));
                }
                Ok(Self::Bracket(intent))
            }
            "exit" => {
                read::check_fields(file, path, object, &["kind"])?;
                Ok(Self::Exit)
            }
            other => Err(ConfigError::range(
                file,
                &read::child(path, "kind"),
                "market_next_open, limit, bracket, or exit",
                other,
            )),
        }
    }

    fn trade(
        file: &str,
        path: &str,
        object: &Map<String, Value>,
        entry: EntryStyle,
        side: Option<SignalSide>,
    ) -> Result<Self, ConfigError> {
        let Some(side) = side else {
            return Err(ConfigError::inconsistent(
                file,
                path,
                "a signal side for a market entry",
                "null",
            ));
        };
        let intent = TradeIntent {
            side: side.to_side(),
            entry,
            stop: stop_from_value(
                file,
                &read::child(path, "stop"),
                read::field(file, path, object, "stop")?,
            )?,
            target: target_from_value(
                file,
                &read::child(path, "target"),
                read::field(file, path, object, "target")?,
            )?,
            valid_until: optional_usize(
                file,
                &read::child(path, "valid_until"),
                object.get("valid_until"),
            )?,
            max_slippage_bps: optional_non_negative(
                file,
                &read::child(path, "max_slippage_bps"),
                object.get("max_slippage_bps"),
            )?,
        };
        if !intent.stop.is_valid() || !intent.target.is_valid() {
            return Err(ConfigError::range(
                file,
                path,
                "a positive stop and target",
                "an invalid specification",
            ));
        }
        Ok(Self::Trade(intent))
    }
}

impl SignalRow {
    pub(super) fn from_value(file: &str, path: &str, value: &Value) -> Result<Self, ConfigError> {
        let object = read::object(file, path, value)?;
        read::check_fields(
            file,
            path,
            object,
            &[
                "bar_index",
                "open_time",
                "available_bar_index",
                "action",
                "side",
                "intent",
                "reason",
                "evidence",
            ],
        )?;
        let bar_index = usize_field(file, path, object, "bar_index")?;
        let open_time = object
            .get("open_time")
            .and_then(Value::as_i64)
            .ok_or_else(|| {
                ConfigError::wrong_type(
                    file,
                    &read::child(path, "open_time"),
                    "an integer",
                    &object
                        .get("open_time")
                        .map_or_else(|| "null".to_owned(), read::observed),
                )
            })?;
        let available_bar_index = usize_field(file, path, object, "available_bar_index")?;
        let action_code = read::string(
            file,
            &read::child(path, "action"),
            read::field(file, path, object, "action")?,
        )?;
        let action = SignalAction::from_code(action_code).ok_or_else(|| {
            ConfigError::range(
                file,
                &read::child(path, "action"),
                "hold, enter, exit, or bracket",
                action_code,
            )
        })?;
        let side = optional_side(file, &read::child(path, "side"), object.get("side"))?;
        let intent = match object.get("intent") {
            Some(Value::Null) | None => None,
            Some(value) => Some(SignalIntent::from_value(
                file,
                &read::child(path, "intent"),
                value,
                side,
            )?),
        };
        let reason = optional_reason(file, &read::child(path, "reason"), object.get("reason"))?;
        let evidence = evidence_from_value(
            file,
            &read::child(path, "evidence"),
            read::field(file, path, object, "evidence")?,
        )?;
        let row = Self {
            bar_index,
            open_time,
            available_bar_index,
            action,
            side,
            intent,
            reason,
            evidence,
        };
        row.validate(file, path)?;
        Ok(row)
    }
}

impl SignalArtifact {
    pub(super) fn from_value(value: &Value, file: &str) -> Result<Self, SignalError> {
        let object = read::object(file, ROOT, value)?;
        read::check_fields(
            file,
            ROOT,
            object,
            &[
                "schema",
                "strategy_pack_id",
                "model_artifact_id",
                "data_version",
                "timeframe",
                "period",
                "rows",
            ],
        )?;
        read::schema(file, object, SIGNAL_ARTIFACT_SCHEMA)?;
        let strategy_pack_id = digest_field(file, ROOT, object, "strategy_pack_id")?;
        let model_artifact_id = digest_field(file, ROOT, object, "model_artifact_id")?;
        let data_version = read::non_empty_string(
            file,
            &read::child(ROOT, "data_version"),
            read::field(file, ROOT, object, "data_version")?,
        )?
        .to_owned();
        let timeframe = parse_timeframe(
            file,
            &read::child(ROOT, "timeframe"),
            read::field(file, ROOT, object, "timeframe")?,
        )?;
        let period_path = read::child(ROOT, "period");
        let period = read::object(
            file,
            &period_path,
            read::field(file, ROOT, object, "period")?,
        )?;
        read::check_fields(file, &period_path, period, &["from", "to"])?;
        let period_from = read::non_empty_string(
            file,
            &read::child(&period_path, "from"),
            read::field(file, &period_path, period, "from")?,
        )?
        .to_owned();
        let period_to = read::non_empty_string(
            file,
            &read::child(&period_path, "to"),
            read::field(file, &period_path, period, "to")?,
        )?
        .to_owned();
        let rows_path = read::child(ROOT, "rows");
        let row_values = read::array(file, &rows_path, read::field(file, ROOT, object, "rows")?)?;
        let mut rows: Vec<SignalRow> = Vec::with_capacity(row_values.len());
        for (index, row) in row_values.iter().enumerate() {
            let path = read::index(&rows_path, index);
            let row = SignalRow::from_value(file, &path, row)?;
            if let Some(previous) = rows.last()
                && previous.bar_index >= row.bar_index
            {
                return Err(ConfigError::inconsistent(
                    file,
                    &read::child(&path, "bar_index"),
                    "a strictly ascending bar index",
                    &row.bar_index.to_string(),
                )
                .into());
            }
            rows.push(row);
        }
        Ok(Self::new(
            strategy_pack_id,
            model_artifact_id,
            data_version,
            timeframe,
            period_from,
            period_to,
            rows,
        ))
    }
}

fn stop_from_value(file: &str, path: &str, value: &Value) -> Result<StopSpec, ConfigError> {
    let object = read::object(file, path, value)?;
    let kind = read::string(
        file,
        &read::child(path, "kind"),
        read::field(file, path, object, "kind")?,
    )?;
    match kind {
        "atr_multiple" => {
            read::check_fields(file, path, object, &["kind", "multiple"])?;
            let multiple = read::number(
                file,
                &read::child(path, "multiple"),
                read::field(file, path, object, "multiple")?,
            )?;
            if multiple <= 0.0 {
                return Err(ConfigError::range(
                    file,
                    &read::child(path, "multiple"),
                    "a positive ATR multiple",
                    &multiple.to_string(),
                ));
            }
            Ok(StopSpec::AtrMultiple { multiple })
        }
        "price" => {
            read::check_fields(file, path, object, &["kind", "price"])?;
            let price = read::number(
                file,
                &read::child(path, "price"),
                read::field(file, path, object, "price")?,
            )?;
            if price <= 0.0 {
                return Err(ConfigError::range(
                    file,
                    &read::child(path, "price"),
                    "a positive stop price",
                    &price.to_string(),
                ));
            }
            Ok(StopSpec::Price { price })
        }
        other => Err(ConfigError::range(
            file,
            &read::child(path, "kind"),
            "atr_multiple or price",
            other,
        )),
    }
}

fn target_from_value(file: &str, path: &str, value: &Value) -> Result<TargetSpec, ConfigError> {
    let object = read::object(file, path, value)?;
    let kind = read::string(
        file,
        &read::child(path, "kind"),
        read::field(file, path, object, "kind")?,
    )?;
    match kind {
        "reward_to_risk" => {
            read::check_fields(file, path, object, &["kind", "multiple"])?;
            let multiple = read::number(
                file,
                &read::child(path, "multiple"),
                read::field(file, path, object, "multiple")?,
            )?;
            if multiple <= 0.0 {
                return Err(ConfigError::range(
                    file,
                    &read::child(path, "multiple"),
                    "a positive reward-to-risk multiple",
                    &multiple.to_string(),
                ));
            }
            Ok(TargetSpec::RewardToRisk { multiple })
        }
        "price" => {
            read::check_fields(file, path, object, &["kind", "price"])?;
            let price = read::number(
                file,
                &read::child(path, "price"),
                read::field(file, path, object, "price")?,
            )?;
            if price <= 0.0 {
                return Err(ConfigError::range(
                    file,
                    &read::child(path, "price"),
                    "a positive target price",
                    &price.to_string(),
                ));
            }
            Ok(TargetSpec::Price { price })
        }
        other => Err(ConfigError::range(
            file,
            &read::child(path, "kind"),
            "reward_to_risk or price",
            other,
        )),
    }
}

fn optional_side(
    file: &str,
    path: &str,
    value: Option<&Value>,
) -> Result<Option<SignalSide>, ConfigError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let code = read::string(file, path, value)?;
            SignalSide::from_code(code)
                .map(Some)
                .ok_or_else(|| ConfigError::range(file, path, "long or short", code))
        }
    }
}

fn optional_reason(
    file: &str,
    path: &str,
    value: Option<&Value>,
) -> Result<Option<SignalReason>, ConfigError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let code = read::string(file, path, value)?;
            SignalReason::from_code(code)
                .map(Some)
                .ok_or_else(|| ConfigError::range(file, path, "a known signal reason", code))
        }
    }
}

fn optional_usize(
    file: &str,
    path: &str,
    value: Option<&Value>,
) -> Result<Option<usize>, ConfigError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let number = read::u64(file, path, value)?;
            usize::try_from(number).map(Some).map_err(|_| {
                ConfigError::range(file, path, "a representable bar", &number.to_string())
            })
        }
    }
}

fn optional_non_negative(
    file: &str,
    path: &str,
    value: Option<&Value>,
) -> Result<Option<f64>, ConfigError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let number = read::number(file, path, value)?;
            if number < 0.0 {
                return Err(ConfigError::range(
                    file,
                    path,
                    "a non-negative number",
                    &number.to_string(),
                ));
            }
            Ok(Some(number))
        }
    }
}

fn evidence_from_value(file: &str, path: &str, value: &Value) -> Result<Vec<Value>, ConfigError> {
    let values = read::array(file, path, value)?;
    let mut evidence = Vec::with_capacity(values.len());
    for (index, item) in values.iter().enumerate() {
        let item_path = read::index(path, index);
        let object = read::object(file, &item_path, item)?;
        let kind = read::string(
            file,
            &read::child(&item_path, "kind"),
            read::field(file, &item_path, object, "kind")?,
        )?;
        match kind {
            "event" => {
                read::check_fields(
                    file,
                    &item_path,
                    object,
                    &[
                        "kind",
                        "event_code",
                        "event_type",
                        "subtype",
                        "source",
                        "occurred_bar",
                        "available_bar",
                        "direction",
                    ],
                )?;
                for name in [
                    "event_code",
                    "event_type",
                    "subtype",
                    "source",
                    "occurred_bar",
                    "available_bar",
                ] {
                    let _ = read::u64(
                        file,
                        &read::child(&item_path, name),
                        read::field(file, &item_path, object, name)?,
                    )?;
                }
                let direction = integer_field(file, &item_path, object, "direction")?;
                if !(-1..=1).contains(&direction) {
                    return Err(ConfigError::range(
                        file,
                        &read::child(&item_path, "direction"),
                        "a direction in -1..=1",
                        &direction.to_string(),
                    ));
                }
            }
            "state" => {
                read::check_fields(
                    file,
                    &item_path,
                    object,
                    &["kind", "timeframe_secs", "field", "value", "bar_index"],
                )?;
                let _ = read::u64(
                    file,
                    &read::child(&item_path, "timeframe_secs"),
                    read::field(file, &item_path, object, "timeframe_secs")?,
                )?;
                let _ = read::non_empty_string(
                    file,
                    &read::child(&item_path, "field"),
                    read::field(file, &item_path, object, "field")?,
                )?;
                let value = integer_field(file, &item_path, object, "value")?;
                if i8::try_from(value).is_err() {
                    return Err(ConfigError::range(
                        file,
                        &read::child(&item_path, "value"),
                        "a signed state code",
                        &value.to_string(),
                    ));
                }
                let _ = read::u64(
                    file,
                    &read::child(&item_path, "bar_index"),
                    read::field(file, &item_path, object, "bar_index")?,
                )?;
            }
            other => {
                return Err(ConfigError::range(
                    file,
                    &read::child(&item_path, "kind"),
                    "event or state",
                    other,
                ));
            }
        }
        evidence.push(item.clone());
    }
    Ok(evidence)
}

fn integer_field(
    file: &str,
    path: &str,
    object: &Map<String, Value>,
    name: &str,
) -> Result<i64, ConfigError> {
    read::field(file, path, object, name)?
        .as_i64()
        .ok_or_else(|| {
            ConfigError::wrong_type(
                file,
                &read::child(path, name),
                "an integer",
                &object
                    .get(name)
                    .map_or_else(|| "null".to_owned(), read::observed),
            )
        })
}

fn usize_field(
    file: &str,
    path: &str,
    object: &Map<String, Value>,
    name: &str,
) -> Result<usize, ConfigError> {
    let field_path = read::child(path, name);
    let number = read::u64(file, &field_path, read::field(file, path, object, name)?)?;
    usize::try_from(number).map_err(|_| {
        ConfigError::range(
            file,
            &field_path,
            "a representable bar index",
            &number.to_string(),
        )
    })
}

fn digest_field(
    file: &str,
    path: &str,
    object: &Map<String, Value>,
    name: &str,
) -> Result<Digest, ConfigError> {
    let field_path = read::child(path, name);
    let text = read::string(file, &field_path, read::field(file, path, object, name)?)?;
    Digest::parse(text).map_err(|error| {
        ConfigError::range(file, &field_path, "a sha256 digest", &error.to_string())
    })
}
