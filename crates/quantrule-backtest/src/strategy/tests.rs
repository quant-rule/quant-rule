//! Unit tests for the strategy type boundary.

use quantrule_data::Candle;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::timeframe::TimeframeModel;

use super::*;

/// A test adapter that supplies the position the stateless rule does not own.
struct Tracked {
    strategy: AxisAgreementStrategy,
    position: StrategyPosition,
}

impl Tracked {
    fn new(config: StrategyConfig) -> Self {
        Self {
            strategy: AxisAgreementStrategy::new(config),
            position: StrategyPosition::flat(),
        }
    }

    fn default() -> Self {
        Self::new(StrategyConfig::default())
    }

    fn on_bar(&mut self, input: &StrategyInput) -> Decision {
        if self.position.is_open() {
            self.position.bars_held += 1;
        }
        let decision = self.strategy.decide(input, self.position);
        match decision {
            Decision::Enter(entry) => {
                self.position = StrategyPosition::opened(entry.intent.side, input.bar_index);
            }
            Decision::Exit(_) => self.position = StrategyPosition::flat(),
            Decision::Hold => {}
        }
        decision
    }

    fn position(&self) -> Option<Side> {
        self.position.side
    }

    fn config(&self) -> StrategyConfig {
        self.strategy.config()
    }
}

/// The entry an adapter with the default risk policy produces.
fn entry(side: Side, bar: StrategyInput) -> Entry {
    Entry {
        bar,
        intent: RiskPolicy::default().intent(side),
    }
}

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
    let entry = entry(Side::Long, input);
    let exit = Exit {
        side: Side::Long,
        bar: input,
        reason: ExitReason::AgreementLost,
    };

    assert_eq!(entry.side(), Side::Long);
    assert_eq!(entry.bar, input);
    assert_eq!(exit.side, Side::Long);
    assert_eq!(exit.bar, input);
    assert_eq!(exit.reason, ExitReason::AgreementLost);
}

#[test]
fn a_test_strategy_can_enter_and_hold() {
    struct EnterOnLong;

    impl Strategy for EnterOnLong {
        fn decide(&self, input: &StrategyInput, _position: StrategyPosition) -> Decision {
            match Agreement::classify(input, &StrategyConfig::default()) {
                Some(Agreement::Long) => Decision::Enter(entry(Side::Long, *input)),
                _ => Decision::Hold,
            }
        }
    }

    let strategy = EnterOnLong;
    let long = bar(Some(0.2), Some(0.3), Some(0.4));
    let neutral = bar(Some(0.2), Some(0.3), Some(-0.4));

    assert_eq!(
        strategy.decide(&long, StrategyPosition::flat()),
        Decision::Enter(entry(Side::Long, long))
    );
    assert_eq!(
        strategy.decide(&neutral, StrategyPosition::flat()),
        Decision::Hold
    );
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

fn strategy_input(
    timeframe_secs: u64,
    bar_index: usize,
    direction: Option<f64>,
    momentum_force: Option<f64>,
    structure_score: Option<f64>,
) -> StrategyInput {
    StrategyInput::new(
        timeframe_secs,
        bar_index,
        1_700_000_000 + bar_index as i64 * 900,
        direction,
        momentum_force,
        structure_score,
    )
}

fn run(strategy: &mut Tracked, inputs: &[StrategyInput]) -> Vec<Decision> {
    inputs.iter().map(|input| strategy.on_bar(input)).collect()
}

#[test]
fn agreement_strategy_defaults_to_the_first_configuration() {
    let strategy = Tracked::default();

    assert_eq!(strategy.config(), StrategyConfig::default());
    assert_eq!(strategy.position(), None);
}

#[test]
fn agreement_strategy_carries_the_configured_risk_policy() {
    let risk = RiskPolicy::new(3.0, 2.0).expect("the risk policy is valid");
    let strategy = AxisAgreementStrategy::with_risk(StrategyConfig::default(), risk);
    let input = bar(Some(0.2), Some(0.5), Some(1.0));

    match strategy.decide(&input, StrategyPosition::flat()) {
        Decision::Enter(entry) => {
            assert_eq!(entry.intent.side, Side::Long);
            assert_eq!(entry.intent.stop, StopSpec::AtrMultiple { multiple: 3.0 });
            assert_eq!(
                entry.intent.target,
                TargetSpec::RewardToRisk { multiple: 2.0 }
            );
        }
        other => panic!("expected an entry, got {other:?}"),
    }
}

#[test]
fn agreement_strategy_enters_long_on_agreement() {
    let mut strategy = Tracked::default();
    let input = bar(Some(0.2), Some(0.5), Some(1.0));

    assert_eq!(
        strategy.on_bar(&input),
        Decision::Enter(entry(Side::Long, input))
    );
    assert_eq!(strategy.position(), Some(Side::Long));
}

#[test]
fn agreement_strategy_enters_short_on_agreement() {
    let mut strategy = Tracked::default();
    let input = bar(Some(-0.2), Some(-0.5), Some(-1.0));

    assert_eq!(
        strategy.on_bar(&input),
        Decision::Enter(entry(Side::Short, input))
    );
    assert_eq!(strategy.position(), Some(Side::Short));
}

#[test]
fn agreement_strategy_holds_while_flat_without_agreement() {
    let mut strategy = Tracked::default();

    assert_eq!(
        strategy.on_bar(&bar(Some(0.2), Some(0.2), Some(-0.2))),
        Decision::Hold
    );
    assert_eq!(
        strategy.on_bar(&bar(None, Some(0.2), Some(1.0))),
        Decision::Hold
    );
    assert_eq!(
        strategy.on_bar(&bar(Some(0.1), Some(0.2), Some(1.0))),
        Decision::Hold
    );
    assert_eq!(strategy.position(), None);
}

#[test]
fn agreement_strategy_exits_long_on_neutral() {
    let mut strategy = Tracked::default();
    let long = bar(Some(0.2), Some(0.5), Some(1.0));
    let neutral = bar(Some(0.2), Some(0.0), Some(0.5));

    assert!(matches!(strategy.on_bar(&long), Decision::Enter(_)));
    assert_eq!(
        strategy.on_bar(&neutral),
        Decision::Exit(Exit {
            side: Side::Long,
            bar: neutral,
            reason: ExitReason::AgreementLost,
        })
    );
    assert_eq!(strategy.position(), None);
}

#[test]
fn agreement_strategy_exits_short_on_neutral() {
    let mut strategy = Tracked::default();
    let short = bar(Some(-0.2), Some(-0.5), Some(-1.0));
    let neutral = bar(Some(-0.2), Some(0.0), Some(-0.5));

    assert!(matches!(strategy.on_bar(&short), Decision::Enter(_)));
    assert_eq!(
        strategy.on_bar(&neutral),
        Decision::Exit(Exit {
            side: Side::Short,
            bar: neutral,
            reason: ExitReason::AgreementLost,
        })
    );
    assert_eq!(strategy.position(), None);
}

#[test]
fn agreement_strategy_exits_on_the_opposite_agreement_without_reversing() {
    let mut strategy = Tracked::default();
    let long = strategy_input(900, 0, Some(0.2), Some(0.5), Some(1.0));
    let short = strategy_input(900, 1, Some(-0.3), Some(-0.4), Some(-0.8));

    assert!(matches!(strategy.on_bar(&long), Decision::Enter(_)));
    assert_eq!(
        strategy.on_bar(&short),
        Decision::Exit(Exit {
            side: Side::Long,
            bar: short,
            reason: ExitReason::AgreementLost,
        })
    );
    assert_eq!(strategy.position(), None);

    let next_short = strategy_input(900, 2, Some(-0.3), Some(-0.4), Some(-0.8));
    assert_eq!(
        strategy.on_bar(&next_short),
        Decision::Enter(entry(Side::Short, next_short))
    );
    assert_eq!(strategy.position(), Some(Side::Short));
}

#[test]
fn agreement_strategy_holds_on_an_unavailable_reading() {
    let mut strategy = Tracked::default();
    let long = strategy_input(900, 0, Some(0.2), Some(0.5), Some(1.0));

    assert!(matches!(strategy.on_bar(&long), Decision::Enter(_)));
    assert_eq!(
        strategy.on_bar(&strategy_input(900, 1, None, Some(0.5), Some(1.0))),
        Decision::Hold
    );
    assert_eq!(
        strategy.on_bar(&strategy_input(
            900,
            2,
            Some(f64::NAN),
            Some(0.5),
            Some(1.0)
        )),
        Decision::Hold
    );
    assert_eq!(strategy.position(), Some(Side::Long));
}

#[test]
fn agreement_strategy_ignores_other_timeframes() {
    let mut strategy = Tracked::default();

    assert_eq!(
        strategy.on_bar(&strategy_input(60, 0, Some(0.2), Some(0.5), Some(1.0))),
        Decision::Hold
    );
    assert_eq!(strategy.position(), None);

    let long = strategy_input(900, 1, Some(0.2), Some(0.5), Some(1.0));
    assert!(matches!(strategy.on_bar(&long), Decision::Enter(_)));
    assert_eq!(
        strategy.on_bar(&strategy_input(60, 2, Some(0.0), Some(0.0), Some(0.0))),
        Decision::Hold
    );
    assert_eq!(strategy.position(), Some(Side::Long));
}

#[test]
fn agreement_strategy_uses_the_configured_thresholds() {
    let config = StrategyConfig::new(900, 0.2, -0.05).unwrap();
    let mut strategy = Tracked::new(config);

    assert_eq!(
        strategy.on_bar(&strategy_input(900, 0, Some(0.15), Some(0.15), Some(0.15))),
        Decision::Hold
    );

    let short = strategy_input(900, 1, Some(-0.06), Some(-0.06), Some(-0.06));
    assert_eq!(
        strategy.on_bar(&short),
        Decision::Enter(entry(Side::Short, short))
    );
}

#[test]
fn agreement_strategy_is_deterministic() {
    let inputs: Vec<StrategyInput> = (0..12)
        .map(|index| {
            let reading = match index % 4 {
                0 => Some(0.2),
                1 => Some(-0.2),
                2 => Some(0.0),
                _ => None,
            };
            strategy_input(900, index, reading, reading, reading)
        })
        .collect();

    let mut first = Tracked::default();
    let expected = run(&mut first, &inputs);

    for _ in 0..10 {
        let mut again = Tracked::default();
        assert_eq!(run(&mut again, &inputs), expected);
    }
}

#[test]
fn agreement_strategy_is_causal() {
    let inputs: Vec<StrategyInput> = vec![
        strategy_input(900, 0, Some(0.2), Some(0.5), Some(1.0)),
        strategy_input(900, 1, Some(0.2), Some(0.5), Some(1.0)),
        strategy_input(900, 2, Some(0.0), Some(0.0), Some(0.0)),
        strategy_input(900, 3, Some(-0.2), Some(-0.5), Some(-1.0)),
        strategy_input(900, 4, Some(-0.2), Some(-0.5), Some(-1.0)),
    ];
    let prefix = 3;

    let mut short = Tracked::default();
    let prefix_decisions = run(&mut short, &inputs[..prefix]);

    let mut full = Tracked::default();
    let full_decisions = run(&mut full, &inputs);
    assert_eq!(full_decisions[..prefix], prefix_decisions);

    let mut changed = inputs.clone();
    changed[4] = strategy_input(900, 4, Some(0.9), Some(0.9), Some(0.9));
    let mut other = Tracked::default();
    let changed_decisions = run(&mut other, &changed);
    assert_eq!(changed_decisions[..prefix], prefix_decisions);
}

#[test]
fn agreement_strategy_handles_an_empty_series() {
    let mut strategy = Tracked::default();

    assert!(run(&mut strategy, &[]).is_empty());
    assert_eq!(strategy.position(), None);
}
