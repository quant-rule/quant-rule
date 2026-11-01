use quantrule_data::Candle;
use quantrule_model::input::TimeframeInput;

use super::{DatasetMeta, ExportBundle, ExportError, ExportFile, render_dataset};
use crate::layers::{LayerConfig, LayerSet, build_layers};

const M1: u64 = 60;
const M5: u64 = 300;
const M15: u64 = 900;
const H1: u64 = 3_600;

fn candles(count: usize, start: i64, step: i64, offset: f64) -> Vec<Candle> {
    (0..count)
        .map(|index| {
            let price = 100.0 + offset + (index % 11) as f64 * 0.1;
            Candle::new(
                index as u64,
                start + index as i64 * step,
                price,
                price + 0.2,
                price - 0.2,
                price + 0.01,
                Some(1.0 + (index % 3) as f64),
            )
            .expect("the test candle is valid")
        })
        .collect()
}

fn fixture() -> (LayerSet, Vec<Candle>) {
    let m1 = candles(5, 0, 60, 0.0);
    let m5 = candles(2, 0, 300, 1.0);
    let m15 = candles(1, 0, 900, 2.0);
    let h1 = candles(1, 0, 3_600, 3.0);
    let set = build_layers(
        &[
            TimeframeInput {
                timeframe_secs: M1,
                candles: &m1,
            },
            TimeframeInput {
                timeframe_secs: M5,
                candles: &m5,
            },
            TimeframeInput {
                timeframe_secs: M15,
                candles: &m15,
            },
            TimeframeInput {
                timeframe_secs: H1,
                candles: &h1,
            },
        ],
        &LayerConfig::default(),
    )
    .expect("the fixture builds");
    (set, m1)
}

fn meta() -> DatasetMeta {
    DatasetMeta {
        root: "data".to_string(),
        dataset: "demo".to_string(),
        data_version: "source:TEST:60:0-240".to_string(),
        model_version: "0.1.0".to_string(),
        structure_scale_code: 2,
    }
}

fn bundle() -> ExportBundle {
    let (set, m1) = fixture();
    render_dataset(&set, &m1, &meta()).expect("the fixture renders")
}

fn file<'a>(bundle: &'a ExportBundle, name: &str) -> &'a ExportFile {
    bundle
        .files
        .iter()
        .find(|file| file.name == name)
        .expect("the file is present")
}

#[test]
fn two_renders_are_byte_identical() {
    let (set, m1) = fixture();
    let first = render_dataset(&set, &m1, &meta()).expect("the first render");
    let second = render_dataset(&set, &m1, &meta()).expect("the second render");
    assert_eq!(first, second);
}

#[test]
fn the_layer_widths_match_the_contract() {
    let bundle = bundle();
    for (name, width) in [("layer1.csv", 61), ("layer2.csv", 73), ("layer3.csv", 356)] {
        let text = &file(&bundle, name).contents;
        let mut lines = text.lines();
        let header = lines.next().expect("a header");
        assert_eq!(header.split(',').count(), width, "{name} header");
        for line in lines {
            assert_eq!(line.split(',').count(), width, "{name} row");
        }
    }
}

#[test]
fn a_missing_higher_block_is_written_as_nulls() {
    let bundle = bundle();
    let layer3 = &file(&bundle, "layer3.csv").contents;
    let mut lines = layer3.lines();
    let header: Vec<&str> = lines.next().expect("a header").split(',').collect();
    let m5_bar = header
        .iter()
        .position(|name| *name == "m5_bar_index")
        .expect("the M5 block");
    let m5_staleness = header
        .iter()
        .position(|name| *name == "m5_staleness_secs")
        .expect("the M5 staleness");
    let ladder = header
        .iter()
        .position(|name| *name == "agg_ladder")
        .expect("the ladder");

    let first: Vec<&str> = layer3
        .lines()
        .nth(1)
        .expect("the first row")
        .split(',')
        .collect();
    assert_eq!(first[m5_bar], "");
    assert_eq!(first[ladder], "60");

    let rows: Vec<Vec<&str>> = layer3
        .lines()
        .skip(1)
        .map(|line| line.split(',').collect())
        .collect();
    assert_eq!(rows[4][m5_bar], "0");
    assert_eq!(rows[4][m5_staleness], "0");
    assert_eq!(rows[4][ladder], "60;300");
}

#[test]
fn null_is_an_empty_field_and_negative_zero_is_zero() {
    let (set, m1) = fixture();
    let bundle = render_dataset(&set, &m1, &meta()).expect("the fixture renders");
    let layer1 = &file(&bundle, "layer1.csv").contents;
    let header: Vec<&str> = layer1
        .lines()
        .next()
        .expect("a header")
        .split(',')
        .collect();
    let direction = header
        .iter()
        .position(|name| *name == "direction")
        .expect("the direction column");
    let first: Vec<&str> = layer1.lines().nth(1).expect("a row").split(',').collect();
    assert_eq!(first[direction], "");
    assert_eq!(super::render::format_float(-0.0), "0");
    assert_eq!(super::render::format_float(0.5), "0.5");
}

#[test]
fn the_raw_file_keeps_the_source_format_and_an_empty_volume() {
    let no_volume =
        vec![Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, None).expect("the test candle is valid")];
    let set = build_layers(
        &[TimeframeInput {
            timeframe_secs: M1,
            candles: &no_volume,
        }],
        &LayerConfig::default(),
    )
    .expect("the one-bar fixture builds");
    let bundle = render_dataset(&set, &no_volume, &meta()).expect("the fixture renders");
    let raw = &file(&bundle, "m1.csv").contents;
    assert!(raw.starts_with("Date,Open,High,Low,Close,Volume\n"));
    assert_eq!(raw.lines().count(), 2);
    let row = raw.lines().nth(1).expect("a raw row");
    assert_eq!(row.split(',').count(), 6);
    assert!(row.ends_with(','));
}

#[test]
fn the_manifest_records_the_identity_and_counts() {
    let bundle = bundle();
    let manifest = &file(&bundle, "manifest.json").contents;
    assert!(manifest.contains("\"dataset\": \"demo\""));
    assert!(manifest.contains("\"data_version\": \"source:TEST:60:0-240\""));
    assert!(manifest.contains("\"model_version\": \"0.1.0\""));
    assert!(manifest.contains("\"timeframes_secs\": [60, 300, 900, 3600]"));
    assert!(manifest.contains("\"layer1\": 9"));
    assert!(manifest.contains("\"layer2\": 9"));
    assert!(manifest.contains("\"layer3\": 5"));
    assert!(manifest.contains("\"raw\": {\"60\": 5}"));
    assert!(manifest.contains("\"path\": \"data/raw/demo/m1.csv\""));
    assert!(manifest.contains("\"path\": \"data/processed/demo/layer1.csv\""));
    assert_eq!(
        manifest.matches("\"path\":").count(),
        4,
        "the raw and the three layer files"
    );
}

#[test]
fn the_dictionary_documents_every_column() {
    let bundle = bundle();
    let dictionary = &file(&bundle, "data_dictionary.csv").contents;
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

    let layer3 = &file(&bundle, "layer3.csv").contents;
    let header = layer3.lines().next().expect("a header");
    for name in header.split(',') {
        let needle = format!(",{name},");
        assert!(
            dictionary.lines().any(|line| line.contains(&needle)),
            "the dictionary does not document {name}"
        );
    }
}

#[test]
fn the_codes_cover_every_used_registry() {
    let bundle = bundle();
    let codes = &file(&bundle, "codes.csv").contents;
    let keys: Vec<(&str, &str)> = codes
        .lines()
        .skip(1)
        .map(|line| {
            let mut fields = line.split(',');
            (
                fields.next().expect("registry"),
                fields.next().expect("code"),
            )
        })
        .collect();

    for code in 0..=2 {
        assert!(keys.contains(&("readiness", code.to_string().as_str())));
    }
    for code in 1..=6 {
        assert!(keys.contains(&("unavailable_reason", code.to_string().as_str())));
    }
    for code in 0..=9 {
        assert!(keys.contains(&("resolution_bucket", code.to_string().as_str())));
    }
    for code in 1..=2 {
        assert!(keys.contains(&("scale", code.to_string().as_str())));
    }
    for code in 1..=4 {
        assert!(keys.contains(&("context", code.to_string().as_str())));
    }
    for code in ["-1", "0", "1"] {
        assert!(keys.contains(&("state", code)));
    }
    for code in ["-2", "-1", "0", "1", "2"] {
        assert!(keys.contains(&("structure_state", code)));
    }
}

#[test]
fn the_readme_names_the_dataset_and_the_version() {
    let bundle = bundle();
    let readme = &file(&bundle, "README.md").contents;
    assert!(readme.contains("# Research dataset: demo"));
    assert!(readme.contains("source:TEST:60:0-240"));
    assert!(readme.contains("| layer3.csv | 5 |"));
    assert!(readme.contains("quantrule research export"));
}

#[test]
fn an_unsafe_dataset_name_or_an_empty_identity_is_rejected() {
    let (set, m1) = fixture();
    let mut invalid = meta();
    invalid.dataset = "..".to_string();
    assert_eq!(
        render_dataset(&set, &m1, &invalid),
        Err(ExportError::InvalidDataset)
    );
    invalid.dataset = "a/b".to_string();
    assert_eq!(
        render_dataset(&set, &m1, &invalid),
        Err(ExportError::InvalidDataset)
    );
    invalid.dataset = String::new();
    assert_eq!(
        render_dataset(&set, &m1, &invalid),
        Err(ExportError::EmptyDataset)
    );
    let mut invalid = meta();
    invalid.data_version = String::new();
    assert_eq!(
        render_dataset(&set, &m1, &invalid),
        Err(ExportError::EmptyDataVersion)
    );
    let mut invalid = meta();
    invalid.root = String::new();
    assert_eq!(
        render_dataset(&set, &m1, &invalid),
        Err(ExportError::EmptyRoot)
    );
}

#[test]
fn the_manifest_records_a_configured_but_empty_higher_timeframe() {
    let m1 = candles(5, 0, 60, 0.0);
    let set = build_layers(
        &[TimeframeInput {
            timeframe_secs: M1,
            candles: &m1,
        }],
        &LayerConfig::default(),
    )
    .expect("the one-timeframe fixture builds");
    let bundle = super::render_dataset_with_ladder(&set, &m1, &meta(), &[M5, M15, H1])
        .expect("the configured ladder renders");

    let manifest = &file(&bundle, "manifest.json").contents;
    assert!(manifest.contains("\"timeframes_secs\": [60, 300, 900, 3600]"));
    let layer3 = &file(&bundle, "layer3.csv").contents;
    assert!(
        layer3
            .lines()
            .next()
            .expect("a header")
            .contains("h1_bar_index")
    );
    let readme = &file(&bundle, "README.md").contents;
    assert!(readme.contains("- Timeframes (seconds): 60, 300, 900, 3600"));
}

#[test]
fn a_row_count_mismatch_is_rejected() {
    let (mut set, m1) = fixture();
    set.layer2.pop();
    assert!(matches!(
        render_dataset(&set, &m1, &meta()),
        Err(ExportError::Validation(_))
    ));
}

#[test]
fn an_out_of_range_resolution_is_rejected() {
    let (mut set, m1) = fixture();
    set.layer2[0].resolution.direction = crate::resolution::AxisResolution {
        continuous: Some(2.0),
        ordinal: Some(9),
        reason: None,
    };
    assert!(matches!(
        render_dataset(&set, &m1, &meta()),
        Err(ExportError::Validation(_))
    ));
}

#[test]
fn an_inconsistent_staleness_is_rejected() {
    let (mut set, m1) = fixture();
    set.layer3[4].higher[0].staleness_secs = 999;
    assert!(matches!(
        render_dataset(&set, &m1, &meta()),
        Err(ExportError::Validation(_))
    ));
}

#[test]
fn an_event_available_before_it_occurred_is_rejected() {
    let (mut set, m1) = fixture();
    set.events.push(crate::layers::EventRow {
        timeframe_secs: M5,
        event_type_code: 1,
        event_subtype_code: 1,
        source_code: 1,
        occurred_bar_index: 2,
        available_bar_index: 1,
        direction_code: 0,
        reference_code: None,
        primary_value: None,
        reference_value: None,
    });
    assert!(matches!(
        render_dataset(&set, &m1, &meta()),
        Err(ExportError::Validation(_))
    ));
}

#[test]
fn a_code_value_outside_the_code_table_is_rejected() {
    let (mut set, m1) = fixture();
    set.layer3[4].higher[0].metrics.structure.scale_code = 9;
    assert!(matches!(
        render_dataset(&set, &m1, &meta()),
        Err(ExportError::Validation(_))
    ));
}

#[test]
fn a_non_finite_metric_is_rejected() {
    let (mut set, m1) = fixture();
    set.layer1[0].metrics.direction.direction = Some(f64::NAN);
    set.layer2[0].layer1 = set.layer1[0];
    set.layer3[0].base = set.layer2[0];
    assert!(matches!(
        render_dataset(&set, &m1, &meta()),
        Err(ExportError::Validation(_))
    ));
}

#[test]
fn the_m1_cap_is_enforced_before_rendering() {
    let (set, _) = fixture();
    let raw = vec![
        Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, None).expect("the test candle is valid");
        100_001
    ];
    assert!(matches!(
        render_dataset(&set, &raw, &meta()),
        Err(ExportError::Validation(_))
    ));
}
