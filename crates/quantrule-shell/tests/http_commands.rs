//! Printed-output tests for the HTTP shell surface.
//!
//! Every test runs the real `quantrule` binary against the in-process fixture
//! and asserts on stdout, stderr, exit status, and the request the fixture saw.
//! No test reads `data/`, a live network, or the wall clock.

mod support;

use std::fs;
use std::process::Output;

use serde_json::json;
use support::{Fixture, canonical_digest, config_hash, data_version, digest, image_digest, run_id};

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}; stderr: {}",
        output.status.code(),
        stderr(output)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "expected failure; stdout: {}",
        stdout(output)
    );
}

#[test]
fn health_prints_the_aggregate_and_sorted_services() {
    let fixture = Fixture::start();
    let output = fixture.run(&["health"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!(
            "api: quantrule.api/v1\nstate: degraded\nqueue_depth: 2\nworkers: 1\n\
             image_digest: {}\nservice minio: degraded (slow)\nservice postgres: healthy\n",
            image_digest()
        )
    );
    let request = fixture.last_request();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/health");
    assert_eq!(request.header("x-quantrule-profile"), Some("default"));
}

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

#[test]
fn config_list_and_show_print_validity_without_inventing_a_hash() {
    let fixture = Fixture::start();
    let list = fixture.run(&["config", "list"]);
    assert_success(&list);
    assert_eq!(
        stdout(&list),
        format!(
            "configs: 2\ncfg_abc name=run origin=repository valid=true message=none hash={}\n\
             cfg_bad name=broken origin=upload valid=false message=portfolio.initial_cash must be positive hash=none\n",
            config_hash()
        )
    );

    let show = fixture.run(&["config", "show", "cfg_bad"]);
    assert_success(&show);
    assert_eq!(
        stdout(&show),
        "config_id: cfg_bad\nname: broken\norigin: upload\nvalid: false\n\
         message: portfolio.initial_cash must be positive\nconfig_hash: none\n"
    );
}

#[test]
fn run_submit_sends_the_contract_body() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "submit", &data_version(), "cfg_abc"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!("run_id: {}\nstatus: queued\ncreated: true\n", run_id())
    );
    let request = fixture.last_request();
    assert_eq!(request.method, "POST");
    assert_eq!(request.path, "/runs");
    assert_eq!(
        request.json(),
        json!({ "data_version": data_version(), "config_id": "cfg_abc" })
    );
}

#[test]
fn run_status_prints_stages_provenance_artifacts_and_digest() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "status", &run_id()]);
    assert_success(&output);
    let text = stdout(&output);
    assert!(text.starts_with(&format!("run_id: {}\nstatus: succeeded\n", run_id())));
    assert!(text.contains("stage data: done 1200ms"));
    assert!(text.contains("stage evidence: done 120ms"));
    assert!(text.contains(&format!("digest: {}", canonical_digest())));
    assert!(text.ends_with("error: none\n"));
}

#[test]
fn run_list_passes_the_filters_and_sorts_the_rows() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "list", "--status", "succeeded", "--limit", "5"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!(
            "runs: 1\n{} status=succeeded config=cfg_abc data={} submitted=2026-11-13T12:00:00Z elapsed=1200ms\n",
            run_id(),
            data_version()
        )
    );
    assert_eq!(
        fixture
            .last_request()
            .query
            .get("status")
            .map(String::as_str),
        Some("succeeded")
    );
    assert_eq!(
        fixture
            .last_request()
            .query
            .get("limit")
            .map(String::as_str),
        Some("5")
    );
}

#[test]
fn run_fetch_writes_every_artifact_and_verifies_its_digest() {
    let fixture = Fixture::start();
    let directory = fixture.home().join("artifacts");
    let output = fixture.run(&[
        "run",
        "fetch",
        &run_id(),
        directory.to_str().expect("UTF-8 path"),
    ]);
    assert_success(&output);
    let written = fs::read(directory.join("results.json")).expect("the artifact is written");
    assert_eq!(written, b"{\"ok\":true}\n");
    let text = stdout(&output);
    assert!(text.contains("artifact results.json: bytes="));
    assert!(text.contains("written="));
    assert_eq!(
        fixture.last_request().path,
        format!("/runs/{}/artifacts/results.json", run_id())
    );
}

#[test]
fn run_cancel_prints_the_new_state() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "cancel", &run_id()]);
    assert_success(&output);
    assert!(stdout(&output).contains("status: cancelled"));
    assert_eq!(
        fixture.last_request().path,
        format!("/runs/{}/cancel", run_id())
    );
}

#[test]
fn inspect_candles_and_timeframes_print_coverage() {
    let fixture = Fixture::start();
    let candles = fixture.run(&["inspect", "candles", &data_version()]);
    assert_success(&candles);
    assert_eq!(
        stdout(&candles),
        format!(
            "data_version: {}\nname: candles.csv\ncandles: 3\nfirst_bar: 1704067200\n\
             last_bar: 1704067320\nbytes: 96\n",
            data_version()
        )
    );

    let timeframes = fixture.run(&["inspect", "timeframes", &data_version()]);
    assert_success(&timeframes);
    assert_eq!(
        stdout(&timeframes),
        format!(
            "data_version: {}\ntimeframes: 3\n\
             1m: bars=3 partial_buckets=0 first_bar=1704067200 last_bar=1704067320\n\
             5m: bars=1 partial_buckets=1 first_bar=1704067200 last_bar=1704067200\n\
             15m: bars=0 partial_buckets=1 first_bar=- last_bar=-\n",
            data_version()
        )
    );
}

#[test]
fn inspect_events_prints_one_json_record_per_line() {
    let fixture = Fixture::start();
    let output = fixture.run(&["inspect", "events", &data_version(), "--timeframe", "5m"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "{\"bar\":10,\"source\":2,\"type\":1}\n{\"bar\":11,\"source\":3,\"type\":2}\n"
    );
    let request = fixture.last_request();
    assert_eq!(
        request.query.get("timeframe").map(String::as_str),
        Some("5m")
    );
    assert_eq!(
        request.query.get("data_version").map(String::as_str),
        Some(data_version().as_str())
    );
}

#[test]
fn verify_prints_the_server_digest() {
    let fixture = Fixture::start();
    let output = fixture.run(&["verify", &run_id()]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        format!("run_id: {}\ndigest: {}\n", run_id(), canonical_digest())
    );
    assert_eq!(
        fixture.last_request().path,
        format!("/runs/{}/digest", run_id())
    );
}

#[test]
fn profile_list_marks_the_active_profile() {
    let fixture = Fixture::start();
    let output = fixture.run(&["profile", "list"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "active: default\nprofiles: 2\n\
         alpha name=Alpha datasets=1 runs=0 simulations=0\n\
         default name=Default datasets=1 runs=1 simulations=1\n"
    );
}

#[test]
fn profile_create_derives_a_slug_and_sends_the_contract_body() {
    let fixture = Fixture::start();
    let output = fixture.run(&["profile", "create", "Research Lab"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "created: true\nprofile_id: research-lab\nname: Research Lab\n\
         datasets: 0\nruns: 0\nsimulations: 0\n"
    );
    assert_eq!(
        fixture.last_request().json(),
        json!({ "profile_id": "research-lab", "name": "Research Lab" })
    );
}

#[test]
fn profile_use_confirms_the_profile_and_stores_it() {
    let fixture = Fixture::start();
    let output = fixture.run(&["profile", "use", "alpha"]);
    assert_success(&output);
    assert_eq!(stdout(&output), "active: alpha\n");
    assert_eq!(fixture.stored_profile().as_deref(), Some("alpha"));

    fixture.clear_requests();
    let listed = fixture.run(&["dataset", "list"]);
    assert_success(&listed);
    assert_eq!(
        fixture.last_request().header("x-quantrule-profile"),
        Some("alpha")
    );
}

#[test]
fn an_explicit_profile_wins_and_is_never_stored() {
    let fixture = Fixture::start();
    fixture.store_profile("alpha");
    let output = fixture.run(&["--profile", "default", "dataset", "list"]);
    assert_success(&output);
    assert_eq!(
        fixture.last_request().header("x-quantrule-profile"),
        Some("default")
    );
    assert_eq!(fixture.stored_profile().as_deref(), Some("alpha"));
}

#[test]
fn profile_use_refuses_a_global_profile_and_leaves_the_default() {
    let fixture = Fixture::start();
    fixture.store_profile("alpha");
    let output = fixture.run(&["--profile", "alpha", "profile", "use", "default"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("profile use cannot run with --profile"));
    assert_eq!(fixture.stored_profile().as_deref(), Some("alpha"));
}

#[test]
fn an_unknown_stored_profile_is_not_replaced_by_the_shell() {
    let fixture = Fixture::start();
    fixture.store_profile("ghost");
    let output = fixture.run(&["dataset", "list"]);
    assert_failure(&output);
    assert!(
        stderr(&output).contains("profile.not_found: profile ghost is unknown"),
        "{}",
        stderr(&output)
    );
    assert_eq!(fixture.stored_profile().as_deref(), Some("ghost"));
}

#[test]
fn a_missing_profile_is_refused_by_profile_use_without_a_write() {
    let fixture = Fixture::start();
    let output = fixture.run(&["profile", "use", "ghost"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("profile.not_found"));
    assert_eq!(fixture.stored_profile(), None);
}

#[test]
fn simulation_list_prints_sorted_rows() {
    let fixture = Fixture::start();
    let output = fixture.run(&["simulation", "list", "--status", "draft"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "simulations: 2\nsim-1 status=draft windows=3 marks=0 premise=first\n\
         sim-2 status=marking windows=4 marks=2 premise=second\n"
    );
    assert_eq!(
        fixture
            .last_request()
            .query
            .get("status")
            .map(String::as_str),
        Some("draft")
    );
}

#[test]
fn simulation_create_sends_the_contract_body() {
    let fixture = Fixture::start();
    let bundle = digest('1');
    let output = fixture.run(&[
        "simulation",
        "create",
        "--premise",
        "a higher low holds",
        "--bundle",
        &bundle,
        "--seed",
        "7",
        "--criteria",
        "good entry within five bars",
    ]);
    assert_success(&output);
    assert_eq!(stdout(&output), "sim_id: sim-3\n");
    assert_eq!(
        fixture.last_request().json(),
        json!({
            "premise": "a higher low holds",
            "bundle": bundle,
            "seed": 7,
            "criteria": "good entry within five bars",
        })
    );
}

#[test]
fn simulation_windows_prints_the_gateway_selection() {
    let fixture = Fixture::start();
    let output = fixture.run(&["simulation", "windows", "sim-1"]);
    assert_success(&output);
    assert_eq!(
        stdout(&output),
        "{\"sim_id\":\"sim-1\",\"windows\":[{\"end\":120,\"start\":0,\"window\":1},{\"end\":240,\"start\":121,\"window\":2}]}\n"
    );
    assert_eq!(
        fixture.last_request().path,
        "/simulation/runs/sim-1/windows"
    );
}

#[test]
fn simulation_mark_sends_every_mark_field() {
    let fixture = Fixture::start();
    let output = fixture.run(&[
        "simulation",
        "mark",
        "sim-1",
        "--chart",
        "w1",
        "--bar",
        "12",
        "--action",
        "entry",
        "--quality",
        "good",
        "--side",
        "long",
        "--note",
        "higher low",
    ]);
    assert_success(&output);
    assert_eq!(stdout(&output), "mark_id: mark-1\n");
    assert_eq!(
        fixture.last_request().json(),
        json!({
            "chart": "w1",
            "bar": 12,
            "action": "entry",
            "quality": "good",
            "side": "long",
            "note": "higher low",
        })
    );
}

#[test]
fn simulation_marks_and_oracle_print_their_records() {
    let fixture = Fixture::start();
    let marked = fixture.run(&[
        "simulation",
        "mark",
        "sim-1",
        "--chart",
        "w1",
        "--bar",
        "12",
        "--action",
        "entry",
        "--quality",
        "good",
        "--side",
        "long",
    ]);
    assert_success(&marked);

    let marks = fixture.run(&["simulation", "marks", "sim-1", "--origin", "human"]);
    assert_success(&marks);
    let text = stdout(&marks);
    assert!(text.contains("\"mark_id\":\"mark-1\""), "{text}");
    assert!(text.contains("\"origin\":\"human\""), "{text}");
    assert_eq!(
        fixture
            .last_request()
            .query
            .get("origin")
            .map(String::as_str),
        Some("human")
    );

    let oracle = fixture.run(&["simulation", "oracle", "sim-1"]);
    assert_success(&oracle);
    assert!(stdout(&oracle).contains("\"origin\":\"oracle\""));
}

#[test]
fn simulation_check_and_decide_print_the_gateway_result() {
    let fixture = Fixture::start();
    let check = fixture.run(&["simulation", "check", "sim-1"]);
    assert_success(&check);
    assert_eq!(
        stdout(&check),
        "{\"events\":31,\"sim_id\":\"sim-1\",\"verdict\":\"pass\"}\n"
    );

    let decide = fixture.run(&[
        "simulation",
        "decide",
        "sim-1",
        "test",
        "--note",
        "looks stable",
    ]);
    assert_success(&decide);
    assert_eq!(
        stdout(&decide),
        "{\"decision\":\"test\",\"note\":\"looks stable\",\"sim_id\":\"sim-1\"}\n"
    );
    assert_eq!(
        fixture.last_request().json(),
        json!({ "decision": "test", "note": "looks stable" })
    );
}

#[test]
fn the_error_envelope_keeps_its_stable_code() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "status", &digest('9')]);
    assert_failure(&output);
    assert_eq!(stderr(&output), "error: run.not_found: run is unknown\n");
    assert_eq!(stdout(&output), "");
}

#[test]
fn a_parse_error_names_the_usage_without_a_request() {
    let fixture = Fixture::start();
    let output = fixture.run(&["run", "list", "--limit", "0"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("--limit must be between 1 and 200"));
    assert!(fixture.requests().is_empty());
}

#[test]
fn a_missing_required_option_is_refused_without_a_request() {
    let fixture = Fixture::start();
    let output = fixture.run(&["simulation", "create", "--premise", "p"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("--bundle is required"));
    assert!(fixture.requests().is_empty());
}

#[test]
fn the_same_command_prints_the_same_bytes() {
    let fixture = Fixture::start();
    let first = fixture.run(&["run", "status", &run_id()]);
    let second = fixture.run(&["run", "status", &run_id()]);
    assert_success(&first);
    assert_success(&second);
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(fixture.requests().len(), 2);
}

#[test]
fn interactive_and_non_interactive_output_are_identical() {
    let fixture = Fixture::start();
    let non_interactive = fixture.run(&["dataset", "list"]);
    let interactive = fixture.run_interactive("dataset list\nquit\n");
    assert_success(&non_interactive);
    assert_success(&interactive);
    assert_eq!(interactive.stdout, non_interactive.stdout);
}

#[test]
fn json_output_matches_the_gateway_value() {
    let fixture = Fixture::start();
    let output = fixture.run(&["--json", "run", "status", &run_id()]);
    assert_success(&output);
    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("the output is one JSON value");
    assert_eq!(value["run_id"], run_id());
    assert_eq!(value["provenance"]["model_version"], "M1.0");
    assert_eq!(value["digest"], canonical_digest());
}

#[test]
fn help_does_not_need_a_gateway() {
    let fixture = Fixture::start();
    let output = fixture.run(&["help", "run", "submit"]);
    assert_success(&output);
    assert!(stdout(&output).starts_with("run submit\n"));
    assert!(stdout(&output).contains("usage:  run submit <data_version> <config_id>"));
    assert!(fixture.requests().is_empty());
}

#[test]
fn a_retired_stage_command_is_unknown() {
    let fixture = Fixture::start();
    let output = fixture.run(&["backtest", "run", "--config", "x"]);
    assert_failure(&output);
    assert!(stderr(&output).contains("unknown command 'backtest'"));
    assert!(fixture.requests().is_empty());
}
