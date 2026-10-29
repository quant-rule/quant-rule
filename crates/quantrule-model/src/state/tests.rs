use super::{MarketState, MarketStateConfig, TimeframeConsensus, TimeframeInput};
use crate::aggregate::ReferenceSet;
use crate::axes::direction::{DirectionBar, DirectionSeries};
use crate::axes::momentum::{MomentumBar, MomentumSeries};
use crate::axes::progression::{ProgressionBar, ProgressionConfig, ProgressionSeries};
use crate::axes::structure::{StructureBar, StructureSeries};
use crate::context::{ContextBar, ContextSeries};
use crate::events::divergence::DivergenceEventSeries;
use crate::events::level::LevelEventSeries;
use crate::events::signal::SignalEventSeries;
use crate::kernel::{Readiness, UnavailableReason};
use crate::readings::Readings;
use quantrule_data::{Candle, CandleValues, Timeframe, TimeframeSet};

const READINGS_FIXTURE: &str = include_str!("../../tests/fixtures/readings_golden.csv");
const READINGS_HEADER: &str = "bar,timestamp,open,high,low,close,volume,ema20,ema50,ema200,rsi14,atr14,rvol20,minor_candidate_high,minor_candidate_low,minor_pivot_high,minor_pivot_low,minor_pivot_occurred_at,major_candidate_high,major_candidate_low,major_pivot_high,major_pivot_low,major_pivot_occurred_at";
const CONSENSUS_FIXTURE: &str = include_str!("../../tests/fixtures/consensus_golden.csv");
const CONSENSUS_HEADER: &str = "timeframe_secs,direction,momentum_force,structure_score,direction_percentile,momentum_percentile,structure_percentile,bullish_axis_count,bearish_axis_count,neutral_axis_count,weakest_axis_code";

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
                .expect("valid candle"),
        );
    }
    candles
}

fn snapshot(
    timeframe_secs: u64,
    direction: Option<f64>,
    momentum: Option<f64>,
    structure: Option<f64>,
    direction_percentile: Option<f64>,
    momentum_percentile: Option<f64>,
    structure_percentile: Option<f64>,
) -> crate::aggregate::TimeframeSnapshot {
    let state_code = |value: Option<f64>| {
        value.map(|value| {
            if value > 0.1 {
                1
            } else if value < -0.1 {
                -1
            } else {
                0
            }
        })
    };
    crate::aggregate::TimeframeSnapshot {
        timeframe_secs,
        latest_bar_index: 10,
        direction: DirectionBar {
            direction,
            state_code: state_code(direction),
            size_percentile: direction_percentile,
            readiness: if direction.is_some() {
                Readiness::Ready
            } else {
                Readiness::Unavailable
            },
            unavailable_reason: direction
                .is_none()
                .then_some(UnavailableReason::InsufficientBars),
            state_age_bars: 0,
            state_changes_50: 0,
        },
        momentum: MomentumBar {
            force: momentum,
            acceleration: None,
            intensification: None,
            state_code: state_code(momentum),
            size_percentile: momentum_percentile,
            readiness: if momentum.is_some() {
                Readiness::Ready
            } else {
                Readiness::Unavailable
            },
            unavailable_reason: momentum
                .is_none()
                .then_some(UnavailableReason::InsufficientBars),
            state_age_bars: 0,
            state_changes_50: 0,
        },
        structure: StructureBar {
            high_relation_code: None,
            low_relation_code: None,
            score: structure,
            state_code: state_code(structure),
            scale_code: 2,
            ema_distance_atr_20: None,
            ema_distance_atr_50: None,
            ema_distance_atr_200: None,
            ema_cross_code_20: None,
            ema_cross_code_50: None,
            ema_cross_code_200: None,
            ema_retest_code_20: None,
            ema_retest_code_50: None,
            ema_retest_code_200: None,
            size_percentile: structure_percentile,
            readiness: if structure.is_some() {
                Readiness::Ready
            } else {
                Readiness::Unavailable
            },
            unavailable_reason: structure
                .is_none()
                .then_some(UnavailableReason::InsufficientBars),
            state_age_bars: 0,
            state_changes_50: 0,
        },
        progression: ProgressionBar {
            progress: None,
            stage_code: None,
            leg_direction: None,
            leg_scale_code: None,
            published_leg: None,
            minor_leg: None,
            major_leg: None,
            size_percentile: None,
            readiness: Readiness::Unavailable,
            unavailable_reason: Some(UnavailableReason::InsufficientBars),
            state_age_bars: 0,
            state_changes_50: 0,
        },
        context: ContextBar {
            atr_ratio: None,
            rvol_ratio: None,
            context_code: None,
        },
        atr: None,
        references: ReferenceSet::new(Vec::new()),
        events: Vec::new(),
        minor_origin_time_secs: None,
        major_origin_time_secs: None,
    }
}

#[test]
fn the_market_state_assembles_the_latest_readings_and_events() {
    let candles = load_candles();
    let state = MarketState::compute(
        &[TimeframeInput {
            timeframe_secs: 60,
            candles: &candles,
        }],
        &MarketStateConfig::default(),
    );

    assert_eq!(state.timeframes.len(), 1);
    assert_eq!(state.aggregates.ladder, [60]);
    assert_eq!(state.consensus.len(), 1);
    let snapshot = &state.timeframes[0].snapshot;
    let latest = candles.len() - 1;
    assert_eq!(snapshot.latest_bar_index, latest);

    let readings = Readings::compute(&candles);
    assert_eq!(
        snapshot.direction,
        DirectionSeries::compute(&readings)
            .latest()
            .expect("direction")
    );
    assert_eq!(
        snapshot.momentum,
        MomentumSeries::compute(&readings)
            .latest()
            .expect("momentum")
    );
    assert_eq!(
        snapshot.structure,
        StructureSeries::compute(&candles, &readings)
            .latest()
            .expect("structure")
    );
    assert_eq!(
        snapshot.progression,
        ProgressionSeries::compute_with_config(&candles, &readings, ProgressionConfig::new(60))
            .latest()
            .expect("progression")
    );
    assert_eq!(
        snapshot.context,
        ContextSeries::compute(&readings).latest().expect("context")
    );
    assert_eq!(snapshot.atr, readings.atr14.get(latest));
    assert!(snapshot.references.len() <= 7);
    assert!(
        state.timeframes[0]
            .levels
            .iter()
            .all(|level| level.available_at <= latest)
    );

    let level = LevelEventSeries::compute(
        &candles,
        &readings,
        &crate::events::level::LevelEventConfig::from_secs(60).expect("timeframe"),
    );
    let divergence = DivergenceEventSeries::compute(
        &readings,
        &crate::events::divergence::DivergenceConfig::from_secs(
            60,
            crate::readings::PivotScale::Major,
        )
        .expect("timeframe"),
    );
    let signal = SignalEventSeries::compute(
        &readings,
        &DirectionSeries::compute(&readings),
        &MomentumSeries::compute(&readings),
        &StructureSeries::compute(&candles, &readings),
        &crate::events::signal::SignalConfig::from_secs(60).expect("timeframe"),
    );
    assert_eq!(
        snapshot.events.len(),
        level.len() + divergence.len() + signal.len()
    );
    assert!(
        snapshot
            .events
            .windows(2)
            .all(|pair| pair[0].available_bar_index <= pair[1].available_bar_index)
    );
}

#[test]
fn the_market_state_builds_the_four_timeframe_ladder() {
    let candles = load_candles();
    let mut set = TimeframeSet::new();
    let base = 1_700_000_000_i64.div_euclid(60) * 60;
    for (index, candle) in candles.iter().enumerate() {
        set.push_values(CandleValues {
            timestamp: base + index as i64 * 60,
            open: candle.open(),
            high: candle.high(),
            low: candle.low(),
            close: candle.close(),
            volume: candle.volume(),
        })
        .expect("the fixture candle is accepted");
    }
    set.finish().expect("the set finishes");

    let m1: Vec<Candle> = set.series(Timeframe::M1).iter().copied().collect();
    let m5: Vec<Candle> = set.series(Timeframe::M5).iter().copied().collect();
    let m15: Vec<Candle> = set.series(Timeframe::M15).iter().copied().collect();
    let h1: Vec<Candle> = set.series(Timeframe::H1).iter().copied().collect();

    let state = MarketState::compute(
        &[
            TimeframeInput {
                timeframe_secs: 60,
                candles: &m1,
            },
            TimeframeInput {
                timeframe_secs: 300,
                candles: &m5,
            },
            TimeframeInput {
                timeframe_secs: 900,
                candles: &m15,
            },
            TimeframeInput {
                timeframe_secs: 3600,
                candles: &h1,
            },
        ],
        &MarketStateConfig::default(),
    );

    assert_eq!(state.timeframes.len(), 4);
    assert_eq!(state.aggregates.ladder, [60, 300, 900, 3600]);
    assert_eq!(state.consensus.len(), 4);
    assert_eq!(state.aggregates.progression.progression_timeframe_count, 4);
    for (state, series_len) in
        state
            .timeframes
            .iter()
            .zip([m1.len(), m5.len(), m15.len(), h1.len()])
    {
        assert_eq!(state.snapshot.latest_bar_index, series_len - 1);
    }
}

#[test]
fn consensus_counts_only_present_axes_and_uses_the_lowest_percentile() {
    let all = TimeframeConsensus::compute(&snapshot(
        60,
        Some(0.5),
        Some(-0.5),
        Some(0.0),
        Some(10.0),
        Some(20.0),
        Some(30.0),
    ));
    assert_eq!(
        (
            all.bullish_axis_count,
            all.bearish_axis_count,
            all.neutral_axis_count
        ),
        (1, 1, 1)
    );
    assert_eq!(all.weakest_directional_axis_code, Some(1));

    let missing = TimeframeConsensus::compute(&snapshot(
        300,
        None,
        Some(0.5),
        None,
        None,
        Some(40.0),
        None,
    ));
    assert_eq!(
        (
            missing.bullish_axis_count,
            missing.bearish_axis_count,
            missing.neutral_axis_count
        ),
        (1, 0, 0)
    );
    assert_eq!(missing.weakest_directional_axis_code, Some(2));

    let boundary = TimeframeConsensus::compute(&snapshot(
        900,
        Some(0.1),
        Some(-0.1),
        None,
        Some(50.0),
        Some(60.0),
        None,
    ));
    assert_eq!(
        (
            boundary.bullish_axis_count,
            boundary.bearish_axis_count,
            boundary.neutral_axis_count
        ),
        (0, 0, 2)
    );
    assert_eq!(boundary.weakest_directional_axis_code, Some(1));
}

#[test]
fn the_snapshot_consensus_delegates_to_from_bars() {
    let snapshot = snapshot(
        60,
        Some(0.5),
        Some(-0.5),
        Some(0.0),
        Some(10.0),
        Some(20.0),
        Some(30.0),
    );
    let direct = TimeframeConsensus::from_bars(
        60,
        &snapshot.direction,
        &snapshot.momentum,
        &snapshot.structure,
    );
    assert_eq!(direct, TimeframeConsensus::compute(&snapshot));
}

#[test]
fn the_consensus_matches_the_golden_fixture() {
    let mut lines = CONSENSUS_FIXTURE.lines();
    assert_eq!(lines.next(), Some(CONSENSUS_HEADER));
    let mut rows = 0;
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 11);
        let timeframe_secs: u64 = fields[0].parse().expect("timeframe");
        let direction = number(fields[1]);
        let momentum = number(fields[2]);
        let structure = number(fields[3]);
        let direction_percentile = number(fields[4]);
        let momentum_percentile = number(fields[5]);
        let structure_percentile = number(fields[6]);
        let expected_bullish: u8 = fields[7].parse().expect("bullish");
        let expected_bearish: u8 = fields[8].parse().expect("bearish");
        let expected_neutral: u8 = fields[9].parse().expect("neutral");
        let expected_weakest: Option<u8> = if fields[10].is_empty() {
            None
        } else {
            Some(fields[10].parse().expect("weakest"))
        };

        let consensus = TimeframeConsensus::compute(&snapshot(
            timeframe_secs,
            direction,
            momentum,
            structure,
            direction_percentile,
            momentum_percentile,
            structure_percentile,
        ));
        assert_eq!(consensus.bullish_axis_count, expected_bullish);
        assert_eq!(consensus.bearish_axis_count, expected_bearish);
        assert_eq!(consensus.neutral_axis_count, expected_neutral);
        assert_eq!(consensus.weakest_directional_axis_code, expected_weakest);
        rows += 1;
    }
    assert_eq!(rows, 3);
}

#[test]
fn an_empty_state_is_empty() {
    let empty = MarketState::compute(&[], &MarketStateConfig::default());
    assert!(empty.timeframes.is_empty());
    assert!(empty.aggregates.ladder.is_empty());
    assert!(empty.consensus.is_empty());

    let input = TimeframeInput {
        timeframe_secs: 60,
        candles: &[],
    };
    let empty = MarketState::compute(&[input], &MarketStateConfig::default());
    assert!(empty.timeframes.is_empty());
}

#[test]
fn the_state_is_deterministic() {
    let candles = load_candles();
    let input = TimeframeInput {
        timeframe_secs: 60,
        candles: &candles,
    };
    let first = MarketState::compute(&[input], &MarketStateConfig::default());
    let second = MarketState::compute(&[input], &MarketStateConfig::default());
    assert_eq!(first, second);
    assert_eq!(first.timeframe(60), second.timeframe(60));
    assert_eq!(first.timeframe(61), None);
}
