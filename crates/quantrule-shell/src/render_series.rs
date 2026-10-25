//! Rendering for the per-bar model series.
//!
//! The shell maps the model's numeric bars and codes to the command's JSON
//! shape. It does not compute an axis, aggregate a timeframe, or detect an
//! event; it only formats values and fixed presentation labels.

use std::io::{self, Write};

use quantrule_data::format_utc_timestamp;
use quantrule_model::run::ModelBar;
use serde_json::{Value, json};

use crate::labels;

/// Writes one `ModelBar` as a single NDJSON line.
pub(crate) fn write_bar(bar: &ModelBar, writer: &mut impl Write) -> io::Result<()> {
    let value = bar_value(bar);
    let text = serde_json::to_string(&value).map_err(io::Error::other)?;
    writer.write_all(text.as_bytes())?;
    writer.write_all(b"\n")
}

fn bar_value(bar: &ModelBar) -> Value {
    let candle = &bar.candle;
    let direction = &bar.direction;
    let momentum = &bar.momentum;
    let structure = &bar.structure;
    let progression = &bar.progression;
    let context = &bar.context;
    let consensus = &bar.consensus;

    json!({
        "timeframe": labels::timeframe_label(bar.timeframe_secs),
        "bar": bar.bar_index,
        "series_index": candle.series_index(),
        "datetime": format_utc_timestamp(candle.timestamp()),
        "open": candle.open(),
        "high": candle.high(),
        "low": candle.low(),
        "close": candle.close(),
        "volume": candle.volume(),
        "readings": {
            "ema20": bar.readings.ema20,
            "ema50": bar.readings.ema50,
            "ema200": bar.readings.ema200,
            "rsi14": bar.readings.rsi14,
            "atr14": bar.readings.atr14,
            "rvol20": bar.readings.rvol20,
        },
        "direction": {
            "value": direction.direction,
            "state": direction.state_code,
            "state_label": direction.state_code.map(labels::direction_state_label),
            "percentile": direction.size_percentile,
            "readiness": direction.readiness.code(),
            "readiness_label": labels::readiness_label(direction.readiness.code()),
            "reason": direction.unavailable_reason.map(|reason| reason.code()),
            "reason_label": direction.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": direction.state_age_bars,
            "state_changes_50": direction.state_changes_50,
        },
        "momentum": {
            "force": momentum.force,
            "acceleration": momentum.acceleration,
            "intensification": momentum.intensification,
            "state": momentum.state_code,
            "state_label": momentum.state_code.map(labels::momentum_state_label),
            "percentile": momentum.size_percentile,
            "readiness": momentum.readiness.code(),
            "readiness_label": labels::readiness_label(momentum.readiness.code()),
            "reason": momentum.unavailable_reason.map(|reason| reason.code()),
            "reason_label": momentum.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": momentum.state_age_bars,
            "state_changes_50": momentum.state_changes_50,
        },
        "structure": {
            "score": structure.score,
            "code": structure.state_code,
            "code_label": structure.state_code.map(labels::structure_label),
            "scale": labels::scale_label(structure.scale_code),
            "high_relation": structure.high_relation_code,
            "low_relation": structure.low_relation_code,
            "ema_distance_atr_20": structure.ema_distance_atr_20,
            "ema_distance_atr_50": structure.ema_distance_atr_50,
            "ema_distance_atr_200": structure.ema_distance_atr_200,
            "ema_cross_code_20": structure.ema_cross_code_20,
            "ema_cross_code_50": structure.ema_cross_code_50,
            "ema_cross_code_200": structure.ema_cross_code_200,
            "ema_retest_code_20": structure.ema_retest_code_20,
            "ema_retest_code_50": structure.ema_retest_code_50,
            "ema_retest_code_200": structure.ema_retest_code_200,
            "percentile": structure.size_percentile,
            "readiness": structure.readiness.code(),
            "readiness_label": labels::readiness_label(structure.readiness.code()),
            "reason": structure.unavailable_reason.map(|reason| reason.code()),
            "reason_label": structure.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": structure.state_age_bars,
            "state_changes_50": structure.state_changes_50,
        },
        "progression": {
            "progress": progression.progress,
            "stage": progression.stage_code,
            "stage_label": progression.stage_code.map(labels::stage_label),
            "leg_direction": progression.leg_direction,
            "leg_direction_label": progression.leg_direction.map(labels::leg_direction_label),
            "leg_scale": progression.leg_scale_code.map(labels::scale_label),
            "percentile": progression.size_percentile,
            "readiness": progression.readiness.code(),
            "readiness_label": labels::readiness_label(progression.readiness.code()),
            "reason": progression.unavailable_reason.map(|reason| reason.code()),
            "reason_label": progression.unavailable_reason.map(|reason| labels::reason_label(reason.code())),
            "state_age_bars": progression.state_age_bars,
            "state_changes_50": progression.state_changes_50,
        },
        "context": {
            "atr_ratio": context.atr_ratio,
            "rvol_ratio": context.rvol_ratio,
            "code": context.context_code,
            "code_label": context.context_code.map(labels::context_label),
        },
        "consensus": {
            "bullish": consensus.bullish_axis_count,
            "bearish": consensus.bearish_axis_count,
            "neutral": consensus.neutral_axis_count,
            "weakest_axis": consensus.weakest_directional_axis_code,
            "weakest_axis_label": consensus.weakest_directional_axis_code.map(labels::axis_label),
        },
    })
}
