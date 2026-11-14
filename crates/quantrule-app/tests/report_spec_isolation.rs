//! Spec isolation over the shared report fixture.
//!
//! The property belongs to the application service, which owns the model-report
//! use case and the spec file boundary. The fixture set is shared with
//! `tools/research/test_spec_preflight.py`, so the bytes are read from one place
//! instead of duplicated.
//!
//! No test reads `data/`, the network, or the wall clock.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use quantrule_app::service::AppService;
use quantrule_app::stages::model::ModelReportRequest;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../quantrule-shell/tests/fixtures")
        .join(name)
}

fn output_dir(name: &str) -> PathBuf {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("report-spec-isolation")
        .join(name);
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("the output directory is created");
    path
}

#[test]
fn two_model_specs_with_disjoint_active_events_stay_disjoint() {
    let config = fixture("config-run/run.json");
    let mut codes = Vec::new();
    for (name, spec_name) in [("rsi", "spec-v2-rsi.json"), ("ema", "spec-v2-ema.json")] {
        let out = output_dir(&format!("isolation-{name}"));
        let spec = fixture(&format!("config-run/{spec_name}"));
        AppService::run_model_report(&ModelReportRequest {
            config: &config,
            spec: Some(&spec),
            out: Some(&out),
        })
        .expect("the model report succeeds");
        let csv = String::from_utf8(
            fs::read(out.join("model").join("events.csv")).expect("events.csv is readable"),
        )
        .expect("events.csv is UTF-8");
        let mut lines = csv.lines();
        lines.next();
        codes.push(
            lines
                .map(|line| {
                    line.split(',')
                        .nth(2)
                        .expect("an event_code column")
                        .parse::<u16>()
                        .expect("a numeric event code")
                })
                .collect::<BTreeSet<_>>(),
        );
    }
    assert!(codes[0].contains(&8), "the RSI threshold event is missing");
    assert!(codes[1].contains(&9), "the EMA pair event is missing");
    assert!(
        codes[0].is_disjoint(&codes[1]),
        "the two active-event selections overlap"
    );
}
