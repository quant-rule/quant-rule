//! The deterministic chart projection of the market report.
//!
//! The projection selects H1 when present, otherwise the slowest present
//! timeframe, and reduces that series to at most [`CHART_BUCKET_LIMIT`]
//! buckets. A bucket carries the first open, the maximum high, the minimum low,
//! the last close, the summed volume, and the last published axis value. An
//! ordinal code is never averaged.
//!
//! The module is pure: it reads the run and the per-timeframe reports it is
//! given. It performs no I/O, reads no clock, and uses no randomness.

use crate::run::{ModelRun, TimeframeModel};

use super::order::event_cmp;
use super::{
    CHART_BUCKET_LIMIT, CHART_TIMEFRAME_SECS, ChartReport, ReportChartBar, ReportChartEvent,
    TimeframeReport,
};

impl ChartReport {
    /// Projects the selected series onto the report's chart buckets.
    pub(super) fn compute(run: &ModelRun, timeframes: &[TimeframeReport]) -> Self {
        let Some(timeframe_secs) = select_timeframe(timeframes) else {
            return Self::empty();
        };
        let Some(timeframe) = run.timeframe(timeframe_secs) else {
            return Self::empty();
        };

        let source_bars = timeframe.len();
        if source_bars == 0 {
            return Self::empty();
        }
        let bucket_size = source_bars.div_ceil(CHART_BUCKET_LIMIT);
        let mut bars = Vec::with_capacity(source_bars.div_ceil(bucket_size));
        let mut start = 0;
        while start < source_bars {
            let end = start.saturating_add(bucket_size).min(source_bars);
            if let Some(bar) = bucket(timeframe, start, end) {
                bars.push(bar);
            }
            start = end;
        }

        let mut records = timeframe.events().to_vec();
        records.sort_by(event_cmp);
        let events = records
            .iter()
            .map(|event| ReportChartEvent {
                bucket_index: event.available_bar_index / bucket_size,
                available_bar_index: event.available_bar_index,
                occurred_bar_index: event.occurred_bar_index,
                event_type_code: event.event_type.code(),
                event_subtype_code: event.event_subtype_code,
                direction_code: event.direction_code,
            })
            .collect();

        Self {
            timeframe_secs: Some(timeframe_secs),
            source_bars,
            bucket_size,
            bars,
            events,
        }
    }

    /// The report's empty chart.
    fn empty() -> Self {
        Self {
            timeframe_secs: None,
            source_bars: 0,
            bucket_size: 1,
            bars: Vec::new(),
            events: Vec::new(),
        }
    }
}

/// Selects H1 when present, otherwise the slowest present timeframe.
fn select_timeframe(timeframes: &[TimeframeReport]) -> Option<u64> {
    if let Some(h1) = timeframes
        .iter()
        .find(|timeframe| timeframe.timeframe_secs == CHART_TIMEFRAME_SECS && timeframe.bars > 0)
    {
        return Some(h1.timeframe_secs);
    }
    timeframes
        .iter()
        .rev()
        .find(|timeframe| timeframe.bars > 0)
        .map(|timeframe| timeframe.timeframe_secs)
}

/// Builds one bucket over `[start, end)`.
fn bucket(timeframe: &TimeframeModel, start: usize, end: usize) -> Option<ReportChartBar> {
    let first = timeframe.candle(start)?;
    let last = timeframe.candle(end.checked_sub(1)?)?;
    let mut high = first.high();
    let mut low = first.low();
    let mut volume = Some(0.0);
    let mut direction = None;
    let mut momentum = None;
    let mut structure = None;
    let mut progression_stage_code = None;
    let mut progression_progress = None;

    for bar in start..end {
        let Some(candle) = timeframe.candle(bar) else {
            continue;
        };
        if candle.high().total_cmp(&high).is_gt() {
            high = candle.high();
        }
        if candle.low().total_cmp(&low).is_lt() {
            low = candle.low();
        }
        match candle.volume() {
            Some(value) => {
                if let Some(total) = volume.as_mut() {
                    *total += value;
                }
            }
            None => volume = None,
        }
        if let Some(value) = timeframe.direction().get(bar).and_then(|bar| bar.direction) {
            direction = Some(value);
        }
        if let Some(value) = timeframe.momentum().get(bar).and_then(|bar| bar.force) {
            momentum = Some(value);
        }
        if let Some(value) = timeframe.structure().get(bar).and_then(|bar| bar.score) {
            structure = Some(value);
        }
        if let Some(value) = timeframe
            .progression()
            .get(bar)
            .and_then(|bar| bar.stage_code)
        {
            progression_stage_code = Some(value);
        }
        if let Some(value) = timeframe
            .progression()
            .get(bar)
            .and_then(|bar| bar.progress)
        {
            progression_progress = Some(value);
        }
    }

    Some(ReportChartBar {
        first_bar_index: start,
        last_bar_index: end.checked_sub(1)?,
        open_time: first.timestamp(),
        last_open_time: last.timestamp(),
        open: first.open(),
        high,
        low,
        close: last.close(),
        volume,
        direction,
        momentum,
        structure,
        progression_stage_code,
        progression_progress,
    })
}
