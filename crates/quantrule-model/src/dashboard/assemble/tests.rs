use super::{
    AxisDetail, AxisReadiness, AxisState, AxisValue, BiasState, ChurnClass, Coverage,
    DashboardConfig, DashboardState, Domain, Readiness, Stage, TimeframeState, UnavailableReason,
    absent_structure_scale_code, bias_state, rank_size, signed_position,
};
use crate::readings::PivotScale;
use crate::readings::roles::AxisCode;
use crate::run::{ModelRun, TimeframeModel};
use crate::state::{MarketStateConfig, TimeframeInput};
use quantrule_data::Candle;

const READINGS_FIXTURE: &str = include_str!("../../../tests/fixtures/readings_golden.csv");
const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";

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

fn fixture_candles() -> Vec<Candle> {
    let mut lines = READINGS_FIXTURE.lines();
    assert_eq!(lines.next(), Some(READINGS_HEADER));
    let mut candles = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 23);
        let series_index = fields[0].parse().expect("bar");
        let timestamp = fields[1].parse().expect("timestamp");
        let open = number(fields[2]).expect("open");
        let high = number(fields[3]).expect("high");
        let low = number(fields[4]).expect("low");
        let close = number(fields[5]).expect("close");
        let volume = number(fields[6]);
        candles.push(
            Candle::new(series_index, timestamp, open, high, low, close, volume)
                .expect("valid fixture candle"),
        );
    }
    candles
}

fn model(candles: &[Candle], timeframe_secs: u64) -> TimeframeModel {
    TimeframeModel::compute(
        &TimeframeInput {
            timeframe_secs,
            candles,
        },
        &MarketStateConfig::default(),
    )
    .expect("the fixture is non-empty")
}

fn run(candles: &[Candle], timeframes: &[u64]) -> ModelRun {
    let inputs: Vec<TimeframeInput<'_>> = timeframes
        .iter()
        .map(|&timeframe_secs| TimeframeInput {
            timeframe_secs,
            candles,
        })
        .collect();
    ModelRun::compute(&inputs, &MarketStateConfig::default())
}

fn view(run: &ModelRun, ladder: &[u64]) -> DashboardState {
    DashboardState::from_run(run, ladder, &DashboardConfig::default())
}

fn axis(state: &TimeframeState, axis_code: AxisCode) -> &AxisState {
    state
        .axes
        .iter()
        .find(|axis| axis.axis == axis_code)
        .expect("exactly one state per axis code")
}

#[test]
fn the_latest_state_maps_the_per_timeframe_envelope() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);
    let latest = timeframe.len() - 1;
    let bar = timeframe.bar(latest).expect("the latest bar");
    let state = TimeframeState::at(&timeframe, latest).expect("the latest state");

    assert!(state.present);
    assert_eq!(state.timeframe_secs, 60);
    assert_eq!(state.latest_bar, Some(latest));
    assert_eq!(
        state.latest_open_time,
        Some(candles[latest].timestamp()),
        "the as-of time is the latest candle's open time"
    );
    assert_eq!(state.window_start, 0);
    assert_eq!(state.window_len, candles.len());
    assert_eq!(state.axes.len(), 4);
    for axis_code in AxisCode::ALL {
        assert_eq!(axis(&state, axis_code).axis, axis_code);
    }

    assert_eq!(state.diagnostics.atr_ratio, bar.context.atr_ratio);
    assert_eq!(state.diagnostics.rvol_ratio, bar.context.rvol_ratio);
    assert_eq!(state.diagnostics.context_code, bar.context.context_code);

    let consensus = state.derived.timeframe_consensus;
    assert_eq!(
        consensus.bullish_axis_count,
        bar.consensus.bullish_axis_count
    );
    assert_eq!(
        consensus.bearish_axis_count,
        bar.consensus.bearish_axis_count
    );
    assert_eq!(
        consensus.neutral_axis_count,
        bar.consensus.neutral_axis_count
    );
    assert_eq!(
        consensus.weakest_directional_axis_code,
        bar.consensus.weakest_directional_axis_code
    );

    assert_eq!(TimeframeState::at(&timeframe, timeframe.len()), None);
}

#[test]
fn all_four_axes_map_their_positions_sizes_and_readiness() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);
    let latest = timeframe.len() - 1;
    let bar = timeframe.bar(latest).expect("the latest bar");
    let state = TimeframeState::at(&timeframe, latest).expect("the latest state");

    let direction = axis(&state, AxisCode::Direction);
    match bar.direction.direction {
        Some(value) => assert_eq!(
            direction.position,
            AxisValue::signed(value, Domain::DirectionalBias)
        ),
        None => assert!(direction.position.is_undefined()),
    }
    match bar.direction.size_percentile {
        Some(pct) => assert_eq!(direction.size, AxisValue::rank(pct)),
        None => assert!(direction.size.is_undefined()),
    }
    assert_eq!(direction.readiness_code, bar.direction.readiness.code());
    assert_eq!(
        direction.readiness,
        AxisReadiness::from(bar.direction.readiness)
    );
    let AxisDetail::Direction(direction_detail) = &direction.detail else {
        panic!("the Direction axis carries Direction detail");
    };
    assert_eq!(
        direction_detail.bias_state,
        bias_state(bar.direction.direction)
    );
    assert_eq!(
        direction_detail.magnitude_pct,
        bar.direction.size_percentile
    );

    let momentum = axis(&state, AxisCode::Momentum);
    match bar.momentum.acceleration {
        Some(value) => assert_eq!(
            momentum.position,
            AxisValue::signed(value, Domain::Acceleration)
        ),
        None => assert!(momentum.position.is_undefined()),
    }
    match bar.momentum.size_percentile {
        Some(pct) => assert_eq!(momentum.size, AxisValue::rank(pct)),
        None => assert!(momentum.size.is_undefined()),
    }
    assert_eq!(momentum.readiness_code, bar.momentum.readiness.code());
    assert_eq!(
        momentum.readiness,
        AxisReadiness::from(bar.momentum.readiness)
    );
    let AxisDetail::Momentum(momentum_detail) = &momentum.detail else {
        panic!("the Momentum axis carries Momentum detail");
    };
    assert_eq!(momentum_detail.force, bar.momentum.force);
    assert_eq!(momentum_detail.acceleration, bar.momentum.acceleration);
    assert_eq!(
        momentum_detail.intensification,
        bar.momentum.intensification
    );

    let structure = axis(&state, AxisCode::Structure);
    match bar.structure.score {
        Some(value) => assert_eq!(
            structure.position,
            AxisValue::signed(value, Domain::StructureRelation)
        ),
        None => assert!(structure.position.is_undefined()),
    }
    match bar.structure.size_percentile {
        Some(pct) => assert_eq!(structure.size, AxisValue::rank(pct)),
        None => assert!(structure.size.is_undefined()),
    }
    assert_eq!(structure.readiness_code, bar.structure.readiness.code());
    assert_eq!(
        structure.readiness,
        AxisReadiness::from(bar.structure.readiness)
    );
    let AxisDetail::Structure(structure_detail) = &structure.detail else {
        panic!("the Structure axis carries Structure detail");
    };
    assert_eq!(structure_detail.score, bar.structure.score);
    assert_eq!(
        structure_detail.high_relation_code,
        bar.structure.high_relation_code
    );
    assert_eq!(
        structure_detail.low_relation_code,
        bar.structure.low_relation_code
    );
    assert_eq!(structure_detail.scale_code, bar.structure.scale_code);
    assert_eq!(
        structure_detail.ema_distance_atr_20,
        bar.structure.ema_distance_atr_20
    );

    let progression = axis(&state, AxisCode::Progression);
    match bar.progression.stage_code.and_then(Stage::from_code) {
        Some(stage) => assert_eq!(progression.position, AxisValue::ordinal(stage)),
        None => assert!(progression.position.is_undefined()),
    }
    match bar.progression.size_percentile {
        Some(pct) => assert_eq!(progression.size, AxisValue::rank(pct)),
        None => assert!(progression.size.is_undefined()),
    }
    assert_eq!(progression.readiness_code, bar.progression.readiness.code());
    assert_eq!(
        progression.readiness,
        AxisReadiness::from(bar.progression.readiness)
    );
    let AxisDetail::Progression(progression_detail) = &progression.detail else {
        panic!("the Progression axis carries Progression detail");
    };
    assert_eq!(progression_detail.progress, bar.progression.progress);
    assert_eq!(progression_detail.stage_code, bar.progression.stage_code);
    assert_eq!(
        progression_detail.leg_direction,
        bar.progression.leg_direction
    );
    assert_eq!(
        progression_detail.displacement_pct,
        bar.progression.size_percentile
    );
    assert_eq!(
        progression_detail.recent_pace_window_bars,
        super::RECENT_PACE_WINDOW_BARS
    );
}

#[test]
fn every_reachable_stage_maps_to_its_anchor() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);
    let mut seen = [false; 6];

    for bar in 0..timeframe.len() {
        let model_bar = timeframe.bar(bar).expect("a bar");
        let Some(code) = model_bar.progression.stage_code else {
            continue;
        };
        let state = TimeframeState::at(&timeframe, bar).expect("a state");
        let position = &axis(&state, AxisCode::Progression).position;
        assert_eq!(
            position.as_stage(),
            Stage::from_code(code),
            "bar {bar} published stage code {code}"
        );
        seen[usize::try_from(code).expect("a non-negative stage code")] = true;
    }

    assert!(
        seen.into_iter().any(|found| found),
        "the fixture must publish at least one stage"
    );
}

#[test]
fn missing_timeframe_is_typed_absence() {
    let candles = fixture_candles();
    let run = run(&candles, &[60]);
    let view = view(&run, &[300, 60]);

    assert_eq!(view.ladder, vec![60, 300]);
    let present = &view.timeframes[0];
    let absent = &view.timeframes[1];
    assert!(present.present);
    assert!(!absent.present);
    assert_eq!(absent.timeframe_secs, 300);
    assert_eq!(absent.latest_bar, None);
    assert_eq!(absent.latest_open_time, None);
    assert_eq!(absent.window_start, 0);
    assert_eq!(absent.window_len, 0);
    assert_eq!(absent.axes.len(), 4);
    for state in &absent.axes {
        assert!(state.position.is_undefined());
        assert!(state.size.is_undefined());
        assert_eq!(state.readiness, AxisReadiness::Dashed);
        assert_eq!(state.readiness_code, Readiness::Unavailable.code());
        assert!(state.readings.is_empty());
    }
    let detail_kinds: Vec<u8> = absent
        .axes
        .iter()
        .map(|state| match &state.detail {
            AxisDetail::Direction(_) => 1,
            AxisDetail::Momentum(_) => 2,
            AxisDetail::Structure(_) => 3,
            AxisDetail::Progression(_) => 4,
        })
        .collect();
    assert_eq!(detail_kinds, vec![1, 2, 3, 4]);

    assert_eq!(view.cross_tf_state.present_timeframes, vec![60]);
    assert_eq!(
        view.cross_tf_state.agreement.coverage,
        Coverage {
            present: 1,
            configured: 2
        }
    );
    assert_eq!(
        view.cross_tf_state.churn.coverage,
        Coverage {
            present: 1,
            configured: 2
        }
    );
    assert_eq!(view.cross_tf_state.progression_leg_direction.len(), 2);
    assert_eq!(
        view.cross_tf_state.progression_leg_direction[1].timeframe_secs,
        300
    );
    assert_eq!(
        view.cross_tf_state.progression_leg_direction[1].direction,
        None
    );
}

#[test]
fn an_empty_run_builds_one_absent_row_per_ladder_member() {
    let run = ModelRun::compute(&[], &MarketStateConfig::default());
    let view = DashboardState::from_run(&run, &[300, 60, 300], &DashboardConfig::default());

    assert_eq!(view.ladder, vec![60, 300]);
    assert_eq!(view.timeframes.len(), 2);
    assert!(view.timeframes.iter().all(|state| !state.present));
    assert!(
        view.timeframes
            .iter()
            .all(|state| state.axes.iter().all(|axis| axis.position.is_undefined()))
    );
    assert!(view.cross_tf_state.present_timeframes.is_empty());
    assert!(view.cross_tf_state.aggregates.ladder.is_empty());
    assert_eq!(
        view.cross_tf_state.agreement.coverage,
        Coverage {
            present: 0,
            configured: 2
        }
    );
}

#[test]
fn a_single_candle_builds_an_unavailable_state() {
    let candle = Candle::new(0, 1_700_000_000, 100.0, 100.5, 99.5, 100.0, Some(10.0))
        .expect("a valid candle");
    let timeframe = model(&[candle], 60);
    let state = TimeframeState::at(&timeframe, 0).expect("the only bar");

    assert!(state.present);
    assert_eq!(state.latest_bar, Some(0));
    assert_eq!(state.window_len, 1);
    for axis in &state.axes {
        assert!(axis.position.is_undefined());
        assert!(axis.size.is_undefined());
        assert_eq!(axis.readiness, AxisReadiness::Dashed);
        assert_eq!(axis.readiness_code, Readiness::Unavailable.code());
    }
}

#[test]
fn readiness_maps_the_exact_model_code() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);
    let mut saw_unavailable = false;
    let mut saw_provisional = false;
    let mut saw_ready = false;

    for bar in 0..timeframe.len() {
        let state = TimeframeState::at(&timeframe, bar).expect("a bar");
        for axis in &state.axes {
            match axis.readiness_code {
                0 => {
                    assert_eq!(axis.readiness, AxisReadiness::Dashed);
                    saw_unavailable = true;
                }
                1 => {
                    assert_eq!(axis.readiness, AxisReadiness::Dashed);
                    saw_provisional = true;
                }
                2 => {
                    assert_eq!(axis.readiness, AxisReadiness::Solid);
                    saw_ready = true;
                }
                code => panic!("unexpected readiness code {code}"),
            }
        }
    }

    assert!(saw_unavailable, "the fixture must have an unavailable bar");
    assert!(saw_provisional, "the fixture must have a provisional bar");
    assert!(saw_ready, "the fixture must have a ready bar");
}

#[test]
fn a_future_bar_does_not_change_a_past_state() {
    let candles = fixture_candles();
    let bar = 199;
    let full = model(&candles, 60);
    let prefix = model(&candles[..=bar], 60);

    let past = TimeframeState::at(&full, bar).expect("the full model has the bar");
    let bounded = TimeframeState::at(&prefix, bar).expect("the prefix model has the bar");
    assert_eq!(
        past, bounded,
        "appending later bars must not change an earlier state"
    );
    assert_eq!(past.window_len, bar + 1);

    let mut altered = candles.clone();
    let future = altered[bar + 1];
    altered[bar + 1] = Candle::new(
        future.series_index(),
        future.timestamp(),
        future.open(),
        future.high() + 50.0,
        future.low() - 50.0,
        future.close() + 10.0,
        future.volume(),
    )
    .expect("a valid altered future candle");
    let altered = model(&altered, 60);
    let after_change = TimeframeState::at(&altered, bar).expect("the altered model has the bar");
    assert_eq!(
        past, after_change,
        "changing a later bar must not change an earlier state"
    );
}

#[test]
fn reordered_inputs_and_ladders_are_deterministic() {
    let candles = fixture_candles();
    let forward = run(&candles, &[60, 300]);
    let reversed = run(&candles, &[300, 60]);
    assert_eq!(forward, reversed);

    let first = view(&forward, &[60, 300, 3600]);
    let second = DashboardState::from_run(
        &reversed,
        &[3600, 300, 60, 300],
        &DashboardConfig::default(),
    );
    let third = view(&forward, &[60, 300, 3600]);

    assert_eq!(first, second);
    assert_eq!(first, third);
    assert_eq!(first.ladder, vec![60, 300, 3600]);
    assert_eq!(
        first.cross_tf_state.configured_timeframes,
        vec![60, 300, 3600]
    );
}

#[test]
fn bias_state_uses_the_inclusive_neutral_corridor() {
    assert_eq!(bias_state(None), BiasState::Undefined);
    assert_eq!(bias_state(Some(0.1)), BiasState::Neutral);
    assert_eq!(bias_state(Some(-0.1)), BiasState::Neutral);
    assert_eq!(bias_state(Some(0.0)), BiasState::Neutral);
    assert_eq!(bias_state(Some(0.100_001)), BiasState::Bullish);
    assert_eq!(bias_state(Some(-0.100_001)), BiasState::Bearish);
}

#[test]
fn a_zero_value_is_not_absence() {
    let signed = signed_position(
        Some(0.0),
        Domain::DirectionalBias,
        Some(UnavailableReason::InsufficientBars),
    );
    assert_eq!(signed, AxisValue::signed(0.0, Domain::DirectionalBias));
    assert!(!signed.is_undefined());

    let rank = rank_size(Some(0.0));
    assert_eq!(rank, AxisValue::rank(0.0));
    assert!(!rank.is_undefined());
}

#[test]
fn momentum_position_is_absent_when_only_force_is_available() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);
    let bar = (0..timeframe.len())
        .find(|&bar| {
            let model_bar = timeframe.bar(bar).expect("a bar");
            model_bar.momentum.force.is_some() && model_bar.momentum.acceleration.is_none()
        })
        .expect("the fixture has a force without its three-bar change");

    let state = TimeframeState::at(&timeframe, bar).expect("a state");
    let momentum = axis(&state, AxisCode::Momentum);
    assert!(momentum.position.is_undefined());
    let AxisDetail::Momentum(detail) = &momentum.detail else {
        panic!("the Momentum axis carries Momentum detail");
    };
    assert!(detail.force.is_some());
    assert_eq!(momentum.readiness_code, Readiness::Provisional.code());
}

#[test]
fn progression_absence_carries_the_origin_reason() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);
    let state = TimeframeState::at(&timeframe, 0).expect("the first bar");
    let progression = axis(&state, AxisCode::Progression);

    assert_eq!(
        progression.position,
        AxisValue::undefined(UnavailableReason::OriginUnconfirmed)
    );
}

#[test]
fn confirmed_level_count_is_bounded_by_availability() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);
    let bar = 199;
    let state = TimeframeState::at(&timeframe, bar).expect("a state");
    let structure = axis(&state, AxisCode::Structure);
    let AxisDetail::Structure(detail) = &structure.detail else {
        panic!("the Structure axis carries Structure detail");
    };
    let expected = timeframe
        .levels()
        .iter()
        .filter(|level| level.available_at <= bar)
        .count();

    assert_eq!(detail.confirmed_level_count, expected);
}

#[test]
fn the_cross_timeframe_envelope_uses_the_configured_thresholds() {
    let candles = fixture_candles();
    let run = run(&candles, &[60, 300]);
    let config = DashboardConfig::new(9, 3);
    let view = DashboardState::from_run(&run, &[3600, 300, 60], &config);

    assert_eq!(
        view.cross_tf_state.configured_timeframes,
        vec![60, 300, 3600]
    );
    assert_eq!(view.cross_tf_state.present_timeframes, vec![60, 300]);
    assert_eq!(view.cross_tf_state.aggregates.ladder, vec![60, 300]);
    assert_eq!(
        view.cross_tf_state.agreement.coverage,
        Coverage {
            present: 2,
            configured: 3
        }
    );
    assert_eq!(view.cross_tf_state.agreement.of, 3);
    assert_eq!(view.cross_tf_state.agreement.min_valid_timeframes, 3);
    assert_eq!(view.cross_tf_state.churn.threshold, 9);
    assert_eq!(view.cross_tf_state.churn.window, 50);
    assert_eq!(view.cross_tf_state.churn.churn_class, ChurnClass::Building);
    assert_eq!(view.cross_tf_state.progression_leg_direction.len(), 3);
}

#[test]
fn a_missing_row_carries_the_configured_structure_scale() {
    let candles = fixture_candles();
    let config = MarketStateConfig::new(PivotScale::Minor);
    let run = ModelRun::compute(
        &[TimeframeInput {
            timeframe_secs: 60,
            candles: &candles,
        }],
        &config,
    );
    let view = DashboardState::from_run(&run, &[60, 300], &DashboardConfig::default());
    let structure = axis(&view.timeframes[1], AxisCode::Structure);
    let AxisDetail::Structure(detail) = &structure.detail else {
        panic!("the Structure axis carries Structure detail");
    };

    assert_eq!(
        detail.scale_code,
        PivotScale::Minor.code(),
        "the absent row carries the scale the present run used"
    );
    assert_eq!(absent_structure_scale_code(&run), PivotScale::Minor.code());
}
