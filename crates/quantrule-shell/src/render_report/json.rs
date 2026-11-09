//! The numeric JSON envelope for the flat model report.
//!
//! The shell copies every published number, null, list, and stable code from
//! [`FlatReport`]; it computes no statistic and adds no label. The envelope
//! carries the compiled model version, the view-contract version, the caller's
//! data version, and the requested period.

use std::io;

use quantrule_model::report::{
    FlatContext, FlatEvent, FlatEventSeries, FlatLeg, FlatLevel, FlatProgression, FlatReport,
    FlatTimeframe,
};
use serde_json::{Value, json};

use super::VIEW_CONTRACT;

/// Renders the report envelope as compact JSON bytes with a trailing newline.
///
/// # Errors
///
/// Returns the serializer's I/O error when the value cannot be encoded.
pub(crate) fn json_bytes(
    report: &FlatReport,
    data_version: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> io::Result<Vec<u8>> {
    let value = json!({
        "model_version": env!("CARGO_PKG_VERSION"),
        "view_contract": VIEW_CONTRACT,
        "data_version": data_version,
        "requested_period": {
            "from": from,
            "to": to,
        },
        "flat_report": report_value(report),
    });
    let text = serde_json::to_string(&value).map_err(io::Error::other)?;
    Ok(format!("{text}\n").into_bytes())
}

/// Renders the flat v4 model result.
///
/// The result carries the effective config and the flat per-timeframe model
/// summary. It carries no axis, cross-timeframe aggregate, chart, prediction, or
/// confidence field.
///
/// # Errors
///
/// Returns the serializer's I/O error when the value cannot be encoded.
pub(crate) fn model_result_bytes(
    report: &FlatReport,
    ladder: &[u64],
    data_version: &str,
    from: Option<&str>,
    to: Option<&str>,
    effective_config: &Value,
) -> io::Result<Vec<u8>> {
    let value = json!({
        "schema": "quantrule.model-result/v4",
        "data_version": data_version,
        "model_version": env!("CARGO_PKG_VERSION"),
        "registry_version": quantrule_model::REGISTRY_VERSION,
        "requested_period": {
            "from": from,
            "to": to,
        },
        "effective_config": effective_config,
        "ladder": ladder,
        "timeframes": report.timeframes.iter().map(timeframe_value).collect::<Vec<_>>(),
    });
    let text = serde_json::to_string_pretty(&value).map_err(io::Error::other)?;
    Ok(format!("{text}\n").into_bytes())
}

fn report_value(report: &FlatReport) -> Value {
    json!({
        "timeframes": report.timeframes.iter().map(timeframe_value).collect::<Vec<_>>(),
    })
}

fn timeframe_value(timeframe: &FlatTimeframe) -> Value {
    json!({
        "timeframe_secs": timeframe.timeframe_secs,
        "bars": timeframe.bars,
        "expected_bars": timeframe.expected_bars,
        "missing_bars": timeframe.missing_bars,
        "coverage_share": timeframe.coverage_share,
        "first_open_time": timeframe.first_open_time,
        "last_open_time": timeframe.last_open_time,
        "latest": timeframe.latest.as_ref().map(latest_value),
        "levels": timeframe.levels.iter().map(level_value).collect::<Vec<_>>(),
        "events": event_series_value(&timeframe.events),
        "progression": timeframe.progression.as_ref().map(progression_value),
    })
}

fn latest_value(latest: &quantrule_model::report::FlatLatest) -> Value {
    json!({
        "bar_index": latest.bar_index,
        "open_time": latest.open_time,
        "readings": {
            "ema_fast": latest.readings.ema_fast,
            "ema_mid": latest.readings.ema_mid,
            "ema_slow": latest.readings.ema_slow,
            "rsi": latest.readings.rsi,
            "atr": latest.readings.atr,
            "rvol": latest.readings.rvol,
            "macd_line": latest.readings.macd_line,
            "macd_signal": latest.readings.macd_signal,
            "macd_histogram": latest.readings.macd_histogram,
            "adx": latest.readings.adx,
            "di_plus": latest.readings.di_plus,
            "di_minus": latest.readings.di_minus,
        },
        "states": {
            "ema_stack": latest.states.ema_stack,
            "rsi_zone": latest.states.rsi_zone,
            "macd_state": latest.states.macd_state,
            "macd_histogram_state": latest.states.macd_histogram_state,
            "adx_regime": latest.states.adx_regime,
            "adx_slope": latest.states.adx_slope,
            "di_state": latest.states.di_state,
            "rvol_regime": latest.states.rvol_regime,
        },
        "context": context_value(&latest.context),
    })
}

fn context_value(context: &FlatContext) -> Value {
    json!({
        "atr_ratio": context.atr_ratio,
        "rvol_ratio": context.rvol_ratio,
        "context_code": context.context_code,
    })
}

fn level_value(level: &FlatLevel) -> Value {
    json!({
        "reference_code": level.reference_code,
        "scale_code": level.scale_code,
        "kind_code": level.kind_code,
        "price": level.price,
        "occurred_bar_index": level.occurred_bar_index,
        "available_bar_index": level.available_bar_index,
        "distance_atr": level.distance_atr,
    })
}

fn event_series_value(events: &FlatEventSeries) -> Value {
    json!({
        "total": events.total,
        "by_type": events.by_type,
        "by_subtype": {
            "level": events.by_subtype.level,
            "divergence": events.by_subtype.divergence,
            "signal": events.by_subtype.signal,
            "other": events.by_subtype.other,
        },
        "latest": events.latest.iter().map(event_value).collect::<Vec<_>>(),
        "recent": events.recent.iter().map(event_value).collect::<Vec<_>>(),
    })
}

fn event_value(event: &FlatEvent) -> Value {
    json!({
        "event_type_code": event.event_type_code,
        "subtype_code": event.subtype_code,
        "source_code": event.source_code,
        "timeframe_secs": event.timeframe_secs,
        "occurred_bar_index": event.occurred_bar_index,
        "available_bar_index": event.available_bar_index,
        "direction_code": event.direction_code,
        "reference_code": event.reference_code,
        "primary_value": event.primary_value,
        "reference_value": event.reference_value,
        "data_completeness": event.data_completeness,
    })
}

fn progression_value(progression: &FlatProgression) -> Value {
    json!({
        "progress": progression.progress,
        "stage_code": progression.stage_code,
        "leg_direction": progression.leg_direction,
        "leg_scale_code": progression.leg_scale_code,
        "published": progression.published.as_ref().map(leg_value),
        "minor": progression.minor.as_ref().map(leg_value),
        "major": progression.major.as_ref().map(leg_value),
    })
}

fn leg_value(leg: &FlatLeg) -> Value {
    json!({
        "leg_id": leg.leg_id,
        "timeframe_secs": leg.timeframe_secs,
        "scale_code": leg.scale_code,
        "origin_price": leg.origin_price,
        "origin_occurred_at": leg.origin_occurred_at,
        "origin_available_at": leg.origin_available_at,
        "origin_confirmed_code": leg.origin_confirmed_code,
        "direction_code": leg.direction_code,
        "target_price": leg.target_price,
        "target_basis_code": leg.target_basis_code,
        "target_version": leg.target_version,
        "target_stable_code": leg.target_stable_code,
        "progress": leg.progress,
        "displacement_price": leg.displacement_price,
        "displacement_atr": leg.displacement_atr,
        "lifecycle_code": leg.lifecycle_code,
        "stage_code": leg.stage_code,
        "target_reached_code": leg.target_reached_code,
    })
}
