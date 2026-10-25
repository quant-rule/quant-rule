//! The 100,001-row cap workflow test.
//!
//! The test generates one row over the batch cap and proves that both the
//! research export and the store reject it before creating any artifact. It is
//! a boundary test, not a 100k benchmark: the rejection happens before the
//! model or the database is opened.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use quantrule_data::format_utc_timestamp;

const BIN: &str = env!("CARGO_BIN_EXE_quantrule");
const BARS: usize = 100_001;
const BASE_EPOCH: i64 = 1_735_689_600;

fn artifact_dir(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/cap-workflow-tests")
        .join(name);
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("the artifact directory is created");
    path.to_str()
        .expect("the artifact path is UTF-8")
        .to_owned()
}

fn source(path: &str) {
    let mut text = String::from("Date,Open,High,Low,Close,Volume\n");
    for index in 0..BARS {
        let stamp = format_utc_timestamp(BASE_EPOCH + index as i64 * 60);
        let price = 100.0 + (index % 100) as f64 * 0.01;
        text.push_str(&format!(
            "{} {}+00:00,{price:.5},{:.5},{:.5},{:.5},1000\n",
            &stamp[..10],
            &stamp[11..19],
            price + 0.1,
            price - 0.1,
            price + 0.05
        ));
    }
    fs::write(path, text).expect("the over-cap source is written");
}

fn run(args: &[&str]) -> Output {
    Command::new(BIN)
        .args(args)
        .output()
        .expect("the quantrule binary runs")
}

fn assert_rejected(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected a cap rejection; stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("line 100002") && stderr.contains("100000 candles"),
        "the cap error was not clear: {stderr}"
    );
}

#[test]
fn over_cap_research_and_store_reject_without_artifacts() {
    let root = artifact_dir("over-cap");
    let csv = format!("{root}/source.csv");
    source(&csv);

    let export_root = format!("{root}/export");
    let output = run(&[
        "research",
        "export",
        &csv,
        "over-cap",
        "--data-version",
        "test:over-cap:60:0-100001",
        "--root",
        &export_root,
    ]);
    assert_rejected(&output);
    assert!(
        !Path::new(&export_root).join("processed/over-cap").exists(),
        "the rejected export left a dataset directory"
    );

    let db = format!("{root}/store.db");
    let output = run(&[
        "store",
        "ingest",
        &csv,
        "--dataset",
        "over-cap",
        "--data-version",
        "test:over-cap:60:0-100001",
        "--db",
        &db,
    ]);
    assert_rejected(&output);
    assert!(
        !Path::new(&db).exists(),
        "the rejected ingest created a database"
    );
}
