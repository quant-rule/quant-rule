use super::{DirectionBar, bias_state, direction_tenure};
use crate::dashboard::{BiasContinuity, BiasState, TenureOriginKind};
use crate::kernel::Readiness;

fn bar(direction: Option<f64>) -> DirectionBar {
    DirectionBar {
        direction,
        state_code: None,
        size_percentile: None,
        readiness: if direction.is_some() {
            Readiness::Provisional
        } else {
            Readiness::Unavailable
        },
        unavailable_reason: None,
        state_age_bars: 0,
        state_changes_50: 0,
    }
}

fn series(values: &[Option<f64>]) -> Vec<DirectionBar> {
    values.iter().copied().map(bar).collect()
}

fn repeated(value: Option<f64>, count: usize) -> Vec<Option<f64>> {
    vec![value; count]
}

#[test]
fn the_inclusive_corridor_separates_neutral_from_a_bias() {
    assert_eq!(bias_state(None), BiasState::Undefined);
    assert_eq!(bias_state(Some(0.1)), BiasState::Neutral);
    assert_eq!(bias_state(Some(-0.1)), BiasState::Neutral);
    assert_eq!(bias_state(Some(0.0)), BiasState::Neutral);
    assert_eq!(bias_state(Some(0.100_001)), BiasState::Bullish);
    assert_eq!(bias_state(Some(-0.100_001)), BiasState::Bearish);
}

#[test]
fn a_neutral_pause_keeps_the_bias_tenure() {
    let mut values = repeated(Some(0.2), 40);
    values.extend(repeated(Some(0.0), 6));
    let bars = series(&values);

    let evidence = direction_tenure(&bars, 45);
    assert_eq!(evidence.bias_state, BiasState::Neutral);
    assert_eq!(evidence.bias_continuity, BiasContinuity::Continuous);
    assert_eq!(evidence.tenure_bars, Some(46));
    assert_eq!(evidence.origin_kind, TenureOriginKind::FirstEstablishment);
    assert_eq!(evidence.origin_bar, Some(0));
    assert_eq!(
        evidence.tenure_percentile, None,
        "45 prior samples are below the 50-sample minimum"
    );
}

#[test]
fn a_genuine_reversal_resets_the_tenure_and_origin() {
    let mut values = repeated(Some(0.2), 5);
    values.extend(repeated(Some(-0.2), 2));
    let bars = series(&values);

    let reversal = direction_tenure(&bars, 5);
    assert_eq!(reversal.bias_state, BiasState::Bearish);
    assert_eq!(reversal.bias_continuity, BiasContinuity::Continuous);
    assert_eq!(reversal.tenure_bars, Some(1));
    assert_eq!(reversal.origin_kind, TenureOriginKind::Reversal);
    assert_eq!(reversal.origin_bar, Some(5));

    let continued = direction_tenure(&bars, 6);
    assert_eq!(continued.tenure_bars, Some(2));
    assert_eq!(continued.origin_kind, TenureOriginKind::Reversal);
    assert_eq!(continued.origin_bar, Some(5));
}

#[test]
fn an_unavailable_bar_freezes_the_tenure_and_marks_interruption() {
    let mut values = repeated(Some(0.2), 10);
    values.push(None);
    values.push(Some(0.2));
    values.push(None);
    values.push(Some(0.2));
    let bars = series(&values);

    let paused = direction_tenure(&bars, 10);
    assert_eq!(paused.bias_state, BiasState::Undefined);
    assert_eq!(paused.bias_continuity, BiasContinuity::Interrupted);
    assert_eq!(paused.tenure_bars, None);
    assert_eq!(paused.origin_kind, TenureOriginKind::FirstEstablishment);
    assert_eq!(paused.origin_bar, Some(0));

    let resumed = direction_tenure(&bars, 11);
    assert_eq!(resumed.bias_state, BiasState::Bullish);
    assert_eq!(resumed.bias_continuity, BiasContinuity::Interrupted);
    assert_eq!(resumed.tenure_bars, Some(11));
    assert_eq!(resumed.origin_bar, Some(0));

    let paused_again = direction_tenure(&bars, 12);
    assert_eq!(paused_again.tenure_bars, None);
    assert_eq!(paused_again.bias_continuity, BiasContinuity::Interrupted);

    let resumed_again = direction_tenure(&bars, 13);
    assert_eq!(resumed_again.tenure_bars, Some(12));
    assert_eq!(resumed_again.origin_bar, Some(0));
}

#[test]
fn a_neutral_return_after_a_gap_resumes_the_tenure() {
    let mut values = repeated(Some(0.2), 5);
    values.push(None);
    values.push(Some(0.0));
    values.push(Some(-0.2));
    let bars = series(&values);

    let neutral = direction_tenure(&bars, 6);
    assert_eq!(neutral.bias_state, BiasState::Neutral);
    assert_eq!(neutral.bias_continuity, BiasContinuity::Interrupted);
    assert_eq!(neutral.tenure_bars, Some(6));
    assert_eq!(neutral.origin_bar, Some(0));

    // The neutral bar was observed, so the later opposite sign is a genuine
    // observed reversal rather than one hidden inside the gap.
    let reversal = direction_tenure(&bars, 7);
    assert_eq!(reversal.bias_state, BiasState::Bearish);
    assert_eq!(reversal.bias_continuity, BiasContinuity::Continuous);
    assert_eq!(reversal.tenure_bars, Some(1));
    assert_eq!(reversal.origin_kind, TenureOriginKind::Reversal);
    assert_eq!(reversal.origin_bar, Some(7));
}

#[test]
fn an_opposite_return_after_a_gap_starts_a_new_origin() {
    let mut values = repeated(Some(0.2), 5);
    values.push(None);
    values.push(Some(-0.2));
    values.push(Some(-0.2));
    let bars = series(&values);

    let new_origin = direction_tenure(&bars, 6);
    assert_eq!(new_origin.bias_state, BiasState::Bearish);
    assert_eq!(new_origin.bias_continuity, BiasContinuity::Continuous);
    assert_eq!(new_origin.tenure_bars, Some(1));
    assert_eq!(
        new_origin.origin_kind,
        TenureOriginKind::NewOriginAfterUnavailable
    );
    assert_eq!(
        new_origin.origin_bar,
        Some(6),
        "the origin is the first observed bar, never backdated into the gap"
    );

    let continued = direction_tenure(&bars, 7);
    assert_eq!(continued.tenure_bars, Some(2));
    assert_eq!(
        continued.origin_kind,
        TenureOriginKind::NewOriginAfterUnavailable
    );
    assert_eq!(continued.origin_bar, Some(6));
}

#[test]
fn neutral_bars_before_the_first_bias_establish_no_tenure() {
    let bars = series(&[Some(0.0), Some(0.0), Some(0.2)]);

    let before = direction_tenure(&bars, 1);
    assert_eq!(before.bias_state, BiasState::Neutral);
    assert_eq!(before.bias_continuity, BiasContinuity::Continuous);
    assert_eq!(before.tenure_bars, None);
    assert_eq!(before.origin_kind, TenureOriginKind::None);
    assert_eq!(before.origin_bar, None);

    let established = direction_tenure(&bars, 2);
    assert_eq!(established.tenure_bars, Some(1));
    assert_eq!(
        established.origin_kind,
        TenureOriginKind::FirstEstablishment
    );
    assert_eq!(established.origin_bar, Some(2));
}

#[test]
fn the_percentile_needs_fifty_prior_samples() {
    let bars = series(&repeated(Some(0.2), 51));

    let forty_nine = direction_tenure(&bars, 49);
    assert_eq!(forty_nine.tenure_bars, Some(50));
    assert_eq!(
        forty_nine.tenure_percentile, None,
        "49 prior tenures are below the minimum; the current tenure is not a sample"
    );

    let fifty = direction_tenure(&bars, 50);
    assert_eq!(fifty.tenure_bars, Some(51));
    assert_eq!(fifty.tenure_percentile, Some(100.0));
}

#[test]
fn the_percentile_is_capped_at_252_and_never_samples_the_current_bar() {
    let mut values = repeated(Some(0.2), 300);
    values.push(Some(-0.2));
    let bars = series(&values);

    let at_peak = direction_tenure(&bars, 299);
    assert_eq!(at_peak.tenure_bars, Some(300));
    assert_eq!(at_peak.tenure_percentile, Some(100.0));

    // The prior window is the last 252 tenures (49 through 300), all above
    // the current 1. The first tenure of 1 is outside the window, and the
    // current bar is not its own sample, so the percentile is a measured zero.
    let reversed = direction_tenure(&bars, 300);
    assert_eq!(reversed.tenure_bars, Some(1));
    assert_eq!(reversed.tenure_percentile, Some(0.0));
}

#[test]
fn inclusive_ties_rank_at_the_top() {
    let values: Vec<Option<f64>> = (0..60)
        .map(|bar| if bar % 2 == 0 { Some(0.2) } else { Some(-0.2) })
        .collect();
    let bars = series(&values);

    let evidence = direction_tenure(&bars, 50);
    assert_eq!(evidence.tenure_bars, Some(1));
    assert_eq!(
        evidence.tenure_percentile,
        Some(100.0),
        "all 50 prior tenures tie the current tenure and ties are inclusive"
    );
}

#[test]
fn a_bar_outside_the_series_is_undefined() {
    let bars = series(&[Some(0.2)]);
    assert_eq!(
        direction_tenure(&bars, 1),
        super::DirectionTenure::undefined()
    );
    assert_eq!(
        direction_tenure(&[], 0),
        super::DirectionTenure::undefined()
    );
}

#[test]
fn tenure_is_causal_and_deterministic() {
    let values: Vec<Option<f64>> = (0..80)
        .map(|bar| match bar % 11 {
            0..=2 => Some(0.3),
            3 => None,
            4 | 5 => Some(0.0),
            6 | 7 => Some(-0.4),
            _ => None,
        })
        .collect();
    let bars = series(&values);

    for bar in 0..bars.len() {
        let full = direction_tenure(&bars, bar);
        let prefix = direction_tenure(&bars[..=bar], bar);
        assert_eq!(full, prefix, "bar {bar} changed with a later prefix");

        let repeated_run = direction_tenure(&bars, bar);
        assert_eq!(full, repeated_run, "bar {bar} is not deterministic");
    }
}
