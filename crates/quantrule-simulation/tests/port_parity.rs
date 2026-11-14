//! Port-parity comparison for PORT-001.
//!
//! The fixture is captured from the retired Python simulation harness at commit
//! b63b9015. Every case names the stage it exercises; this test runs the Rust
//! port on the case input and compares the complete expected object, including
//! every numeric value, to the fixture tolerance.

// port-parity test: PORT-001
const FIXTURE: &str = "docs/port-parity/PORT-001.golden.json";

use std::collections::BTreeSet;
use std::path::Path;

use quantrule_simulation::{check, exit, marks, meta, oracle, review, window};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
struct Fixture {
    schema: String,
    id: String,
    source: String,
    captured: String,
    tolerance: f64,
    cases: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    name: String,
    stage: String,
    #[serde(default)]
    sim: Vec<String>,
    #[serde(default)]
    notes: String,
    input: Value,
    expected: Value,
}

#[derive(Debug, Deserialize)]
struct CheckInput {
    events: Vec<f64>,
    control: Vec<f64>,
    half_years: Vec<f64>,
    criteria: check::Criteria,
}

#[derive(Debug, Deserialize)]
struct WindowInput {
    span: u64,
    count: u32,
    width: u64,
    seed: u64,
}

#[derive(Debug, Deserialize)]
struct MarksInput {
    marks: Vec<marks::Mark>,
    window: window::Window,
}

#[derive(Debug, Deserialize)]
struct OracleInput {
    window: window::Window,
    side: marks::Side,
    bars: Vec<oracle::Bar>,
}

#[derive(Debug, Deserialize)]
struct ReviewInput {
    human: Vec<marks::Mark>,
    oracle: Vec<marks::Mark>,
}

#[derive(Debug, Deserialize)]
struct ExitInput {
    entries: Vec<ExitEntry>,
}

#[derive(Debug, Deserialize)]
struct ExitEntry {
    entry: f64,
    stop: f64,
    side: marks::Side,
    bars: Vec<exit::Bar>,
    horizon: usize,
    policies: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MetaInput {
    bars: Vec<meta::Bar>,
    marks: Vec<meta::Mark>,
    config: meta::Config,
}

fn fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(FIXTURE)
}

fn load_fixture() -> Fixture {
    let text = std::fs::read_to_string(fixture_path()).expect("read fixture");
    serde_json::from_str(&text).expect("parse fixture")
}

fn run_stage(stage: &str, input: &Value) -> Value {
    match stage {
        "meta" => {
            let input: MetaInput = serde_json::from_value(input.clone()).expect("meta input shape");
            let result = meta::build(&input.bars, &input.marks, &input.config).expect("meta");
            serde_json::to_value(result).expect("meta output shape")
        }
        "check" => {
            let input: CheckInput =
                serde_json::from_value(input.clone()).expect("check input shape");
            let result = check::check(
                &input.events,
                &input.control,
                &input.half_years,
                input.criteria,
            )
            .expect("check");
            let verdict = if result.events < input.criteria.min_events
                || result.control < input.criteria.min_events
            {
                "underpowered"
            } else if result.passed {
                "survives"
            } else {
                "no_pattern"
            };
            json!({
                "verdict": verdict,
                "filtered": {
                    "n": result.events,
                    "mean_net": result.mean_net,
                    "hit_rate": result.hit_rate,
                    "positive_half_years": result.positive_half_years,
                },
                "control": {
                    "n": result.control,
                    "mean_net": result.control_mean_net,
                    "hit_rate": result.control_hit_rate,
                },
            })
        }
        "exit" => {
            let input: ExitInput = serde_json::from_value(input.clone()).expect("exit input shape");
            let mut entries = Vec::new();
            for entry in input.entries {
                let mut results = serde_json::Map::new();
                for policy in &entry.policies {
                    let name = policy.clone();
                    let policy = match policy.as_str() {
                        "fixed_2r" => exit::ExitPolicy::Fixed { target_r: 2.0 },
                        "time" => exit::ExitPolicy::Time {
                            bars: entry.horizon,
                        },
                        "partial_1r" => exit::ExitPolicy::Partial {
                            first_r: 1.0,
                            final_r: 2.0,
                        },
                        other => panic!("unknown exit policy {other}"),
                    };
                    let result =
                        exit::evaluate(&entry.bars, entry.entry, entry.stop, entry.side, policy)
                            .expect("exit");
                    results.insert(name, json!({"r": result.r, "bars": result.bars}));
                }
                entries.push(Value::Object(results));
            }
            json!({"entries": entries})
        }
        "window" => {
            let input: WindowInput =
                serde_json::from_value(input.clone()).expect("window input shape");
            let windows =
                window::select(input.span, input.count, input.width, input.seed).expect("window");
            json!({"windows": windows})
        }
        "marks" => {
            let input: MarksInput =
                serde_json::from_value(input.clone()).expect("marks input shape");
            let accepted = marks::Mark::validate(&input.marks, &[input.window]).is_ok();
            let bars: Vec<u64> = if accepted {
                input.marks.iter().map(|mark| mark.bar).collect()
            } else {
                Vec::new()
            };
            json!({"accepted": accepted, "bars": bars})
        }
        "oracle" => {
            let input: OracleInput =
                serde_json::from_value(input.clone()).expect("oracle input shape");
            let labels = oracle::label(&input.bars, input.window, input.side).expect("oracle");
            let labels: Vec<Value> = labels
                .iter()
                .map(|mark| {
                    json!({
                        "bar": mark.bar,
                        "action": mark.action,
                        "quality": mark.quality,
                        "side": mark.side,
                    })
                })
                .collect();
            json!({"marks": labels})
        }
        "review" => {
            let input: ReviewInput =
                serde_json::from_value(input.clone()).expect("review input shape");
            let result = review::review(&input.human, &input.oracle);
            json!({"human": result.human, "oracle": result.oracle})
        }
        other => panic!("unknown stage {other}"),
    }
}

fn assert_matches(expected: &Value, actual: &Value, tolerance: f64, path: &str) {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
            assert_eq!(
                expected.len(),
                actual.len(),
                "{path}: expected {} keys, got {}",
                expected.len(),
                actual.len()
            );
            for (key, value) in expected {
                let actual = actual
                    .get(key)
                    .unwrap_or_else(|| panic!("{path}.{key}: missing from the Rust output"));
                assert_matches(value, actual, tolerance, &format!("{path}.{key}"));
            }
        }
        (Value::Array(expected), Value::Array(actual)) => {
            assert_eq!(
                expected.len(),
                actual.len(),
                "{path}: expected {} items, got {}",
                expected.len(),
                actual.len()
            );
            for (index, (expected, actual)) in expected.iter().zip(actual).enumerate() {
                assert_matches(expected, actual, tolerance, &format!("{path}[{index}]"));
            }
        }
        (Value::Number(expected), Value::Number(actual)) => {
            let expected = expected
                .as_f64()
                .unwrap_or_else(|| panic!("{path}: expected is not a float"));
            let actual = actual
                .as_f64()
                .unwrap_or_else(|| panic!("{path}: actual is not a float"));
            let difference = (expected - actual).abs();
            assert!(
                difference <= tolerance,
                "{path}: expected {expected}, got {actual}, difference {difference} > {tolerance}"
            );
        }
        _ => assert_eq!(expected, actual, "{path}"),
    }
}

#[test]
fn the_fixture_envelope_is_valid() {
    let fixture = load_fixture();
    assert_eq!(fixture.schema, "quantrule.port-parity-fixture/v1");
    assert_eq!(fixture.id, "PORT-001");
    assert!(!fixture.source.is_empty());
    assert_eq!(fixture.captured, "2026-11-14");
    assert!(fixture.tolerance > 0.0 && fixture.tolerance <= 1e-9);
    assert!(!fixture.cases.is_empty());
    let mut names = BTreeSet::new();
    for case in &fixture.cases {
        assert!(!case.name.is_empty());
        assert!(
            names.insert(case.name.clone()),
            "duplicate case {}",
            case.name
        );
        assert!(!case.stage.is_empty());
        assert!(!case.notes.is_empty(), "{} needs a note", case.name);
    }
}

#[test]
fn the_fixture_covers_every_sim_run() {
    let fixture = load_fixture();
    let covered: BTreeSet<&str> = fixture
        .cases
        .iter()
        .flat_map(|case| case.sim.iter().map(String::as_str))
        .collect();
    for id in [
        "SIM-01", "SIM-02", "SIM-03", "SIM-04", "SIM-05", "SIM-06", "SIM-07", "SIM-08", "SIM-09",
        "SIM-10",
    ] {
        assert!(covered.contains(id), "the fixture does not cover {id}");
    }
}

#[test]
fn every_case_matches_the_python_reference() {
    let fixture = load_fixture();
    for case in &fixture.cases {
        let actual = run_stage(&case.stage, &case.input);
        assert_matches(
            &case.expected,
            &actual,
            fixture.tolerance,
            &format!("{} ({})", case.name, case.stage),
        );
    }
}

#[test]
fn a_reordered_input_produces_the_same_output() {
    let fixture = load_fixture();
    for case in &fixture.cases {
        let mut reordered = case.input.clone();
        match case.stage.as_str() {
            "meta" => {
                let marks = reordered
                    .get_mut("marks")
                    .and_then(Value::as_array_mut)
                    .expect("meta marks");
                marks.reverse();
            }
            "check" => {
                for field in ["events", "control"] {
                    let values = reordered
                        .get_mut(field)
                        .and_then(Value::as_array_mut)
                        .expect("check values");
                    values.reverse();
                }
            }
            "oracle" => {
                let bars = reordered
                    .get_mut("bars")
                    .and_then(Value::as_array_mut)
                    .expect("oracle bars");
                bars.reverse();
            }
            "review" => {
                for field in ["human", "oracle"] {
                    if let Some(values) = reordered.get_mut(field).and_then(Value::as_array_mut) {
                        values.reverse();
                    }
                }
            }
            "window" | "exit" | "marks" => {}
            other => panic!("unknown stage {other}"),
        }
        let first = run_stage(&case.stage, &case.input);
        let second = run_stage(&case.stage, &reordered);
        assert_eq!(first, second, "{} is not order-independent", case.name);
    }
}

#[test]
fn the_same_case_is_deterministic() {
    let fixture = load_fixture();
    for case in &fixture.cases {
        let first = run_stage(&case.stage, &case.input);
        let second = run_stage(&case.stage, &case.input);
        assert_eq!(first, second, "{} changed between runs", case.name);
    }
}
