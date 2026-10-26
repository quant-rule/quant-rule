use super::direction::direction_tenure;
use super::structure::structure_activity;
use super::{
    AxisDetail, AxisReadiness, AxisState, AxisValue, ChurnClass, Coverage, DashboardConfig,
    DashboardState, Domain, Readiness, TimeframeState, UnavailableReason,
    absent_structure_scale_code, rank_size, signed_position,
};
use crate::dashboard::Stage;
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
    let tenure = direction_tenure(timeframe.direction().bars(), latest);
    match bar.direction.direction {
        Some(value) => assert_eq!(
            direction.position,
            AxisValue::signed(value, Domain::DirectionalBias)
        ),
        None => assert!(direction.position.is_undefined()),
    }
    match tenure.tenure_percentile {
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
    assert_eq!(direction_detail.bias_state, tenure.bias_state);
    assert_eq!(direction_detail.bias_continuity, tenure.bias_continuity);
    assert_eq!(direction_detail.bias_tenure_bars, tenure.tenure_bars);
    assert_eq!(direction_detail.tenure_origin_kind, tenure.origin_kind);
    assert_eq!(direction_detail.tenure_origin_bar, tenure.origin_bar);
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
    let activity = structure_activity(&timeframe, latest);
    assert_eq!(structure_detail.activity_count, activity.count);
    assert_eq!(structure_detail.activity_window, activity.window);
    assert_eq!(structure_detail.activity_per_bar, activity.per_bar);
    assert!(
        !structure_detail.level_references.is_empty(),
        "the latest fixture bar has current reference prices"
    );

    let progression = axis(&state, AxisCode::Progression);
    match bar.progression.stage_code.and_then(Stage::from_code) {
        Some(stage) => assert_eq!(progression.position, AxisValue::ordinal(stage)),
        None => assert_eq!(
            progression.position,
            AxisValue::undefined(UnavailableReason::OriginUnconfirmed)
        ),
    }
    // The radius is the recent-pace percentile, not the model's displacement
    // percentile. The latest bar's published leg is still forming, so pace is
    // absent and the displacement percentile stays a detail.
    assert_eq!(
        progression.size,
        AxisValue::undefined(UnavailableReason::OriginUnconfirmed)
    );
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
        progression_detail.displacement_pct, bar.progression.size_percentile,
        "the displacement percentile remains a separate detail"
    );
    assert_eq!(progression_detail.recent_pace_signed_atr_per_bar, None);
    assert_eq!(progression_detail.recent_pace_direction, None);
    assert_eq!(progression_detail.lifecycle_code, Some(0));
    assert_eq!(progression_detail.origin_bar, Some(latest));
    assert_eq!(progression_detail.origin_available_bar, None);
    assert_eq!(
        progression_detail.average_net_displacement_atr_per_bar,
        Some(0.336_548_496_290_132_84)
    );
    assert!(!progression_detail.recent_pace_frozen_terminal);
    assert_eq!(
        progression_detail.recent_pace_window_bars,
        super::RECENT_PACE_WINDOW_BARS
    );
}

#[test]
fn a_confirmed_published_leg_gets_pace_absence_not_the_origin_reason() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);
    let bar_index = 57;
    let state = TimeframeState::at(&timeframe, bar_index).expect("a state");
    let bar = timeframe.bar(bar_index).expect("a bar");
    let leg = bar
        .progression
        .published_leg()
        .expect("bar 57 publishes a leg");
    assert_eq!(leg.origin_confirmed_code, 1);

    let progression = axis(&state, AxisCode::Progression);
    assert_eq!(
        progression.position,
        AxisValue::ordinal(Stage::Late),
        "the confirmed leg publishes stage 4"
    );
    assert_eq!(
        progression.size,
        AxisValue::undefined(UnavailableReason::InsufficientBars),
        "a confirmed leg without 50 measured paces has insufficient history"
    );
    assert_eq!(progression.readiness_code, bar.progression.readiness.code());

    let AxisDetail::Progression(detail) = &progression.detail else {
        panic!("the Progression axis carries Progression detail");
    };
    assert_eq!(detail.lifecycle_code, Some(1));
    assert_eq!(detail.origin_bar, Some(55));
    assert_eq!(detail.origin_available_bar, Some(57));
    assert_eq!(detail.displacement_pct, bar.progression.size_percentile);
    assert_eq!(detail.recent_pace_signed_atr_per_bar, None);
    assert_eq!(detail.recent_pace_direction, None);
    assert_eq!(
        detail.average_net_displacement_atr_per_bar,
        Some(0.766_693_161_214_573_9 / 2.0)
    );
    assert!(!detail.recent_pace_frozen_terminal);
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
fn direction_size_is_the_tenure_percentile_and_magnitude_is_detail() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);

    let bar = (0..timeframe.len())
        .find(|&bar| {
            let model_bar = timeframe.bar(bar).expect("a bar");
            let tenure = direction_tenure(timeframe.direction().bars(), bar);
            tenure.tenure_percentile.is_some()
                && model_bar.direction.size_percentile.is_some()
                && tenure.tenure_percentile != model_bar.direction.size_percentile
        })
        .expect("the fixture has a bar where tenure and magnitude percentiles differ");

    let state = TimeframeState::at(&timeframe, bar).expect("a state");
    let model_bar = timeframe.bar(bar).expect("a bar");
    let tenure = direction_tenure(timeframe.direction().bars(), bar);
    let direction = axis(&state, AxisCode::Direction);

    assert_eq!(
        direction.size,
        AxisValue::rank(tenure.tenure_percentile.expect("a tenure percentile"))
    );
    let AxisDetail::Direction(detail) = &direction.detail else {
        panic!("the Direction axis carries Direction detail");
    };
    assert_eq!(
        detail.magnitude_pct, model_bar.direction.size_percentile,
        "the old magnitude percentile remains visible as detail"
    );
    assert_ne!(
        direction.size.as_rank(),
        detail.magnitude_pct,
        "the radius is not the magnitude"
    );
}

#[test]
fn missing_unavailable_direction_carries_the_direction_reason() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);

    let bar = (0..timeframe.len())
        .find(|&bar| {
            timeframe
                .bar(bar)
                .expect("a bar")
                .direction
                .direction
                .is_none()
        })
        .expect("the fixture has an unavailable Direction bar");
    let state = TimeframeState::at(&timeframe, bar).expect("a state");
    let model_bar = timeframe.bar(bar).expect("a bar");
    let direction = axis(&state, AxisCode::Direction);
    let AxisValue::Undefined { reason } = direction.size else {
        panic!("an unavailable Direction has no tenure size");
    };
    assert_eq!(
        Some(reason),
        model_bar.direction.unavailable_reason,
        "the view-only size keeps the Direction reason"
    );
}

#[test]
fn structure_activity_is_isolated_from_position_and_size() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);

    let bar = (0..timeframe.len())
        .find(|&bar| structure_activity(&timeframe, bar).count > 0)
        .expect("the fixture has a Structure-active bar");
    let model_bar = timeframe.bar(bar).expect("a bar");
    let state = TimeframeState::at(&timeframe, bar).expect("a state");
    let structure = axis(&state, AxisCode::Structure);

    match model_bar.structure.score {
        Some(value) => assert_eq!(
            structure.position,
            AxisValue::signed(value, Domain::StructureRelation)
        ),
        None => assert!(structure.position.is_undefined()),
    }
    match model_bar.structure.size_percentile {
        Some(pct) => assert_eq!(structure.size, AxisValue::rank(pct)),
        None => assert!(structure.size.is_undefined()),
    }
    let AxisDetail::Structure(detail) = &structure.detail else {
        panic!("the Structure axis carries Structure detail");
    };
    assert!(detail.activity_count > 0);
    assert_eq!(
        detail.activity_count,
        structure_activity(&timeframe, bar).count
    );
}

#[test]
fn a_zero_activity_count_is_a_measured_zero() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);
    let state = TimeframeState::at(&timeframe, 0).expect("the first bar");
    let structure = axis(&state, AxisCode::Structure);
    let AxisDetail::Structure(detail) = &structure.detail else {
        panic!("the Structure axis carries Structure detail");
    };

    assert_eq!(detail.activity_count, 0);
    assert_eq!(detail.activity_window, 1);
    assert_eq!(detail.activity_per_bar, Some(0.0));
}

#[test]
fn structure_evidence_is_copied_from_the_published_model() {
    let candles = fixture_candles();
    let timeframe = model(&candles, 60);
    let latest = timeframe.len() - 1;
    let model_bar = timeframe.bar(latest).expect("the latest bar");
    let state = TimeframeState::at(&timeframe, latest).expect("a state");
    let structure = axis(&state, AxisCode::Structure);
    let AxisDetail::Structure(detail) = &structure.detail else {
        panic!("the Structure axis carries Structure detail");
    };

    assert_eq!(detail.score, model_bar.structure.score);
    assert_eq!(
        detail.high_relation_code,
        model_bar.structure.high_relation_code
    );
    assert_eq!(
        detail.low_relation_code,
        model_bar.structure.low_relation_code
    );
    assert_eq!(detail.scale_code, model_bar.structure.scale_code);
    assert_eq!(
        detail.ema_distance_atr_20,
        model_bar.structure.ema_distance_atr_20
    );
    assert_eq!(
        detail.ema_distance_atr_50,
        model_bar.structure.ema_distance_atr_50
    );
    assert_eq!(
        detail.ema_distance_atr_200,
        model_bar.structure.ema_distance_atr_200
    );
    assert_eq!(
        detail.ema_cross_code_20,
        model_bar.structure.ema_cross_code_20
    );
    assert_eq!(
        detail.ema_cross_code_50,
        model_bar.structure.ema_cross_code_50
    );
    assert_eq!(
        detail.ema_cross_code_200,
        model_bar.structure.ema_cross_code_200
    );
    assert_eq!(
        detail.ema_retest_code_20,
        model_bar.structure.ema_retest_code_20
    );
    assert_eq!(
        detail.ema_retest_code_50,
        model_bar.structure.ema_retest_code_50
    );
    assert_eq!(
        detail.ema_retest_code_200,
        model_bar.structure.ema_retest_code_200
    );

    let expected = timeframe
        .levels()
        .iter()
        .filter(|level| level.available_at <= latest)
        .count();
    assert_eq!(detail.confirmed_level_count, expected);
    assert_eq!(
        detail.level_references.len(),
        detail
            .level_references
            .iter()
            .map(|reference| reference.reference_kind_code)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        "each reference kind appears at most once"
    );
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
