//! Dataset command output tests.

use std::fs;

use crate::cases::assertions::{assert_success, stdout};
use crate::support::{Fixture, data_version};

#[test]
fn dataset_list_prints_one_sorted_row() {
    let fixture = Fixture::start();
    let output = fixture.run(&["dataset", "list"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!(
            "datasets: 1\n{} name=candles.csv candles=3 first_bar=1704067200 last_bar=1704067320 bytes=96\n",
            data_version()
        )
    );
}

#[test]
fn dataset_show_prints_the_record_and_uses_one_segment() {
    let fixture = Fixture::start();
    let output = fixture.run(&["dataset", "show", &data_version()]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!(
            "data_version: {}\nname: candles.csv\ncandles: 3\nfirst_bar: 1704067200\n\
             last_bar: 1704067320\nbytes: 96\nobject_key: datasets/{}\nsha256: {}\n\
             quality: none\ncreated_at: 2026-11-14T00:00:00Z\n",
            data_version(),
            data_version(),
            data_version()
        )
    );
    assert_eq!(
        fixture.last_request().path,
        format!("/catalog/datasets/{}", data_version())
    );
}

#[test]
fn dataset_upload_sends_the_file_and_prints_the_record() {
    let fixture = Fixture::start();
    let csv = b"datetime,open,high,low,close,volume\n2024-01-01T00:00:00Z,1,2,0.5,1.5,10\n";
    let path = fixture.home().join("my candles.csv");
    fs::create_dir_all(fixture.home()).expect("the home exists");
    fs::write(&path, csv).expect("the fixture CSV is written");
    let output = fixture.run(&[
        "dataset",
        "upload",
        path.to_str().expect("UTF-8 path"),
        "--name",
        "my candles.csv",
    ]);
    assert_success(&output);
    assert!(stdout(&output).starts_with("created: true\ndata_version: "));
    let request = fixture.last_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/catalog/datasets");
    assert_eq!(
        request.query.get("name").map(String::as_str),
        Some("my candles.csv")
    );
    assert_eq!(request.body, csv);
    assert_eq!(request.header("content-type"), Some("text/csv"));
}

#[test]
fn dataset_upload_tolerant_prints_the_quality_report() {
    let fixture = Fixture::start();
    let path = fixture.home().join("candles.csv");
    fs::create_dir_all(fixture.home()).expect("the home exists");
    fs::write(&path, b"bad\n").expect("the fixture CSV is written");
    let output = fixture.run(&[
        "dataset",
        "upload",
        path.to_str().expect("UTF-8 path"),
        "--tolerant",
    ]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(
        text.contains(
            "quality: rows=8 accepted=4 rejected=4 duplicates=1 out_of_order=1 invalid=2"
        )
    );
    assert!(text.contains("skipped 4 duplicate"));
    assert!(text.contains("skipped 5 misaligned"));
    assert_eq!(fixture.last_request().path, "/catalog/datasets/tolerant");
}
