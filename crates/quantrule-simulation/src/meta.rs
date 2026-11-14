//! Meta-labelling the human marks and checking the context split.
//!
//! A human mark says where an entry looked good or bad with the future visible.
//! The meta stage replaces that judgement with a measured label: the forward
//! result over a fixed horizon, grouped by the context that was knowable at the
//! entry. The best context cell with enough marks becomes a candidate split, and
//! the split is checked on a later window that carries no marks.
//!
//! The stage is descriptive. It does not claim an edge, it does not fit a model,
//! and it never reaches the strategy path. It is the Rust port of
//! `tools/simulation/meta.py`; the numeric rules and bucket boundaries match the
//! Python reference, and every fixture value is compared to `1e-9`.
//!
//! `port-parity: PORT-001` names the golden fixture this module is compared
//! against. The comparison test is `crates/quantrule-simulation/tests/port_parity.rs`.

// port-parity: PORT-001

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::check::{CheckError, Criteria, check};
use crate::marks::{MarkAction, MarkQuality, Side};

/// The fewest marks a cell needs before it may become the candidate split.
pub const MIN_CELL: usize = 30;
/// The forward horizon of the check, in bars.
pub const CHECK_HORIZON: usize = 20;
/// The round-trip cost proxy, in basis points.
pub const COST_BPS: f64 = 6.0;
/// The fewest marks a half-year needs before it counts as a direction sample.
pub const MIN_HALF_YEAR_EVENTS: usize = 5;
/// The fewest positive half-years a surviving split needs.
pub const MIN_POSITIVE_HALF_YEARS: usize = 3;
/// How far a mark may sit from a bar timestamp, in seconds.
pub const MAX_NEAREST_SECONDS: i64 = 450;

/// Why a meta result cannot be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaError {
    /// A bar or mark carried a value that is not a finite number.
    NotFinite,
    /// The reused check refused its own inputs.
    Check(CheckError),
}

impl std::fmt::Display for MetaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFinite => formatter.write_str("a meta input is not a finite number"),
            Self::Check(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for MetaError {}

impl From<CheckError> for MetaError {
    fn from(error: CheckError) -> Self {
        Self::Check(error)
    }
}

/// One model reading the meta stage needs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bar {
    /// The bar identity inside the bundle.
    pub bar: u64,
    /// The bar open time, in UTC seconds.
    pub timestamp: i64,
    /// Open.
    pub open: f64,
    /// High.
    pub high: f64,
    /// Low.
    pub low: f64,
    /// Close.
    pub close: f64,
    /// Average true range at the bar, when the model published one.
    pub atr: Option<f64>,
    /// The EMA stack code: -1 bear, 0 mixed, 1 bull.
    pub ema_stack: Option<i32>,
    /// The RSI zone code: -1 oversold, 0 middle, 1 overbought.
    pub rsi_zone: Option<i32>,
    /// Relative strength index.
    pub rsi: Option<f64>,
    /// ATR ratio.
    pub atr_ratio: Option<f64>,
    /// Average directional index.
    pub adx: Option<f64>,
    /// Where the close sat inside the bar, from 0 to 1.
    pub close_position: Option<f64>,
    /// Upper wick as a fraction of the range.
    pub upper_wick: Option<f64>,
    /// Lower wick as a fraction of the range.
    pub lower_wick: Option<f64>,
    /// Body as a fraction of the range.
    pub body_ratio: Option<f64>,
}

/// One human entry mark.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    /// The chart the mark belongs to.
    pub chart: String,
    /// The timestamp the client recorded, as an ISO UTC string.
    pub marked_time_utc: String,
    /// The same timestamp as UTC seconds, parsed at the boundary.
    pub moment: i64,
    /// The bar the mark names.
    pub bar: u64,
    /// Entry or exit.
    pub action: MarkAction,
    /// Good, bad, or neutral.
    pub quality: MarkQuality,
    /// Long or short.
    pub side: Side,
    /// Why the mark was made.
    pub note: String,
}

/// The forward path of one entry, in R.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ForwardPath {
    /// The close-to-close result over the horizon, signed by the side.
    pub return_50: f64,
    /// The best excursion in favour, in ATR.
    pub mfe_50: f64,
    /// The worst excursion against, in ATR.
    pub mae_50: f64,
}

/// One labelled entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    /// The parsed mark moment, used to filter the label window.
    /// Not serialized: the recorded ISO string is the public field.
    #[serde(skip)]
    moment: i64,
    /// The chart the mark belonged to.
    pub chart: String,
    /// The recorded mark timestamp.
    pub marked_time_utc: String,
    /// The bar the mark mapped to.
    pub bar: u64,
    /// The side.
    pub side: Side,
    /// The measured quality.
    pub quality: MarkQuality,
    /// The signed forward result.
    pub return_50: f64,
    /// The best excursion in favour, in ATR.
    pub mfe_50: f64,
    /// The worst excursion against, in ATR.
    pub mae_50: f64,
    /// The EMA stack at the entry.
    pub ema_stack: String,
    /// The RSI zone at the entry.
    pub rsi_zone: String,
    /// The RSI reading.
    pub rsi: Option<f64>,
    /// The ATR ratio.
    pub atr_ratio: Option<f64>,
    /// The ADX reading.
    pub adx: Option<f64>,
    /// The close position.
    pub close_position: Option<f64>,
    /// The upper wick ratio.
    pub upper_wick: Option<f64>,
    /// The lower wick ratio.
    pub lower_wick: Option<f64>,
    /// The body ratio.
    pub body_ratio: Option<f64>,
    /// The two-level volatility state.
    pub atr_bucket: String,
}

/// One context cell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    /// The side.
    pub side: Side,
    /// The EMA stack label.
    pub ema_stack: String,
    /// The ATR bucket label.
    pub atr_bucket: String,
    /// Marks in the cell.
    pub n: usize,
    /// Mean signed forward result.
    pub mean_return: f64,
    /// Share of positive forward results.
    pub hit_rate: f64,
    /// Mean best excursion in favour.
    pub mean_mfe: f64,
    /// Mean worst excursion against.
    pub mean_mae: f64,
}

/// One half-year bucket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalfYear {
    /// The bucket label, such as `2020H1`.
    pub bucket: String,
    /// Events in the bucket.
    pub n: usize,
    /// Mean net result in the bucket.
    pub mean_net: f64,
}

/// The statistics of one check set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stats {
    /// Events in the set.
    pub n: usize,
    /// Mean net result, absent when the set is empty.
    pub mean_net: Option<f64>,
    /// Share of positive net results, absent when the set is empty.
    pub hit_rate: Option<f64>,
    /// The half-year buckets, ordered by label.
    pub half_years: Vec<HalfYear>,
    /// Half-year buckets with enough events and a positive mean.
    pub positive_half_years: usize,
}

/// The verdict of the context screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// One set was too small to speak.
    Underpowered,
    /// The filtered set cleared every criterion.
    Survives,
    /// The filtered set did not clear the screen.
    NoPattern,
}

/// The candidate split checked on the unmarked window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Check {
    /// The cell under test.
    pub cell: Cell,
    /// The filtered set.
    pub filtered: Stats,
    /// The unfiltered control.
    pub control: Stats,
    /// Whether a side had fewer than [`MIN_CELL`] events.
    pub underpowered: bool,
    /// The screen verdict.
    pub verdict: Verdict,
}

/// The complete meta stage result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaResult {
    /// The labelled entries, in chart and moment order.
    pub rows: Vec<Row>,
    /// Marks that could not be mapped to a bar or a path.
    pub unmatched: usize,
    /// The context cells, ordered by side, stack, and bucket.
    pub cells: Vec<Cell>,
    /// The best cell with at least the minimum sample.
    pub candidate: Option<Cell>,
    /// The check, present only when a candidate cell exists.
    pub check: Option<Check>,
}

/// The windows and thresholds of one meta run.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The first moment a mark is labelled.
    pub label_from: i64,
    /// The last moment a mark is labelled.
    pub label_to: i64,
    /// The first moment the check may enter.
    pub check_from: i64,
    /// The last moment the check may enter.
    pub check_to: i64,
    /// The forward horizon of the label, in bars.
    pub horizon: usize,
    /// The fewest marks a candidate cell needs.
    pub min_cell: usize,
}

impl Config {
    /// A config with the reference thresholds and the given windows.
    #[must_use]
    pub fn new(
        label_from: i64,
        label_to: i64,
        check_from: i64,
        check_to: i64,
        horizon: usize,
    ) -> Self {
        Self {
            label_from,
            label_to,
            check_from,
            check_to,
            horizon,
            min_cell: MIN_CELL,
        }
    }
}

/// A check row: one event bar and the next-bar entry it implies.
#[derive(Debug, Clone, Copy, PartialEq)]
struct CheckRow {
    entry_index: usize,
    exit_index: usize,
    entry_time: i64,
    entry_open: f64,
    exit_close: f64,
}

/// Label every entry mark by its forward path and the context at the entry.
///
/// Marks are sorted by chart, moment, bar, side, action, quality, and note
/// before they are paired, so the result never depends on the order the caller
/// supplied. Bars must be ordered by timestamp; [`build`] enforces that.
#[must_use]
pub fn label_marks(bars: &[Bar], marks: &[Mark], horizon: usize) -> (Vec<Row>, usize) {
    let mut ordered: Vec<&Mark> = marks
        .iter()
        .filter(|mark| mark.action == MarkAction::Entry)
        .collect();
    ordered.sort_by_key(|mark| {
        (
            mark.chart.clone(),
            mark.moment,
            mark.bar,
            side_code(mark.side),
            action_code(mark.action),
            quality_code(mark.quality),
            mark.note.clone(),
        )
    });

    let mut rows = Vec::new();
    let mut unmatched = 0;
    for mark in ordered {
        let Some(index) = nearest_bar(bars, mark.moment, MAX_NEAREST_SECONDS) else {
            unmatched += 1;
            continue;
        };
        let Some(path) = forward_path(bars, index, mark.side, horizon) else {
            unmatched += 1;
            continue;
        };
        let bar = &bars[index];
        rows.push(Row {
            moment: mark.moment,
            chart: mark.chart.clone(),
            marked_time_utc: mark.marked_time_utc.clone(),
            bar: bar.bar,
            side: mark.side,
            quality: quality_of(path.return_50),
            return_50: path.return_50,
            mfe_50: path.mfe_50,
            mae_50: path.mae_50,
            ema_stack: stack_label(bar.ema_stack).to_owned(),
            rsi_zone: zone_label(bar.rsi_zone).to_owned(),
            rsi: bar.rsi,
            atr_ratio: bar.atr_ratio,
            adx: bar.adx,
            close_position: bar.close_position,
            upper_wick: bar.upper_wick,
            lower_wick: bar.lower_wick,
            body_ratio: bar.body_ratio,
            atr_bucket: atr_bucket(bar.atr_ratio).to_owned(),
        });
    }
    (rows, unmatched)
}

/// Find the bar nearest to a moment.
///
/// A tie goes to the earlier bar, matching the reference's `bisect` scan. A bar
/// farther than `max_distance` seconds is not a match.
#[must_use]
pub fn nearest_bar(bars: &[Bar], moment: i64, max_distance: i64) -> Option<usize> {
    if bars.is_empty() || max_distance < 0 {
        return None;
    }
    let position = bars.partition_point(|bar| bar.timestamp < moment);
    let lower = position.checked_sub(1);
    let upper = (position < bars.len()).then_some(position);
    let choose = |left: Option<usize>, right: Option<usize>| -> Option<usize> {
        match (left, right) {
            (Some(left), Some(right)) => {
                let left_distance = bars[left].timestamp.abs_diff(moment);
                let right_distance = bars[right].timestamp.abs_diff(moment);
                if left_distance <= right_distance {
                    Some(left)
                } else {
                    Some(right)
                }
            }
            (Some(index), None) | (None, Some(index)) => Some(index),
            (None, None) => None,
        }
    };
    let index = choose(lower, upper)?;
    if bars[index].timestamp.abs_diff(moment) > max_distance.unsigned_abs() {
        return None;
    }
    Some(index)
}

/// The forward path of an entry, or `None` when the horizon cannot be measured.
#[must_use]
pub fn forward_path(bars: &[Bar], index: usize, side: Side, horizon: usize) -> Option<ForwardPath> {
    let entry = bars.get(index)?;
    let atr = entry.atr?;
    if !atr.is_finite() || atr <= 0.0 {
        return None;
    }
    let end = index.checked_add(horizon)?.min(bars.len().checked_sub(1)?);
    if end <= index {
        return None;
    }
    let entry_price = entry.close;
    if !entry_price.is_finite() || entry_price <= 0.0 {
        return None;
    }

    let mut mfe = 0.0_f64;
    let mut mae = 0.0_f64;
    for bar in &bars[index + 1..=end] {
        if !bar.high.is_finite() || !bar.low.is_finite() || !bar.close.is_finite() {
            return None;
        }
        let (favorable, adverse) = match side {
            Side::Long => (bar.high - entry_price, entry_price - bar.low),
            Side::Short => (entry_price - bar.low, bar.high - entry_price),
        };
        if favorable > mfe {
            mfe = favorable;
        }
        if adverse > mae {
            mae = adverse;
        }
    }
    let sign = match side {
        Side::Long => 1.0,
        Side::Short => -1.0,
    };
    Some(ForwardPath {
        return_50: (bars[end].close - entry_price) / entry_price * sign,
        mfe_50: mfe / atr,
        mae_50: mae / atr,
    })
}

/// The two-level volatility state used by SIM-01.
#[must_use]
pub fn atr_bucket(value: Option<f64>) -> &'static str {
    match value {
        None => "unknown",
        Some(value) if value >= 1.0 => "ge1",
        Some(_) => "lt1",
    }
}

/// Group the labelled entries by side, EMA stack, and ATR bucket.
#[must_use]
pub fn cell_table(rows: &[Row]) -> Vec<Cell> {
    let mut groups: BTreeMap<(String, String, String), Vec<&Row>> = BTreeMap::new();
    for row in rows {
        groups
            .entry((
                side_label(row.side).to_owned(),
                row.ema_stack.clone(),
                row.atr_bucket.clone(),
            ))
            .or_default()
            .push(row);
    }
    groups
        .into_iter()
        .filter_map(|((_, stack, bucket), group)| {
            let first = group.first()?;
            let returns: Vec<f64> = group.iter().map(|row| row.return_50).collect();
            Some(Cell {
                side: first.side,
                ema_stack: stack,
                atr_bucket: bucket,
                n: group.len(),
                mean_return: mean(&returns)?,
                hit_rate: win_rate(&returns)?,
                mean_mfe: mean(&group.iter().map(|row| row.mfe_50).collect::<Vec<_>>())?,
                mean_mae: mean(&group.iter().map(|row| row.mae_50).collect::<Vec<_>>())?,
            })
        })
        .collect()
}

/// Pick the highest-mean cell with at least `minimum` marks.
///
/// Ties keep the table order, which is side, stack, and bucket order. The table
/// order is the final tie-break, so the choice is deterministic.
#[must_use]
pub fn best_cell(cells: &[Cell], minimum: usize) -> Option<Cell> {
    let mut eligible: Vec<&Cell> = cells.iter().filter(|cell| cell.n >= minimum).collect();
    eligible.sort_by(|left, right| {
        right
            .mean_return
            .partial_cmp(&left.mean_return)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| right.n.cmp(&left.n))
    });
    eligible.first().map(|cell| (*cell).clone())
}

/// Build the filtered and control rows for the check window.
///
/// The filtered set keeps the bars whose EMA stack and ATR bucket match the
/// cell. The control keeps every bar that can form a complete forward path.
#[must_use]
fn check_bars(
    bars: &[Bar],
    cell: &Cell,
    from: i64,
    to: i64,
    horizon: usize,
) -> (Vec<CheckRow>, Vec<CheckRow>) {
    let mut filtered = Vec::new();
    let mut control = Vec::new();
    for (index, bar) in bars.iter().enumerate() {
        if bar.timestamp < from || bar.timestamp > to {
            continue;
        }
        let Some(entry_index) = index.checked_add(1) else {
            continue;
        };
        let Some(exit_index) = entry_index.checked_add(horizon) else {
            continue;
        };
        let (Some(entry), Some(exit)) = (bars.get(entry_index), bars.get(exit_index)) else {
            continue;
        };
        let row = CheckRow {
            entry_index,
            exit_index,
            entry_time: entry.timestamp,
            entry_open: entry.open,
            exit_close: exit.close,
        };
        control.push(row);
        let Some(stack) = bar.ema_stack else {
            continue;
        };
        let Some(ratio) = bar.atr_ratio else {
            continue;
        };
        if cell.ema_stack == stack_label(Some(stack)) && cell.atr_bucket == atr_bucket(Some(ratio))
        {
            filtered.push(row);
        }
    }
    (filtered, control)
}

/// Run the context screen for one candidate cell.
///
/// The check reuses [`crate::check::check`] for the six named failures, so the
/// threshold arithmetic has one implementation. The underpowered rule is the
/// reference's: either the filtered set or the control needs
/// [`MIN_CELL`] events before the screen can speak.
///
/// # Errors
///
/// Returns [`MetaError`] when an input is not finite or the reused check refuses
/// its own inputs.
pub fn evaluate(bars: &[Bar], cell: &Cell, from: i64, to: i64) -> Result<Check, MetaError> {
    let (filtered_rows, control_rows) = check_bars(bars, cell, from, to, CHECK_HORIZON);
    let filtered_rows = non_overlapping(&filtered_rows);
    let control_rows = non_overlapping(&control_rows);
    let (filtered, filtered_returns) = stats(&filtered_rows, cell.side)?;
    let (control, control_returns) = stats(&control_rows, cell.side)?;
    let underpowered = filtered.n < MIN_CELL || control.n < MIN_CELL;
    let verdict = if underpowered {
        Verdict::Underpowered
    } else {
        let half_years: Vec<f64> = filtered
            .half_years
            .iter()
            .filter(|bucket| bucket.n >= MIN_HALF_YEAR_EVENTS)
            .map(|bucket| bucket.mean_net)
            .collect();
        let criteria = Criteria {
            min_events: MIN_CELL,
            min_mean_net: 0.0,
            min_hit_rate: 0.5,
            half_years: half_years.len(),
            min_positive_half_years: MIN_POSITIVE_HALF_YEARS,
        };
        let result = check(&filtered_returns, &control_returns, &half_years, criteria)?;
        if result.passed {
            Verdict::Survives
        } else {
            Verdict::NoPattern
        }
    };
    Ok(Check {
        cell: cell.clone(),
        filtered,
        control,
        underpowered,
        verdict,
    })
}

/// Run the meta stage.
///
/// Bars are sorted by timestamp and bar before use. Marks are sorted inside
/// [`label_marks`], so a reordered mark set produces the same rows, cells, and
/// verdict.
///
/// # Errors
///
/// Returns [`MetaError`] when a bar or mark carries a non-finite number, or the
/// reused check refuses its inputs.
pub fn build(bars: &[Bar], marks: &[Mark], config: &Config) -> Result<MetaResult, MetaError> {
    let mut ordered = bars.to_vec();
    ordered.sort_by_key(|bar| (bar.timestamp, bar.bar));
    validate_bars(&ordered)?;
    let (all_rows, unmatched) = label_marks(&ordered, marks, config.horizon);
    let rows: Vec<Row> = all_rows
        .into_iter()
        .filter(|row| row.moment >= config.label_from && row.moment <= config.label_to)
        .collect();
    let cells = cell_table(&rows);
    let candidate = best_cell(&cells, config.min_cell);
    let check = match &candidate {
        Some(cell) => Some(evaluate(
            &ordered,
            cell,
            config.check_from,
            config.check_to,
        )?),
        None => None,
    };
    Ok(MetaResult {
        rows,
        unmatched,
        cells,
        candidate,
        check,
    })
}

fn validate_bars(bars: &[Bar]) -> Result<(), MetaError> {
    for bar in bars {
        let values = [
            Some(bar.open),
            Some(bar.high),
            Some(bar.low),
            Some(bar.close),
            bar.atr,
            bar.atr_ratio,
            bar.rsi,
            bar.adx,
            bar.close_position,
            bar.upper_wick,
            bar.lower_wick,
            bar.body_ratio,
        ];
        if values.into_iter().flatten().any(|value| !value.is_finite()) {
            return Err(MetaError::NotFinite);
        }
    }
    Ok(())
}

fn stats(rows: &[CheckRow], side: Side) -> Result<(Stats, Vec<f64>), MetaError> {
    if rows.is_empty() {
        return Ok((
            Stats {
                n: 0,
                mean_net: None,
                hit_rate: None,
                half_years: Vec::new(),
                positive_half_years: 0,
            },
            Vec::new(),
        ));
    }
    let mut returns = Vec::with_capacity(rows.len());
    let mut buckets: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for row in rows {
        let gross = match side {
            Side::Long => (row.exit_close - row.entry_open) / row.entry_open,
            Side::Short => (row.entry_open - row.exit_close) / row.entry_open,
        };
        let value = gross - COST_BPS / 10_000.0;
        if !value.is_finite() {
            return Err(MetaError::NotFinite);
        }
        returns.push(value);
        buckets
            .entry(half_year(row.entry_time))
            .or_default()
            .push(value);
    }
    let mut half_years = Vec::with_capacity(buckets.len());
    let mut positive = 0;
    for (bucket, values) in buckets {
        let bucket_mean = mean(&values).ok_or(MetaError::NotFinite)?;
        if values.len() >= MIN_HALF_YEAR_EVENTS && bucket_mean > 0.0 {
            positive += 1;
        }
        half_years.push(HalfYear {
            bucket,
            n: values.len(),
            mean_net: bucket_mean,
        });
    }
    Ok((
        Stats {
            n: returns.len(),
            mean_net: mean(&returns),
            hit_rate: win_rate(&returns),
            half_years,
            positive_half_years: positive,
        },
        returns,
    ))
}

fn non_overlapping(rows: &[CheckRow]) -> Vec<CheckRow> {
    let mut selected = Vec::new();
    let mut last_exit: Option<usize> = None;
    for row in rows {
        if last_exit.is_some_and(|last| row.entry_index <= last) {
            continue;
        }
        selected.push(*row);
        last_exit = Some(row.exit_index);
    }
    selected
}

fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let sum: f64 = values.iter().sum();
    Some(sum / values.len() as f64)
}

fn win_rate(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().filter(|value| **value > 0.0).count() as f64 / values.len() as f64)
}

fn quality_of(value: f64) -> MarkQuality {
    if value > 0.0 {
        MarkQuality::Good
    } else if value < 0.0 {
        MarkQuality::Bad
    } else {
        MarkQuality::Neutral
    }
}

fn stack_label(code: Option<i32>) -> &'static str {
    match code {
        Some(-1) => "bear",
        Some(0) => "mixed",
        Some(1) => "bull",
        _ => "—",
    }
}

fn zone_label(code: Option<i32>) -> &'static str {
    match code {
        Some(-1) => "oversold",
        Some(0) => "middle",
        Some(1) => "overbought",
        _ => "—",
    }
}

fn side_label(side: Side) -> &'static str {
    match side {
        Side::Long => "long",
        Side::Short => "short",
    }
}

const fn side_code(side: Side) -> u8 {
    match side {
        Side::Long => 0,
        Side::Short => 1,
    }
}

const fn action_code(action: MarkAction) -> u8 {
    match action {
        MarkAction::Entry => 0,
        MarkAction::Exit => 1,
    }
}

const fn quality_code(quality: MarkQuality) -> u8 {
    // The reference sorts the quality field as text: bad < good < neutral.
    match quality {
        MarkQuality::Bad => 0,
        MarkQuality::Good => 1,
        MarkQuality::Neutral => 2,
    }
}

/// The UTC half-year label for a timestamp.
fn half_year(timestamp: i64) -> String {
    let days = timestamp.div_euclid(86_400);
    let (year, month, _) = civil_from_days(days);
    format!("{year}H{}", if month <= 6 { 1 } else { 2 })
}

/// Convert days since the Unix epoch to a civil date.
///
/// This is Howard Hinnant's `civil_from_days`, which is exact for every date the
/// system records and needs no clock or calendar dependency.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    (
        if month <= 2 { year + 1 } else { year },
        month as u32,
        day as u32,
    )
}

#[cfg(test)]
mod tests;
