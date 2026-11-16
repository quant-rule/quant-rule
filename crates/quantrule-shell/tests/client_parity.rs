//! Cross-client parity: one fixture, two clients, one normalized view.
//!
//! The shell runs the real command surface with `--json` and the fixed
//! `default` profile against the in-process fixture server. The browser test
//! (`apps/web/src/lib/clientParity.test.ts`) reads the same
//! `tests/fixtures/client-parity/api.json` through its API layer. Both must
//! build `view.json`; a value either client changes or drops fails that suite.
//!
//! No test starts a gateway, reads the network, or depends on wall-clock time.

mod support;

use serde_json::{Value, json};
use support::{Fixture, client_parity_api, client_parity_view};

const PROFILE: &str = "default";

/// Run one shell command with the fixed profile and decode its JSON view.
fn command(fixture: &Fixture, args: &[&str]) -> Value {
    let mut argv = vec!["--json", "--profile", PROFILE];
    argv.extend_from_slice(args);
    let output = fixture.run(&argv);
    assert!(
        output.status.success(),
        "command {args:?} failed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("the shell prints one JSON value")
}

/// The command names the shell registers, read from `help all`.
///
/// The help renderer owns the registry, so a command added without a browser
/// control is visible here rather than only in a unit test.
fn shell_commands(fixture: &Fixture) -> Vec<String> {
    let output = fixture.run(&["help", "all"]);
    assert!(
        output.status.success(),
        "help all failed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("help is UTF-8");
    let mut names = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.is_empty() || line.starts_with("  ") {
            continue;
        }
        if let Some(next) = lines.next()
            && next.starts_with("  usage:")
        {
            names.push(line.to_owned());
        }
    }
    names
}

/// The normalized view the shell presents for the shared fixture.
fn shell_view(fixture: &Fixture) -> Value {
    let api = client_parity_api();
    let dataset_id = api["dataset_id"].as_str().expect("the fixture dataset id");
    let config_id = api["config_id"].as_str().expect("the fixture config id");
    let run_id = api["run_id"].as_str().expect("the fixture run id");
    let timeframe = api["timeframe"].as_str().expect("the fixture timeframe");

    let commands = shell_commands(fixture);
    let health = command(fixture, &["health"]);
    let datasets = command(fixture, &["dataset", "list"]);
    let dataset = command(fixture, &["dataset", "show", dataset_id]);
    let configs = command(fixture, &["config", "list"]);
    let config = command(fixture, &["config", "show", config_id]);
    let runs = command(fixture, &["run", "list"]);
    let run = command(fixture, &["run", "status", run_id]);
    let profiles = command(fixture, &["profile", "list"]);
    let candles = command(fixture, &["inspect", "candles", dataset_id]);
    let timeframes = command(fixture, &["inspect", "timeframes", dataset_id]);
    let events = command(
        fixture,
        &["inspect", "events", dataset_id, "--timeframe", timeframe],
    );
    let digest = command(fixture, &["verify", run_id]);
    let directory = fixture.fetch_directory();
    let fetched = command(
        fixture,
        &["run", "fetch", run_id, &directory.display().to_string()],
    );
    let artifact = &fetched["artifacts"][0];

    json!({
        "profile": PROFILE,
        "commands": commands,
        "health": health,
        "datasets": datasets,
        "dataset": dataset,
        "configs": configs,
        "config": config,
        "runs": runs,
        "run": run,
        "candles": candles,
        "timeframes": timeframes,
        "events": events,
        "digest": digest,
        "artifact": {
            "name": artifact["name"],
            "bytes": artifact["bytes"],
            "sha256": artifact["sha256"],
        },
        "profiles": profiles,
    })
}

/// Compare two views, returning a readable difference when they diverge.
fn difference(left: &Value, right: &Value) -> Option<String> {
    if left == right {
        return None;
    }
    Some(format!(
        "the client views differ\nshell:\n{}\nexpected:\n{}",
        serde_json::to_string_pretty(left).expect("the shell view is JSON"),
        serde_json::to_string_pretty(right).expect("the expected view is JSON"),
    ))
}

#[test]
fn the_shell_view_matches_the_shared_client_parity_view() {
    let fixture = Fixture::start_parity();
    let view = shell_view(&fixture);
    if let Some(difference) = difference(&view, &client_parity_view()) {
        panic!("{difference}");
    }

    let requests = fixture.requests();
    assert_eq!(requests.len(), 14, "one request per parity operation");
    let api = client_parity_api();
    let dataset_id = api["dataset_id"].as_str().expect("the fixture dataset id");
    let run_id = api["run_id"].as_str().expect("the fixture run id");
    let paths: Vec<String> = requests
        .iter()
        .map(|request| request.path.clone())
        .collect();
    assert_eq!(
        paths,
        vec![
            String::from("/health"),
            String::from("/catalog/datasets"),
            format!("/catalog/datasets/{dataset_id}"),
            String::from("/catalog/configs"),
            String::from("/catalog/configs"),
            String::from("/runs"),
            format!("/runs/{run_id}"),
            String::from("/profiles"),
            String::from("/inspect/candles"),
            String::from("/inspect/timeframes"),
            String::from("/inspect/events"),
            format!("/runs/{run_id}/digest"),
            format!("/runs/{run_id}"),
            format!("/runs/{run_id}/artifacts/results.json"),
        ]
    );
    for request in &requests {
        assert_eq!(request.method, "GET");
        assert_eq!(
            request.header("x-quantrule-profile"),
            Some(PROFILE),
            "{} must carry the fixed profile",
            request.path
        );
    }
}

#[test]
fn a_seeded_shell_divergence_fails_the_comparison() {
    let fixture = Fixture::start_parity();
    let mut view = shell_view(&fixture);
    view["health"]["queue_depth"] = json!(3);

    let difference =
        difference(&view, &client_parity_view()).expect("a changed queue depth must be rejected");
    assert!(difference.contains("\"queue_depth\": 3"), "{difference}");
}
