//! Golden and causality tests for the extended readings.
//!
//! The fixture is a checked-in CSV of the 260-bar readings fixture and every
//! extended reading. It is a regression lock generated from the implementation,
//! not an independent oracle. The v4 readings keep their independent fixture.

use quantrule_data::Candle;
use quantrule_model::readings::{ExtendedConfig, ExtendedReadings, Readings, ReadingsConfig};

const CANDLES_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const READINGS_FIXTURE: &str = include_str!("fixtures/extended_readings_golden.csv");
const HEADER: &str = "bar,timestamp,vwap,vwap_distance_atr,vwap_slope,vwap_band_upper,vwap_band_lower,poc,vah,val,poc_distance_atr,va_position,profile_skew,profile_concentration,poc_migration,rvol_fast,rvol_slow,rvol_ratio,volume_oscillator,climax_flag,climax_direction,climax_strength,absorption_flag,absorption_direction,absorption_strength,obv,obv_ma,obv_slope,cmf,cmf_slope,mfi,ad_line,ad_ma,atr_ratio,atr_slope,vol_regime,range_expansion,mar_fast,mar_slow,mar_ratio,keltner_upper,keltner_lower,keltner_position,donchian_upper,donchian_lower,donchian_position,structure_state,lower_high_count,lower_low_count,higher_high_count,higher_low_count,fib_direction,fib_low,fib_high,fib_236,fib_382,fib_500,fib_618,fib_786,fib_position,fib_nearest_ratio,fib_nearest_atr,band_upper,band_lower,band_center,band_touches,band_age,band_width_atr,band_position,last_pivot_high,last_pivot_low,level_touches,level_age,aroon_up,aroon_down,aroon_osc,body_ratio,upper_wick_ratio,lower_wick_ratio,close_position,pattern_code,pattern_direction,gap_atr,gap_filled,path_efficiency,direction_changes,bars_since_high,bars_since_low,drawdown_atr,range_position";
const CUTOFF: usize = 200;

const CANDLES_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";

fn number(field: &str) -> Option<f64> {
    if field.is_empty() {
        None
    } else {
        Some(
            field
                .parse::<f64>()
                .unwrap_or_else(|error| panic!("invalid number {field:?}: {error}")),
        )
    }
}

fn load_candles() -> Vec<Candle> {
    let mut lines = CANDLES_FIXTURE.lines();
    assert_eq!(
        lines.next(),
        Some(CANDLES_HEADER),
        "the readings fixture header changed"
    );
    let mut candles = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 23, "wrong column count in {line:?}");
        candles.push(
            Candle::new(
                fields[0].parse::<u64>().expect("the bar index is a u64"),
                fields[1].parse::<i64>().expect("the timestamp is an i64"),
                number(fields[2]).expect("the open is present"),
                number(fields[3]).expect("the high is present"),
                number(fields[4]).expect("the low is present"),
                number(fields[5]).expect("the close is present"),
                number(fields[6]),
            )
            .expect("the fixture candle is valid"),
        );
    }
    candles
}

fn compute(candles: &[Candle]) -> ExtendedReadings {
    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    ExtendedReadings::compute(candles, &readings, &ExtendedConfig::DEFAULT)
}

fn optional(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn render(candles: &[Candle], extended: &ExtendedReadings) -> String {
    let mut csv = String::from(HEADER);
    csv.push('\n');
    for (bar, candle) in candles.iter().enumerate() {
        let values = [
            bar.to_string(),
            candle.timestamp().to_string(),
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
            optional(extended.lower_high_count.get(bar)),
            optional(extended.lower_low_count.get(bar)),
            optional(extended.higher_high_count.get(bar)),
            optional(extended.higher_low_count.get(bar)),
            optional(extended.fib_direction.get(bar)),
            optional(extended.fib_low.get(bar)),
            optional(extended.fib_high.get(bar)),
            optional(extended.fib_236.get(bar)),
            optional(extended.fib_382.get(bar)),
            optional(extended.fib_500.get(bar)),
            optional(extended.fib_618.get(bar)),
            optional(extended.fib_786.get(bar)),
            optional(extended.fib_position.get(bar)),
            optional(extended.fib_nearest_ratio.get(bar)),
            optional(extended.fib_nearest_atr.get(bar)),
            optional(extended.band_upper.get(bar)),
            optional(extended.band_lower.get(bar)),
            optional(extended.band_center.get(bar)),
            optional(extended.band_touches.get(bar)),
            optional(extended.band_age.get(bar)),
            optional(extended.band_width_atr.get(bar)),
            optional(extended.band_position.get(bar)),
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
    csv
}

#[test]
fn the_extended_readings_match_the_golden_fixture() {
    let candles = load_candles();
    let extended = compute(&candles);
    assert_eq!(render(&candles, &extended), READINGS_FIXTURE);
}

#[test]
fn appending_bars_does_not_change_past_readings() {
    let candles = load_candles();
    let full = render(&candles, &compute(&candles));
    let prefix = render(&candles[..CUTOFF], &compute(&candles[..CUTOFF]));
    let expected: Vec<&str> = full.lines().take(CUTOFF + 1).collect();
    let observed: Vec<&str> = prefix.lines().collect();
    assert_eq!(expected, observed, "a past reading changed");
}

#[test]
fn repeated_computation_is_identical() {
    let candles = load_candles();
    assert_eq!(compute(&candles), compute(&candles));
}

#[test]
fn an_empty_series_has_only_the_header() {
    let readings = Readings::compute(&[]);
    let extended = ExtendedReadings::compute(&[], &readings, &ExtendedConfig::DEFAULT);
    assert_eq!(render(&[], &extended), format!("{HEADER}\n"));
}

#[test]
#[ignore = "regenerates the extended readings fixture deliberately"]
fn regenerate_extended_readings_fixture() {
    let candles = load_candles();
    let extended = compute(&candles);
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/extended_readings_golden.csv"
    );
    std::fs::write(path, render(&candles, &extended)).expect("the fixture is writable");
}
