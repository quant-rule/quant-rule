//! End-to-end verification of the research dataset.
//!
//! The tests build a deterministic M1 series, derive the ladder with the data
//! engine, build the three layers, and verify the contract's structural rules:
//! row counts, the Layer 2 derivation, resolution ranges, the causal as-of
//! blocks, the exported byte shape, the dictionary, the code table, and a
//! sample analysis from the exported rows.

use std::collections::BTreeMap;
use std::time::Instant;

use quantrule_analytics::alignment::{latest_closed_index, staleness_secs};
use quantrule_analytics::layers::{LayerConfig, LayerSet, build_layers};
use quantrule_analytics::research::{DatasetMeta, ExportBundle, ExportFile, render_dataset};
use quantrule_analytics::resolution::{AxisResolution, clamp_unit, ordinal_bucket};
use quantrule_analytics::summary::summarize;
use quantrule_data::{Candle, CandleValues, Timeframe, TimeframeSet};
use quantrule_model::input::TimeframeInput;

struct Dataset {
    m1: Vec<Candle>,
    m5: Vec<Candle>,
    m15: Vec<Candle>,
    h1: Vec<Candle>,
    layers: LayerSet,
}

fn build(count: usize) -> Dataset {
    let mut set = TimeframeSet::full();
    for index in 0..count {
        let price = 100.0 + (index % 97) as f64 * 0.1 + (index % 13) as f64 * 0.03;
        set.push_values(CandleValues {
            timestamp: index as i64 * 60,
            open: price,
            high: price + 0.2,
            low: price - 0.2,
            close: price + 0.01,
            volume: Some(1.0 + (index % 5) as f64),
        })
        .expect("the fixture candle is valid");
    }
    set.finish().expect("the fixture buckets are valid");

    let m1: Vec<Candle> = set.series(Timeframe::M1).iter().copied().collect();
    let m5: Vec<Candle> = set.series(Timeframe::M5).iter().copied().collect();
    let m15: Vec<Candle> = set.series(Timeframe::M15).iter().copied().collect();
    let h1: Vec<Candle> = set.series(Timeframe::H1).iter().copied().collect();

    let layers = build_layers(
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
                timeframe_secs: 3_600,
                candles: &h1,
            },
        ],
        &LayerConfig::default(),
    )
    .expect("the layers build");

    Dataset {
        m1,
        m5,
        m15,
        h1,
        layers,
    }
}

fn meta() -> DatasetMeta {
    DatasetMeta {
        root: "data".to_string(),
        dataset: "verify".to_string(),
        data_version: "verify:60:0-600".to_string(),
        model_version: "0.1.0".to_string(),
        structure_scale_code: 2,
    }
}

fn file<'a>(bundle: &'a ExportBundle, name: &str) -> &'a ExportFile {
    bundle
        .files
        .iter()
        .find(|file| file.name == name)
        .expect("the file is present")
}

#[test]
fn the_layer_counts_match_the_inputs() {
    let dataset = build(600);
    assert_eq!(
        dataset.layers.layer1.len(),
        dataset.m1.len() + dataset.m5.len() + dataset.m15.len() + dataset.h1.len()
    );
    assert_eq!(dataset.layers.layer2.len(), dataset.layers.layer1.len());
    assert_eq!(dataset.layers.layer3.len(), dataset.m1.len());
    assert_eq!(dataset.layers.timeframes, vec![60, 300, 900, 3600]);
}

#[test]
fn layer_two_derives_from_every_layer_one_metric() {
    let dataset = build(600);
    for (layer1, layer2) in dataset.layers.layer1.iter().zip(&dataset.layers.layer2) {
        assert_eq!(&layer2.layer1, layer1);
        let metrics = &layer1.metrics;
        assert_eq!(
            layer2.resolution.direction,
            AxisResolution::new(
                metrics.direction.direction,
                metrics.direction.unavailable_reason_code
            )
        );
        assert_eq!(
            layer2.resolution.momentum,
            AxisResolution::new(
                metrics.momentum.force,
                metrics.momentum.unavailable_reason_code
            )
        );
        assert_eq!(
            layer2.resolution.structure,
            AxisResolution::new(
                metrics.structure.score,
                metrics.structure.unavailable_reason_code
            )
        );
        assert_eq!(
            layer2.resolution.progression,
            AxisResolution::new(
                metrics.progression.progress.map(clamp_unit),
                metrics.progression.unavailable_reason_code
            )
        );
    }
}

#[test]
fn every_resolution_is_in_range_and_matches_its_bucket() {
    let dataset = build(600);
    for row in &dataset.layers.layer2 {
        for resolution in [
            row.resolution.direction,
            row.resolution.momentum,
            row.resolution.structure,
            row.resolution.progression,
        ] {
            match resolution.continuous {
                Some(value) => {
                    assert!((-1.0..=1.0).contains(&value), "continuous {value}");
                    assert_eq!(resolution.ordinal, Some(ordinal_bucket(value)));
                    assert_eq!(resolution.reason, None);
                }
                None => {
                    assert_eq!(resolution.ordinal, None);
                    assert!(resolution.reason.is_some());
                }
            }
        }
    }
}

#[test]
fn every_block_is_the_latest_closed_higher_bar() {
    let dataset = build(600);
    let mut open_times: BTreeMap<u64, Vec<i64>> = BTreeMap::new();
    open_times.insert(300, dataset.m5.iter().map(Candle::timestamp).collect());
    open_times.insert(900, dataset.m15.iter().map(Candle::timestamp).collect());
    open_times.insert(3_600, dataset.h1.iter().map(Candle::timestamp).collect());

    for row in &dataset.layers.layer3 {
        let base_open = row.base.layer1.candle.timestamp();
        let base_close = base_open + 60;
        for block in &row.higher {
            let times = open_times
                .get(&block.timeframe_secs)
                .expect("a known timeframe");
            let index = latest_closed_index(base_open, 60, times, block.timeframe_secs)
                .expect("a closed higher bar");
            assert_eq!(index, block.bar_index);
            assert_eq!(block.candle.timestamp(), times[index]);
            let block_close = times[index] + block.timeframe_secs as i64;
            assert!(block_close <= base_close, "a block closes after the base");
            assert_eq!(
                block.staleness_secs,
                staleness_secs(base_open, 60, times[index], block.timeframe_secs)
                    .expect("a non-negative staleness")
            );
        }
    }
}

#[test]
fn the_export_is_byte_identical_and_complete() {
    let dataset = build(600);
    let first = render_dataset(&dataset.layers, &dataset.m1, &meta()).expect("the first render");
    let second = render_dataset(&dataset.layers, &dataset.m1, &meta()).expect("the second render");
    assert_eq!(first, second);

    for (name, width) in [("layer1.csv", 61), ("layer2.csv", 73), ("layer3.csv", 356)] {
        let text = &file(&first, name).contents;
        for line in text.lines() {
            assert_eq!(line.split(',').count(), width, "{name}: {line}");
        }
    }

    let dictionary = &file(&first, "data_dictionary.csv").contents;
    let count = |name: &str| {
        let prefix = format!("{name},");
        dictionary
            .lines()
            .skip(1)
            .filter(|line| line.starts_with(&prefix))
            .count()
    };
    assert_eq!(count("m1.csv"), 6);
    assert_eq!(count("layer1.csv"), 61);
    assert_eq!(count("layer2.csv"), 73);
    assert_eq!(count("layer3.csv"), 356);

    let codes = &file(&first, "codes.csv").contents;
    assert!(codes.contains("resolution_bucket,0,"));
    assert!(codes.contains("unavailable_reason,6,"));
    assert!(codes.contains("event_subtype.divergence,4,"));

    let manifest = &file(&first, "manifest.json").contents;
    assert!(manifest.contains("\"data_version\": \"verify:60:0-600\""));
    assert!(manifest.contains("\"layer3\": 600"));
    assert_eq!(manifest.matches("\"path\":").count(), 4);
}

#[test]
fn a_sample_analysis_matches_the_exported_rows() {
    let dataset = build(600);
    let bundle = render_dataset(&dataset.layers, &dataset.m1, &meta()).expect("the render");
    let layer2 = &file(&bundle, "layer2.csv").contents;
    let mut lines = layer2.lines();
    let header: Vec<&str> = lines.next().expect("a header").split(',').collect();
    let index = |name: &str| {
        header
            .iter()
            .position(|column| *column == name)
            .expect("the column")
    };
    let direction = index("direction_continuous");
    let ordinal = index("direction_ordinal");
    let momentum = index("momentum_continuous");

    let mut count = 0_usize;
    let mut sum = 0.0;
    let mut ordinals = [0_usize; 10];
    let mut pairs = Vec::new();
    for line in lines {
        let fields: Vec<&str> = line.split(',').collect();
        if fields[0] != "60" {
            continue;
        }
        if fields[direction].is_empty() {
            continue;
        }
        let value: f64 = fields[direction].parse().expect("a value");
        count += 1;
        sum += value;
        let bucket: usize = fields[ordinal].parse().expect("a bucket");
        ordinals[bucket] += 1;
        if !fields[momentum].is_empty() {
            pairs.push((value, fields[momentum].parse::<f64>().expect("a momentum")));
        }
    }

    let summary = summarize(&dataset.layers);
    let axis = summary
        .axes
        .iter()
        .find(|axis| axis.timeframe_secs == 60 && axis.axis == "direction")
        .expect("the direction summary");
    assert_eq!(axis.count, count);
    assert_eq!(axis.ordinal_counts, ordinals);
    assert!((axis.mean.expect("a mean") - sum / count as f64).abs() < 1e-12);

    let relation = summary
        .relations
        .iter()
        .find(|relation| {
            relation.timeframe_secs == 60
                && relation.left == "direction"
                && relation.right == "momentum"
        })
        .expect("the relation");
    match (relation.correlation, pearson(&pairs)) {
        (Some(expected), Some(actual)) => assert!((expected - actual).abs() < 1e-12),
        (None, None) => {}
        (expected, actual) => panic!("correlation mismatch: {expected:?} vs {actual:?}"),
    }
}

#[test]
fn the_dataset_is_causal() {
    let prefix = build(300);
    let full = build(360);
    assert_eq!(prefix.layers.layer3.len(), 300);
    for (earlier, later) in prefix.layers.layer3.iter().zip(&full.layers.layer3) {
        assert_eq!(earlier, later);
    }
}

fn pearson(pairs: &[(f64, f64)]) -> Option<f64> {
    if pairs.len() < 2 {
        return None;
    }
    let count = pairs.len() as f64;
    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    let mut sum_xy = 0.0;
    let mut sum_xx = 0.0;
    let mut sum_yy = 0.0;
    for (x, y) in pairs {
        sum_x += x;
        sum_y += y;
        sum_xy += x * y;
        sum_xx += x * x;
        sum_yy += y * y;
    }
    let numerator = count * sum_xy - sum_x * sum_y;
    let denominator = ((count * sum_xx - sum_x * sum_x) * (count * sum_yy - sum_y * sum_y)).sqrt();
    if denominator <= 0.0 {
        None
    } else {
        Some((numerator / denominator).clamp(-1.0, 1.0))
    }
}

#[test]
#[ignore = "100k dataset benchmark; run explicitly with --release"]
fn a_100k_dataset_export_benchmark() {
    let started = Instant::now();
    let dataset = build(100_000);
    let build_elapsed = started.elapsed();

    let render_started = Instant::now();
    let bundle = render_dataset(&dataset.layers, &dataset.m1, &meta()).expect("the render");
    let render_elapsed = render_started.elapsed();

    let layer3_bytes = file(&bundle, "layer3.csv").contents.len();
    let total_bytes: usize = bundle.files.iter().map(|file| file.contents.len()).sum();
    eprintln!(
        "100k dataset: {} Layer 1, {} Layer 2, {} Layer 3 rows; build {:.3} s, render {:.3} s; layer3 {} bytes; all files {} bytes",
        bundle.layer1_rows,
        bundle.layer2_rows,
        bundle.layer3_rows,
        build_elapsed.as_secs_f64(),
        render_elapsed.as_secs_f64(),
        layer3_bytes,
        total_bytes
    );

    assert_eq!(bundle.layer1_rows, 128_332);
    assert_eq!(bundle.layer2_rows, 128_332);
    assert_eq!(bundle.layer3_rows, 100_000);
}
