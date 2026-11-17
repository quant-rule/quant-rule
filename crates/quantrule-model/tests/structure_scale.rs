//! `structure.scale` selects the confirmed pivot scale of the structure
//! readings and sequence events.
//!
//! The default major scale keeps the existing output. A minor-scale config must
//! change the `last_pivot_high`/`last_pivot_low` readings and the lower-high /
//! lower-low sequence events; otherwise the config field is cosmetic.

use quantrule_data::Candle;
use quantrule_model::events::codes;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::readings::PivotScale;
use quantrule_model::timeframe::TimeframeModel;

fn candles() -> Vec<Candle> {
    let highs = [
        1.0, 2.0, 1.0, 2.0, 1.0, 10.0, 2.0, 1.0, 2.0, 3.0, 1.0, 1.5, 2.0, 0.5, 0.5, 1.0, 0.5, 0.5,
        0.5, 0.5,
    ];
    highs
        .iter()
        .enumerate()
        .map(|(index, high)| {
            let low = 0.1;
            let mid = (high + low) / 2.0;
            Candle::new(index as u64, index as i64, mid, *high, low, mid, Some(1.0))
                .expect("the test candle is valid")
        })
        .collect()
}

fn model(scale: PivotScale) -> TimeframeModel {
    let candles = candles();
    TimeframeModel::compute(
        &TimeframeInput::new(3_600, &candles),
        &MarketStateConfig::new(scale),
    )
    .expect("the timeframe model computes")
}

#[test]
fn a_minor_scale_changes_the_structure_reading_and_the_sequence_event() {
    let major = model(PivotScale::Major);
    let minor = model(PivotScale::Minor);

    assert_eq!(major.extended().last_pivot_high.get(7), None);
    assert_eq!(minor.extended().last_pivot_high.get(7), Some(10.0));
    assert_eq!(major.extended().last_pivot_high.get(11), Some(10.0));
    assert_eq!(minor.extended().last_pivot_high.get(11), Some(3.0));

    let major_sequence = major
        .extended_events()
        .events()
        .iter()
        .any(|event| event.event_code == codes::SIGNAL_LOWER_HIGH_SEQUENCE);
    assert!(
        !major_sequence,
        "the major scale has only one confirmed high and cannot form a sequence"
    );

    let minor_sequence: Vec<_> = minor
        .extended_events()
        .events()
        .iter()
        .filter(|event| event.event_code == codes::SIGNAL_LOWER_HIGH_SEQUENCE)
        .collect();
    assert_eq!(minor_sequence.len(), 2);
    assert_eq!(minor_sequence[0].occurred_bar_index, 12);
    assert_eq!(minor_sequence[0].available_bar_index, 14);
    assert_eq!(minor_sequence[0].primary_value, Some(2.0));
    assert_eq!(minor_sequence[1].occurred_bar_index, 15);
    assert_eq!(minor_sequence[1].available_bar_index, 17);
    assert_eq!(minor_sequence[1].primary_value, Some(3.0));
}

#[test]
fn the_model_default_scale_is_major() {
    let default = model(PivotScale::Major);
    assert_eq!(default.extended().last_pivot_high.get(11), Some(10.0));
}
