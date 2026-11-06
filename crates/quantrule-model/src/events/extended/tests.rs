//! Extended event tests.

use super::{
    ExtendedEventConfig, ExtendedEventSeries, atr_divergences, deduplicate_occurrences,
    divergence_events, flow_divergences, level_extended_events, participation_divergences,
    participation_events, profile_events, push, vwap_events,
};
use crate::events::{EventRecord, EventType, codes};
use crate::readings::{
    ExtendedConfig, ExtendedReadings, PivotKind, ReadingSeries, Readings, ReadingsConfig,
};
use quantrule_data::Candle;

fn candle(index: u64, open: f64, high: f64, low: f64, close: f64, volume: f64) -> Candle {
    Candle::new(index, index as i64, open, high, low, close, Some(volume)).expect("valid candle")
}

fn compute(candles: &[Candle]) -> ExtendedEventSeries {
    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    let extended = ExtendedReadings::compute(candles, &readings, &ExtendedConfig::DEFAULT);
    let config = ExtendedEventConfig::from_secs(3600).expect("timeframe");
    ExtendedEventSeries::compute(candles, &readings, &extended, &config)
}

fn mfi_event(reference: f64, direction: i8) -> EventRecord {
    let mut events = Vec::new();
    push(
        &mut events,
        codes::SIGNAL_MFI_THRESHOLD,
        EventType::Signal,
        15,
        26,
        3_600,
        7,
        direction,
        Some(55.0),
        Some(reference),
    );
    events.remove(0)
}

#[test]
fn distinct_threshold_crossings_on_one_bar_are_kept() {
    let mut events = vec![mfi_event(20.0, 1), mfi_event(50.0, 1)];
    deduplicate_occurrences(&mut events);
    assert_eq!(events.len(), 2, "both threshold crossings must survive");
}

#[test]
fn exact_duplicate_occurrences_are_removed() {
    let mut events = vec![mfi_event(20.0, 1), mfi_event(20.0, 1)];
    deduplicate_occurrences(&mut events);
    assert_eq!(events.len(), 1);
}

#[test]
fn vwap_cross_is_a_level_event() {
    let mut candles = Vec::new();
    for index in 0..30 {
        let price = if index < 25 { 10.0 } else { 12.0 };
        candles.push(candle(index, price, price, price, price, 1.0));
    }
    let events = compute(&candles);
    assert!(events.events().iter().any(|event| {
        event.source_code == 19 && event.event_type == crate::events::EventType::Level
    }));
}

#[test]
fn vwap_reclaim_has_both_directions() {
    let config = ExtendedEventConfig::from_secs(3600).expect("timeframe");
    let cases = [
        (
            (0..8)
                .map(|i| if i < 6 { 99.0 } else { 101.0 })
                .collect::<Vec<_>>(),
            1,
        ),
        (
            (0..8)
                .map(|i| if i < 6 { 101.0 } else { 99.0 })
                .collect::<Vec<_>>(),
            -1,
        ),
    ];
    for (closes, expected) in cases {
        let candles: Vec<Candle> = closes
            .iter()
            .enumerate()
            .map(|(index, close)| {
                candle(index as u64, *close, close + 0.5, close - 0.5, *close, 10.0)
            })
            .collect();
        let mut extended = ExtendedReadings::empty(8);
        extended.vwap = ReadingSeries::new(vec![Some(100.0); 8]);
        let mut events = Vec::new();
        vwap_events(
            &candles,
            &extended,
            &config,
            6,
            candles[6],
            Some(1.0),
            &mut events,
        );
        let reclaims: Vec<_> = events
            .iter()
            .filter(|event| event.event_code == codes::SIGNAL_VWAP_RECLAIM)
            .collect();
        assert_eq!(reclaims.len(), 1, "closes {closes:?}");
        assert_eq!(reclaims[0].direction_code, expected);
    }
}

#[test]
fn no_events_on_an_empty_or_short_series() {
    assert!(compute(&[]).is_empty());
    let short = [candle(0, 10.0, 10.0, 10.0, 10.0, 1.0)];
    assert!(compute(&short).is_empty());
}

#[test]
fn appending_future_bars_does_not_change_past_events() {
    let candles: Vec<Candle> = (0..40)
        .map(|index| {
            let base = 10.0 + (index as f64 * 0.2).sin();
            candle(
                index,
                base,
                base + 0.5,
                base - 0.5,
                base + 0.1,
                1.0 + index as f64,
            )
        })
        .collect();
    let full = compute(&candles);
    let prefix = compute(&candles[..25]);
    let full_past: Vec<_> = full
        .events()
        .iter()
        .filter(|event| event.available_bar_index < 25)
        .copied()
        .collect();
    let prefix_events: Vec<_> = prefix.events().to_vec();
    assert_eq!(full_past, prefix_events);
}

#[test]
fn repeated_computation_is_identical() {
    let candles: Vec<Candle> = (0..40)
        .map(|index| {
            let base = 10.0 + (index as f64 * 0.2).sin();
            candle(
                index,
                base,
                base + 0.5,
                base - 0.5,
                base + 0.1,
                1.0 + index as f64,
            )
        })
        .collect();
    assert_eq!(compute(&candles), compute(&candles));
}

fn compute_with(candles: &[Candle], config: ExtendedConfig) -> ExtendedEventSeries {
    let readings = Readings::compute_with_config(candles, &ReadingsConfig::DEFAULT);
    let extended = ExtendedReadings::compute(candles, &readings, &config);
    let event_config = ExtendedEventConfig::from_secs(3600).expect("timeframe");
    ExtendedEventSeries::compute(candles, &readings, &extended, &event_config)
}

#[test]
fn a_volume_expansion_is_a_participation_event() {
    let mut candles: Vec<Candle> = (0..30)
        .map(|index| candle(index, 100.0, 101.0, 99.0, 100.0, 10.0))
        .collect();
    candles[29] = candle(29, 100.0, 101.0, 99.0, 100.0, 100.0);
    let events = compute(&candles);
    assert!(
        events
            .events()
            .iter()
            .any(|event| event.source_code == 6 && event.event_subtype_code == 11)
    );
}

#[test]
fn a_climax_reversal_is_a_participation_event() {
    let mut candles: Vec<Candle> = (0..30)
        .map(|index| candle(index, 100.0, 101.0, 99.0, 100.0, 10.0))
        .collect();
    candles[29] = candle(29, 100.0, 103.0, 99.0, 100.5, 100.0);
    let events = compute(&candles);
    assert!(events.events().iter().any(|event| {
        event.source_code == 21 && event.event_subtype_code == 11 && event.direction_code == -1
    }));
}

#[test]
fn an_absorption_break_is_a_participation_event() {
    let mut candles: Vec<Candle> = (0..21)
        .map(|index| candle(index, 100.0, 100.5, 99.5, 100.0, 10.0))
        .collect();
    candles[19] = candle(19, 100.0, 100.1, 99.9, 100.05, 30.0);
    candles[20] = candle(20, 100.05, 101.2, 100.0, 101.0, 10.0);
    let config = ExtendedConfig {
        vwap_window: 5,
        profile_window: 5,
        profile_bins: 4,
        rvol_fast: 5,
        rvol_slow: 5,
        volume_oscillator: 5,
        ..ExtendedConfig::DEFAULT
    };
    let events = compute_with(&candles, config);
    assert!(events.events().iter().any(|event| {
        event.source_code == 22 && event.event_subtype_code == 12 && event.direction_code == 1
    }));
}

#[test]
fn an_obv_cross_is_a_flow_event() {
    let mut candles: Vec<Candle> = (0..30)
        .map(|index| {
            let up = index % 2 == 0;
            candle(
                index,
                100.0,
                101.0,
                99.0,
                if up { 100.5 } else { 99.5 },
                10.0,
            )
        })
        .collect();
    candles[29] = candle(29, 100.0, 102.0, 100.0, 101.5, 100.0);
    let events = compute(&candles);
    assert!(
        events
            .events()
            .iter()
            .any(|event| event.source_code == 24 && event.event_subtype_code == 15)
    );
}

#[test]
fn a_cmf_zero_cross_is_a_flow_event() {
    let mut candles: Vec<Candle> = Vec::new();
    for index in 0..30 {
        if index < 15 {
            candles.push(candle(index, 101.0, 101.0, 99.0, 99.0, 10.0));
        } else {
            candles.push(candle(index, 99.0, 101.0, 99.0, 101.0, 10.0));
        }
    }
    let events = compute(&candles);
    assert!(
        events
            .events()
            .iter()
            .any(|event| event.source_code == 25 && event.event_subtype_code == 15)
    );
}

#[test]
fn a_keltner_break_is_a_level_event() {
    let mut candles: Vec<Candle> = (0..30)
        .map(|index| candle(index, 100.0, 101.0, 99.0, 100.0, 10.0))
        .collect();
    candles[29] = candle(29, 100.0, 106.5, 99.0, 106.0, 10.0);
    let events = compute(&candles);
    assert!(
        events
            .events()
            .iter()
            .any(|event| event.source_code == 31 && event.event_subtype_code == 3)
    );
}

#[test]
fn a_donchian_break_is_a_level_event() {
    let mut candles: Vec<Candle> = (0..30)
        .map(|index| candle(index, 100.0, 101.0, 99.0, 100.0, 10.0))
        .collect();
    candles[29] = candle(29, 100.0, 102.5, 100.0, 102.0, 10.0);
    let events = compute(&candles);
    assert!(
        events
            .events()
            .iter()
            .any(|event| event.source_code == 32 && event.event_subtype_code == 3)
    );
}

#[test]
fn a_filled_gap_is_a_level_event() {
    let mut candles: Vec<Candle> = (0..20)
        .map(|index| candle(index, 100.0, 100.5, 99.5, 100.0, 10.0))
        .collect();
    candles[19] = candle(19, 102.0, 102.5, 99.0, 99.5, 10.0);
    let events = compute(&candles);
    assert!(
        events
            .events()
            .iter()
            .any(|event| event.source_code == 39 && event.event_subtype_code == 1)
    );
}

#[test]
fn atr_divergence_subtypes_are_directional() {
    use super::divergence_subtype;
    use crate::readings::PivotKind;
    assert_eq!(divergence_subtype(PivotKind::Low, -1.0, 1.0), Some((1, 1)));
    assert_eq!(
        divergence_subtype(PivotKind::High, 1.0, -1.0),
        Some((2, -1))
    );
    assert_eq!(divergence_subtype(PivotKind::Low, 1.0, -1.0), None);
}

#[test]
fn a_momentum_bar_is_a_candle_signal() {
    let mut candles: Vec<Candle> = (0..30)
        .map(|index| candle(index, 100.0, 101.0, 99.0, 100.0, 10.0))
        .collect();
    candles[29] = candle(29, 100.0, 106.0, 99.0, 105.5, 10.0);
    let events = compute(&candles);
    assert!(
        events
            .events()
            .iter()
            .any(|event| event.source_code == 37 && event.event_subtype_code == 10)
    );
}

#[test]
fn a_stale_extreme_is_a_distinct_event() {
    let mut candles: Vec<Candle> = (0..40)
        .map(|index| candle(index, 100.0, 100.5, 99.5, 100.0, 10.0))
        .collect();
    candles[0] = candle(0, 100.0, 110.0, 99.0, 100.0, 10.0);
    let events = compute(&candles);
    assert!(
        events.events().iter().any(|event| {
            event.event_code == codes::SIGNAL_EXTREME_STALE && event.event_type == EventType::Signal
        }),
        "the stale extreme was not published"
    );
    assert!(
        !events.events().iter().any(|event| {
            event.event_code == codes::SIGNAL_EXTREME_STALE && event.occurred_bar_index == 0
        }),
        "the fresh extreme was misclassified as stale"
    );
}

#[test]
fn a_gap_continuation_is_a_gap_signal() {
    let mut candles: Vec<Candle> = (0..20)
        .map(|index| candle(index, 100.0, 100.5, 99.5, 100.0, 10.0))
        .collect();
    candles[19] = candle(19, 102.0, 103.0, 101.0, 102.5, 10.0);
    let events = compute(&candles);
    assert!(
        events
            .events()
            .iter()
            .any(|event| event.source_code == 39 && event.event_subtype_code == 20)
    );
}

fn pivot_candles(kind: PivotKind, first: f64, second: f64) -> Vec<Candle> {
    (0..30)
        .map(|index| {
            let index = index as u64;
            match kind {
                PivotKind::Low => {
                    let low = if index == 5 {
                        first
                    } else if index == 15 {
                        second
                    } else {
                        101.0
                    };
                    candle(index, 101.0, 101.5, low, 101.0, 10.0)
                }
                PivotKind::High => {
                    let high = if index == 5 {
                        first
                    } else if index == 15 {
                        second
                    } else {
                        100.0
                    };
                    candle(index, 100.0, high, 99.5, 100.0, 10.0)
                }
            }
        })
        .collect()
}

fn two_pivot_values(len: usize, first: f64, second: f64) -> ReadingSeries {
    let mut values = vec![None; len];
    values[5] = Some(first);
    values[15] = Some(second);
    ReadingSeries::new(values)
}

fn series_at(len: usize, values: &[(usize, f64)]) -> ReadingSeries {
    let mut data = vec![None; len];
    for (bar, value) in values {
        data[*bar] = Some(*value);
    }
    ReadingSeries::new(data)
}

fn event_config() -> ExtendedEventConfig {
    ExtendedEventConfig::from_secs(3600).expect("timeframe")
}

fn vwap_profile_codes(
    kind: PivotKind,
    first: f64,
    second: f64,
    first_value: f64,
    second_value: f64,
) -> Vec<u16> {
    let candles = pivot_candles(kind, first, second);
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    extended.vwap_distance_atr = two_pivot_values(candles.len(), first_value, second_value);
    extended.poc = two_pivot_values(candles.len(), first_value, second_value);
    let mut events = Vec::new();
    divergence_events(&readings, &extended, &event_config(), &mut events);
    events.iter().map(|event| event.event_code).collect()
}

fn participation_codes(
    kind: PivotKind,
    first: f64,
    second: f64,
    first_value: f64,
    second_value: f64,
) -> Vec<u16> {
    let candles = pivot_candles(kind, first, second);
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    extended.rvol_slow = two_pivot_values(candles.len(), first_value, second_value);
    extended.rvol_fast = two_pivot_values(candles.len(), first_value, second_value);
    let mut events = Vec::new();
    participation_divergences(&readings, &extended, &event_config(), &mut events);
    events.iter().map(|event| event.event_code).collect()
}

fn flow_codes(
    kind: PivotKind,
    first: f64,
    second: f64,
    first_value: f64,
    second_value: f64,
) -> Vec<u16> {
    let candles = pivot_candles(kind, first, second);
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    for series in [
        &mut extended.obv,
        &mut extended.cmf,
        &mut extended.mfi,
        &mut extended.ad_line,
    ] {
        *series = two_pivot_values(candles.len(), first_value, second_value);
    }
    let mut events = Vec::new();
    flow_divergences(&readings, &extended, &event_config(), &mut events);
    events.iter().map(|event| event.event_code).collect()
}

fn atr_codes(
    kind: PivotKind,
    first: f64,
    second: f64,
    first_value: f64,
    second_value: f64,
) -> Vec<u16> {
    let candles = pivot_candles(kind, first, second);
    let readings = Readings::compute(&candles);
    let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
    extended.atr_ratio = two_pivot_values(candles.len(), first_value, second_value);
    let mut events = Vec::new();
    atr_divergences(&readings, &extended, &event_config(), &mut events);
    events.iter().map(|event| event.event_code).collect()
}

#[test]
fn divergence_sources_cover_their_events() {
    assert_eq!(
        vwap_profile_codes(PivotKind::Low, 99.0, 98.0, 0.5, 1.5),
        vec![
            codes::DIVERGENCE_VWAP_REGULAR_BULLISH,
            codes::DIVERGENCE_PROFILE_POC_BULLISH
        ]
    );
    assert_eq!(
        vwap_profile_codes(PivotKind::Low, 99.0, 100.0, 1.5, 0.5),
        vec![codes::DIVERGENCE_VWAP_HIDDEN_BULLISH]
    );
    assert_eq!(
        vwap_profile_codes(PivotKind::High, 101.0, 102.0, 1.5, 0.5),
        vec![
            codes::DIVERGENCE_VWAP_REGULAR_BEARISH,
            codes::DIVERGENCE_PROFILE_POC_BEARISH
        ]
    );
    assert_eq!(
        vwap_profile_codes(PivotKind::High, 101.0, 100.5, 0.5, 1.5),
        vec![codes::DIVERGENCE_VWAP_HIDDEN_BEARISH]
    );

    assert_eq!(
        participation_codes(PivotKind::Low, 99.0, 98.0, 0.5, 1.5),
        vec![
            codes::DIVERGENCE_RVOL_BULLISH,
            codes::DIVERGENCE_CLIMAX_BULLISH
        ]
    );
    assert_eq!(
        participation_codes(PivotKind::High, 101.0, 102.0, 1.5, 0.5),
        vec![
            codes::DIVERGENCE_RVOL_BEARISH,
            codes::DIVERGENCE_CLIMAX_BEARISH
        ]
    );

    assert_eq!(
        flow_codes(PivotKind::Low, 99.0, 98.0, 0.5, 1.5),
        vec![
            codes::DIVERGENCE_OBV_REGULAR_BULLISH,
            codes::DIVERGENCE_CMF_REGULAR_BULLISH,
            codes::DIVERGENCE_MFI_REGULAR_BULLISH,
            codes::DIVERGENCE_AD_REGULAR_BULLISH
        ]
    );
    assert_eq!(
        flow_codes(PivotKind::Low, 99.0, 100.0, 1.5, 0.5),
        vec![
            codes::DIVERGENCE_OBV_HIDDEN_BULLISH,
            codes::DIVERGENCE_CMF_HIDDEN_BULLISH,
            codes::DIVERGENCE_MFI_HIDDEN_BULLISH
        ]
    );
    assert_eq!(
        flow_codes(PivotKind::High, 101.0, 102.0, 1.5, 0.5),
        vec![
            codes::DIVERGENCE_OBV_REGULAR_BEARISH,
            codes::DIVERGENCE_CMF_REGULAR_BEARISH,
            codes::DIVERGENCE_MFI_REGULAR_BEARISH,
            codes::DIVERGENCE_AD_REGULAR_BEARISH
        ]
    );
    assert_eq!(
        flow_codes(PivotKind::High, 101.0, 100.5, 0.5, 1.5),
        vec![
            codes::DIVERGENCE_OBV_HIDDEN_BEARISH,
            codes::DIVERGENCE_CMF_HIDDEN_BEARISH,
            codes::DIVERGENCE_MFI_HIDDEN_BEARISH
        ]
    );

    assert_eq!(
        atr_codes(PivotKind::Low, 99.0, 98.0, 0.5, 1.5),
        vec![codes::DIVERGENCE_ATR_BULLISH]
    );
    assert_eq!(
        atr_codes(PivotKind::High, 101.0, 102.0, 1.5, 0.5),
        vec![codes::DIVERGENCE_ATR_BEARISH]
    );
}

fn flat_candles() -> Vec<Candle> {
    (0..30)
        .map(|index| candle(index, 100.0, 101.0, 99.0, 100.5, 10.0))
        .collect()
}

fn level_events_with(
    candles: &[Candle],
    bar: usize,
    overrides: impl FnOnce(&mut ExtendedReadings),
) -> Vec<EventRecord> {
    let readings = Readings::compute(candles);
    let mut extended = ExtendedReadings::compute(candles, &readings, &ExtendedConfig::DEFAULT);
    overrides(&mut extended);
    let mut events = Vec::new();
    level_extended_events(
        candles,
        &readings,
        &extended,
        &event_config(),
        bar,
        &mut events,
    );
    events
}

fn participation_events_with(
    candles: &[Candle],
    bar: usize,
    overrides: impl FnOnce(&mut ExtendedReadings),
) -> Vec<EventRecord> {
    let readings = Readings::compute(candles);
    let mut extended = ExtendedReadings::compute(candles, &readings, &ExtendedConfig::DEFAULT);
    overrides(&mut extended);
    let mut events = Vec::new();
    participation_events(
        candles,
        &readings,
        &extended,
        &event_config(),
        bar,
        &mut events,
    );
    events
}

#[test]
fn absorption_wick_and_profile_events_are_exercised() {
    let mut candles = flat_candles();
    let len = candles.len();

    let absorption = participation_events_with(&candles, 29, |extended| {
        extended.absorption_flag = series_at(len, &[(29, 1.0)]);
        extended.absorption_strength = series_at(len, &[(29, 1.0)]);
        extended.vwap = series_at(len, &[(29, 100.0)]);
    });
    assert!(
        absorption
            .iter()
            .any(|event| event.event_code == codes::LEVEL_ABSORPTION_TEST)
    );

    candles[29] = candle(29, 102.0, 111.0, 101.0, 110.0, 10.0);
    let broken = participation_events_with(&candles, 29, |extended| {
        extended.absorption_flag = series_at(len, &[(28, 1.0)]);
        extended.absorption_direction = series_at(len, &[(28, 1.0)]);
    });
    assert!(
        broken
            .iter()
            .any(|event| event.event_code == codes::SIGNAL_ABSORPTION_BREAK)
    );

    candles[29] = candle(29, 100.0, 101.0, 99.0, 100.0, 10.0);
    let rejected = participation_events_with(&candles, 29, |extended| {
        extended.absorption_flag = series_at(len, &[(28, 1.0)]);
        extended.absorption_direction = series_at(len, &[(28, 1.0)]);
    });
    assert!(
        rejected
            .iter()
            .any(|event| event.event_code == codes::SIGNAL_ABSORPTION_REJECTION)
    );

    let mut wick_candles = flat_candles();
    wick_candles[29] = candle(29, 100.0, 101.0, 99.0, 100.0, 10.0);
    let wick = level_events_with(&wick_candles, 29, |extended| {
        extended.upper_wick_ratio = series_at(len, &[(29, 0.7)]);
        extended.last_pivot_high = series_at(len, &[(29, 101.0)]);
    });
    assert!(
        wick.iter()
            .any(|event| event.event_code == codes::LEVEL_WICK_TEST)
    );

    let profile = {
        let candles = flat_candles();
        let readings = Readings::compute(&candles);
        let mut extended = ExtendedReadings::compute(&candles, &readings, &ExtendedConfig::DEFAULT);
        extended.poc = series_at(len, &[(29, 100.0)]);
        extended.profile_concentration = series_at(len, &[(28, 0.4), (29, 0.6)]);
        let mut events = Vec::new();
        profile_events(
            &candles,
            &readings,
            &extended,
            &event_config(),
            29,
            &mut events,
        );
        events
    };
    assert!(
        profile
            .iter()
            .any(|event| event.event_code == codes::SIGNAL_PROFILE_CONCENTRATION)
    );
}
