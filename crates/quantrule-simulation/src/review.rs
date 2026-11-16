//! The descriptive review of one run's marks.
//!
//! The review pairs each chart, side, and quality, measures the close-to-close
//! return of the marked round trip, aggregates the oracle's MFE and MAE to the
//! rule exit, and describes the context at the entry. It is descriptive: it
//! compares good and bad marks and shows how much of the favorable move the
//! rule kept. It does not fit a model, optimize a threshold, or claim an edge.
//!
//! `port-parity: PORT-001` names the golden fixture this module is compared
//! against.

// port-parity: PORT-001

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::bundle::{Bundle, Catalog, mean};
use crate::marks::{Mark, MarkQuality, Side, count_by_quality};
use crate::oracle::Detail;

/// Why a review cannot be produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewError {
    /// A bar the review names is not in the bundle.
    MissingBar(u64),
}

impl std::fmt::Display for ReviewError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingBar(bar) => write!(formatter, "bar {bar} is not in the bundle"),
        }
    }
}

impl std::error::Error for ReviewError {}

/// One entry/exit pair.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pair {
    /// The chart.
    pub chart: String,
    /// The side.
    pub side: Side,
    /// The quality.
    pub quality: MarkQuality,
    /// The entry bar.
    pub entry_bar: u64,
    /// The exit bar.
    pub exit_bar: u64,
}

/// One pair measured close to close.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeStats {
    /// The pair.
    pub pair: Pair,
    /// The entry close.
    pub entry_price: f64,
    /// The exit close.
    pub exit_price: f64,
    /// The signed return, absent when the entry price is zero.
    pub return_pct: Option<f64>,
    /// The ATR at the entry.
    pub atr: Option<f64>,
}

/// The context at one entry bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    /// The EMA stack label.
    pub ema_stack: String,
    /// The RSI zone label.
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
    /// The registry names of the events available at the entry.
    pub events: Vec<String>,
}

/// One row of the round-trip table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRow {
    /// The quality.
    pub quality: MarkQuality,
    /// The side.
    pub side: Side,
    /// Pairs in the group.
    pub n: usize,
    /// Mean signed return.
    pub mean_return: Option<f64>,
    /// Mean favorable excursion to the rule exit, in ATR.
    pub mean_mfe: Option<f64>,
    /// Mean adverse excursion to the rule exit, in ATR.
    pub mean_mae: Option<f64>,
    /// Mean captured share of the favorable move, for good marks.
    pub captured: Option<f64>,
}

/// One row of the context table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextRow {
    /// The quality.
    pub quality: MarkQuality,
    /// The side.
    pub side: Side,
    /// EMA stack counts.
    pub stacks: Vec<(String, usize)>,
    /// RSI zone counts.
    pub zones: Vec<(String, usize)>,
    /// Mean RSI.
    pub mean_rsi: Option<f64>,
    /// Mean ATR ratio.
    pub mean_atr_ratio: Option<f64>,
    /// Mean ADX.
    pub mean_adx: Option<f64>,
    /// Mean close position.
    pub mean_close_position: Option<f64>,
    /// Mean upper wick ratio.
    pub mean_upper_wick: Option<f64>,
    /// Mean lower wick ratio.
    pub mean_lower_wick: Option<f64>,
    /// Mean body ratio.
    pub mean_body_ratio: Option<f64>,
    /// The six most common other events at the entries.
    pub events: Vec<(String, usize)>,
}

/// The review of one run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Review {
    /// Marks per quality and side.
    pub mark_counts: Vec<(MarkQuality, Side, usize)>,
    /// The round-trip table, ordered by quality and side.
    pub round_trips: Vec<TableRow>,
    /// The context table, ordered by quality and side.
    pub contexts: Vec<ContextRow>,
}

type GroupKey = (String, &'static str, &'static str);
type PairGroups<'a> = BTreeMap<GroupKey, (Vec<&'a Mark>, Vec<&'a Mark>)>;

/// Pair the first entry and the first exit for each chart, side, and quality.
#[must_use]
pub fn pairs(marks: &[Mark]) -> Vec<Pair> {
    let mut groups: PairGroups<'_> = BTreeMap::new();
    for row in marks {
        let key = (row.chart.clone(), row.side.as_str(), row.quality.as_str());
        let group = groups.entry(key).or_default();
        match row.action {
            crate::marks::MarkAction::Entry => group.0.push(row),
            crate::marks::MarkAction::Exit => group.1.push(row),
        }
    }
    let mut pairs = Vec::new();
    for ((chart, _, _), (entries, exits)) in groups {
        if entries.is_empty() || exits.is_empty() {
            continue;
        }
        let mut entries = entries;
        entries.sort_by_key(|row| row.bar);
        let mut exits = exits;
        exits.sort_by_key(|row| row.bar);
        let entry = entries[0];
        let exit = exits
            .iter()
            .copied()
            .find(|row| row.bar >= entry.bar)
            .unwrap_or(exits[0]);
        pairs.push(Pair {
            chart: chart.clone(),
            side: entry.side,
            quality: entry.quality,
            entry_bar: entry.bar,
            exit_bar: exit.bar,
        });
    }
    pairs
}

/// Measure one pair close to close.
///
/// # Errors
///
/// Returns [`ReviewError::MissingBar`] when a paired bar is not in the bundle.
pub fn trade_stats(bundle: &Bundle, pair: &Pair) -> Result<TradeStats, ReviewError> {
    let entry_index = bundle
        .index_of(pair.entry_bar)
        .ok_or(ReviewError::MissingBar(pair.entry_bar))?;
    let exit_index = bundle
        .index_of(pair.exit_bar)
        .ok_or(ReviewError::MissingBar(pair.exit_bar))?;
    let entry_price = bundle.bars[entry_index].close;
    let exit_price = bundle.bars[exit_index].close;
    let atr = bundle.bars[entry_index].atr;
    let return_pct = if entry_price == 0.0 {
        None
    } else {
        Some((exit_price - entry_price) / entry_price * pair.side.sign())
    };
    Ok(TradeStats {
        pair: pair.clone(),
        entry_price,
        exit_price,
        return_pct,
        atr,
    })
}

/// Describe the context at one entry bar.
///
/// # Errors
///
/// Returns [`ReviewError::MissingBar`] when the bar is not in the bundle.
pub fn context(bundle: &Bundle, bar_id: u64, catalog: &Catalog) -> Result<Context, ReviewError> {
    let index = bundle
        .index_of(bar_id)
        .ok_or(ReviewError::MissingBar(bar_id))?;
    let bar = bundle.bars[index];
    Ok(Context {
        ema_stack: stack_label(bar.ema_stack).to_owned(),
        rsi_zone: zone_label(bar.rsi_zone).to_owned(),
        rsi: bar.rsi,
        atr_ratio: bar.atr_ratio,
        adx: bar.adx,
        close_position: bar.close_position,
        upper_wick: bar.upper_wick,
        lower_wick: bar.lower_wick,
        body_ratio: bar.body_ratio,
        events: bundle
            .events_at(bar_id)
            .iter()
            .map(|event| event.label(catalog))
            .collect(),
    })
}

/// Build the review of one run.
///
/// # Errors
///
/// Returns [`ReviewError::MissingBar`] when a marked bar is not in the bundle.
pub fn review(
    bundle: &Bundle,
    marks: &[Mark],
    details: &[Detail],
    catalog: &Catalog,
    focus_events: &BTreeSet<String>,
) -> Result<Review, ReviewError> {
    let stats: Vec<TradeStats> = pairs(marks)
        .iter()
        .map(|pair| trade_stats(bundle, pair))
        .collect::<Result<_, _>>()?;
    let detail_by_key: BTreeMap<(String, Side, MarkQuality), &Detail> = details
        .iter()
        .map(|row| ((row.chart.clone(), row.side, row.quality), row))
        .collect();
    let counts = count_by_quality(marks);
    let mut mark_counts: Vec<(MarkQuality, Side, usize)> = counts
        .into_iter()
        .map(|((quality, side), count)| (quality, side, count))
        .collect();
    mark_counts.sort_by_key(|(quality, side, _)| (quality.as_str(), side.as_str()));
    let round_trips = table(&stats, &detail_by_key);
    let contexts = context_table(&stats, bundle, catalog, focus_events)?;
    Ok(Review {
        mark_counts,
        round_trips,
        contexts,
    })
}

fn table(
    rows: &[TradeStats],
    details: &BTreeMap<(String, Side, MarkQuality), &Detail>,
) -> Vec<TableRow> {
    let mut groups: BTreeMap<(&'static str, &'static str), Vec<&TradeStats>> = BTreeMap::new();
    for row in rows {
        groups
            .entry((row.pair.quality.as_str(), row.pair.side.as_str()))
            .or_default()
            .push(row);
    }
    groups
        .into_iter()
        .map(|((quality, _side), group)| {
            let returns: Vec<Option<f64>> = group.iter().map(|row| row.return_pct).collect();
            let detail_rows: Vec<&Detail> = group
                .iter()
                .filter_map(|row| {
                    details
                        .get(&(row.pair.chart.clone(), row.pair.side, row.pair.quality))
                        .copied()
                })
                .collect();
            let mfes: Vec<Option<f64>> = detail_rows
                .iter()
                .map(|row| Some(row.mfe_to_rule_atr))
                .collect();
            let maes: Vec<Option<f64>> = detail_rows
                .iter()
                .map(|row| Some(row.mae_to_rule_atr))
                .collect();
            let mut captured = Vec::new();
            if quality == MarkQuality::Good.as_str() {
                for row in &group {
                    let detail =
                        details.get(&(row.pair.chart.clone(), row.pair.side, row.pair.quality));
                    let mfe = detail.map(|detail| detail.mfe_to_rule_atr);
                    if let (Some(mfe), Some(atr)) = (mfe, row.atr)
                        && mfe != 0.0
                        && atr != 0.0
                        && row.entry_price != 0.0
                    {
                        captured
                            .push(row.return_pct.unwrap_or(0.0) / (mfe * atr / row.entry_price));
                    }
                }
            }
            TableRow {
                quality: group[0].pair.quality,
                side: group[0].pair.side,
                n: group.len(),
                mean_return: mean_or_none(&returns),
                mean_mfe: mean_or_none(&mfes),
                mean_mae: mean_or_none(&maes),
                captured: mean(&captured),
            }
        })
        .collect()
}

fn context_table(
    rows: &[TradeStats],
    bundle: &Bundle,
    catalog: &Catalog,
    focus_events: &BTreeSet<String>,
) -> Result<Vec<ContextRow>, ReviewError> {
    let mut groups: BTreeMap<(&'static str, &'static str), Vec<&TradeStats>> = BTreeMap::new();
    for row in rows {
        groups
            .entry((row.pair.quality.as_str(), row.pair.side.as_str()))
            .or_default()
            .push(row);
    }
    groups
        .into_iter()
        .map(|((_quality, _side), group)| {
            let contexts: Vec<Context> = group
                .iter()
                .map(|row| context(bundle, row.pair.entry_bar, catalog))
                .collect::<Result<_, _>>()?;
            let mut stacks: BTreeMap<String, usize> = BTreeMap::new();
            let mut zones: BTreeMap<String, usize> = BTreeMap::new();
            let mut events: BTreeMap<String, usize> = BTreeMap::new();
            for context in &contexts {
                *stacks.entry(context.ema_stack.clone()).or_insert(0) += 1;
                *zones.entry(context.rsi_zone.clone()).or_insert(0) += 1;
                for event in &context.events {
                    if focus_events.contains(event) {
                        continue;
                    }
                    *events.entry(event.clone()).or_insert(0) += 1;
                }
            }
            let mut top_events: Vec<(String, usize)> = events.into_iter().collect();
            top_events
                .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            top_events.truncate(6);
            Ok(ContextRow {
                quality: group[0].pair.quality,
                side: group[0].pair.side,
                stacks: stacks.into_iter().collect(),
                zones: zones.into_iter().collect(),
                mean_rsi: mean_or_none(&contexts.iter().map(|row| row.rsi).collect::<Vec<_>>()),
                mean_atr_ratio: mean_or_none(
                    &contexts.iter().map(|row| row.atr_ratio).collect::<Vec<_>>(),
                ),
                mean_adx: mean_or_none(&contexts.iter().map(|row| row.adx).collect::<Vec<_>>()),
                mean_close_position: mean_or_none(
                    &contexts
                        .iter()
                        .map(|row| row.close_position)
                        .collect::<Vec<_>>(),
                ),
                mean_upper_wick: mean_or_none(
                    &contexts
                        .iter()
                        .map(|row| row.upper_wick)
                        .collect::<Vec<_>>(),
                ),
                mean_lower_wick: mean_or_none(
                    &contexts
                        .iter()
                        .map(|row| row.lower_wick)
                        .collect::<Vec<_>>(),
                ),
                mean_body_ratio: mean_or_none(
                    &contexts
                        .iter()
                        .map(|row| row.body_ratio)
                        .collect::<Vec<_>>(),
                ),
                events: top_events,
            })
        })
        .collect()
}

impl Review {
    /// Render the review document for one run.
    #[must_use]
    pub fn render(&self, run_id: &str, premise: &str) -> String {
        let mut lines: Vec<String> = vec![
            format!("# Review — {run_id}"),
            String::new(),
            format!("- Premise: {premise}"),
            "- Model: `M1.3`".to_owned(),
            "- Claim type: hindsight labels; no edge claim".to_owned(),
            String::new(),
            "## Mark counts".to_owned(),
            String::new(),
            "| Quality | Side | Marks |".to_owned(),
            "|---|---|---:|".to_owned(),
        ];
        for (quality, side, count) in &self.mark_counts {
            lines.push(format!(
                "| {} | {} | {count} |",
                quality.as_str(),
                side.as_str()
            ));
        }
        lines.extend([
            String::new(),
            "## Round trips".to_owned(),
            String::new(),
            "| Quality | Side | n | Mark return (close) | MFE to exit (ATR) | MAE to exit (ATR) | Captured (good only) |".to_owned(),
            "|---|---|---:|---:|---:|---:|---:|".to_owned(),
        ]);
        for row in &self.round_trips {
            let captured = row.captured.map_or_else(|| "—".to_owned(), format_pct);
            lines.push(format!(
                "| {} | {} | {} | {} | {} | {} | {captured} |",
                row.quality.as_str(),
                row.side.as_str(),
                row.n,
                format_mean(row.mean_return),
                format_mean(row.mean_mfe),
                format_mean(row.mean_mae)
            ));
        }
        lines.extend([
            String::new(),
            "## Context at the entry".to_owned(),
            String::new(),
            "| Quality | Side | EMA stack | RSI zone | Mean RSI | Mean ATR ratio | Mean ADX | Close pos | Upper wick | Lower wick | Body | Other events at entry |".to_owned(),
            "|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---|".to_owned(),
        ]);
        for row in &self.contexts {
            let stacks = row
                .stacks
                .iter()
                .map(|(key, value)| format!("{key} {value}"))
                .collect::<Vec<_>>()
                .join(", ");
            let zones = row
                .zones
                .iter()
                .map(|(key, value)| format!("{key} {value}"))
                .collect::<Vec<_>>()
                .join(", ");
            let events = if row.events.is_empty() {
                "—".to_owned()
            } else {
                row.events
                    .iter()
                    .map(|(key, value)| format!("{key} {value}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            lines.push(format!(
                "| {} | {} | {stacks} | {zones} | {} | {} | {} | {} | {} | {} | {} | {events} |",
                row.quality.as_str(),
                row.side.as_str(),
                format_mean(row.mean_rsi),
                format_mean(row.mean_atr_ratio),
                format_mean(row.mean_adx),
                format_mean(row.mean_close_position),
                format_mean(row.mean_upper_wick),
                format_mean(row.mean_lower_wick),
                format_mean(row.mean_body_ratio)
            ));
        }
        lines.extend([
            String::new(),
            "## Candidate rule".to_owned(),
            String::new(),
            "- Good marks share: (fill after the visual review)".to_owned(),
            "- Bad marks share: (fill after the visual review)".to_owned(),
            "- Candidate entry condition: (at most two conditions)".to_owned(),
            "- Candidate exit:".to_owned(),
            "- Rejected candidate:".to_owned(),
            String::new(),
            "## Limits".to_owned(),
            String::new(),
            "- The marks use future bars by design. They are labels, not signals.".to_owned(),
            "- The good/bad split is the oracle's forward-path criterion, not a market prediction; the context table is the only part that can discriminate before the exit.".to_owned(),
            "- The mark return is close-to-close at the marked bars, not a fill price.".to_owned(),
            "- The oracle is a heuristic; the AI visual review is subjective.".to_owned(),
            "- A sample under 30 non-overlapping windows is underpowered.".to_owned(),
            "- The check window (2020–2022) is separate and is not marked here.".to_owned(),
            String::new(),
        ]);
        lines.join("\n")
    }
}

fn mean_or_none(values: &[Option<f64>]) -> Option<f64> {
    let present: Vec<f64> = values.iter().filter_map(|value| *value).collect();
    mean(&present)
}

fn format_mean(value: Option<f64>) -> String {
    value.map_or_else(|| "—".to_owned(), |value| format!("{value:.2}"))
}

fn format_pct(value: f64) -> String {
    format!("{:+.2}%", value * 100.0)
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
mod tests {
    use super::*;
    use crate::marks::{Mark, MarkAction, MarkQuality, Side};

    fn bundle() -> Bundle {
        let bars = [100.0, 101.0]
            .iter()
            .enumerate()
            .map(|(index, close)| {
                crate::bundle::Bar::from_prices(
                    index as u64,
                    1_514_764_800 + index as i64 * 900,
                    *close,
                    close + 0.5,
                    close - 0.5,
                    *close,
                )
            })
            .collect();
        Bundle::new(900, bars, Vec::new())
    }

    fn marks() -> Vec<Mark> {
        vec![
            Mark {
                chart: "sim-w001.png".to_owned(),
                bar: 0,
                marked_time_utc: "2018-01-01T00:00:00Z".to_owned(),
                action: MarkAction::Entry,
                quality: MarkQuality::Good,
                side: Side::Long,
                note: String::new(),
            },
            Mark {
                chart: "sim-w001.png".to_owned(),
                bar: 1,
                marked_time_utc: "2018-01-01T00:15:00Z".to_owned(),
                action: MarkAction::Exit,
                quality: MarkQuality::Good,
                side: Side::Long,
                note: String::new(),
            },
        ]
    }

    #[test]
    fn a_pair_is_measured_close_to_close() {
        let pairs = pairs(&marks());
        assert_eq!(pairs.len(), 1);
        let stats = trade_stats(&bundle(), &pairs[0]).expect("stats");
        assert!((stats.return_pct.expect("return") - 0.01).abs() < 1e-12);
    }

    #[test]
    fn the_review_renders_the_tables() {
        let details = [Detail {
            chart: "sim-w001.png".to_owned(),
            run_id: "SIM-T".to_owned(),
            window_id: "w001".to_owned(),
            side: Side::Long,
            quality: MarkQuality::Good,
            entry_bar: 0,
            entry_index: 0,
            entry_price: 100.0,
            atr: 1.0,
            rule_exit_bar: Some(1),
            perfect_exit_bar: Some(1),
            mfe_atr: 2.0,
            mae_atr: 0.5,
            mfe_to_rule_atr: 2.0,
            mae_to_rule_atr: 0.5,
            mfe_before_bad_stop_atr: 2.0,
            good: true,
            bad: false,
        }];
        let review = review(
            &bundle(),
            &marks(),
            &details,
            &Catalog::new(),
            &BTreeSet::new(),
        )
        .expect("review");
        let text = review.render("SIM-T", "test premise");
        assert!(text.contains("Review — SIM-T"));
        assert!(text.contains("| good | long |"));
        assert!(text.contains("test premise"));
    }
}
