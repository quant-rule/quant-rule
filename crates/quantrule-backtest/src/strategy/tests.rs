//! Unit tests for the strategy type boundary.

use quantrule_data::Candle;
use quantrule_model::run::TimeframeModel;
use quantrule_model::state::{MarketStateConfig, TimeframeInput};

use super::*;

fn bar(
    direction: Option<f64>,
    momentum_force: Option<f64>,
    structure_score: Option<f64>,
) -> StrategyInput {
    StrategyInput::new(
        900,
        7,
        1_700_000_000,
        direction,
        momentum_force,
        structure_score,
    )
}

#[test]
fn default_config_is_the_first_strategy() {
    let config = StrategyConfig::default();

    assert_eq!(config.timeframe_secs(), 900);
    assert_eq!(config.long_threshold(), 0.1);
    assert_eq!(config.short_threshold(), -0.1);
}

#[test]
fn config_accepts_asymmetric_thresholds() {
    let config = StrategyConfig::new(900, 0.2, -0.05).unwrap();

    assert_eq!(config.timeframe_secs(), 900);
    assert_eq!(config.long_threshold(), 0.2);
    assert_eq!(config.short_threshold(), -0.05);
}

#[test]
fn config_rejects_zero_timeframe() {
    assert_eq!(
        StrategyConfig::new(0, 0.1, -0.1),
        Err(StrategyConfigError::ZeroTimeframe)
    );
}

#[test]
fn config_rejects_non_finite_thresholds() {
    assert_eq!(
        StrategyConfig::new(900, f64::NAN, -0.1),
        Err(StrategyConfigError::NonFiniteLongThreshold)
    );
    assert_eq!(
        StrategyConfig::new(900, f64::INFINITY, -0.1),
        Err(StrategyConfigError::NonFiniteLongThreshold)
    );
    assert_eq!(
        StrategyConfig::new(900, 0.1, f64::NEG_INFINITY),
        Err(StrategyConfigError::NonFiniteShortThreshold)
    );
}

#[test]
fn config_rejects_thresholds_outside_the_zero_band() {
    assert_eq!(
        StrategyConfig::new(900, -0.1, -0.2),
        Err(StrategyConfigError::NegativeLongThreshold)
    );
    assert_eq!(
        StrategyConfig::new(900, 0.2, 0.1),
        Err(StrategyConfigError::PositiveShortThreshold)
    );
}

#[test]
fn config_rejects_long_not_above_short() {
    assert_eq!(
        StrategyConfig::new(900, 0.1, 0.1),
        Err(StrategyConfigError::LongNotAboveShort)
    );
    assert_eq!(
        StrategyConfig::new(900, 0.0, 0.0),
        Err(StrategyConfigError::LongNotAboveShort)
    );
}

#[test]
fn config_error_messages_are_stable() {
    assert_eq!(
        StrategyConfigError::ZeroTimeframe.to_string(),
        "the strategy timeframe must be non-zero"
    );
    assert_eq!(
        StrategyConfigError::LongNotAboveShort.to_string(),
        "the long threshold must be greater than the short threshold"
    );
}

#[test]
fn long_requires_all_three_readings_strictly_above() {
    let config = StrategyConfig::default();

    assert_eq!(
        Agreement::classify(&bar(Some(0.100_001), Some(0.2), Some(1.0)), &config),
        Some(Agreement::Long)
    );
}

#[test]
fn short_requires_all_three_readings_strictly_below() {
    let config = StrategyConfig::default();

    assert_eq!(
        Agreement::classify(&bar(Some(-0.100_001), Some(-0.2), Some(-1.0)), &config),
        Some(Agreement::Short)
    );
}

#[test]
fn a_mixed_bar_is_neutral() {
    let config = StrategyConfig::default();

    assert_eq!(
        Agreement::classify(&bar(Some(0.2), Some(0.2), Some(-0.2)), &config),
        Some(Agreement::Neutral)
    );
}

#[test]
fn equality_at_the_long_threshold_is_neutral() {
    let config = StrategyConfig::default();

    assert_eq!(
        Agreement::classify(&bar(Some(0.1), Some(0.2), Some(1.0)), &config),
        Some(Agreement::Neutral)
    );
}

#[test]
fn equality_at_the_short_threshold_is_neutral() {
    let config = StrategyConfig::default();

    assert_eq!(
        Agreement::classify(&bar(Some(-0.1), Some(-0.2), Some(-1.0)), &config),
        Some(Agreement::Neutral)
    );
}

#[test]
fn a_measured_zero_is_neutral_not_missing() {
    let config = StrategyConfig::default();

    assert_eq!(
        Agreement::classify(&bar(Some(0.0), Some(0.0), Some(0.0)), &config),
        Some(Agreement::Neutral)
    );
}

#[test]
fn a_missing_reading_produces_no_agreement() {
    let config = StrategyConfig::default();

    assert_eq!(
        Agreement::classify(&bar(None, Some(0.2), Some(1.0)), &config),
        None
    );
    assert_eq!(
        Agreement::classify(&bar(Some(0.2), None, Some(1.0)), &config),
        None
    );
    assert_eq!(
        Agreement::classify(&bar(Some(0.2), Some(0.2), None), &config),
        None
    );
}

#[test]
fn a_non_finite_reading_produces_no_agreement() {
    let config = StrategyConfig::default();

    assert_eq!(
        Agreement::classify(&bar(Some(f64::NAN), Some(0.2), Some(1.0)), &config),
        None
    );
    assert_eq!(
        Agreement::classify(&bar(Some(0.2), Some(f64::INFINITY), Some(1.0)), &config),
        None
    );
    assert_eq!(
        Agreement::classify(&bar(Some(0.2), Some(0.2), Some(f64::NEG_INFINITY)), &config),
        None
    );
}

#[test]
fn classification_is_deterministic() {
    let config = StrategyConfig::default();
    let input = bar(Some(0.2), Some(0.3), Some(0.4));
    let first = Agreement::classify(&input, &config);

    for _ in 0..100 {
        assert_eq!(Agreement::classify(&input, &config), first);
    }
}

#[test]
fn agreement_maps_to_the_entry_side() {
    assert_eq!(Agreement::Long.side(), Some(Side::Long));
    assert_eq!(Agreement::Short.side(), Some(Side::Short));
    assert_eq!(Agreement::Neutral.side(), None);
}

#[test]
fn side_codes_and_opposites_are_stable() {
    assert_eq!(Side::Long.code(), 1);
    assert_eq!(Side::Short.code(), -1);
    assert_eq!(Side::Long.opposite(), Side::Short);
    assert_eq!(Side::Short.opposite(), Side::Long);
}

#[test]
fn entry_and_exit_records_preserve_the_bar() {
    let input = bar(Some(0.2), Some(0.3), Some(0.4));
    let entry = Entry {
        side: Side::Long,
        bar: input,
    };
    let exit = Exit {
        side: Side::Long,
        bar: input,
        reason: ExitReason::AgreementLost,
    };

    assert_eq!(entry.side, Side::Long);
    assert_eq!(entry.bar, input);
    assert_eq!(exit.side, Side::Long);
    assert_eq!(exit.bar, input);
    assert_eq!(exit.reason, ExitReason::AgreementLost);
}

#[test]
fn a_test_strategy_can_enter_and_hold() {
    struct EnterOnLong;

    impl Strategy for EnterOnLong {
        fn on_bar(&mut self, input: &StrategyInput) -> Decision {
            match Agreement::classify(input, &StrategyConfig::default()) {
                Some(Agreement::Long) => Decision::Enter(Entry {
                    side: Side::Long,
                    bar: *input,
                }),
                _ => Decision::Hold,
            }
        }
    }

    let mut strategy = EnterOnLong;
    let long = bar(Some(0.2), Some(0.3), Some(0.4));
    let neutral = bar(Some(0.2), Some(0.3), Some(-0.4));

    assert_eq!(
        strategy.on_bar(&long),
        Decision::Enter(Entry {
            side: Side::Long,
            bar: long,
        })
    );
    assert_eq!(strategy.on_bar(&neutral), Decision::Hold);
}

#[test]
fn from_model_bar_projects_only_the_three_readings() {
    let candles: Vec<Candle> = (0_u64..3)
        .map(|index| {
            Candle::new(
                index,
                1_700_000_000 + index as i64 * 900,
                100.0,
                101.0,
                99.0,
                100.5,
                Some(1.0),
            )
            .unwrap()
        })
        .collect();
    let input = TimeframeInput {
        timeframe_secs: 900,
        candles: &candles,
    };
    let model = TimeframeModel::compute(&input, &MarketStateConfig::default()).unwrap();
    let model_bar = model.bar(0).unwrap();

    let snapshot = StrategyInput::from_model_bar(&model_bar);

    assert_eq!(snapshot.timeframe_secs, 900);
    assert_eq!(snapshot.bar_index, 0);
    assert_eq!(snapshot.open_time, 1_700_000_000);
    assert_eq!(snapshot.direction, model_bar.direction.direction);
    assert_eq!(snapshot.momentum_force, model_bar.momentum.force);
    assert_eq!(snapshot.structure_score, model_bar.structure.score);
}
