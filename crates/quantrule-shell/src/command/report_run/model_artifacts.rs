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
        "timeframe_secs,bar,timestamp,open,high,low,close,volume,ema_fast,ema_mid,ema_slow,rsi,atr,rvol,macd_line,macd_signal,macd_histogram,adx,di_plus,di_minus,ema_stack,rsi_zone,macd_state,adx_regime,adx_slope,di_state,progression_stage_code,progression_leg_direction,atr_ratio,rvol_ratio,context_code,vwap,vwap_distance_atr,vwap_slope,vwap_band_upper,vwap_band_lower,poc,vah,val,poc_distance_atr,va_position,profile_skew,profile_concentration,poc_migration,rvol_fast,rvol_slow,rvol_ratio,volume_oscillator,climax_flag,climax_direction,climax_strength,absorption_flag,absorption_direction,absorption_strength,obv,obv_ma,obv_slope,cmf,cmf_slope,mfi,ad_line,ad_ma,atr_ratio,atr_slope,vol_regime,range_expansion,mar_fast,mar_slow,mar_ratio,keltner_upper,keltner_lower,keltner_position,donchian_upper,donchian_lower,donchian_position,structure_state,last_pivot_high,last_pivot_low,level_touches,level_age,aroon_up,aroon_down,aroon_osc,body_ratio,upper_wick_ratio,lower_wick_ratio,close_position,pattern_code,pattern_direction,gap_atr,gap_filled,path_efficiency,direction_changes,bars_since_high,bars_since_low,drawdown_atr,range_position\n",
    );
    for &timeframe_secs in ladder {
        let Some(timeframe) = run.timeframe(timeframe_secs) else {
            continue;
        };
        let readings = timeframe.readings();
        let extended = timeframe.extended();
        let macd = timeframe.macd();
        let adx = timeframe.adx();
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
            let progression = timeframe.progression().get(bar);
            let adx_state = timeframe.adx_state(bar);
            let context = timeframe.context().get(bar);
            let values = [
                timeframe_secs.to_string(),
                bar.to_string(),
                candle.timestamp().to_string(),
                candle.open().to_string(),
                candle.high().to_string(),
                candle.low().to_string(),
                candle.close().to_string(),
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
                optional(adx.adx.get(bar)),
                optional(adx.di_plus.get(bar)),
                optional(adx.di_minus.get(bar)),
                optional(ema_stack(ema_fast, ema_mid, ema_slow).map(f64::from)),
                optional(rsi.map(rsi_zone).map(f64::from)),
                optional(macd_state(macd_line, macd_signal).map(f64::from)),
                optional(adx_state.adx_regime.map(f64::from)),
                optional(adx_state.adx_slope.map(f64::from)),
                optional(adx_state.di_state.map(f64::from)),
                optional(progression.and_then(|bar| bar.stage_code).map(f64::from)),
                optional(progression.and_then(|bar| bar.leg_direction).map(f64::from)),
                optional(context.and_then(|bar| bar.atr_ratio)),
                optional(context.and_then(|bar| bar.rvol_ratio)),
                optional(context.and_then(|bar| bar.context_code).map(f64::from)),
                optional(extended.vwap.get(bar)),
                optional(extended.vwap_distance_atr.get(bar)),
                optional(extended.vwap_slope.get(bar)),
                optional(extended.vwap_band_upper.get(bar)),
                optional(extended.vwap_band_lower.get(bar)),
                optional(extended.poc.get(bar)),
                optional(extended.vah.get(bar)),
                optional(extended.val.get(bar)),
                optional(extended.poc_distance_atr.get(bar)),
                optional(extended.va_position.get(bar)),
                optional(extended.profile_skew.get(bar)),
                optional(extended.profile_concentration.get(bar)),
                optional(extended.poc_migration.get(bar)),
                optional(extended.rvol_fast.get(bar)),
                optional(extended.rvol_slow.get(bar)),
                optional(extended.rvol_ratio.get(bar)),
                optional(extended.volume_oscillator.get(bar)),
                optional(extended.climax_flag.get(bar)),
                optional(extended.climax_direction.get(bar)),
                optional(extended.climax_strength.get(bar)),
                optional(extended.absorption_flag.get(bar)),
                optional(extended.absorption_direction.get(bar)),
                optional(extended.absorption_strength.get(bar)),
                optional(extended.obv.get(bar)),
                optional(extended.obv_ma.get(bar)),
                optional(extended.obv_slope.get(bar)),
                optional(extended.cmf.get(bar)),
                optional(extended.cmf_slope.get(bar)),
                optional(extended.mfi.get(bar)),
                optional(extended.ad_line.get(bar)),
                optional(extended.ad_ma.get(bar)),
                optional(extended.atr_ratio.get(bar)),
                optional(extended.atr_slope.get(bar)),
                optional(extended.vol_regime.get(bar)),
                optional(extended.range_expansion.get(bar)),
                optional(extended.mar_fast.get(bar)),
                optional(extended.mar_slow.get(bar)),
                optional(extended.mar_ratio.get(bar)),
                optional(extended.keltner_upper.get(bar)),
                optional(extended.keltner_lower.get(bar)),
                optional(extended.keltner_position.get(bar)),
                optional(extended.donchian_upper.get(bar)),
                optional(extended.donchian_lower.get(bar)),
                optional(extended.donchian_position.get(bar)),
                optional(extended.structure_state.get(bar)),
                optional(extended.last_pivot_high.get(bar)),
                optional(extended.last_pivot_low.get(bar)),
                optional(extended.level_touches.get(bar)),
                optional(extended.level_age.get(bar)),
                optional(extended.aroon_up.get(bar)),
                optional(extended.aroon_down.get(bar)),
                optional(extended.aroon_osc.get(bar)),
                optional(extended.body_ratio.get(bar)),
                optional(extended.upper_wick_ratio.get(bar)),
                optional(extended.lower_wick_ratio.get(bar)),
                optional(extended.close_position.get(bar)),
                optional(extended.pattern_code.get(bar)),
                optional(extended.pattern_direction.get(bar)),
                optional(extended.gap_atr.get(bar)),
                optional(extended.gap_filled.get(bar)),
                optional(extended.path_efficiency.get(bar)),
                optional(extended.direction_changes.get(bar)),
                optional(extended.bars_since_high.get(bar)),
                optional(extended.bars_since_low.get(bar)),
                optional(extended.drawdown_atr.get(bar)),
                optional(extended.range_position.get(bar)),
            ];
            csv.push_str(&values.join(","));
            csv.push('\n');
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
        for (event_index, event) in timeframe
            .events()
            .iter()
            .chain(timeframe.extended_events().events())
            .enumerate()
        {
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
        let timeframe = run.timeframe(300).expect("the timeframe");
        assert_eq!(
            events.lines().count(),
            timeframe.events().len() + timeframe.extended_events().len() + 1
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
