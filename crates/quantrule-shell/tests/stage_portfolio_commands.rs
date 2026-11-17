//! The `stage run portfolio` printed-output contract.

mod support;

use serde_json::json;
use support::{Fixture, portfolio_decisions_artifact_id, portfolio_stage_id};

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("the output is UTF-8")
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "expected success, got {:?}; stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn stage_run_portfolio_and_fetch_render_the_nested_bundles() {
    let fixture = Fixture::start();
    let decisions = portfolio_decisions_artifact_id();
    let run = fixture.run(&["stage", "run", "portfolio", &decisions, "cfg_abc"]);
    assert_success(&run);
    assert_eq!(
        stdout(&run),
        format!(
            "stage_run_id: {}\nstate: queued\ncreated: true\n",
            portfolio_stage_id()
        )
    );
    assert_eq!(fixture.last_request().path, "/stages/portfolio/runs");
    assert_eq!(
        fixture.last_request().json(),
        json!({
            "kind": "portfolio",
            "config_id": "cfg_abc",
            "input": {
                "kind": "decisions_artifact",
                "decisions_artifact_id": decisions,
            },
        })
    );

    let show = fixture.run(&["stage", "show", &portfolio_stage_id()]);
    assert_success(&show);
    let text = stdout(&show);
    assert!(text.contains("kind: portfolio"));
    assert!(text.contains(&format!(
        "input: decisions_artifact {}",
        portfolio_decisions_artifact_id()
    )));
    assert!(text.contains("artifact execution/orders.csv:"));
    assert!(text.contains("artifact portfolio/metrics.json:"));

    let directory = fixture.home().join("portfolio-stage");
    let fetch = fixture.run(&[
        "stage",
        "fetch",
        &portfolio_stage_id(),
        directory.to_str().expect("UTF-8 path"),
    ]);
    assert_success(&fetch);
    assert!(directory.join("execution/orders.csv").is_file());
    assert!(directory.join("portfolio/metrics.json").is_file());
    let text = stdout(&fetch);
    assert!(text.contains("artifact execution/orders.csv:"));
    assert!(text.contains("artifact portfolio/metrics.json:"));
}
