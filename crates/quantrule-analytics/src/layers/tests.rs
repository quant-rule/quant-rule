use quantrule_data::Candle;
use quantrule_model::axes::structure::registry::ConfirmedLevelRegistry;
use quantrule_model::readings::{PivotKind, PivotScale, PivotSeries};
use quantrule_model::state::{MarketState, MarketStateConfig, TimeframeInput};

use super::{
    AGGREGATE_COLUMNS, HIGHER_BLOCK_COLUMNS, LAYER1_COLUMNS, LAYER2_COLUMNS, Layer2Row,
    LayerConfig, LayerError, Metrics, build_layers, layer3_columns,
};
use crate::resolution::clamp_unit;

const M1: u64 = 60;
const M5: u64 = 300;
const H1: u64 = 3_600;

fn input(timeframe_secs: u64, candles: &[Candle]) -> TimeframeInput<'_> {
    TimeframeInput {
        timeframe_secs,
        candles,
    }
}

fn candles(count: usize, start: i64, step: i64, offset: f64) -> Vec<Candle> {
    (0..count)
        .map(|index| {
            let price = 100.0 + offset + (index % 37) as f64 * 0.1 + (index % 7) as f64 * 0.05;
            Candle::new(
                index as u64,
                start + index as i64 * step,
                price,
                price + 0.2,
                price - 0.2,
                price + 0.01,
                Some(1.0 + (index % 5) as f64),
            )
            .expect("the test candle is valid")
        })
        .collect()
}

/// A series with confirmed pivots at both scales, one unconfirmed candidate,
/// and a confirmed-level registry that includes a same-bar scale tie.
fn pivot_candles() -> Vec<Candle> {
    let highs = [
        10.0, 11.0, 12.0, 11.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 10.0, 13.0, 12.0, 11.0, 10.0,
        9.0, 8.0, 7.0, 8.0, 9.0, 10.0, 10.0, 10.0, 10.0,
    ];
    let lows = [
        9.0, 9.5, 10.0, 9.5, 9.0, 9.0, 9.0, 8.0, 7.0, 8.0, 9.0, 9.0, 9.0, 9.0, 9.0, 8.0, 7.0, 6.0,
        7.0, 8.0, 9.0, 9.0, 9.0, 9.0,
    ];
    highs
        .iter()
        .zip(lows)
        .enumerate()
        .map(|(index, (high, low))| {
            Candle::new(
                index as u64,
                index as i64 * 60,
                (high + low) / 2.0,
                *high,
                low,
                (high + low) / 2.0,
                Some(1.0),
            )
            .expect("the test candle is valid")
        })
        .collect()
}

fn kind_code(kind: PivotKind) -> u8 {
    match kind {
        PivotKind::High => 1,
        PivotKind::Low => 2,
    }
}

/// M1 bars from 08:49 to 09:18, M5 bars from 08:50 to 09:15, H1 bars at 08:00
/// and 09:00. The M1 bars at index 14 and 15 are 09:03 and 09:04.
fn alignment_fixture() -> (Vec<Candle>, Vec<Candle>, Vec<Candle>) {
    (
        candles(30, 31_740, 60, 0.0),
        candles(6, 31_800, 300, 1.0),
        candles(2, 28_800, 3_600, 2.0),
    )
}

#[test]
fn the_contract_column_widths_are_fixed() {
    assert_eq!(LAYER1_COLUMNS, 61);
    assert_eq!(LAYER2_COLUMNS, 73);
    assert_eq!(HIGHER_BLOCK_COLUMNS, 67);
    assert_eq!(AGGREGATE_COLUMNS, 82);
    assert_eq!(layer3_columns(0), 155);
    assert_eq!(layer3_columns(3), 356);
}

#[test]
fn every_bar_is_one_layer_one_and_layer_two_row() {
    let (m1, m5, h1) = alignment_fixture();
    let set = build_layers(
        &[input(M1, &m1), input(M5, &m5), input(H1, &h1)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");

    assert_eq!(set.timeframes, vec![M1, M5, H1]);
    assert_eq!(set.layer1.len(), m1.len() + m5.len() + h1.len());
    assert_eq!(set.layer2.len(), set.layer1.len());
    assert_eq!(set.layer3.len(), m1.len());

    assert_eq!(
        set.layer1
            .iter()
            .map(|row| row.timeframe_secs)
            .collect::<Vec<_>>(),
        [vec![M1; m1.len()], vec![M5; m5.len()], vec![H1; h1.len()]].concat()
    );
    for row in &set.layer1 {
        let series = match row.timeframe_secs {
            M1 => &m1,
            M5 => &m5,
            _ => &h1,
        };
        assert_eq!(row.candle, series[row.bar_index]);
    }
}

#[test]
fn layer_two_derives_from_layer_one() {
    let (m1, m5, h1) = alignment_fixture();
    let set = build_layers(
        &[input(M1, &m1), input(M5, &m5), input(H1, &h1)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");

    for (layer1, layer2) in set.layer1.iter().zip(&set.layer2) {
        assert_eq!(&layer2.layer1, layer1);
        assert_eq!(Layer2Row::from_layer1(*layer1), *layer2);
        let metrics = &layer1.metrics;
        assert_eq!(
            layer2.resolution.direction.continuous,
            metrics.direction.direction
        );
        assert_eq!(
            layer2.resolution.momentum.continuous,
            metrics.momentum.force
        );
        assert_eq!(
            layer2.resolution.structure.continuous,
            metrics.structure.score
        );
        assert_eq!(
            layer2.resolution.progression.continuous,
            metrics.progression.progress.map(clamp_unit)
        );
        for (resolution, value, reason) in [
            (
                layer2.resolution.direction,
                metrics.direction.direction,
                metrics.direction.unavailable_reason_code,
            ),
            (
                layer2.resolution.momentum,
                metrics.momentum.force,
                metrics.momentum.unavailable_reason_code,
            ),
            (
                layer2.resolution.structure,
                metrics.structure.score,
                metrics.structure.unavailable_reason_code,
            ),
            (
                layer2.resolution.progression,
                metrics.progression.progress.map(clamp_unit),
                metrics.progression.unavailable_reason_code,
            ),
        ] {
            assert_eq!(resolution.ordinal.is_some(), value.is_some());
            assert_eq!(
                resolution.reason,
                if value.is_none() { reason } else { None }
            );
        }
    }
}

#[test]
fn layer_two_constructor_keeps_the_contract_boundaries_and_clamp() {
    let (m1, m5, h1) = alignment_fixture();
    let set = build_layers(
        &[input(M1, &m1), input(M5, &m5), input(H1, &h1)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");
    let base = *set.layer1.first().expect("the fixture has a row");

    for (value, ordinal) in [(-1.0, 0), (-0.8, 1), (0.0, 5), (0.8, 9), (1.0, 9)] {
        let mut layer1 = base;
        layer1.metrics.direction.direction = Some(value);
        layer1.metrics.direction.unavailable_reason_code = None;
        let layer2 = Layer2Row::from_layer1(layer1);
        assert_eq!(layer2.resolution.direction.continuous, Some(value));
        assert_eq!(layer2.resolution.direction.ordinal, Some(ordinal));
        assert_eq!(layer2.resolution.direction.reason, None);
    }

    for (raw, clamped, ordinal) in [(1.5, 1.0, 9), (-0.25, -0.25, 3)] {
        let mut layer1 = base;
        layer1.metrics.progression.progress = Some(raw);
        layer1.metrics.progression.unavailable_reason_code = None;
        let layer2 = Layer2Row::from_layer1(layer1);
        assert_eq!(layer2.layer1.metrics.progression.progress, Some(raw));
        assert_eq!(layer2.resolution.progression.continuous, Some(clamped));
        assert_eq!(layer2.resolution.progression.ordinal, Some(ordinal));
        assert_eq!(layer2.resolution.progression.reason, None);
    }
}

#[test]
fn layer_three_uses_the_latest_closed_higher_bar() {
    let (m1, m5, h1) = alignment_fixture();
    let set = build_layers(
        &[input(M1, &m1), input(M5, &m5), input(H1, &h1)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");

    // 09:03 closes at 09:04; the 09:00-09:05 M5 bar is open.
    let three = &set.layer3[14];
    let m5_block = three
        .higher
        .iter()
        .find(|block| block.timeframe_secs == M5)
        .expect("the M5 block");
    assert_eq!(m5_block.bar_index, 1);
    assert_eq!(m5_block.candle.timestamp(), 32_100);
    assert_eq!(m5_block.staleness_secs, 240);

    // 09:04 closes at 09:05, when the 09:00-09:05 M5 bar closes.
    let four = &set.layer3[15];
    let m5_block = four
        .higher
        .iter()
        .find(|block| block.timeframe_secs == M5)
        .expect("the M5 block");
    assert_eq!(m5_block.bar_index, 2);
    assert_eq!(m5_block.candle.timestamp(), 32_400);
    assert_eq!(m5_block.staleness_secs, 0);

    // The H1 bar at 08:00 closes at 09:00, so both rows use it.
    let h1_block = four
        .higher
        .iter()
        .find(|block| block.timeframe_secs == H1)
        .expect("the H1 block");
    assert_eq!(h1_block.bar_index, 0);
    assert_eq!(h1_block.staleness_secs, 300);
}

#[test]
fn a_block_is_absent_until_the_higher_bar_closes() {
    let (m1, m5, h1) = alignment_fixture();
    let set = build_layers(
        &[input(M1, &m1), input(M5, &m5), input(H1, &h1)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");

    // 08:49: neither the first M5 close (08:55) nor the first H1 close (09:00)
    // has happened yet.
    assert!(set.layer3[0].higher.is_empty());
    assert_eq!(set.layer3[0].aggregates.ladder, vec![M1]);

    let late = &set.layer3[14];
    assert_eq!(late.aggregates.ladder, vec![M1, M5, H1]);
    assert_eq!(late.higher.len(), 2);
}

#[test]
fn every_block_is_the_stored_bar_and_closes_before_the_base() {
    let (m1, m5, h1) = alignment_fixture();
    let set = build_layers(
        &[input(M1, &m1), input(M5, &m5), input(H1, &h1)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");

    for row in &set.layer3 {
        assert_eq!(&row.base, layer2_row(&set, M1, row.base.layer1.bar_index));
        let base_close = row.base.layer1.candle.timestamp() + M1 as i64;
        for block in &row.higher {
            let layer1 = set
                .layer1
                .iter()
                .find(|row| {
                    row.timeframe_secs == block.timeframe_secs && row.bar_index == block.bar_index
                })
                .expect("the block's Layer 1 row");
            let layer2 = layer2_row(&set, block.timeframe_secs, block.bar_index);
            assert_eq!(block.candle, layer1.candle);
            assert_eq!(block.metrics, layer1.metrics);
            assert_eq!(block.resolution, layer2.resolution);
            let block_close = block.candle.timestamp() + block.timeframe_secs as i64;
            assert!(block_close <= base_close, "a block closes after the base");
            assert_eq!(block.staleness_secs, (base_close - block_close) as u64);
        }
    }
}

#[test]
fn layer_three_is_causal() {
    let (m1, m5, h1) = alignment_fixture();
    let full = build_layers(
        &[input(M1, &m1), input(M5, &m5), input(H1, &h1)],
        &LayerConfig::default(),
    )
    .expect("the full fixture builds");
    let prefix = build_layers(
        &[input(M1, &m1[..20]), input(M5, &m5), input(H1, &h1)],
        &LayerConfig::default(),
    )
    .expect("the prefix fixture builds");

    assert_eq!(prefix.layer3.len(), 20);
    for (earlier, later) in prefix.layer3.iter().zip(&full.layer3) {
        assert_eq!(earlier, later);
    }
}

#[test]
fn reordered_inputs_and_repeated_runs_are_identical() {
    let (m1, m5, h1) = alignment_fixture();
    let forward = build_layers(
        &[input(M1, &m1), input(M5, &m5), input(H1, &h1)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");
    let reordered = build_layers(
        &[input(H1, &h1), input(M1, &m1), input(M5, &m5)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");
    let repeated = build_layers(
        &[input(M5, &m5), input(H1, &h1), input(M1, &m1)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");

    assert_eq!(forward, reordered);
    assert_eq!(forward, repeated);
}

#[test]
fn pivot_and_level_records_match_the_model() {
    let m1 = pivot_candles();
    let set = build_layers(&[input(M1, &m1)], &LayerConfig::default()).expect("the fixture builds");
    let pivots = PivotSeries::compute(&m1);
    let registry = ConfirmedLevelRegistry::from_pivots(&pivots);

    let candidate_count: usize = PivotScale::ALL
        .iter()
        .map(|scale| pivots.candidate_count(*scale))
        .sum();
    let confirmed_count: usize = PivotScale::ALL
        .iter()
        .map(|scale| pivots.confirmed_count(*scale))
        .sum();
    assert_eq!(set.pivots.len(), candidate_count);
    assert_eq!(
        set.pivots
            .iter()
            .filter(|row| row.confirmed_code == 1)
            .count(),
        confirmed_count
    );
    assert!(confirmed_count > 0, "the fixture must confirm pivots");

    for row in &set.pivots {
        assert_eq!(row.timeframe_secs, M1);
        let scale = match row.scale_code {
            1 => PivotScale::Minor,
            2 => PivotScale::Major,
            other => panic!("unexpected scale code {other}"),
        };
        let candidate = pivots
            .candidate(scale, row.occurred_bar_index)
            .expect("every row came from a candidate");
        assert_eq!(row.kind_code, kind_code(candidate.kind));
        assert_eq!(row.price, candidate.price);
        assert_eq!(row.occurred_bar_index, candidate.occurred_at);
        if row.confirmed_code == 1 {
            let pivot = pivots
                .confirmed_at(scale, row.available_bar_index)
                .expect("the confirmed pivot is at its availability bar");
            assert_eq!(pivot.kind, candidate.kind);
            assert_eq!(pivot.price, candidate.price);
            assert_eq!(pivot.occurred_at, candidate.occurred_at);
            assert!(pivot.available_at > pivot.occurred_at);
        } else {
            assert_eq!(row.confirmed_code, 0);
            assert_eq!(row.available_bar_index, candidate.available_at);
            assert_eq!(row.available_bar_index, row.occurred_bar_index);
        }
    }

    for scale in PivotScale::ALL {
        for pivot in pivots.all_confirmed(scale) {
            assert!(
                set.pivots.iter().any(|row| {
                    row.scale_code == scale.code()
                        && row.kind_code == kind_code(pivot.kind)
                        && row.occurred_bar_index == pivot.occurred_at
                        && row.available_bar_index == pivot.available_at
                        && row.confirmed_code == 1
                }),
                "a confirmed pivot is missing from the rows"
            );
        }
    }

    assert_eq!(set.levels.len(), registry.len());
    for (index, (row, level)) in set.levels.iter().zip(registry.levels()).enumerate() {
        assert_eq!(row.timeframe_secs, M1);
        assert_eq!(row.reference_code, 1001 + index as u32);
        assert_eq!(row.reference_code, level.reference_code);
        assert_eq!(row.scale_code, level.scale.code());
        assert_eq!(row.kind_code, kind_code(level.kind));
        assert_eq!(row.price, level.price);
        assert_eq!(row.occurred_bar_index, level.occurred_at);
        assert_eq!(row.available_bar_index, level.available_at);
        assert!(row.available_bar_index >= row.occurred_bar_index);
    }

    for pair in set.pivots.windows(2) {
        let left = (
            pair[0].timeframe_secs,
            pair[0].occurred_bar_index,
            pair[0].scale_code,
            pair[0].kind_code,
        );
        let right = (
            pair[1].timeframe_secs,
            pair[1].occurred_bar_index,
            pair[1].scale_code,
            pair[1].kind_code,
        );
        assert!(left <= right, "pivot rows are not ordered");
    }
    for pair in set.levels.windows(2) {
        let left = (pair[0].timeframe_secs, pair[0].reference_code);
        let right = (pair[1].timeframe_secs, pair[1].reference_code);
        assert!(left <= right, "level rows are not ordered");
    }

    let candidate = set
        .pivots
        .iter()
        .find(|row| row.confirmed_code == 0)
        .expect("the fixture has an unconfirmed candidate");
    assert_eq!(candidate.available_bar_index, candidate.occurred_bar_index);
}

#[test]
fn pivot_and_level_records_are_built_for_every_timeframe() {
    let (m1, m5, h1) = alignment_fixture();
    let set = build_layers(
        &[input(H1, &h1), input(M1, &m1), input(M5, &m5)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");

    for timeframe in [M1, M5, H1] {
        let pivots = PivotSeries::compute(match timeframe {
            M1 => &m1,
            M5 => &m5,
            _ => &h1,
        });
        let expected = PivotScale::ALL
            .iter()
            .map(|scale| pivots.candidate_count(*scale))
            .sum::<usize>();
        assert_eq!(
            set.pivots
                .iter()
                .filter(|row| row.timeframe_secs == timeframe)
                .count(),
            expected
        );
        assert!(
            set.levels
                .iter()
                .all(|row| row.timeframe_secs != timeframe || row.reference_code >= 1001)
        );
    }
}

#[test]
fn layer_one_matches_the_model_snapshot_at_the_latest_bar() {
    let m1 = candles(300, 0, 60, 0.0);
    let set = build_layers(&[input(M1, &m1)], &LayerConfig::default()).expect("the fixture builds");
    let state = MarketState::compute(
        &[TimeframeInput {
            timeframe_secs: M1,
            candles: &m1,
        }],
        &MarketStateConfig::default(),
    );
    let snapshot = &state.timeframes[0].snapshot;
    let expected = Metrics::from_bars(
        &snapshot.direction,
        &snapshot.momentum,
        &snapshot.structure,
        &snapshot.progression,
        &snapshot.context,
    );

    let last = set.layer1.last().expect("one row");
    assert_eq!(last.metrics, expected);
    assert_eq!(last.bar_index, m1.len() - 1);
}

#[test]
fn a_single_candle_has_one_row_and_no_block() {
    let m1 = candles(1, 0, 60, 0.0);
    let set = build_layers(&[input(M1, &m1)], &LayerConfig::default()).expect("the fixture builds");

    assert_eq!(set.layer1.len(), 1);
    assert_eq!(set.layer3.len(), 1);
    assert!(set.pivots.is_empty());
    assert!(set.levels.is_empty());
    assert!(set.layer3[0].higher.is_empty());
    assert_eq!(set.layer3[0].aggregates.ladder, vec![M1]);
    assert_eq!(set.layer1[0].metrics.direction.direction, None);
    assert_eq!(
        set.layer1[0].metrics.direction.unavailable_reason_code,
        Some(1)
    );
    assert!(
        set.layer1[0]
            .metrics
            .structure
            .unavailable_reason_code
            .is_some()
    );
}

#[test]
fn layer_three_requires_an_m1_base() {
    let m5 = candles(6, 0, 300, 0.0);
    let h1 = candles(2, 0, 3_600, 0.0);
    let set = build_layers(&[input(M5, &m5), input(H1, &h1)], &LayerConfig::default())
        .expect("the fixture builds");

    assert!(set.layer3.is_empty());
    assert_eq!(set.layer1.len(), m5.len() + h1.len());
}

#[test]
fn empty_inputs_are_omitted() {
    let m1 = candles(3, 0, 60, 0.0);
    let empty: [Candle; 0] = [];
    let set = build_layers(
        &[input(M5, &empty), input(M1, &m1)],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");

    assert_eq!(set.timeframes, vec![M1]);
    assert_eq!(set.layer1.len(), m1.len());
}

#[test]
fn a_zero_timeframe_is_rejected() {
    let m1 = candles(3, 0, 60, 0.0);
    let result = build_layers(&[input(0, &m1)], &LayerConfig::default());
    assert_eq!(result, Err(LayerError::ZeroTimeframe));
}

#[test]
fn a_duplicate_timeframe_is_rejected() {
    let m1 = candles(3, 0, 60, 0.0);
    let result = build_layers(&[input(M1, &m1), input(M1, &m1)], &LayerConfig::default());
    assert_eq!(
        result,
        Err(LayerError::DuplicateTimeframe { timeframe_secs: M1 })
    );
}

#[test]
#[ignore = "100k-bar layer benchmark; run explicitly with --release"]
fn a_100k_m1_layer_build_benchmark() {
    let m1 = candles(100_000, 0, 60, 0.0);
    let m5 = candles(20_000, 0, 300, 1.0);
    let m15 = candles(6_666, 0, 900, 2.0);
    let h1 = candles(1_666, 0, 3_600, 3.0);

    let started = std::time::Instant::now();
    let set = build_layers(
        &[
            input(M1, &m1),
            input(M5, &m5),
            input(900, &m15),
            input(H1, &h1),
        ],
        &LayerConfig::default(),
    )
    .expect("the benchmark builds");
    let elapsed = started.elapsed();

    eprintln!(
        "layer benchmark: {} Layer 1 rows, {} Layer 3 rows in {:.3} s",
        set.layer1.len(),
        set.layer3.len(),
        elapsed.as_secs_f64()
    );
    assert_eq!(set.layer3.len(), m1.len());
    assert_eq!(set.layer1.len(), m1.len() + m5.len() + m15.len() + h1.len());
}

fn layer2_row(set: &super::LayerSet, timeframe_secs: u64, bar_index: usize) -> &super::Layer2Row {
    set.layer2
        .iter()
        .find(|row| {
            row.layer1.timeframe_secs == timeframe_secs && row.layer1.bar_index == bar_index
        })
        .expect("the Layer 2 row")
}
