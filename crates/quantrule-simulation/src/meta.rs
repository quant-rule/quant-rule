//! Meta-labelling the human marks and checking the context split.
//!
//! A human mark says where an entry looked good or bad with the future visible.
//! The meta stage replaces that judgement with a measured label: the forward
//! result over a fixed horizon, grouped by the context that was knowable at the
//! entry. The best context cell with enough marks becomes a candidate split, and
//! the split is checked on a later window that carries no marks.
//!
//! The stage is descriptive. It does not claim an edge, it does not fit a model,
//! and it never reaches the strategy path. It is the Rust port of the retired
//! harness meta stage; the numeric rules and bucket boundaries match the
//! Python reference, and every fixture value is compared to `1e-9`.
//!
//! `port-parity: PORT-001` names the golden fixture this module is compared
//! against. The comparison test is `crates/quantrule-simulation/tests/port_parity.rs`.

// port-parity: PORT-001

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bundle::{Bar, Bundle, mean};
use crate::check::{self, MIN_POSITIVE_HALF_YEARS};
use crate::marks::{Mark, MarkAction, MarkQuality, Side};

/// The fewest marks a cell needs before it may become the candidate split.
pub const MIN_CELL: usize = 30;
/// The forward horizon of the check, in bars.
pub const CHECK_HORIZON: usize = 20;
/// The round-trip cost proxy, in basis points.
pub const COST_BPS: f64 = 6.0;
/// How far a mark may sit from a bar timestamp, in seconds.
pub const MAX_NEAREST_SECONDS: i64 = 450;

/// Why a meta result cannot be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaError {
    /// A bar carried a value that is not a finite number.
    NotFinite,
}

impl std::fmt::Display for MetaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFinite => formatter.write_str("a meta input is not a finite number"),
        }
    }
}

impl std::error::Error for MetaError {}

/// The forward path of one entry, in ATR.
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

impl Verdict {
    /// The schema word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Underpowered => "underpowered",
            Self::Survives => "survives",
            Self::NoPattern => "no_pattern",
        }
    }
}

/// The candidate split checked on the unmarked window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Check {
    /// The cell under test.
    pub cell: Cell,
    /// The filtered set.
    pub filtered: check::Stats,
    /// The unfiltered control.
    pub control: check::Stats,
    /// Whether a side had fewer than [`MIN_CELL`] events.
    pub underpowered: bool,
    /// The screen verdict.
    pub verdict: Verdict,
}

/// The complete meta stage result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetaResult {
    /// The labelled entries, in the order the marks were supplied.
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

/// Label every entry mark by its forward path and the context at the entry.
///
/// Rows follow the order the marks were supplied, which is the order the
/// reference writes them in. The context tables and the candidate do not depend
/// on that order.
#[must_use]
pub fn label_marks(bundle: &Bundle, marks: &[Mark], horizon: usize) -> (Vec<Row>, usize) {
    let mut rows = Vec::new();
    let mut unmatched = 0;
    for mark in marks {
        if mark.action != MarkAction::Entry {
            continue;
        }
        let Ok(moment) = crate::bundle::parse_utc(&mark.marked_time_utc) else {
            unmatched += 1;
            continue;
        };
        let Some(index) = nearest_bar(&bundle.bars, moment, MAX_NEAREST_SECONDS) else {
            unmatched += 1;
            continue;
        };
        let Some(path) = forward_path(&bundle.bars, index, mark.side, horizon) else {
            unmatched += 1;
            continue;
        };
        let bar = &bundle.bars[index];
        rows.push(Row {
            moment,
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
    let index = match (lower, upper) {
        (Some(left), Some(right)) => {
            let left_distance = bars[left].timestamp.abs_diff(moment);
            let right_distance = bars[right].timestamp.abs_diff(moment);
            if left_distance <= right_distance {
                left
            } else {
                right
            }
        }
        (Some(index), None) | (None, Some(index)) => index,
        (None, None) => return None,
    };
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
    let end = (index + horizon).min(bars.len().checked_sub(1)?);
    if end <= index {
        return None;
    }
    let mut mfe = 0.0_f64;
    let mut mae = 0.0_f64;
    for bar in &bars[index + 1..=end] {
        let (favorable, adverse) = match side {
            Side::Long => (bar.high - entry.close, entry.close - bar.low),
            Side::Short => (entry.close - bar.low, bar.high - entry.close),
        };
        if favorable > mfe {
            mfe = favorable;
        }
        if adverse > mae {
            mae = adverse;
        }
    }
    Some(ForwardPath {
        return_50: (bars[end].close - entry.close) / entry.close * side.sign(),
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
                row.side.as_str().to_owned(),
                row.ema_stack.clone(),
                row.atr_bucket.clone(),
            ))
            .or_default()
            .push(row);
    }
    groups
        .into_iter()
        .filter_map(|((_side, stack, bucket), group)| {
            let first = group.first()?;
            let returns: Vec<f64> = group.iter().map(|row| row.return_50).collect();
            Some(Cell {
                side: first.side,
                ema_stack: stack,
                atr_bucket: bucket,
                n: group.len(),
                mean_return: mean(&returns)?,
                hit_rate: hit_rate(&returns)?,
                mean_mfe: mean(&group.iter().map(|row| row.mfe_50).collect::<Vec<_>>())?,
                mean_mae: mean(&group.iter().map(|row| row.mae_50).collect::<Vec<_>>())?,
            })
        })
        .collect()
}

/// Pick the highest-mean cell with at least `minimum` marks.
///
/// Ties keep the table order, which is side, stack, and bucket order, so the
/// choice is deterministic.
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
#[must_use]
fn check_rows(
    bars: &[Bar],
    cell: &Cell,
    from: i64,
    to: i64,
    horizon: usize,
) -> (Vec<check::Row>, Vec<check::Row>) {
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
        let row = check::Row {
            event_bar: *bar,
            direction: None,
            reference: None,
            confirmation_bar: None,
            entry_index,
            entry_time: entry.timestamp,
            exit_index,
            entry_open: entry.open,
            exit_close: exit.close,
        };
        control.push(row.clone());
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
/// # Errors
///
/// Returns [`MetaError`] when a bar carries a non-finite value.
pub fn evaluate(bars: &[Bar], cell: &Cell, from: i64, to: i64) -> Result<Check, MetaError> {
    validate_bars(bars)?;
    let (filtered_rows, control_rows) = check_rows(bars, cell, from, to, CHECK_HORIZON);
    let filtered = check::stats(
        &check::non_overlapping(&filtered_rows, CHECK_HORIZON),
        cell.side.as_str(),
        COST_BPS,
    );
    let control = check::stats(
        &check::non_overlapping(&control_rows, CHECK_HORIZON),
        cell.side.as_str(),
        COST_BPS,
    );
    let underpowered = filtered.n < MIN_CELL || control.n < MIN_CELL;
    let survives = !underpowered
        && filtered.mean_net.is_some_and(|value| value > 0.0)
        && filtered.hit_rate.is_some_and(|value| value > 0.5)
        && filtered.mean_net > control.mean_net
        && filtered.hit_rate > control.hit_rate
        && filtered
            .positive_half_years
            .is_some_and(|value| value >= MIN_POSITIVE_HALF_YEARS);
    let verdict = if underpowered {
        Verdict::Underpowered
    } else if survives {
        Verdict::Survives
    } else {
        Verdict::NoPattern
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
/// # Errors
///
/// Returns [`MetaError`] when a bar carries a non-finite number.
pub fn build(bundle: &Bundle, marks: &[Mark], config: &Config) -> Result<MetaResult, MetaError> {
    validate_bars(&bundle.bars)?;
    let (all_rows, unmatched) = label_marks(bundle, marks, config.horizon);
    let rows: Vec<Row> = all_rows
        .into_iter()
        .filter(|row| row.moment >= config.label_from && row.moment <= config.label_to)
        .collect();
    let cells = cell_table(&rows);
    let candidate = best_cell(&cells, config.min_cell);
    let check = match &candidate {
        Some(cell) => Some(evaluate(
            &bundle.bars,
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

fn hit_rate(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().filter(|value| **value > 0.0).count() as f64 / values.len() as f64)
    }
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

#[cfg(test)]
mod tests;
