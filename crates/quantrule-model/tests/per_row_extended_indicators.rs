//! Row-level contracts for the extended indicator readings.
//!
//! Each test names one indicator and compares every published column against the
//! checked-in extended readings fixture, then checks prefix causality,
//! determinism, and empty/single/short safety. The fixture is a regression lock
//! generated from the implementation, not an independent oracle.

use quantrule_data::Candle;
use quantrule_model::events::codes;
use quantrule_model::events::extended::{ExtendedEventConfig, ExtendedEventSeries};
use quantrule_model::readings::{
    ExtendedConfig, ExtendedReadings, ReadingSeries, Readings, ReadingsConfig,
};

const CANDLES_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const READINGS_FIXTURE: &str = include_str!("fixtures/extended_readings_golden.csv");
const CANDLES_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const CUTOFF: usize = 200;

type Columns = Vec<(&'static str, ReadingSeries)>;

struct Fixture {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Fixture {
    fn column(&self, name: &str) -> usize {
        self.header
            .iter()
            .position(|column| column == name)
            .unwrap_or_else(|| panic!("the fixture has no column {name}"))
    }
}

fn number(field: &str) -> Option<f64> {
    if field.is_empty() {
        None
    } else {
        Some(field.parse().expect("a fixture number"))
    }
}

fn load_candles() -> Vec<Candle> {
    let mut lines = CANDLES_FIXTURE.lines();
    assert_eq!(lines.next(), Some(CANDLES_HEADER));
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 23);
            Candle::new(
                fields[0].parse().expect("bar"),
                fields[1].parse().expect("timestamp"),
                number(fields[2]).expect("open"),
                number(fields[3]).expect("high"),
                number(fields[4]).expect("low"),
                number(fields[5]).expect("close"),
                number(fields[6]),
            )
            .expect("valid fixture candle")
        })
        .collect()
}

fn load_fixture(text: &str) -> Fixture {
    let mut lines = text.lines();
    let header = lines
        .next()
        .expect("the fixture header")
        .split(',')
        .map(str::to_string)
        .collect();
    let rows = lines
        .map(|line| line.split(',').map(str::to_string).collect())
        .collect();
    Fixture { header, rows }
}

fn compute(candles: &[Candle]) -> ExtendedReadings {
    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    ExtendedReadings::compute(candles, &readings, &ExtendedConfig::DEFAULT)
}

fn assert_column(series: &ReadingSeries, fixture: &Fixture, column: usize, label: &str) {
    assert_eq!(series.len(), fixture.rows.len(), "{label} length changed");
    for (bar, row) in fixture.rows.iter().enumerate() {
        assert_eq!(
            series.get(bar).map(f64::to_bits),
            number(&row[column]).map(f64::to_bits),
            "{label} changed at bar {bar}"
        );
    }
}

fn assert_series_bits(actual: &ReadingSeries, expected: &ReadingSeries, label: &str, bar: usize) {
    assert_eq!(
        actual.get(bar).map(f64::to_bits),
        expected.get(bar).map(f64::to_bits),
        "{label} changed at bar {bar}"
    );
}

fn assert_extended_indicator(id: &str, columns: impl Fn(&ExtendedReadings) -> Columns) {
    let candles = load_candles();
    let fixture = load_fixture(READINGS_FIXTURE);
    assert_eq!(fixture.rows.len(), candles.len());

    let full = compute(&candles);
    let full_columns = columns(&full);
    for (name, series) in &full_columns {
        let column = fixture.column(name);
        assert_column(series, &fixture, column, &format!("{id}.{name}"));
    }

    let again = columns(&compute(&candles));
    for ((name, series), (again_name, again_series)) in full_columns.iter().zip(&again) {
        assert_eq!(name, again_name);
        for bar in 0..series.len() {
            assert_series_bits(again_series, series, &format!("{id}.{name}"), bar);
        }
    }

    let prefix = columns(&compute(&candles[..CUTOFF]));
    for ((name, series), (prefix_name, prefix_series)) in full_columns.iter().zip(&prefix) {
        assert_eq!(name, prefix_name);
        for bar in 0..CUTOFF {
            assert_series_bits(prefix_series, series, &format!("{id}.{name} prefix"), bar);
        }
    }

    let empty = columns(&compute(&[]));
    let single = columns(&compute(&candles[..1]));
    let short = columns(&compute(&candles[..10]));
    for (((name, series), (_, empty_series)), ((_, single_series), (_, short_series))) in
        full_columns
            .iter()
            .zip(&empty)
            .zip(single.iter().zip(&short))
    {
        assert_eq!(empty_series.len(), 0, "{id}.{name} is not empty");
        assert_eq!(single_series.len(), 1, "{id}.{name} single length");
        assert_eq!(short_series.len(), 10, "{id}.{name} short length");
        assert_eq!(series.len(), candles.len(), "{id}.{name} full length");
    }
}

#[test]
fn vwap_row_contract() {
    assert_extended_indicator("vwap", |extended| {
        vec![
            ("vwap", extended.vwap.clone()),
            ("vwap_distance_atr", extended.vwap_distance_atr.clone()),
            ("vwap_slope", extended.vwap_slope.clone()),
            ("vwap_band_upper", extended.vwap_band_upper.clone()),
            ("vwap_band_lower", extended.vwap_band_lower.clone()),
        ]
    });
}

#[test]
fn volume_profile_row_contract() {
    assert_extended_indicator("volume_profile", |extended| {
        vec![
            ("poc", extended.poc.clone()),
            ("vah", extended.vah.clone()),
            ("val", extended.val.clone()),
            ("poc_distance_atr", extended.poc_distance_atr.clone()),
            ("va_position", extended.va_position.clone()),
            ("profile_skew", extended.profile_skew.clone()),
            (
                "profile_concentration",
                extended.profile_concentration.clone(),
            ),
            ("poc_migration", extended.poc_migration.clone()),
        ]
    });
}

#[test]
fn rvol_row_contract() {
    assert_extended_indicator("rvol", |extended| {
        vec![
            ("rvol_fast", extended.rvol_fast.clone()),
            ("rvol_slow", extended.rvol_slow.clone()),
            ("rvol_ratio", extended.rvol_ratio.clone()),
        ]
    });
}

#[test]
fn volume_climax_row_contract() {
    assert_extended_indicator("volume_climax", |extended| {
        vec![
            ("climax_flag", extended.climax_flag.clone()),
            ("climax_direction", extended.climax_direction.clone()),
            ("climax_strength", extended.climax_strength.clone()),
        ]
    });
}

#[test]
fn absorption_row_contract() {
    assert_extended_indicator("absorption", |extended| {
        vec![
            ("absorption_flag", extended.absorption_flag.clone()),
            (
                "absorption_direction",
                extended.absorption_direction.clone(),
            ),
            ("absorption_strength", extended.absorption_strength.clone()),
        ]
    });
}

#[test]
fn volume_oscillator_row_contract() {
    assert_extended_indicator("volume_oscillator", |extended| {
        vec![("volume_oscillator", extended.volume_oscillator.clone())]
    });
}

#[test]
fn obv_row_contract() {
    assert_extended_indicator("obv", |extended| {
        vec![
            ("obv", extended.obv.clone()),
            ("obv_ma", extended.obv_ma.clone()),
            ("obv_slope", extended.obv_slope.clone()),
        ]
    });
}

#[test]
fn cmf_row_contract() {
    assert_extended_indicator("cmf", |extended| {
        vec![
            ("cmf", extended.cmf.clone()),
            ("cmf_slope", extended.cmf_slope.clone()),
        ]
    });
}

#[test]
fn mfi_row_contract() {
    assert_extended_indicator("mfi", |extended| vec![("mfi", extended.mfi.clone())]);
}

#[test]
fn ad_line_row_contract() {
    assert_extended_indicator("ad_line", |extended| {
        vec![
            ("ad_line", extended.ad_line.clone()),
            ("ad_ma", extended.ad_ma.clone()),
        ]
    });
}

#[test]
fn atr_term_row_contract() {
    assert_extended_indicator("atr_term", |extended| {
        vec![
            ("atr_ratio", extended.atr_ratio.clone()),
            ("atr_slope", extended.atr_slope.clone()),
            ("vol_regime", extended.vol_regime.clone()),
        ]
    });
}

#[test]
fn range_expansion_row_contract() {
    assert_extended_indicator("range_expansion", |extended| {
        vec![("range_expansion", extended.range_expansion.clone())]
    });
}

#[test]
fn mar_row_contract() {
    assert_extended_indicator("mar", |extended| {
        vec![
            ("mar_fast", extended.mar_fast.clone()),
            ("mar_slow", extended.mar_slow.clone()),
            ("mar_ratio", extended.mar_ratio.clone()),
        ]
    });
}

#[test]
fn keltner_row_contract() {
    assert_extended_indicator("keltner", |extended| {
        vec![
            ("keltner_upper", extended.keltner_upper.clone()),
            ("keltner_lower", extended.keltner_lower.clone()),
            ("keltner_position", extended.keltner_position.clone()),
        ]
    });
}

#[test]
fn donchian_row_contract() {
    assert_extended_indicator("donchian", |extended| {
        vec![
            ("donchian_upper", extended.donchian_upper.clone()),
            ("donchian_lower", extended.donchian_lower.clone()),
            ("donchian_position", extended.donchian_position.clone()),
        ]
    });
}

#[test]
fn swing_structure_row_contract() {
    assert_extended_indicator("swing_structure", |extended| {
        vec![
            ("structure_state", extended.structure_state.clone()),
            ("last_pivot_high", extended.last_pivot_high.clone()),
            ("last_pivot_low", extended.last_pivot_low.clone()),
        ]
    });
}

#[test]
fn level_strength_row_contract() {
    assert_extended_indicator("level_strength", |extended| {
        vec![
            ("level_touches", extended.level_touches.clone()),
            ("level_age", extended.level_age.clone()),
        ]
    });
}

#[test]
fn aroon_row_contract() {
    assert_extended_indicator("aroon", |extended| {
        vec![
            ("aroon_up", extended.aroon_up.clone()),
            ("aroon_down", extended.aroon_down.clone()),
            ("aroon_osc", extended.aroon_osc.clone()),
        ]
    });
}

#[test]
fn bos_choch_row_contract() {
    // BOS and CHOCH are event-only rows; the fixture proves they are reachable
    // and the prefix check proves they never reach an earlier bar.
    let candles = load_candles();
    let full = extended_events(&candles);
    assert!(
        full.iter()
            .any(|event| event.event_code == codes::LEVEL_BOS),
        "the fixture has no break of structure"
    );
    assert!(
        full.iter()
            .any(|event| event.event_code == codes::LEVEL_CHOCH),
        "the fixture has no change of character"
    );
    assert_eq!(
        full,
        extended_events(&candles),
        "bos_choch is not deterministic"
    );
    for cutoff in [61, 131, 201] {
        let prefix = extended_events(&candles[..cutoff]);
        let expected: Vec<_> = full
            .iter()
            .filter(|event| {
                matches!(event.event_code, codes::LEVEL_BOS | codes::LEVEL_CHOCH)
                    && event.available_bar_index < cutoff
            })
            .copied()
            .collect();
        let actual: Vec<_> = prefix
            .into_iter()
            .filter(|event| matches!(event.event_code, codes::LEVEL_BOS | codes::LEVEL_CHOCH))
            .collect();
        assert_eq!(actual, expected, "bos_choch changed at prefix {cutoff}");
    }
    assert!(extended_events(&[]).is_empty());
    assert!(extended_events(&candles[..1]).is_empty());
}

fn extended_events(candles: &[Candle]) -> Vec<quantrule_model::events::EventRecord> {
    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    let extended = ExtendedReadings::compute(candles, &readings, &ExtendedConfig::DEFAULT);
    ExtendedEventSeries::compute(
        candles,
        &readings,
        &extended,
        &ExtendedEventConfig::new(std::num::NonZeroU64::new(300).expect("a non-zero timeframe")),
    )
    .events()
    .to_vec()
}

#[test]
fn candle_anatomy_row_contract() {
    assert_extended_indicator("candle_anatomy", |extended| {
        vec![
            ("body_ratio", extended.body_ratio.clone()),
            ("upper_wick_ratio", extended.upper_wick_ratio.clone()),
            ("lower_wick_ratio", extended.lower_wick_ratio.clone()),
            ("close_position", extended.close_position.clone()),
        ]
    });
}

#[test]
fn candle_pattern_row_contract() {
    assert_extended_indicator("candle_pattern", |extended| {
        vec![
            ("pattern_code", extended.pattern_code.clone()),
            ("pattern_direction", extended.pattern_direction.clone()),
        ]
    });
}

#[test]
fn gap_row_contract() {
    assert_extended_indicator("gap", |extended| {
        vec![
            ("gap_atr", extended.gap_atr.clone()),
            ("gap_filled", extended.gap_filled.clone()),
        ]
    });
}

#[test]
fn path_efficiency_row_contract() {
    assert_extended_indicator("path_efficiency", |extended| {
        vec![("path_efficiency", extended.path_efficiency.clone())]
    });
}

#[test]
fn direction_changes_row_contract() {
    assert_extended_indicator("direction_changes", |extended| {
        vec![("direction_changes", extended.direction_changes.clone())]
    });
}

#[test]
fn bars_since_extreme_row_contract() {
    assert_extended_indicator("bars_since_extreme", |extended| {
        vec![
            ("bars_since_high", extended.bars_since_high.clone()),
            ("bars_since_low", extended.bars_since_low.clone()),
        ]
    });
}

#[test]
fn drawdown_atr_row_contract() {
    assert_extended_indicator("drawdown_atr", |extended| {
        vec![("drawdown_atr", extended.drawdown_atr.clone())]
    });
}

#[test]
fn range_position_row_contract() {
    assert_extended_indicator("range_position", |extended| {
        vec![("range_position", extended.range_position.clone())]
    });
}

#[test]
fn pivot_sequence_row_contract() {
    assert_extended_indicator("pivot_sequence", |extended| {
        vec![
            ("lower_high_count", extended.lower_high_count.clone()),
            ("lower_low_count", extended.lower_low_count.clone()),
            ("higher_high_count", extended.higher_high_count.clone()),
            ("higher_low_count", extended.higher_low_count.clone()),
        ]
    });
}

#[test]
fn fibonacci_row_contract() {
    assert_extended_indicator("fibonacci", |extended| {
        vec![
            ("fib_direction", extended.fib_direction.clone()),
            ("fib_low", extended.fib_low.clone()),
            ("fib_high", extended.fib_high.clone()),
            ("fib_236", extended.fib_236.clone()),
            ("fib_382", extended.fib_382.clone()),
            ("fib_500", extended.fib_500.clone()),
            ("fib_618", extended.fib_618.clone()),
            ("fib_786", extended.fib_786.clone()),
            ("fib_position", extended.fib_position.clone()),
            ("fib_nearest_ratio", extended.fib_nearest_ratio.clone()),
            ("fib_nearest_atr", extended.fib_nearest_atr.clone()),
        ]
    });
}

#[test]
fn horizontal_band_row_contract() {
    assert_extended_indicator("horizontal_band", |extended| {
        vec![
            ("band_upper", extended.band_upper.clone()),
            ("band_lower", extended.band_lower.clone()),
            ("band_center", extended.band_center.clone()),
            ("band_touches", extended.band_touches.clone()),
            ("band_age", extended.band_age.clone()),
            ("band_width_atr", extended.band_width_atr.clone()),
            ("band_position", extended.band_position.clone()),
        ]
    });
}
