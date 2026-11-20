//! Simulation command output tests.

use serde_json::json;

use crate::cases::assertions::{assert_success, stdout};
use crate::support::{Fixture, digest};

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
        "--inputs",
        "EMA stack and RSI zone",
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
            "inputs": "EMA stack and RSI zone",
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
        "check: pass\nevents: 31\nmean_net: 0.01\nhit_rate: 0.6\ncontrol: 60\ncontrol_mean_net: 0.0\ncontrol_hit_rate: 0.5\npositive_half_years: 3\npassed: true\nside long: survives\n"
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
