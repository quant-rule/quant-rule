use quantrule_data::Candle;
use quantrule_model::state::{MarketState, MarketStateConfig, TimeframeInput};

use super::{
    AGGREGATE_COLUMNS, HIGHER_BLOCK_COLUMNS, LAYER1_COLUMNS, LAYER2_COLUMNS, LayerConfig,
    LayerError, Metrics, build_layers, layer3_columns,
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
