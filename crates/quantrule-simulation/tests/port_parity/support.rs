//! Fixture decoding and stage dispatch for the PORT-001 comparison.

use std::collections::{BTreeMap, BTreeSet};

use quantrule_simulation::{
    Bar, Bundle, Catalog, Chart, CheckConfig, CheckRows, Control, Detail, Entry, Event, ExitConfig,
    Mark, MetaConfig, OracleSpec, WindowRecord, WindowSpec, build_check_from_rows,
    build_exit_with_entries, build_meta, check, label, review, select, write_marks,
};
use serde::Deserialize;
use serde_json::{Map, Value, json};

/// The golden fixture: one case per stage and SIM run.
#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub schema: String,
    pub id: String,
    pub source: String,
    pub captured: String,
    pub tolerance: f64,
    pub notes: String,
    pub datasets_file: String,
    pub cases: Vec<Case>,
}

/// One stage case.
#[derive(Debug, Deserialize)]
pub struct Case {
    pub name: String,
    pub stage: String,
    #[serde(default)]
    pub sim: Vec<String>,
    #[serde(default)]
    pub notes: String,
    pub input: Value,
    pub expected: Value,
}

/// The shared stage inputs.
#[derive(Debug, Deserialize)]
pub struct Datasets {
    pub schema: String,
    pub catalog: BTreeMap<String, String>,
    #[serde(rename = "label-series")]
    pub label_series: Table,
    #[serde(rename = "tail-bars")]
    pub tail_bars: Table,
    #[serde(rename = "meta-context")]
    pub meta_context: BTreeMap<String, Value>,
    #[serde(rename = "label-exit")]
    pub label_exit: BTreeMap<String, Value>,
}

/// A columnar table of numeric rows.
#[derive(Debug, Deserialize)]
pub struct Table {
    #[serde(default)]
    pub first_bar: u64,
    pub fields: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

impl Datasets {
    /// The event catalog.
    pub fn catalog(&self) -> Catalog {
        self.catalog
            .iter()
            .map(|(code, name)| (code.parse().expect("event code"), name.clone()))
            .collect()
    }
}

impl Table {
    /// Decode every row into a bar, absent fields defaulting.
    ///
    /// The bar identity is the table's first bar plus the row position; the
    /// tables carry no explicit identity because the ranges are contiguous.
    pub fn bars(&self) -> Vec<Bar> {
        self.rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                let mut bar = self.bar(row);
                bar.bar = self.first_bar + index as u64;
                bar
            })
            .collect()
    }

    /// Decode the rows as a `(bar, timestamp)` timeline.
    pub fn timeline(&self) -> Vec<(u64, i64)> {
        self.rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                (
                    self.first_bar + index as u64,
                    row[0].as_i64().expect("timestamp"),
                )
            })
            .collect()
    }

    /// Decode one row into a bar.
    pub fn bar(&self, row: &[Value]) -> Bar {
        let mut bar = Bar::default();
        for (field, value) in self.fields.iter().zip(row) {
            if value.is_null() {
                continue;
            }
            match field.as_str() {
                "bar" => bar.bar = value.as_u64().expect("bar"),
                "timestamp" => bar.timestamp = value.as_i64().expect("timestamp"),
                "open" => bar.open = value.as_f64().expect("open"),
                "high" => bar.high = value.as_f64().expect("high"),
                "low" => bar.low = value.as_f64().expect("low"),
                "close" => bar.close = value.as_f64().expect("close"),
                "volume" => bar.volume = value.as_f64().expect("volume"),
                "atr" => bar.atr = value.as_f64(),
                "ema_fast" => bar.ema_fast = value.as_f64(),
                "ema_mid" => bar.ema_mid = value.as_f64(),
                "ema_slow" => bar.ema_slow = value.as_f64(),
                "ema_stack" => bar.ema_stack = value.as_i64().map(|code| code as i32),
                "rsi_zone" => bar.rsi_zone = value.as_i64().map(|code| code as i32),
                "rsi" => bar.rsi = value.as_f64(),
                "rvol" => bar.rvol = value.as_f64(),
                "adx" => bar.adx = value.as_f64(),
                "atr_ratio" => bar.atr_ratio = value.as_f64(),
                "close_position" => bar.close_position = value.as_f64(),
                "upper_wick" => bar.upper_wick = value.as_f64(),
                "lower_wick" => bar.lower_wick = value.as_f64(),
                "body_ratio" => bar.body_ratio = value.as_f64(),
                "last_pivot_low" => bar.last_pivot_low = value.as_f64(),
                "last_pivot_high" => bar.last_pivot_high = value.as_f64(),
                other => panic!("unknown bar field {other}"),
            }
        }
        bar
    }
}

#[derive(Debug, Deserialize)]
struct WindowInput {
    spec: WindowSpec,
    #[serde(default)]
    timeline: Option<String>,
    #[serde(default)]
    bars: Option<Vec<(u64, i64)>>,
    #[serde(default)]
    events: Vec<Event>,
}

#[derive(Debug, Deserialize)]
struct OracleInput {
    spec: OracleSpec,
    run_id: String,
    #[serde(default)]
    series: Option<String>,
    #[serde(default)]
    bars: Option<Vec<Bar>>,
    windows: Vec<WindowIn>,
}

#[derive(Debug, Deserialize)]
struct WindowIn {
    window_id: String,
    start_index: usize,
    end_index: usize,
    candidate_index: usize,
}

#[derive(Debug, Deserialize)]
struct ReviewInput {
    run_id: String,
    premise: String,
    marks: Vec<Mark>,
    details: Vec<Detail>,
    bars: Vec<Bar>,
    events: Vec<Event>,
    focus_events: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CheckInput {
    config: CheckConfig,
    rows: Vec<CheckRowIn>,
    candidate_count: usize,
    data_version: String,
}

#[derive(Debug, Deserialize)]
struct CheckRowIn {
    event_bar: Vec<Value>,
    direction: Option<i32>,
    reference: Option<f64>,
    confirmation_close: Option<f64>,
    entry_index: usize,
    entry_time: i64,
    exit_index: usize,
    entry_open: f64,
    exit_close: f64,
}

#[derive(Debug, Deserialize)]
struct MetaInput {
    config: MetaConfig,
    marks: Vec<Mark>,
    series: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ExitInput {
    config: ExitConfig,
    marks: Vec<Mark>,
    check_entries: Vec<Entry>,
    series: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MarksInput {
    csv: String,
    charts: Vec<Chart>,
}

/// Run one case against the Rust port and return its output.
///
/// # Panics
///
/// Panics when the fixture input has the wrong shape or the stage returns an
/// error the fixture does not expect; a fixture defect is not a test skip.
pub fn run_stage(stage: &str, input: &Value, datasets: &Datasets, catalog: &Catalog) -> Value {
    match stage {
        "window" => run_window(input, datasets, catalog),
        "oracle" => run_oracle(input, datasets),
        "review" => run_review(input, datasets, catalog),
        "check" => run_check(input, datasets, catalog),
        "meta" => run_meta(input, datasets),
        "exit" => run_exit(input, datasets),
        "marks" => run_marks(input),
        other => panic!("unknown stage {other}"),
    }
}

fn run_window(input: &Value, datasets: &Datasets, catalog: &Catalog) -> Value {
    let input: WindowInput = serde_json::from_value(input.clone()).expect("window input");
    let bars = match (&input.timeline, &input.bars) {
        (Some(name), _) => datasets
            .table(name)
            .timeline()
            .iter()
            .map(|(bar, timestamp)| Bar::from_prices(*bar, *timestamp, 0.0, 0.0, 0.0, 0.0))
            .collect(),
        (None, Some(bars)) => bars
            .iter()
            .map(|(bar, timestamp)| Bar::from_prices(*bar, *timestamp, 0.0, 0.0, 0.0, 0.0))
            .collect(),
        _ => panic!("a window case needs a timeline or explicit bars"),
    };
    let bundle = Bundle::new(900, bars, input.events);
    let selection = select(&bundle, &input.spec, catalog).expect("window selection");
    serde_json::to_value(selection).expect("selection json")
}

fn run_oracle(input: &Value, datasets: &Datasets) -> Value {
    let input: OracleInput = serde_json::from_value(input.clone()).expect("oracle input");
    let bars = match (&input.series, &input.bars) {
        (Some(name), _) => datasets.table(name).bars(),
        (None, Some(bars)) => bars.clone(),
        _ => panic!("an oracle case needs a series or explicit bars"),
    };
    let bundle = Bundle::new(900, bars, Vec::new());
    let mut marks = Vec::new();
    let mut details = Vec::new();
    for window in &input.windows {
        let record = WindowRecord {
            window_id: window.window_id.clone(),
            start_index: window.start_index,
            end_index: window.end_index,
            start_bar: 0,
            end_bar: 0,
            start_time: String::new(),
            end_time: String::new(),
            candidate_index: window.candidate_index,
            candidate_bar: 0,
            candidate_time: String::new(),
            events: Vec::new(),
        };
        let chart = format!("{}-{}.png", input.run_id.to_lowercase(), window.window_id);
        let result = label(&bundle, &record, &chart, &input.spec, &input.run_id).expect("oracle");
        marks.extend(result.marks);
        details.extend(result.details);
    }
    json!({"marks": marks, "details": details})
}

fn run_review(input: &Value, _datasets: &Datasets, catalog: &Catalog) -> Value {
    let input: ReviewInput = serde_json::from_value(input.clone()).expect("review input");
    let bundle = Bundle::new(900, input.bars, input.events);
    let focus: BTreeSet<String> = input.focus_events.into_iter().collect();
    let review =
        review::review(&bundle, &input.marks, &input.details, catalog, &focus).expect("review");
    json!({"document": review.render(&input.run_id, &input.premise)})
}

fn run_check(input: &Value, datasets: &Datasets, catalog: &Catalog) -> Value {
    let input: CheckInput = serde_json::from_value(input.clone()).expect("check input");
    let rows = input.rows.iter().map(check_row).collect::<Vec<_>>();
    let control_rows = match input.config.control {
        Control::Bars => {
            let bars: Vec<Bar> = datasets
                .tail_bars
                .bars()
                .into_iter()
                .filter(|bar| {
                    bar.timestamp >= input.config.from && bar.timestamp <= input.config.to
                })
                .collect();
            let bundle = Bundle::new(900, bars, Vec::new());
            Some(check::bar_rows(
                &bundle,
                input.config.from,
                input.config.to,
                input.config.horizon_bars,
            ))
        }
        Control::Event => None,
    };
    let record = build_check_from_rows(
        &input.config,
        CheckRows {
            rows: &rows,
            control_rows: control_rows.as_deref(),
            candidate_count: input.candidate_count,
            data_version: &input.data_version,
        },
        catalog,
    )
    .expect("check");
    json!({"record": record, "document": record.render_markdown()})
}

fn check_row(input: &CheckRowIn) -> check::Row {
    let event_bar = &input.event_bar;
    let bar = Bar {
        ema_stack: event_bar[0].as_i64().map(|code| code as i32),
        atr_ratio: event_bar[1].as_f64(),
        rsi_zone: event_bar[2].as_i64().map(|code| code as i32),
        rvol: event_bar[3].as_f64(),
        close_position: event_bar[4].as_f64(),
        upper_wick: event_bar[5].as_f64(),
        lower_wick: event_bar[6].as_f64(),
        ..Bar::default()
    };
    check::Row {
        event_bar: bar,
        direction: input.direction,
        reference: input.reference,
        confirmation_bar: input.confirmation_close.map(|close| Bar {
            close,
            ..Bar::default()
        }),
        entry_index: input.entry_index,
        entry_time: input.entry_time,
        exit_index: input.exit_index,
        entry_open: input.entry_open,
        exit_close: input.exit_close,
    }
}

fn run_meta(input: &Value, datasets: &Datasets) -> Value {
    let input: MetaInput = serde_json::from_value(input.clone()).expect("meta input");
    let mut bars = Vec::new();
    for name in &input.series {
        bars.extend(datasets.table(name).bars());
    }
    for (key, values) in &datasets.meta_context {
        let bar = key.parse::<u64>().expect("context bar");
        let Some(target) = bars.iter_mut().find(|candidate| candidate.bar == bar) else {
            panic!("meta context names bar {bar} outside the series");
        };
        let values = values.as_array().expect("context values");
        let integer = |index: usize| values[index].as_i64().map(|value| value as i32);
        let number = |index: usize| values[index].as_f64();
        target.ema_stack = integer(0);
        target.rsi_zone = integer(1);
        target.rsi = number(2);
        target.atr_ratio = number(3);
        target.adx = number(4);
        target.close_position = number(5);
        target.upper_wick = number(6);
        target.lower_wick = number(7);
        target.body_ratio = number(8);
    }
    let bundle = Bundle::new(900, bars, Vec::new());
    let result = build_meta(&bundle, &input.marks, &input.config).expect("meta");
    serde_json::to_value(result).expect("meta json")
}

fn run_exit(input: &Value, datasets: &Datasets) -> Value {
    let input: ExitInput = serde_json::from_value(input.clone()).expect("exit input");
    let mut bars = Vec::new();
    for name in &input.series {
        bars.extend(datasets.table(name).bars());
    }
    for (key, values) in &datasets.label_exit {
        let bar = key.parse::<u64>().expect("exit extra bar");
        let Some(target) = bars.iter_mut().find(|candidate| candidate.bar == bar) else {
            panic!("label exit extras name bar {bar} outside the series");
        };
        let values = values.as_array().expect("exit extra values");
        target.ema_fast = values[0].as_f64();
        target.last_pivot_low = values[1].as_f64();
        target.last_pivot_high = values[2].as_f64();
    }
    let bundle = Bundle::new(900, bars, Vec::new());
    let record = build_exit_with_entries(
        &bundle,
        &input.config,
        Some(&input.marks),
        &input.check_entries,
    )
    .expect("exit");
    json!({
        "record": {
            "label_entries": record.label_entries,
            "check_entries": record.check_entries,
            "label_table": record.label_table,
            "check_table": record.check_table,
        },
        "document": record.render_markdown(),
    })
}

fn run_marks(input: &Value) -> Value {
    let input: MarksInput = serde_json::from_value(input.clone()).expect("marks input");
    let raw = quantrule_simulation::read_marks(&input.csv).expect("marks csv");
    match quantrule_simulation::validate_marks(&raw, &input.charts) {
        Ok(validated) => {
            let written = write_marks(&validated);
            json!({"csv": written, "count": validated.len()})
        }
        Err(error) => json!({"error": error_kind(&error)}),
    }
}

fn error_kind(error: &quantrule_simulation::MarkError) -> &'static str {
    use quantrule_simulation::MarkError;
    match error {
        MarkError::Duplicate(..) => "duplicate",
        MarkError::BarOutside(..) => "bar_outside",
        MarkError::UnknownChart(_) => "unknown_chart",
        MarkError::BarNotInteger => "bar_not_integer",
        MarkError::UnknownAction(_) => "unknown_action",
        MarkError::UnknownQuality(_) => "unknown_quality",
        MarkError::UnknownSide(_) => "unknown_side",
        MarkError::NoteTooLong => "note_too_long",
        MarkError::Csv(_) => "csv",
    }
}

impl Datasets {
    fn table(&self, name: &str) -> &Table {
        match name {
            "label-series" => &self.label_series,
            "tail-bars" => &self.tail_bars,
            other => panic!("unknown dataset {other}"),
        }
    }
}

/// Compare every expected value with the actual value to the tolerance.
///
/// An object compares the keys the fixture declares; the actual may carry more
/// (a check record also renders its document). An array must match in length.
///
/// # Panics
///
/// Panics on the first difference, naming the path.
pub fn assert_matches(expected: &Value, actual: &Value, tolerance: f64, path: &str) {
    match (expected, actual) {
        (Value::Object(expected), Value::Object(actual)) => {
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

/// Reverse the list-valued inputs whose stage is order-independent.
///
/// The window selector, the oracle, the review, the meta tables, and the marks
/// writer all re-order internally. The check and exit rows are a time sequence;
/// reversing them is a different input and is not done here.
pub fn reorder(stage: &str, input: &Value) -> Value {
    let mut input = input.clone();
    let Some(object) = input.as_object_mut() else {
        return input;
    };
    match stage {
        "window" => reverse_list(object, "events"),
        "oracle" => {
            if let Some(bars) = object.get_mut("bars").and_then(Value::as_array_mut) {
                bars.reverse();
            }
        }
        "review" => {
            reverse_list(object, "marks");
            reverse_list(object, "details");
            reverse_list(object, "bars");
            reverse_list(object, "events");
        }
        "meta" => reverse_list(object, "marks"),
        "marks" => {
            if let Some(csv) = object.get("csv").and_then(Value::as_str) {
                let mut lines: Vec<&str> = csv.split("\r\n").collect();
                if lines.last() == Some(&"") {
                    lines.pop();
                }
                if lines.len() > 1 {
                    let header = lines.remove(0);
                    lines.reverse();
                    let mut text = String::from(header);
                    for line in lines {
                        text.push_str("\r\n");
                        text.push_str(line);
                    }
                    text.push_str("\r\n");
                    object.insert("csv".to_owned(), Value::String(text));
                }
            }
        }
        _ => {}
    }
    input
}

fn reverse_list(object: &mut Map<String, Value>, key: &str) {
    if let Some(values) = object.get_mut(key).and_then(Value::as_array_mut) {
        values.reverse();
    }
}
