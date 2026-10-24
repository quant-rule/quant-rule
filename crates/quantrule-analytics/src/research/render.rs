//! The byte shape of the layer files.
//!
//! The writer formats one row at a time with the contract's rules: an empty
//! field for null, the shortest round-trip decimal for a float, `0` for
//! negative zero, and `;`-separated lists. The column order matches
//! [`super::columns`].

use std::fmt::{self, Write};

use quantrule_data::Candle;
use quantrule_model::aggregate::{CrossTimeframeAggregates, ScaleStageAggregates};

use super::columns::Column;
use crate::layers::{HigherBlock, Layer1Row, Layer2Row, Layer3Row, Metrics, ReadingsSnapshot};
use crate::resolution::AxisResolution;

/// Writes one row and terminates it with a newline.
pub(crate) struct RowWriter {
    out: String,
    first: bool,
}

impl RowWriter {
    pub(crate) fn new() -> Self {
        Self {
            out: String::new(),
            first: true,
        }
    }

    fn field(&mut self, value: impl fmt::Display) {
        if !self.first {
            self.out.push(',');
        }
        self.first = false;
        let _ = write!(self.out, "{value}");
    }

    fn null(&mut self) {
        self.field("");
    }

    fn opt<T: fmt::Display>(&mut self, value: Option<T>) {
        match value {
            Some(value) => self.field(value),
            None => self.null(),
        }
    }

    fn opt_float(&mut self, value: Option<f64>) {
        match value {
            Some(value) => self.field(format_float(value)),
            None => self.null(),
        }
    }

    fn float(&mut self, value: f64) {
        self.field(format_float(value));
    }

    fn list(&mut self, values: &[u64]) {
        if values.is_empty() {
            self.null();
            return;
        }
        let mut text = String::new();
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                text.push(';');
            }
            let _ = write!(text, "{value}");
        }
        self.field(text);
    }

    fn end(mut self) -> String {
        self.out.push('\n');
        self.out
    }
}

/// The shortest round-trip decimal, with negative zero written as `0`.
pub(crate) fn format_float(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        value.to_string()
    }
}

/// A header line for a column list.
pub(crate) fn header(columns: &[Column]) -> String {
    let mut out = String::new();
    header_into(&mut out, columns);
    out.push('\n');
    out
}

fn header_into(out: &mut String, columns: &[Column]) {
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(column.name);
    }
}

fn block_header_into(out: &mut String, columns: &[Column], prefix: &str) {
    for (index, column) in columns.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "{prefix}{}", column.name);
    }
}

/// The Layer 1 file.
pub(crate) fn layer1(layers: &crate::layers::LayerSet) -> String {
    let mut out = header(&super::columns::LAYER1);
    for row in &layers.layer1 {
        out.push_str(&layer1_row(row));
    }
    out
}

/// The Layer 2 file.
pub(crate) fn layer2(layers: &crate::layers::LayerSet) -> String {
    let mut columns = Vec::with_capacity(73);
    columns.extend_from_slice(&super::columns::LAYER1);
    columns.extend_from_slice(&super::columns::RESOLUTIONS);
    let mut out = header(&columns);
    for row in &layers.layer2 {
        out.push_str(&layer2_row(row));
    }
    out
}

/// The Layer 3 file.
pub(crate) fn layer3(layers: &crate::layers::LayerSet, higher: &[u64]) -> String {
    let columns = super::columns::block_columns();
    let mut header_text = Vec::with_capacity(73);
    header_text.extend_from_slice(&super::columns::LAYER1);
    header_text.extend_from_slice(&super::columns::RESOLUTIONS);
    let mut out = String::new();
    header_into(&mut out, &header_text);
    for timeframe in higher {
        out.push(',');
        block_header_into(&mut out, &columns, &prefix(*timeframe));
    }
    out.push(',');
    header_into(&mut out, &super::columns::AGGREGATES);
    out.push('\n');

    for row in &layers.layer3 {
        out.push_str(&layer3_row(row, higher, &columns));
    }
    out
}

/// The `m5`/`m15`/`h1` prefix for a higher timeframe.
pub(crate) fn prefix(timeframe_secs: u64) -> String {
    match timeframe_secs {
        300 => "m5_".to_string(),
        900 => "m15_".to_string(),
        3_600 => "h1_".to_string(),
        other => format!("tf{other}_"),
    }
}

/// The prefix without the trailing underscore, for dictionary descriptions.
pub(crate) fn prefix_name(timeframe_secs: u64) -> String {
    match timeframe_secs {
        300 => "M5".to_string(),
        900 => "M15".to_string(),
        3_600 => "H1".to_string(),
        other => format!("TF{other}"),
    }
}

fn layer1_row(row: &Layer1Row) -> String {
    let mut w = RowWriter::new();
    w.field(row.timeframe_secs);
    w.field(row.bar_index);
    candle(row.candle, &mut w);
    readings(&row.readings, &mut w);
    metrics(&row.metrics, &mut w);
    w.end()
}

fn layer2_row(row: &Layer2Row) -> String {
    let mut w = RowWriter::new();
    w.field(row.layer1.timeframe_secs);
    w.field(row.layer1.bar_index);
    candle(row.layer1.candle, &mut w);
    readings(&row.layer1.readings, &mut w);
    metrics(&row.layer1.metrics, &mut w);
    resolutions(&row.resolution, &mut w);
    w.end()
}

fn layer3_row(row: &Layer3Row, higher: &[u64], columns: &[Column]) -> String {
    let mut w = RowWriter::new();
    w.field(row.base.layer1.timeframe_secs);
    w.field(row.base.layer1.bar_index);
    candle(row.base.layer1.candle, &mut w);
    readings(&row.base.layer1.readings, &mut w);
    metrics(&row.base.layer1.metrics, &mut w);
    resolutions(&row.base.resolution, &mut w);

    for timeframe in higher {
        match row
            .higher
            .iter()
            .find(|block| block.timeframe_secs == *timeframe)
        {
            Some(block) => block_cells(block, &mut w),
            None => {
                for _ in 0..columns.len() {
                    w.null();
                }
            }
        }
    }

    aggregates(&row.aggregates, &mut w);
    w.end()
}

fn candle(candle: Candle, w: &mut RowWriter) {
    w.field(candle.timestamp());
    w.float(candle.open());
    w.float(candle.high());
    w.float(candle.low());
    w.float(candle.close());
    w.opt_float(candle.volume());
}

fn readings(readings: &ReadingsSnapshot, w: &mut RowWriter) {
    w.opt_float(readings.ema20);
    w.opt_float(readings.ema50);
    w.opt_float(readings.ema200);
    w.opt_float(readings.rsi14);
    w.opt_float(readings.atr14);
    w.opt_float(readings.rvol20);
}

fn metrics(metrics: &Metrics, w: &mut RowWriter) {
    w.opt_float(metrics.direction.direction);
    w.opt(metrics.direction.state_code);
    w.opt_float(metrics.direction.size_percentile);
    w.field(metrics.direction.readiness_code);
    w.opt(metrics.direction.unavailable_reason_code);
    w.field(metrics.direction.state_age_bars);
    w.field(metrics.direction.state_changes_50);

    w.opt_float(metrics.momentum.force);
    w.opt_float(metrics.momentum.acceleration);
    w.opt_float(metrics.momentum.intensification);
    w.opt(metrics.momentum.state_code);
    w.opt_float(metrics.momentum.size_percentile);
    w.field(metrics.momentum.readiness_code);
    w.opt(metrics.momentum.unavailable_reason_code);
    w.field(metrics.momentum.state_age_bars);
    w.field(metrics.momentum.state_changes_50);

    w.opt_float(metrics.structure.score);
    w.opt(metrics.structure.state_code);
    w.field(metrics.structure.scale_code);
    w.opt(metrics.structure.high_relation_code);
    w.opt(metrics.structure.low_relation_code);
    w.opt_float(metrics.structure.ema_distance_atr_20);
    w.opt_float(metrics.structure.ema_distance_atr_50);
    w.opt_float(metrics.structure.ema_distance_atr_200);
    w.opt(metrics.structure.ema_cross_code_20);
    w.opt(metrics.structure.ema_cross_code_50);
    w.opt(metrics.structure.ema_cross_code_200);
    w.opt(metrics.structure.ema_retest_code_20);
    w.opt(metrics.structure.ema_retest_code_50);
    w.opt(metrics.structure.ema_retest_code_200);
    w.opt_float(metrics.structure.size_percentile);
    w.field(metrics.structure.readiness_code);
    w.opt(metrics.structure.unavailable_reason_code);
    w.field(metrics.structure.state_age_bars);
    w.field(metrics.structure.state_changes_50);

    w.opt_float(metrics.progression.progress);
    w.opt(metrics.progression.stage_code);
    w.opt(metrics.progression.leg_direction);
    w.opt(metrics.progression.leg_scale_code);
    w.opt_float(metrics.progression.size_percentile);
    w.field(metrics.progression.readiness_code);
    w.opt(metrics.progression.unavailable_reason_code);
    w.field(metrics.progression.state_age_bars);
    w.field(metrics.progression.state_changes_50);

    w.opt_float(metrics.context.atr_ratio);
    w.opt_float(metrics.context.rvol_ratio);
    w.opt(metrics.context.context_code);
}

fn resolutions(resolutions: &crate::resolution::AxisResolutions, w: &mut RowWriter) {
    resolution(resolutions.direction, w);
    resolution(resolutions.momentum, w);
    resolution(resolutions.structure, w);
    resolution(resolutions.progression, w);
}

fn resolution(resolution: AxisResolution, w: &mut RowWriter) {
    w.opt_float(resolution.continuous);
    w.opt(resolution.ordinal);
    w.opt(resolution.reason);
}

fn block_cells(block: &HigherBlock, w: &mut RowWriter) {
    w.field(block.bar_index);
    candle(block.candle, w);
    metrics(&block.metrics, w);
    resolutions(&block.resolution, w);
    w.field(block.staleness_secs);
}

fn aggregates(aggregates: &CrossTimeframeAggregates, w: &mut RowWriter) {
    w.list(&aggregates.ladder);

    let direction = &aggregates.direction;
    w.list(&direction.timeframes);
    w.field(direction.direction_timeframe_count);
    w.opt_float(direction.direction_mean);
    w.opt_float(direction.direction_bullish_share);
    w.opt_float(direction.direction_bearish_share);
    w.opt_float(direction.direction_neutral_share);
    w.opt_float(direction.direction_gradient);
    w.field(direction.direction_ready_count);

    let momentum = &aggregates.momentum;
    w.list(&momentum.timeframes);
    w.field(momentum.momentum_timeframe_count);
    w.opt_float(momentum.momentum_force_mean);
    w.opt_float(momentum.momentum_acceleration_mean);
    w.opt_float(momentum.momentum_positive_share);
    w.opt_float(momentum.momentum_negative_share);
    w.opt_float(momentum.momentum_force_fast_slow_gap);
    w.opt_float(momentum.momentum_acceleration_fast_slow_gap);
    w.opt_float(momentum.momentum_gradient);
    w.field(momentum.momentum_ready_count);

    let structure = &aggregates.structure;
    w.list(&structure.timeframes);
    w.field(structure.structure_timeframe_count);
    for count in structure.structure_code_histogram {
        w.field(count);
    }
    w.opt_float(structure.structure_bullish_share);
    w.opt_float(structure.structure_bearish_share);
    w.opt_float(structure.structure_neutral_share);
    w.opt(structure.structure_state_span);
    w.opt_float(structure.structure_gradient);
    w.opt_float(structure.structure_ready_share);
    w.field(structure.structure_conflict_timeframe_count);
    w.field(structure.level_cluster_count);
    w.field(structure.largest_level_cluster_timeframe_count);
    w.opt_float(structure.largest_level_cluster_span_atr);

    let progression = &aggregates.progression;
    w.list(&progression.timeframes);
    w.field(progression.progression_timeframe_count);
    stages(&progression.minor, w);
    stages(&progression.major, w);
    w.field(progression.active_minor_leg_count);
    w.field(progression.active_major_leg_count);
    w.opt_float(progression.progression_ready_share);
    w.opt_float(progression.leg_direction_agreement_share);
    w.field(progression.cross_timeframe_leg_match_count);

    let events = &aggregates.events;
    for count in events.event_count_by_type {
        w.field(count);
    }
    for count in events.event_count_by_subtype.level {
        w.field(count);
    }
    for count in events.event_count_by_subtype.divergence {
        w.field(count);
    }
    for count in events.event_count_by_subtype.signal {
        w.field(count);
    }
    w.field(events.divergence_timeframe_count);
    w.opt_float(events.divergence_direction_agreement_share);
    w.field(events.level_event_timeframe_count);
    w.field(events.signal_event_timeframe_count);
}

fn stages(stage: &ScaleStageAggregates, w: &mut RowWriter) {
    for count in stage.stage_histogram {
        w.field(count);
    }
    w.opt(stage.stage_mode);
    w.opt(stage.stage_min);
    w.opt(stage.stage_max);
    w.opt(stage.stage_span);
    w.opt_float(stage.modal_stage_share);
}
