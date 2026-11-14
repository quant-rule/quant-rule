//! Comparing exit rules on the same entries.
//!
//! The entry is not the interesting decision; the exit is. This module takes a
//! fixed entry set and simulates five rules on it: a fixed 2R bracket, an EMA20
//! trail, a last-confirmed-pivot trail, a time stop, and a 1R partial with the
//! remainder at 2R, stop, or time. It compares the captured part of the forward
//! move and never changes the entry.
//!
//! The lab is descriptive. A positive exit on a negative entry set is not an
//! edge; it means the exit is less bad.
//!
//! `port-parity: PORT-001` names the golden fixture this module is compared
//! against.

// port-parity: PORT-001

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bundle::{Bundle, mean};
use crate::marks::{Mark, MarkAction, Side};

/// The record schema.
pub const EXIT_SCHEMA: &str = "quantrule.simulation-exit/v1";
/// The five rules, in the reference's order.
pub const RULES: [&str; 5] = [
    "fixed_2r",
    "ema20_trail",
    "swing_trail",
    "time",
    "partial_1r",
];

/// The fixed entry set and bracket parameters of one lab.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitConfig {
    /// The run identity.
    pub run_id: String,
    /// The first label-entry timestamp.
    pub from: i64,
    /// The last label-entry timestamp.
    pub to: i64,
    /// The first check-entry timestamp.
    pub check_from: i64,
    /// The last check-entry timestamp.
    pub check_to: i64,
    /// The label window as recorded.
    pub from_text: String,
    /// The label window end as recorded.
    pub to_text: String,
    /// The check window as recorded.
    pub check_from_text: String,
    /// The check window end as recorded.
    pub check_to_text: String,
    /// The event code the check entries come from.
    pub event_code: u32,
    /// Bars held at most.
    pub horizon_bars: usize,
    /// The fixed target, in ATR.
    pub target_atr: f64,
    /// The fixed stop, in ATR.
    pub stop_atr: f64,
    /// The partial target, in ATR.
    pub partial_atr: f64,
    /// The round-trip cost proxy, in basis points.
    pub cost_bps: f64,
}

/// Where one fixed entry came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntrySource {
    /// A human or oracle mark.
    Mark,
    /// An event bar.
    Event,
}

/// One fixed entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// The bar index.
    pub index: usize,
    /// The side.
    pub side: Side,
    /// Where the entry came from.
    pub source: EntrySource,
}

/// One rule's result for one entry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RuleResult {
    /// The net return after the cost proxy.
    pub return_pct: f64,
    /// The captured share of the favorable move, absent when there was none.
    pub captured: Option<f64>,
    /// The exit index.
    pub exit_index: usize,
    /// The exit bar identity.
    pub exit_bar: u64,
}

/// The five rule results for one entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntryResult {
    /// The side.
    pub side: Side,
    /// The results by rule name.
    pub results: BTreeMap<String, RuleResult>,
}

/// One aggregated table row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Aggregate {
    /// The set label, `label` or `check`.
    pub set: String,
    /// The side.
    pub side: Side,
    /// The rule.
    pub rule: String,
    /// Entries in the group.
    pub n: usize,
    /// Mean net return.
    pub mean_return: f64,
    /// Share of positive returns.
    pub hit_rate: f64,
    /// Mean captured share, absent when no entry had a favorable move.
    pub mean_captured: Option<f64>,
}

/// One recorded window as text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowText {
    /// The window start.
    #[serde(rename = "from")]
    pub from: String,
    /// The window end.
    #[serde(rename = "to")]
    pub to: String,
}

/// The complete `exit-lab.json` record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExitRecord {
    /// The record schema.
    pub schema: String,
    /// The run identity.
    pub run_id: String,
    /// The model identity.
    pub model: String,
    /// The registry identity.
    pub registry_version: String,
    /// The recorded data version.
    pub data_version: String,
    /// The label window.
    pub label_window: WindowText,
    /// The check window.
    pub check_window: WindowText,
    /// Bars held at most.
    pub horizon_bars: usize,
    /// The round-trip cost proxy.
    pub cost_bps: f64,
    /// Label entries simulated.
    pub label_entries: usize,
    /// Check entries simulated.
    pub check_entries: usize,
    /// The label table.
    pub label_table: Vec<Aggregate>,
    /// The check table.
    pub check_table: Vec<Aggregate>,
    /// The claim boundary.
    pub claim: String,
}

/// Why an exit lab cannot be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExitError {
    /// The entry and stop prices are unusable.
    InvalidPrices,
}

impl std::fmt::Display for ExitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("the entry and stop prices are unusable")
    }
}

impl std::error::Error for ExitError {}

/// Use the marked entries as the fixed entry set.
#[must_use]
pub fn entries_from_marks(bundle: &Bundle, marks: &[Mark], from: i64, to: i64) -> Vec<Entry> {
    let mut entries = Vec::new();
    for mark in marks {
        if mark.action != MarkAction::Entry {
            continue;
        }
        let Some(index) = bundle.index_of(mark.bar) else {
            continue;
        };
        let bar = bundle.bars[index];
        if !(from..=to).contains(&bar.timestamp) {
            continue;
        }
        entries.push(Entry {
            index,
            side: mark.side,
            source: EntrySource::Mark,
        });
    }
    entries
}

/// Use every event bar as the same entry for both sides.
#[must_use]
pub fn entries_from_events(bundle: &Bundle, event_code: u32, from: i64, to: i64) -> Vec<Entry> {
    let mut entries = Vec::new();
    for event in &bundle.events {
        if event.code != event_code {
            continue;
        }
        let Some(index) = bundle.index_of(event.available_bar) else {
            continue;
        };
        let bar = bundle.bars[index];
        if !(from..=to).contains(&bar.timestamp) {
            continue;
        }
        for side in [Side::Long, Side::Short] {
            entries.push(Entry {
                index,
                side,
                source: EntrySource::Event,
            });
        }
    }
    entries.sort_by_key(|entry| (entry.index, entry.side.as_str()));
    entries
}

/// Keep entries that do not overlap a held position on their side.
#[must_use]
pub fn non_overlapping(entries: &[Entry], horizon: usize) -> Vec<Entry> {
    let mut selected = Vec::new();
    let mut last_by_side: BTreeMap<Side, usize> = BTreeMap::new();
    for entry in entries {
        if last_by_side
            .get(&entry.side)
            .is_some_and(|last| entry.index <= *last)
        {
            continue;
        }
        selected.push(*entry);
        last_by_side.insert(entry.side, entry.index + horizon);
    }
    selected
}

fn net_return(entry: f64, exit_price: f64, side: Side, cost_bps: f64) -> f64 {
    let gross = match side {
        Side::Long => (exit_price - entry) / entry,
        Side::Short => (entry - exit_price) / entry,
    };
    gross - cost_bps / 10_000.0
}

struct RuleContext<'a> {
    bundle: &'a Bundle,
    entry: f64,
    side: Side,
    cost_bps: f64,
    mfe_return: f64,
}

impl RuleContext<'_> {
    fn result(&self, exit_price: f64, exit_index: usize, forced_return: Option<f64>) -> RuleResult {
        let value = forced_return
            .unwrap_or_else(|| net_return(self.entry, exit_price, self.side, self.cost_bps));
        RuleResult {
            return_pct: value,
            captured: if self.mfe_return > 0.0 {
                Some(value / self.mfe_return)
            } else {
                None
            },
            exit_index,
            exit_bar: self.bundle.bars[exit_index].bar,
        }
    }
}

/// Simulate the five rules from one entry bar close.
#[must_use]
pub fn simulate(
    bundle: &Bundle,
    index: usize,
    side: Side,
    config: &ExitConfig,
) -> Option<BTreeMap<String, RuleResult>> {
    let bar = bundle.bars.get(index)?;
    let atr = bar.atr?;
    if !atr.is_finite() || atr <= 0.0 {
        return None;
    }
    let entry = bar.close;
    let horizon = config.horizon_bars;
    let target = config.target_atr;
    let stop = config.stop_atr;
    let partial = config.partial_atr;
    let cost = config.cost_bps;
    let end = (index + horizon).min(bundle.bars.len().checked_sub(1)?);
    if end <= index {
        return None;
    }
    let mut mfe = 0.0_f64;
    for position in index + 1..=end {
        let candidate = &bundle.bars[position];
        let favorable = match side {
            Side::Long => candidate.high - entry,
            Side::Short => entry - candidate.low,
        };
        if favorable > mfe {
            mfe = favorable;
        }
    }
    let mfe_atr = mfe / atr;
    let mfe_return = mfe_atr * atr / entry;
    let context = RuleContext {
        bundle,
        entry,
        side,
        cost_bps: cost,
        mfe_return,
    };
    let mut results = BTreeMap::new();

    // Fixed 2R bracket, stop-first on an equal-bar tie.
    let mut exit_price = None;
    let mut exit_index = end;
    for position in index + 1..=end {
        let candidate = &bundle.bars[position];
        match side {
            Side::Long => {
                if candidate.low <= entry - stop * atr {
                    exit_price = Some(entry - stop * atr);
                } else if candidate.high >= entry + target * atr {
                    exit_price = Some(entry + target * atr);
                }
            }
            Side::Short => {
                if candidate.high >= entry + stop * atr {
                    exit_price = Some(entry + stop * atr);
                } else if candidate.low <= entry - target * atr {
                    exit_price = Some(entry - target * atr);
                }
            }
        }
        if exit_price.is_some() {
            exit_index = position;
            break;
        }
    }
    let exit_price = exit_price.unwrap_or(bundle.bars[end].close);
    results.insert(
        "fixed_2r".to_owned(),
        context.result(exit_price, exit_index, None),
    );

    // EMA20 trail.
    let mut exit_price = None;
    let mut exit_index = end;
    for position in index + 1..=end {
        let candidate = &bundle.bars[position];
        let Some(ema) = candidate.ema_fast else {
            continue;
        };
        let crossed = match side {
            Side::Long => candidate.close < ema,
            Side::Short => candidate.close > ema,
        };
        if crossed {
            exit_price = Some(candidate.close);
            exit_index = position;
            break;
        }
    }
    let exit_price = exit_price.unwrap_or(bundle.bars[end].close);
    results.insert(
        "ema20_trail".to_owned(),
        context.result(exit_price, exit_index, None),
    );

    // Swing trail at the last confirmed pivot.
    let mut exit_price = None;
    let mut exit_index = end;
    for position in index + 1..=end {
        let candidate = &bundle.bars[position];
        let level = match side {
            Side::Long => candidate.value("last_pivot_low"),
            Side::Short => candidate.value("last_pivot_high"),
        };
        let Some(level) = level else {
            continue;
        };
        let crossed = match side {
            Side::Long => candidate.close < level,
            Side::Short => candidate.close > level,
        };
        if crossed {
            exit_price = Some(candidate.close);
            exit_index = position;
            break;
        }
    }
    let exit_price = exit_price.unwrap_or(bundle.bars[end].close);
    results.insert(
        "swing_trail".to_owned(),
        context.result(exit_price, exit_index, None),
    );

    // Time stop.
    results.insert(
        "time".to_owned(),
        context.result(bundle.bars[end].close, end, None),
    );

    // 1R partial, remainder at 2R / stop / time.
    let mut half_done = false;
    let mut realized = 0.0_f64;
    let mut exit_index = end;
    let mut broke = false;
    for position in index + 1..=end {
        let candidate = &bundle.bars[position];
        match side {
            Side::Long => {
                if candidate.low <= entry - stop * atr {
                    let stop_return = net_return(entry, entry - stop * atr, side, 0.0);
                    realized = if half_done {
                        realized + 0.5 * stop_return
                    } else {
                        stop_return
                    };
                    exit_index = position;
                    broke = true;
                    break;
                }
                if !half_done && candidate.high >= entry + partial * atr {
                    realized += 0.5 * net_return(entry, entry + partial * atr, side, 0.0);
                    half_done = true;
                }
                if candidate.high >= entry + target * atr {
                    let target_return = net_return(entry, entry + target * atr, side, 0.0);
                    realized = if half_done {
                        realized + 0.5 * target_return
                    } else {
                        target_return
                    };
                    exit_index = position;
                    broke = true;
                    break;
                }
            }
            Side::Short => {
                if candidate.high >= entry + stop * atr {
                    let stop_return = net_return(entry, entry + stop * atr, side, 0.0);
                    realized = if half_done {
                        realized + 0.5 * stop_return
                    } else {
                        stop_return
                    };
                    exit_index = position;
                    broke = true;
                    break;
                }
                if !half_done && candidate.low <= entry - partial * atr {
                    realized += 0.5 * net_return(entry, entry - partial * atr, side, 0.0);
                    half_done = true;
                }
                if candidate.low <= entry - target * atr {
                    let target_return = net_return(entry, entry - target * atr, side, 0.0);
                    realized = if half_done {
                        realized + 0.5 * target_return
                    } else {
                        target_return
                    };
                    exit_index = position;
                    broke = true;
                    break;
                }
            }
        }
    }
    if !broke {
        if half_done {
            realized += 0.5 * net_return(entry, bundle.bars[end].close, side, 0.0);
        } else {
            realized = net_return(entry, bundle.bars[end].close, side, 0.0);
        }
    }
    results.insert(
        "partial_1r".to_owned(),
        context.result(
            bundle.bars[exit_index].close,
            exit_index,
            Some(realized - cost / 10_000.0),
        ),
    );
    Some(results)
}

/// Aggregate the per-entry results by side and rule.
#[must_use]
pub fn aggregate(rows: &[EntryResult], label: &str) -> Vec<Aggregate> {
    let mut groups: BTreeMap<(&str, &str), Vec<&RuleResult>> = BTreeMap::new();
    for row in rows {
        for (rule, result) in &row.results {
            groups
                .entry((row.side.as_str(), rule.as_str()))
                .or_default()
                .push(result);
        }
    }
    groups
        .into_iter()
        .map(|((side, rule), results)| {
            let returns: Vec<f64> = results.iter().map(|result| result.return_pct).collect();
            let captured: Vec<f64> = results
                .iter()
                .filter_map(|result| result.captured)
                .collect();
            Aggregate {
                set: label.to_owned(),
                side: Side::parse(side).expect("stored schema word"),
                rule: rule.to_owned(),
                n: results.len(),
                mean_return: mean(&returns).unwrap_or(0.0),
                hit_rate: returns.iter().filter(|value| **value > 0.0).count() as f64
                    / returns.len() as f64,
                mean_captured: mean(&captured),
            }
        })
        .collect()
}

/// Run the exit comparison over supplied check entries and build the record.
///
/// The label entries come from the marks when present, otherwise from the
/// event code. The check entries are filtered by the per-side non-overlap rule;
/// the filter is idempotent, so an already filtered list is accepted.
///
/// # Errors
///
/// Returns [`ExitError`] when an entry price is unusable.
pub fn build_with_entries(
    bundle: &Bundle,
    config: &ExitConfig,
    marks: Option<&[Mark]>,
    check_entries: &[Entry],
) -> Result<ExitRecord, ExitError> {
    let label_entries = match marks {
        Some(marks) => entries_from_marks(bundle, marks, config.from, config.to),
        None => entries_from_events(bundle, config.event_code, config.from, config.to),
    };
    let mut label_rows = Vec::new();
    for entry in &label_entries {
        if let Some(results) = simulate(bundle, entry.index, entry.side, config) {
            label_rows.push(EntryResult {
                side: entry.side,
                results,
            });
        }
    }
    let mut check_rows = Vec::new();
    for entry in non_overlapping(check_entries, config.horizon_bars) {
        if let Some(results) = simulate(bundle, entry.index, entry.side, config) {
            check_rows.push(EntryResult {
                side: entry.side,
                results,
            });
        }
    }
    let label_table = aggregate(&label_rows, "label");
    let check_table = aggregate(&check_rows, "check");
    Ok(ExitRecord {
        schema: EXIT_SCHEMA.to_owned(),
        run_id: config.run_id.clone(),
        model: "M1.3".to_owned(),
        registry_version: "2026-11-10.2".to_owned(),
        data_version: bundle.data_version().to_owned(),
        label_window: WindowText {
            from: config.from_text.clone(),
            to: config.to_text.clone(),
        },
        check_window: WindowText {
            from: config.check_from_text.clone(),
            to: config.check_to_text.clone(),
        },
        horizon_bars: config.horizon_bars,
        cost_bps: config.cost_bps,
        label_entries: label_rows.len(),
        check_entries: check_rows.len(),
        label_table,
        check_table,
        claim: "descriptive exit comparison; no edge claim".to_owned(),
    })
}

/// Run the exit comparison over a recorded bundle and build the record.
///
/// # Errors
///
/// Returns [`ExitError`] when an entry price is unusable.
pub fn build(
    bundle: &Bundle,
    config: &ExitConfig,
    marks: Option<&[Mark]>,
) -> Result<ExitRecord, ExitError> {
    let entries = entries_from_events(
        bundle,
        config.event_code,
        config.check_from,
        config.check_to,
    );
    build_with_entries(bundle, config, marks, &entries)
}

impl ExitRecord {
    /// Render `exit-lab.md`.
    #[must_use]
    pub fn render_markdown(&self) -> String {
        let mut lines = vec![
            format!("# {} — Exit lab", self.run_id),
            String::new(),
            "- Model: `M1.3`; registry `2026-11-10.2`".to_owned(),
            format!(
                "- Label entries: {}; check entries: {}",
                self.label_entries, self.check_entries
            ),
            format!(
                "- Horizon: {} bars; cost proxy: {} bps",
                self.horizon_bars,
                python_number(self.cost_bps)
            ),
            "- Claim: descriptive exit comparison; no edge claim".to_owned(),
            String::new(),
            "## Label set".to_owned(),
            String::new(),
            "| Side | Rule | n | Mean return | Hit rate | Mean captured |".to_owned(),
            "|---|---|---:|---:|---:|---:|".to_owned(),
        ];
        for row in &self.label_table {
            lines.push(aggregate_line(row));
        }
        lines.extend([
            String::new(),
            "## Check set (2020–2022, non-overlapping)".to_owned(),
            String::new(),
            "| Side | Rule | n | Mean return | Hit rate | Mean captured |".to_owned(),
            "|---|---|---:|---:|---:|---:|".to_owned(),
        ]);
        for row in &self.check_table {
            lines.push(aggregate_line(row));
        }
        lines.extend([
            String::new(),
            "## Reading".to_owned(),
            String::new(),
            "- The comparison holds the entries fixed; it asks only which exit keeps more of the move.".to_owned(),
            "- A positive exit on a negative entry set is not an edge; it means the exit is less bad.".to_owned(),
            "- The check is descriptive and does not use validation or holdout data.".to_owned(),
            String::new(),
        ]);
        lines.join("\n")
    }
}

fn aggregate_line(row: &Aggregate) -> String {
    format!(
        "| {} | {} | {} | {} | {}% | {} |",
        row.side.as_str(),
        row.rule,
        row.n,
        format_return(row.mean_return),
        format_fraction(row.hit_rate * 100.0),
        row.mean_captured
            .map_or_else(|| "—".to_owned(), |value| format!("{value:.2}"))
    )
}

fn format_return(value: f64) -> String {
    format!("{:+.2}%", value * 100.0)
}

fn format_fraction(value: f64) -> String {
    format!("{value:.2}")
}

fn python_number(value: f64) -> String {
    let text = format!("{value}");
    if text.contains('.') || text.contains('e') {
        text
    } else {
        format!("{text}.0")
    }
}

#[cfg(test)]
mod tests;
