//! Row-level contracts for the extended and session events covered by the
//! checked-in regression fixtures.
//!
//! Each test names one event and checks its golden record, availability,
//! uniqueness, determinism, and prefix causality. The extended and session
//! fixtures are regression locks, not independent oracles; the identity,
//! availability, and causality checks are contract facts independent of the
//! generated values.

use std::collections::BTreeSet;

use quantrule_data::Candle;
use quantrule_model::events::EventRecord;
use quantrule_model::events::codes;
use quantrule_model::events::session::SessionEventSeries;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::readings::{Readings, ReadingsConfig};
use quantrule_model::sessions::{SessionConfig, SessionReadings};
use quantrule_model::timeframe::TimeframeModel;

const CANDLES_FIXTURE: &str = include_str!("fixtures/readings_golden.csv");
const EXTENDED_FIXTURE: &str = include_str!("fixtures/extended_events_golden.csv");
const SESSION_FIXTURE: &str = include_str!("fixtures/session_events_golden.csv");
const TIMEFRAME_SECS: u64 = 300;
const CUTOFFS: [usize; 3] = [61, 131, 201];

const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const EVENTS_HEADER: &str = "bar,event_code,event_type_code,event_subtype_code,source_code,timeframe_secs,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Extended,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ExpectedEvent {
    bar: usize,
    event_code: u16,
    event_type_code: u8,
    event_subtype_code: u8,
    source_code: u16,
    timeframe_secs: u64,
    occurred_bar_index: usize,
    available_bar_index: usize,
    direction_code: i8,
    reference_code: Option<u32>,
    primary_value: Option<f64>,
    reference_value: Option<f64>,
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
    assert_eq!(lines.next(), Some(READINGS_HEADER));
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 23);
            Candle::new(
                fields[0].parse().expect("bar"),
                fields[1].parse().expect("timestamp"),
                fields[2].parse().expect("open"),
                fields[3].parse().expect("high"),
                fields[4].parse().expect("low"),
                fields[5].parse().expect("close"),
                if fields[6].is_empty() {
                    None
                } else {
                    Some(fields[6].parse().expect("volume"))
                },
            )
            .expect("valid fixture candle")
        })
        .collect()
}

fn load_events(fixture: &str) -> Vec<ExpectedEvent> {
    let mut lines = fixture
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'));
    assert_eq!(lines.next(), Some(EVENTS_HEADER));
    lines
        .map(|line| {
            let fields: Vec<&str> = line.split(',').collect();
            assert_eq!(fields.len(), 12);
            ExpectedEvent {
                bar: fields[0].parse().expect("bar"),
                event_code: fields[1].parse().expect("event code"),
                event_type_code: fields[2].parse().expect("event type"),
                event_subtype_code: fields[3].parse().expect("subtype"),
                source_code: fields[4].parse().expect("source"),
                timeframe_secs: fields[5].parse().expect("timeframe"),
                occurred_bar_index: fields[6].parse().expect("occurred"),
                available_bar_index: fields[7].parse().expect("available"),
                direction_code: fields[8].parse().expect("direction"),
                reference_code: if fields[9].is_empty() {
                    None
                } else {
                    Some(fields[9].parse().expect("reference"))
                },
                primary_value: number(fields[10]),
                reference_value: number(fields[11]),
            }
        })
        .collect()
}

fn model(candles: &[Candle]) -> TimeframeModel {
    TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs: TIMEFRAME_SECS,
            candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("the fixture produces a model")
}

fn session_events(candles: &[Candle]) -> Vec<EventRecord> {
    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    let sessions = SessionReadings::compute(candles, &readings, &SessionConfig::london());
    SessionEventSeries::compute(
        candles,
        &readings,
        &sessions,
        &SessionConfig::london(),
        TIMEFRAME_SECS,
    )
    .events()
    .to_vec()
}

fn compute(family: Family, candles: &[Candle]) -> Vec<EventRecord> {
    match family {
        Family::Extended => model(candles).extended_events().events().to_vec(),
        Family::Session => session_events(candles),
    }
}

fn fixture(family: Family) -> &'static str {
    match family {
        Family::Extended => EXTENDED_FIXTURE,
        Family::Session => SESSION_FIXTURE,
    }
}

fn assert_record(event: &EventRecord, expected: &ExpectedEvent) {
    assert_eq!(event.event_code, expected.event_code);
    assert_eq!(event.event_type.code(), expected.event_type_code);
    assert_eq!(event.event_subtype_code, expected.event_subtype_code);
    assert_eq!(event.source_code, expected.source_code);
    assert_eq!(event.timeframe_secs, expected.timeframe_secs);
    assert_eq!(event.occurred_bar_index, expected.occurred_bar_index);
    assert_eq!(event.available_bar_index, expected.available_bar_index);
    assert_eq!(event.direction_code, expected.direction_code);
    assert_eq!(event.reference_code, expected.reference_code);
    assert_eq!(event.primary_value, expected.primary_value);
    assert_eq!(event.reference_value, expected.reference_value);
}

type RecordKey = (
    u16,
    u8,
    u8,
    u16,
    u64,
    usize,
    usize,
    i8,
    Option<u32>,
    Option<u64>,
    Option<u64>,
);

fn record_key(event: &EventRecord) -> RecordKey {
    (
        event.event_code,
        event.event_type.code(),
        event.event_subtype_code,
        event.source_code,
        event.timeframe_secs,
        event.occurred_bar_index,
        event.available_bar_index,
        event.direction_code,
        event.reference_code,
        event.primary_value.map(f64::to_bits),
        event.reference_value.map(f64::to_bits),
    )
}

fn assert_row(family: Family, code: u16) {
    let candles = load_candles();
    let expected = load_events(fixture(family));
    let events = compute(family, &candles);

    let actual: Vec<&EventRecord> = events
        .iter()
        .filter(|event| event.event_code == code)
        .collect();
    let expected: Vec<&ExpectedEvent> = expected
        .iter()
        .filter(|event| event.event_code == code)
        .collect();
    assert!(
        !expected.is_empty(),
        "the fixture does not cover event code {code}"
    );
    assert_eq!(
        actual.len(),
        expected.len(),
        "event code {code} changed occurrence count"
    );
    for (event, expected) in actual.iter().zip(&expected) {
        assert_record(event, expected);
    }

    let mut seen = BTreeSet::new();
    for event in &actual {
        assert!(
            event.available_bar_index >= event.occurred_bar_index,
            "event code {code} became available before it occurred: {event:?}"
        );
        assert!(
            seen.insert(record_key(event)),
            "event code {code} published the same record twice"
        );
    }

    let recomputed: Vec<EventRecord> = compute(family, &candles)
        .into_iter()
        .filter(|event| event.event_code == code)
        .collect();
    let actual_records: Vec<EventRecord> = actual.iter().map(|event| **event).collect();
    assert_eq!(
        actual_records, recomputed,
        "event code {code} is not deterministic"
    );

    for cutoff in CUTOFFS {
        let prefix: Vec<EventRecord> = compute(family, &candles[..cutoff])
            .into_iter()
            .filter(|event| event.event_code == code)
            .collect();
        let expected_prefix: Vec<EventRecord> = events
            .iter()
            .filter(|event| event.event_code == code && event.available_bar_index < cutoff)
            .copied()
            .collect();
        assert_eq!(
            prefix, expected_prefix,
            "event code {code} changed in the prefix of length {cutoff}"
        );
    }
}

#[test]
fn level_vwap_test_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_VWAP_TEST);
}

#[test]
fn level_vwap_cross_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_VWAP_CROSS);
}

#[test]
fn level_vwap_break_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_VWAP_BREAK);
}

#[test]
fn divergence_vwap_regular_bearish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_VWAP_REGULAR_BEARISH);
}

#[test]
fn divergence_vwap_hidden_bearish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_VWAP_HIDDEN_BEARISH);
}

#[test]
fn signal_vwap_reclaim_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_VWAP_RECLAIM);
}

#[test]
fn signal_vwap_rejection_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_VWAP_REJECTION);
}

#[test]
fn signal_vwap_slope_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_VWAP_SLOPE);
}

#[test]
fn level_profile_poc_test_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_PROFILE_POC_TEST);
}

#[test]
fn level_profile_vah_test_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_PROFILE_VAH_TEST);
}

#[test]
fn level_profile_val_test_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_PROFILE_VAL_TEST);
}

#[test]
fn level_profile_va_break_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_PROFILE_VA_BREAK);
}

#[test]
fn level_profile_va_reentry_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_PROFILE_VA_REENTRY);
}

#[test]
fn divergence_profile_poc_bullish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_PROFILE_POC_BULLISH);
}

#[test]
fn divergence_profile_poc_bearish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_PROFILE_POC_BEARISH);
}

#[test]
fn signal_profile_va_breakout_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_PROFILE_VA_BREAKOUT);
}

#[test]
fn signal_profile_poc_cross_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_PROFILE_POC_CROSS);
}

#[test]
fn signal_profile_skew_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_PROFILE_SKEW);
}

#[test]
fn divergence_rvol_bullish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_RVOL_BULLISH);
}

#[test]
fn divergence_rvol_bearish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_RVOL_BEARISH);
}

#[test]
fn signal_rvol_expansion_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_RVOL_EXPANSION);
}

#[test]
fn signal_rvol_contraction_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_RVOL_CONTRACTION);
}

#[test]
fn signal_rvol_ratio_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_RVOL_RATIO);
}

#[test]
fn divergence_climax_bullish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_CLIMAX_BULLISH);
}

#[test]
fn divergence_climax_bearish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_CLIMAX_BEARISH);
}

#[test]
fn signal_volume_oscillator_cross_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_VOLUME_OSCILLATOR_CROSS);
}

#[test]
fn divergence_obv_regular_bearish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_OBV_REGULAR_BEARISH);
}

#[test]
fn divergence_obv_hidden_bullish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_OBV_HIDDEN_BULLISH);
}

#[test]
fn signal_obv_cross_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_OBV_CROSS);
}

#[test]
fn signal_obv_slope_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_OBV_SLOPE);
}

#[test]
fn divergence_cmf_regular_bullish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_CMF_REGULAR_BULLISH);
}

#[test]
fn divergence_cmf_regular_bearish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_CMF_REGULAR_BEARISH);
}

#[test]
fn divergence_cmf_hidden_bearish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_CMF_HIDDEN_BEARISH);
}

#[test]
fn signal_cmf_cross_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_CMF_CROSS);
}

#[test]
fn signal_cmf_threshold_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_CMF_THRESHOLD);
}

#[test]
fn divergence_mfi_hidden_bearish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_MFI_HIDDEN_BEARISH);
}

#[test]
fn signal_mfi_threshold_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_MFI_THRESHOLD);
}

#[test]
fn signal_mfi_zone_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_MFI_ZONE);
}

#[test]
fn divergence_ad_regular_bearish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_AD_REGULAR_BEARISH);
}

#[test]
fn signal_ad_cross_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_AD_CROSS);
}

#[test]
fn level_keltner_test_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_KELTNER_TEST);
}

#[test]
fn level_keltner_break_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_KELTNER_BREAK);
}

#[test]
fn level_donchian_test_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_DONCHIAN_TEST);
}

#[test]
fn level_donchian_break_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_DONCHIAN_BREAK);
}

#[test]
fn level_structure_break_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_STRUCTURE_BREAK);
}

#[test]
fn level_strength_test_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_STRENGTH_TEST);
}

#[test]
fn level_gap_fill_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_GAP_FILL);
}

#[test]
fn level_gap_test_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_GAP_TEST);
}

#[test]
fn divergence_atr_bullish_row_contract() {
    assert_row(Family::Extended, codes::DIVERGENCE_ATR_BULLISH);
}

#[test]
fn signal_range_expansion_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_RANGE_EXPANSION);
}

#[test]
fn signal_range_contraction_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_RANGE_CONTRACTION);
}

#[test]
fn signal_mar_regime_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_MAR_REGIME);
}

#[test]
fn signal_keltner_breakout_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_KELTNER_BREAKOUT);
}

#[test]
fn signal_keltner_position_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_KELTNER_POSITION);
}

#[test]
fn signal_donchian_breakout_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_DONCHIAN_BREAKOUT);
}

#[test]
fn signal_donchian_position_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_DONCHIAN_POSITION);
}

#[test]
fn signal_structure_change_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_STRUCTURE_CHANGE);
}

#[test]
fn signal_strength_change_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_STRENGTH_CHANGE);
}

#[test]
fn signal_aroon_cross_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_AROON_CROSS);
}

#[test]
fn signal_aroon_osc_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_AROON_OSC);
}

#[test]
fn signal_candle_doji_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_CANDLE_DOJI);
}

#[test]
fn signal_pattern_engulfing_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_PATTERN_ENGULFING);
}

#[test]
fn signal_pattern_inside_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_PATTERN_INSIDE);
}

#[test]
fn signal_pattern_outside_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_PATTERN_OUTSIDE);
}

#[test]
fn signal_path_efficiency_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_PATH_EFFICIENCY);
}

#[test]
fn signal_direction_changes_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_DIRECTION_CHANGES);
}

#[test]
fn signal_extreme_new_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_EXTREME_NEW);
}

#[test]
fn signal_extreme_stale_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_EXTREME_STALE);
}

#[test]
fn signal_drawdown_threshold_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_DRAWDOWN_THRESHOLD);
}

#[test]
fn signal_drawdown_recovery_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_DRAWDOWN_RECOVERY);
}

#[test]
fn signal_range_position_row_contract() {
    assert_row(Family::Extended, codes::SIGNAL_RANGE_POSITION);
}

#[test]
fn level_bos_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_BOS);
}

#[test]
fn level_choch_row_contract() {
    assert_row(Family::Extended, codes::LEVEL_CHOCH);
}

#[test]
fn signal_session_open_row_contract() {
    assert_row(Family::Session, codes::SIGNAL_SESSION_OPEN);
}

#[test]
fn signal_session_close_row_contract() {
    assert_row(Family::Session, codes::SIGNAL_SESSION_CLOSE);
}

#[test]
fn signal_opening_range_fade_row_contract() {
    assert_row(Family::Session, codes::SIGNAL_OPENING_RANGE_FADE);
}

#[test]
fn signal_session_range_expansion_row_contract() {
    assert_row(Family::Session, codes::SIGNAL_SESSION_RANGE_EXPANSION);
}

#[test]
fn level_session_vwap_test_row_contract() {
    assert_row(Family::Session, codes::LEVEL_SESSION_VWAP_TEST);
}

#[test]
fn level_opening_range_test_row_contract() {
    assert_row(Family::Session, codes::LEVEL_OPENING_RANGE_TEST);
}

#[test]
fn level_opening_range_break_row_contract() {
    assert_row(Family::Session, codes::LEVEL_OPENING_RANGE_BREAK);
}
