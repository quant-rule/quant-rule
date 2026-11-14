//! The simulation mark schema.
//!
//! A mark is one human or oracle judgement: a good or bad entry or exit, on one
//! bar of one chart. The schema is the annotator's seven columns unchanged, so
//! the same file can be read by the existing annotator and by this crate.
//!
//! Validation derives the mark time from the chart bar, refuses a mark outside
//! its chart, refuses a duplicate identity, and trims the note. The identity is
//! `chart, bar, action, side`: a good and a bad mark cannot share one entry or
//! exit on the same side.
//!
//! `port-parity: PORT-001` names the golden fixture this module is compared
//! against.

// port-parity: PORT-001

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bundle::utc_from_seconds;

/// The longest note a mark may carry.
pub const NOTE_LIMIT: usize = 500;

/// The CSV columns, in order.
pub const ANNOTATION_HEADER: [&str; 7] = [
    "chart",
    "bar",
    "marked_time_utc",
    "action",
    "quality",
    "side",
    "note",
];

/// Whether a mark is an entry or an exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkAction {
    /// An entry.
    Entry,
    /// An exit.
    Exit,
}

impl MarkAction {
    /// The schema word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Entry => "entry",
            Self::Exit => "exit",
        }
    }

    /// Parse the schema word.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "entry" => Some(Self::Entry),
            "exit" => Some(Self::Exit),
            _ => None,
        }
    }
}

/// Whether the mark is a good, bad, or neutral example.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkQuality {
    /// A good example.
    Good,
    /// A bad example.
    Bad,
    /// Neither; kept for context.
    Neutral,
}

impl MarkQuality {
    /// The schema word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Bad => "bad",
            Self::Neutral => "neutral",
        }
    }

    /// Parse the schema word.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "good" => Some(Self::Good),
            "bad" => Some(Self::Bad),
            "neutral" => Some(Self::Neutral),
            _ => None,
        }
    }
}

/// The direction of the mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Long.
    Long,
    /// Short.
    Short,
}

impl Side {
    /// The schema word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
        }
    }

    /// Parse the schema word.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "long" => Some(Self::Long),
            "short" => Some(Self::Short),
            _ => None,
        }
    }

    /// The sign a position on this side carries in a return.
    #[must_use]
    pub fn sign(self) -> f64 {
        match self {
            Self::Long => 1.0,
            Self::Short => -1.0,
        }
    }
}

/// A mark exactly as one CSV row carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RawMark {
    /// The chart file name.
    pub chart: String,
    /// The bar identity, as text.
    pub bar: String,
    /// The recorded mark time.
    pub marked_time_utc: String,
    /// Entry or exit.
    pub action: String,
    /// Good, bad, or neutral.
    pub quality: String,
    /// Long or short.
    pub side: String,
    /// Why the mark was made.
    pub note: String,
}

/// One validated mark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    /// The chart file name.
    pub chart: String,
    /// The bar identity.
    pub bar: u64,
    /// The time derived from the chart bar.
    pub marked_time_utc: String,
    /// Entry or exit.
    pub action: MarkAction,
    /// Good, bad, or neutral.
    pub quality: MarkQuality,
    /// Long or short.
    pub side: Side,
    /// Why the mark was made, trimmed to the limit.
    pub note: String,
}

/// One bar a chart can mark, reduced to identity and time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChartBar {
    /// The bar identity.
    pub bar: u64,
    /// The bar open time, in UTC seconds.
    pub timestamp: i64,
}

/// One chart a mark may refer to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Chart {
    /// The chart file name, which the mark names.
    pub file: String,
    /// The bars drawn on the chart.
    pub bars: Vec<ChartBar>,
}

/// Why a mark is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkError {
    /// The chart does not exist.
    UnknownChart(String),
    /// The bar is not an integer.
    BarNotInteger,
    /// The bar is not drawn on the chart.
    BarOutside(String, u64),
    /// The action is not `entry` or `exit`.
    UnknownAction(String),
    /// The quality is not `good`, `bad`, or `neutral`.
    UnknownQuality(String),
    /// The side is not `long` or `short`.
    UnknownSide(String),
    /// The note is longer than [`NOTE_LIMIT`] characters.
    NoteTooLong,
    /// The same chart, bar, action, and side appear twice.
    Duplicate(String, u64, MarkAction, Side),
    /// The CSV text is malformed.
    Csv(String),
}

impl std::fmt::Display for MarkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownChart(chart) => write!(formatter, "unknown chart {chart:?}"),
            Self::BarNotInteger => formatter.write_str("bar must be an integer"),
            Self::BarOutside(chart, bar) => write!(formatter, "bar {bar} is not in {chart}"),
            Self::UnknownAction(action) => write!(formatter, "unknown action {action:?}"),
            Self::UnknownQuality(quality) => write!(formatter, "unknown quality {quality:?}"),
            Self::UnknownSide(side) => write!(formatter, "unknown side {side:?}"),
            Self::NoteTooLong => {
                write!(formatter, "note is longer than {NOTE_LIMIT} characters")
            }
            Self::Duplicate(chart, bar, action, side) => write!(
                formatter,
                "duplicate mark ({chart}, {bar}, {}, {})",
                action.as_str(),
                side.as_str()
            ),
            Self::Csv(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for MarkError {}

impl RawMark {
    /// Build the CSV row from a validated mark.
    #[must_use]
    pub fn from_mark(mark: &Mark) -> Self {
        Self {
            chart: mark.chart.clone(),
            bar: mark.bar.to_string(),
            marked_time_utc: mark.marked_time_utc.clone(),
            action: mark.action.as_str().to_owned(),
            quality: mark.quality.as_str().to_owned(),
            side: mark.side.as_str().to_owned(),
            note: mark.note.clone(),
        }
    }
}

/// Read a marks CSV.
///
/// The parser accepts the reference's quoted fields, doubled quotes, CRLF or LF
/// line endings, and a missing column as an empty value.
///
/// # Errors
///
/// Returns [`MarkError::Csv`] when the text has no header or a quoted field is
/// not closed.
pub fn read_marks(text: &str) -> Result<Vec<RawMark>, MarkError> {
    let records = parse_csv(text)?;
    let Some(header) = records.first() else {
        return Ok(Vec::new());
    };
    let columns: BTreeMap<&str, usize> = header
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let mut rows = Vec::with_capacity(records.len().saturating_sub(1));
    for record in records.iter().skip(1) {
        if record.len() == 1 && record[0].is_empty() {
            continue;
        }
        let get = |name: &str| -> String {
            columns
                .get(name)
                .and_then(|index| record.get(*index))
                .cloned()
                .unwrap_or_default()
        };
        rows.push(RawMark {
            chart: get("chart"),
            bar: get("bar"),
            marked_time_utc: get("marked_time_utc"),
            action: get("action"),
            quality: get("quality"),
            side: get("side"),
            note: get("note"),
        });
    }
    Ok(rows)
}

/// Validate one mark and derive its time from the chart bar.
///
/// # Errors
///
/// Returns [`MarkError`] when the chart is unknown, the bar is malformed or not
/// drawn, a field is not a schema word, or the note is too long.
pub fn validate_mark(raw: &RawMark, charts: &BTreeMap<&str, &Chart>) -> Result<Mark, MarkError> {
    let Some(chart) = charts.get(raw.chart.as_str()) else {
        return Err(MarkError::UnknownChart(raw.chart.clone()));
    };
    let bar: u64 = raw
        .bar
        .trim()
        .parse()
        .map_err(|_| MarkError::BarNotInteger)?;
    let Some(chart_bar) = chart.bars.iter().find(|candidate| candidate.bar == bar) else {
        return Err(MarkError::BarOutside(raw.chart.clone(), bar));
    };
    let Some(action) = MarkAction::parse(&raw.action) else {
        return Err(MarkError::UnknownAction(raw.action.clone()));
    };
    let Some(quality) = MarkQuality::parse(&raw.quality) else {
        return Err(MarkError::UnknownQuality(raw.quality.clone()));
    };
    let Some(side) = Side::parse(&raw.side) else {
        return Err(MarkError::UnknownSide(raw.side.clone()));
    };
    let note = raw.note.trim();
    if note.chars().count() > NOTE_LIMIT {
        return Err(MarkError::NoteTooLong);
    }
    Ok(Mark {
        chart: raw.chart.clone(),
        bar,
        marked_time_utc: utc_from_seconds(chart_bar.timestamp),
        action,
        quality,
        side,
        note: note.to_owned(),
    })
}

/// Validate a set of marks against a chart manifest.
///
/// # Errors
///
/// Returns the first [`MarkError`] a mark reports, or the first duplicate
/// identity.
pub fn validate_marks(raws: &[RawMark], charts: &[Chart]) -> Result<Vec<Mark>, MarkError> {
    let by_name: BTreeMap<&str, &Chart> = charts
        .iter()
        .map(|chart| (chart.file.as_str(), chart))
        .collect();
    let mut validated = Vec::with_capacity(raws.len());
    let mut seen = BTreeMap::new();
    for raw in raws {
        let mark = validate_mark(raw, &by_name)?;
        let key = (mark.chart.clone(), mark.bar, mark.action, mark.side);
        if seen.insert(key.clone(), ()).is_some() {
            return Err(MarkError::Duplicate(key.0, key.1, key.2, key.3));
        }
        validated.push(mark);
    }
    Ok(validated)
}

/// The identity of a mark.
#[must_use]
pub fn annotation_key(mark: &Mark) -> (String, u64, MarkAction, Side) {
    (mark.chart.clone(), mark.bar, mark.action, mark.side)
}

/// Replace a mark with the same identity.
#[must_use]
pub fn upsert(rows: &[Mark], row: Mark) -> Vec<Mark> {
    let key = annotation_key(&row);
    let mut result: Vec<Mark> = rows
        .iter()
        .filter(|item| annotation_key(item) != key)
        .cloned()
        .collect();
    result.push(row);
    result
}

/// Delete a mark by identity.
#[must_use]
pub fn delete(rows: &[Mark], mark: &Mark) -> Vec<Mark> {
    let key = annotation_key(mark);
    rows.iter()
        .filter(|item| annotation_key(item) != key)
        .cloned()
        .collect()
}

/// Count marks by quality and side.
#[must_use]
pub fn count_by_quality(rows: &[Mark]) -> BTreeMap<(MarkQuality, Side), usize> {
    let mut counts = BTreeMap::new();
    for row in rows {
        *counts.entry((row.quality, row.side)).or_insert(0) += 1;
    }
    counts
}

/// Write marks in the reference's stable order and CSV dialect.
///
/// Rows sort by chart, numeric bar, action, and side. The line ending is CRLF,
/// and a field is quoted only when it contains a comma, quote, or newline, so
/// the output round-trips through [`read_marks`] byte for byte.
#[must_use]
pub fn write_marks(rows: &[Mark]) -> String {
    let mut ordered: Vec<&Mark> = rows.iter().collect();
    ordered.sort_by(|left, right| {
        (
            left.chart.as_str(),
            left.bar,
            left.action.as_str(),
            left.side.as_str(),
        )
            .cmp(&(
                right.chart.as_str(),
                right.bar,
                right.action.as_str(),
                right.side.as_str(),
            ))
    });
    let mut text = String::new();
    text.push_str(&ANNOTATION_HEADER.join(","));
    text.push_str("\r\n");
    for row in ordered {
        let bar = row.bar.to_string();
        let fields = [
            row.chart.as_str(),
            bar.as_str(),
            row.marked_time_utc.as_str(),
            row.action.as_str(),
            row.quality.as_str(),
            row.side.as_str(),
            row.note.as_str(),
        ];
        for (index, field) in fields.iter().enumerate() {
            if index > 0 {
                text.push(',');
            }
            text.push_str(&csv_field(field));
        }
        text.push_str("\r\n");
    }
    text
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn parse_csv(text: &str) -> Result<Vec<Vec<String>>, MarkError> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut chars = text.chars().peekable();
    let mut quoted = false;
    let mut field_started = false;
    while let Some(character) = chars.next() {
        if quoted {
            match character {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        field.push('"');
                    } else {
                        quoted = false;
                    }
                }
                other => field.push(other),
            }
            continue;
        }
        match character {
            '"' if !field_started => {
                quoted = true;
                field_started = true;
            }
            ',' => {
                record.push(std::mem::take(&mut field));
                field_started = false;
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
                field_started = false;
            }
            '\n' => {
                record.push(std::mem::take(&mut field));
                records.push(std::mem::take(&mut record));
                field_started = false;
            }
            other => {
                field.push(other);
                field_started = true;
            }
        }
    }
    if quoted {
        return Err(MarkError::Csv("an unterminated quoted field".to_owned()));
    }
    if field_started || !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chart() -> Chart {
        Chart {
            file: "sim-w001.png".to_owned(),
            bars: (0..5)
                .map(|bar| ChartBar {
                    bar,
                    timestamp: 1_514_764_800 + bar as i64 * 900,
                })
                .collect(),
        }
    }

    fn raw(bar: &str, action: &str, quality: &str, side: &str) -> RawMark {
        RawMark {
            chart: "sim-w001.png".to_owned(),
            bar: bar.to_owned(),
            marked_time_utc: String::new(),
            action: action.to_owned(),
            quality: quality.to_owned(),
            side: side.to_owned(),
            note: "higher low into the level".to_owned(),
        }
    }

    fn charts<'a>() -> BTreeMap<&'a str, &'a Chart> {
        BTreeMap::new()
    }

    #[test]
    fn a_valid_mark_gets_its_time_from_the_chart() {
        let chart = chart();
        let mut by_name = charts();
        by_name.insert("sim-w001.png", &chart);
        let mark = validate_mark(&raw("2", "entry", "good", "long"), &by_name).expect("mark");
        assert_eq!(mark.bar, 2);
        assert_eq!(mark.marked_time_utc, "2018-01-01T00:30:00Z");
    }

    #[test]
    fn an_unknown_chart_is_refused() {
        let chart = chart();
        let mut by_name = charts();
        by_name.insert("other.png", &chart);
        assert_eq!(
            validate_mark(&raw("2", "entry", "good", "long"), &by_name),
            Err(MarkError::UnknownChart("sim-w001.png".to_owned()))
        );
    }

    #[test]
    fn a_bar_outside_the_chart_is_refused() {
        let chart = chart();
        let mut by_name = charts();
        by_name.insert("sim-w001.png", &chart);
        assert_eq!(
            validate_mark(&raw("9", "entry", "good", "long"), &by_name),
            Err(MarkError::BarOutside("sim-w001.png".to_owned(), 9))
        );
        assert_eq!(
            validate_mark(&raw("x", "entry", "good", "long"), &by_name),
            Err(MarkError::BarNotInteger)
        );
    }

    #[test]
    fn a_non_schema_word_is_refused() {
        let chart = chart();
        let mut by_name = charts();
        by_name.insert("sim-w001.png", &chart);
        assert_eq!(
            validate_mark(&raw("2", "hold", "good", "long"), &by_name),
            Err(MarkError::UnknownAction("hold".to_owned()))
        );
        assert_eq!(
            validate_mark(&raw("2", "entry", "great", "long"), &by_name),
            Err(MarkError::UnknownQuality("great".to_owned()))
        );
        assert_eq!(
            validate_mark(&raw("2", "entry", "good", "flat"), &by_name),
            Err(MarkError::UnknownSide("flat".to_owned()))
        );
    }

    #[test]
    fn a_long_note_is_refused() {
        let chart = chart();
        let mut by_name = charts();
        by_name.insert("sim-w001.png", &chart);
        let mut mark = raw("2", "entry", "good", "long");
        mark.note = "x".repeat(NOTE_LIMIT + 1);
        assert_eq!(validate_mark(&mark, &by_name), Err(MarkError::NoteTooLong));
    }

    #[test]
    fn a_duplicate_identity_is_refused() {
        let charts = [chart()];
        let rows = [
            raw("2", "entry", "good", "long"),
            raw("2", "entry", "bad", "long"),
        ];
        assert!(matches!(
            validate_marks(&rows, &charts),
            Err(MarkError::Duplicate(_, 2, MarkAction::Entry, Side::Long))
        ));
    }

    #[test]
    fn the_marks_csv_round_trips() {
        let charts = [chart()];
        let rows = [
            raw("3", "exit", "good", "long"),
            raw("2", "entry", "good", "long"),
        ];
        let marks = validate_marks(&rows, &charts).expect("marks");
        let text = write_marks(&marks);
        let back = read_marks(&text).expect("read");
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].bar, "2");
        assert_eq!(back[1].bar, "3");
        let again = validate_marks(&back, &charts).expect("again");
        let mut expected = marks.clone();
        expected.sort_by(|left, right| {
            (left.chart.as_str(), left.bar).cmp(&(right.chart.as_str(), right.bar))
        });
        assert_eq!(again, expected);
    }

    #[test]
    fn a_quoted_note_with_a_comma_is_preserved() {
        let charts = [chart()];
        let mut row = raw("2", "entry", "good", "long");
        row.note = "a, b \"quoted\" note".to_owned();
        let marks = validate_marks(&[row], &charts).expect("marks");
        let text = write_marks(&marks);
        assert!(text.contains("\"a, b \"\"quoted\"\" note\""));
        let back = read_marks(&text).expect("read");
        assert_eq!(back[0].note, "a, b \"quoted\" note");
    }

    #[test]
    fn the_counts_are_by_quality_and_side() {
        let charts = [chart()];
        let rows = [
            raw("2", "entry", "good", "long"),
            raw("3", "entry", "bad", "short"),
            raw("4", "entry", "neutral", "long"),
        ];
        let marks = validate_marks(&rows, &charts).expect("marks");
        let counts = count_by_quality(&marks);
        assert_eq!(counts.get(&(MarkQuality::Good, Side::Long)), Some(&1));
        assert_eq!(counts.get(&(MarkQuality::Bad, Side::Short)), Some(&1));
        assert_eq!(counts.get(&(MarkQuality::Neutral, Side::Long)), Some(&1));
    }

    #[test]
    fn upsert_replaces_and_delete_removes() {
        let charts = [chart()];
        let marks = validate_marks(&[raw("2", "entry", "good", "long")], &charts).expect("marks");
        let mut replacement = marks[0].clone();
        replacement.quality = MarkQuality::Bad;
        let replaced = upsert(&marks, replacement);
        assert_eq!(replaced.len(), 1);
        assert_eq!(replaced[0].quality, MarkQuality::Bad);
        assert!(delete(&replaced, &replaced[0]).is_empty());
    }
}
