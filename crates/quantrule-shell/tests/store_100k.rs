//! Ignored 100k M1 integration benchmark.
//!
//! Run explicitly with:
//! `cargo test -p quantrule-shell --test store_100k -- --ignored --nocapture`
//!
//! The test generates a deterministic 100k M1 CSV under `target/`, ingests it,
//! verifies every stored table, reads bounded query views, runs the model, and
//! exports the research dataset. It reports the observed timings; it does not
//! assert on wall-clock durations.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use quantrule_data::format_utc_timestamp;

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");
const BARS: usize = 100_000;
const DATA_VERSION: &str = "test:100k:60:0-100000";

fn artifact_dir() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/store-100k-tests");
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("the artifact directory is created");
    path.to_str()
        .expect("the artifact path is UTF-8")
        .to_owned()
}

fn write_source(path: &str) {
    let file = File::create(path).expect("the source CSV is created");
    let mut writer = BufWriter::new(file);
    writeln!(writer, "Date,Open,High,Low,Close,Volume").expect("the header writes");
    for index in 0..BARS {
        let timestamp = 1_699_999_980 + index as i64 * 60;
        let base = 100.0 + (index % 97) as f64 * 0.01 + (index / 97) as f64 * 0.0001;
        let open = base;
        let close = base + 0.05;
        let high = close + 0.1;
        let low = open - 0.1;
        let volume = 1_000 + (index % 500) as u64;
        let stamp = format_utc_timestamp(timestamp);
        writeln!(
            writer,
            "{} {}+00:00,{open:.5},{high:.5},{low:.5},{close:.5},{volume}",
            &stamp[..10],
            &stamp[11..19]
        )
        .expect("the row writes");
    }
    writer.flush().expect("the source flush");
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("the quantrule binary runs")
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed with {:?}; stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn stdout_text(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn timed(label: &str, args: &[&str]) -> (Duration, Output) {
    let started = Instant::now();
    let output = run(args);
    let elapsed = started.elapsed();
    assert_success(&output, label);
    eprintln!("100k {label}: {:.3} s", elapsed.as_secs_f64());
    (elapsed, output)
}

fn parse_bars(text: &str) -> usize {
    let marker = "bars=";
    let start = text.find(marker).expect("the bars field") + marker.len();
    let end = text[start..]
        .find(' ')
        .map_or(text.len(), |offset| start + offset);
    text[start..end].parse().expect("the bar count")
}

#[test]
#[ignore = "100k-bar integration benchmark; run explicitly"]
fn a_100k_dataset_round_trips_and_reports_timings() {
    let root = artifact_dir();
    let source = format!("{root}/m1_100k.csv");
    let db = format!("{root}/store.db");
    let export_root = format!("{root}/export");
    eprintln!("100k source: {source}");
    write_source(&source);

    let (ingest_time, ingest) = timed(
        "ingest",
        &[
            "store",
            "ingest",
            &source,
            "--dataset",
            "demo_100k",
            "--data-version",
            DATA_VERSION,
            "--db",
            &db,
        ],
    );
    let ingest_text = stdout_text(&ingest);
    assert!(ingest_text.starts_with("ingested demo_100k bars="));
    let bars = parse_bars(&ingest_text);
    assert!(bars >= BARS, "the ladder bars cover the M1 bars");

    let (model_time, model) = timed("model", &["market-state", &source]);
    assert!(stdout_text(&model).contains("\"ladder\""));

    let (verify_time, verify) = timed(
        "verify",
        &[
            "store",
            "verify",
            &source,
            "--dataset",
            "demo_100k",
            "--data-version",
            DATA_VERSION,
            "--db",
            &db,
        ],
    );
    assert_eq!(
        stdout_text(&verify),
        format!("verified demo_100k bars={bars} tables=7\n")
    );

    let (list_time, list) = timed("list", &["store", "list", "--db", &db]);
    let list_text = stdout_text(&list);
    let lines: Vec<&str> = list_text.lines().collect();
    assert_eq!(lines.len(), 2);
    let header: Vec<&str> = lines[0].split(',').collect();
    let row: Vec<&str> = lines[1].split(',').collect();
    let column = |name: &str| -> &str {
        let index = header
            .iter()
            .position(|column| *column == name)
            .expect("the count column");
        row[index]
    };
    assert_eq!(column("m1_bars"), "100000");
    assert_eq!(column("layer3_rows"), "100000");
    assert_eq!(column("bar_rows"), &bars.to_string());

    let (query_time, query) = timed(
        "query",
        &[
            "db",
            "series",
            "demo_100k",
            "--timeframe",
            "1m",
            "--limit",
            "1000",
            "--db",
            &db,
        ],
    );
    assert_eq!(stdout_text(&query).lines().count(), 1_000);

    let (layer3_time, layer3) = timed(
        "layer3",
        &["db", "layer3", "demo_100k", "--limit", "1", "--db", &db],
    );
    let layer3_header = stdout_text(&layer3);
    let header = layer3_header.lines().next().expect("the Layer 3 header");
    assert_eq!(header.split(',').count(), 285);

    let (export_time, export) = timed(
        "export",
        &[
            "research",
            "export",
            &source,
            "demo_100k",
            "--data-version",
            DATA_VERSION,
            "--root",
            &export_root,
        ],
    );
    let export_text = stdout_text(&export);
    assert!(export_text.contains("layer3 rows: 100000"));
    assert!(export_text.contains("files: 9"));

    let size = fs::metadata(&db).map(|meta| meta.len()).unwrap_or(0);
    eprintln!(
        "100k summary: bars={bars} db={} bytes ingest={:.3}s model={:.3}s verify={:.3}s list={:.3}s query={:.3}s layer3={:.3}s export={:.3}s",
        size,
        ingest_time.as_secs_f64(),
        model_time.as_secs_f64(),
        verify_time.as_secs_f64(),
        list_time.as_secs_f64(),
        query_time.as_secs_f64(),
        layer3_time.as_secs_f64(),
        export_time.as_secs_f64()
    );
}
