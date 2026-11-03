//! Exhaustive model-bundle CSV artifacts.
//!
//! The JSON report is a summary and latest snapshot. These CSV files carry the
//! full per-bar readings, the full event batch, every confirmed level, and the
//! retained progression legs. The module formats numbers; it computes no
//! indicator and applies no rule.

use std::fmt::Write as _;

use quantrule_model::events::{EventRecord, EventType};
use quantrule_model::readings::{PivotKind, PivotScale};
use quantrule_model::run::ModelRun;

/// The per-bar readings and states for every ladder timeframe.
pub(super) fn readings_csv(run: &ModelRun, ladder: &[u64]) -> String {
    let mut csv = String::from(
        "timeframe_secs,bar,timestamp,open,high,low,close,volume,ema_fast,ema_mid,ema_slow,rsi,atr,rvol,macd_line,macd_signal,macd_histogram,ema_stack,rsi_zone,macd_state\n",
    );
    for &timeframe_secs in ladder {
        let Some(timeframe) = run.timeframe(timeframe_secs) else {
            continue;
        };
        let readings = timeframe.readings();
        let macd = timeframe.macd();
        for bar in 0..timeframe.len() {
            let Some(candle) = timeframe.candle(bar) else {
                continue;
            };
            let ema_fast = readings.ema20.get(bar);
            let ema_mid = readings.ema50.get(bar);
            let ema_slow = readings.ema200.get(bar);
            let rsi = readings.rsi14.get(bar);
            let macd_line = macd.line.get(bar);
            let macd_signal = macd.signal.get(bar);
            let _ = writeln!(
                csv,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                timeframe_secs,
                bar,
                candle.timestamp(),
                candle.open(),
                candle.high(),
                candle.low(),
                candle.close(),
                optional(candle.volume()),
                optional(ema_fast),
                optional(ema_mid),
                optional(ema_slow),
                optional(rsi),
                optional(readings.atr14.get(bar)),
                optional(readings.rvol20.get(bar)),
                optional(macd_line),
                optional(macd_signal),
                optional(macd.histogram.get(bar)),
                optional(ema_stack(ema_fast, ema_mid, ema_slow).map(f64::from)),
                optional(rsi.map(rsi_zone).map(f64::from)),
                optional(macd_state(macd_line, macd_signal).map(f64::from)),
            );
        }
    }
    csv
}

/// The full event batch for every ladder timeframe.
pub(super) fn events_csv(run: &ModelRun, ladder: &[u64]) -> String {
    let mut csv = String::from(
        "timeframe_secs,event_index,event_type_code,event_subtype_code,source_code,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value,data_completeness\n",
    );
    for &timeframe_secs in ladder {
        let Some(timeframe) = run.timeframe(timeframe_secs) else {
            continue;
        };
        for (event_index, event) in timeframe.events().iter().enumerate() {
            let _ = writeln!(
                csv,
                "{},{},{},{},{},{},{},{},{},{},{},{}",
                timeframe_secs,
                event_index,
                event.event_type.code(),
                event.event_subtype_code,
                event.source_code,
                event.occurred_bar_index,
                event.available_bar_index,
                event.direction_code,
                optional(event.reference_code.map(f64::from)),
                optional(event.primary_value),
                optional(event.reference_value),
                optional(event_completeness(event)),
            );
        }
    }
    csv
}

/// Every confirmed level for every ladder timeframe.
pub(super) fn levels_csv(run: &ModelRun, ladder: &[u64]) -> String {
    let mut csv = String::from(
        "timeframe_secs,reference_code,scale_code,kind_code,price,occurred_bar_index,available_bar_index,distance_atr\n",
    );
    for &timeframe_secs in ladder {
        let Some(timeframe) = run.timeframe(timeframe_secs) else {
            continue;
        };
        let latest = timeframe.len().saturating_sub(1);
        let close = timeframe.candle(latest).map(|candle| candle.close());
        let atr = timeframe.readings().atr14.get(latest);
        for level in timeframe.levels() {
            let distance = match (close, atr) {
                (Some(close), Some(atr)) if atr > 0.0 => Some((close - level.price) / atr),
                _ => None,
            };
            let kind = match level.kind {
                PivotKind::High => 1.0,
                PivotKind::Low => -1.0,
            };
            let scale = match level.scale {
                PivotScale::Minor => 1.0,
                PivotScale::Major => 2.0,
            };
            let _ = writeln!(
                csv,
                "{},{},{},{},{},{},{},{}",
                timeframe_secs,
                level.reference_code,
                scale,
                kind,
                level.price,
                level.occurred_at,
                level.available_at,
                optional(distance),
            );
        }
    }
    csv
}

/// Every retained progression leg for every ladder timeframe.
pub(super) fn legs_csv(run: &ModelRun, ladder: &[u64]) -> String {
    let mut csv = String::from(
        "timeframe_secs,leg_id,scale_code,origin_price,origin_occurred_at,origin_available_at,origin_confirmed_code,direction_code,target_price,target_basis_code,target_version,target_stable_code,progress,displacement_price,displacement_atr,lifecycle_code,stage_code,target_reached_code\n",
    );
    for &timeframe_secs in ladder {
        let Some(timeframe) = run.timeframe(timeframe_secs) else {
            continue;
        };
        for leg in timeframe.legs() {
            let _ = writeln!(
                csv,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                timeframe_secs,
                leg.leg_id,
                leg.scale_code,
                leg.origin_price,
                leg.origin_occurred_at,
                optional(leg.origin_available_at.map(|value| value as f64)),
                leg.origin_confirmed_code,
                leg.direction_code,
                optional(leg.target_price),
                optional(leg.target_basis_code.map(f64::from)),
                leg.target_version,
                leg.target_stable_code,
                optional(leg.progress),
                optional(leg.displacement_price),
                optional(leg.displacement_atr),
                leg.lifecycle_code,
                optional(leg.stage_code.map(f64::from)),
                optional(leg.target_reached_code.map(f64::from)),
            );
        }
    }
    csv
}

fn optional(value: Option<f64>) -> String {
    value.map_or_else(String::new, |value| value.to_string())
}

fn event_completeness(event: &EventRecord) -> Option<f64> {
    let required = match event.event_type {
        EventType::Level | EventType::Divergence => 2_u8,
        EventType::Signal => 1_u8,
    };
    let present =
        u8::from(event.primary_value.is_some()) + u8::from(event.reference_value.is_some());
    Some(f64::from(present.min(required)) / f64::from(required))
}

fn ema_stack(fast: Option<f64>, mid: Option<f64>, slow: Option<f64>) -> Option<i8> {
    let (fast, mid, slow) = (fast?, mid?, slow?);
    if fast > mid && mid > slow {
        Some(1)
    } else if fast < mid && mid < slow {
        Some(-1)
    } else {
        Some(0)
    }
}

fn rsi_zone(rsi: f64) -> i8 {
    if rsi < 30.0 {
        -1
    } else if rsi > 70.0 {
        1
    } else {
        0
    }
}

fn macd_state(line: Option<f64>, signal: Option<f64>) -> Option<i8> {
    let (line, signal) = (line?, signal?);
    if line > signal {
        Some(1)
    } else if line < signal {
        Some(-1)
    } else {
        Some(0)
    }
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;
    use quantrule_model::input::{MarketStateConfig, TimeframeInput};
    use quantrule_model::run::ModelRun;

    use super::{events_csv, legs_csv, levels_csv, readings_csv};

    fn candles() -> Vec<Candle> {
        (0..90)
            .map(|index| {
                let close = 100.0 + (index as f64 * 0.37).sin() * 3.0;
                Candle::new(
                    index as u64,
                    index as i64 * 300,
                    close,
                    close + 1.0,
                    close - 1.0,
                    close,
                    Some(1.0 + index as f64),
                )
                .expect("the test candle is valid")
            })
            .collect()
    }

    #[test]
    fn the_model_artifacts_have_headers_and_rows() {
        let candles = candles();
        let run = ModelRun::compute(
            &[TimeframeInput::new(300, &candles)],
            &MarketStateConfig::default(),
        );
        let ladder = [300_u64];

        let readings = readings_csv(&run, &ladder);
        assert!(readings.starts_with("timeframe_secs,bar,timestamp"));
        assert_eq!(readings.lines().count(), 91);

        let events = events_csv(&run, &ladder);
        assert!(events.starts_with("timeframe_secs,event_index,event_type_code"));
        assert_eq!(
            events.lines().count(),
            run.timeframe(300).expect("the timeframe").events().len() + 1
        );

        let levels = levels_csv(&run, &ladder);
        assert!(levels.starts_with("timeframe_secs,reference_code,scale_code"));
        assert_eq!(
            levels.lines().count(),
            run.timeframe(300).expect("the timeframe").levels().len() + 1
        );

        let legs = legs_csv(&run, &ladder);
        assert!(legs.starts_with("timeframe_secs,leg_id,scale_code"));
        assert_eq!(
            legs.lines().count(),
            run.timeframe(300).expect("the timeframe").legs().len() + 1
        );
        for csv in [readings, events, levels, legs] {
            assert!(!csv.contains("NaN"), "a CSV contains NaN");
        }
    }
}
