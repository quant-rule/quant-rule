//! The check: an event study on the unmarked window.
//!
//! A candidate rule found by marking is tested on a period that was never
//! marked. The event's available bar is the signal, the next bar's open is the
//! entry, the close after the declared horizon is the exit, and the declared
//! cost proxy is subtracted. The rule's filtered set is compared with an
//! unfiltered control, and the direction must hold across half-years.
//!
//! The check is a screen, not a backtest. A survivor goes to strategy research
//! with a frozen configuration and a pre-registration; it is not an edge and it
//! is not a baseline.
//!
//! `port-parity: PORT-001` names the golden fixture this module is compared
//! against.

// port-parity: PORT-001

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bundle::{Bar, Bundle, Catalog, half_year, mean};

/// The record schema.
pub const CHECK_SCHEMA: &str = "quantrule.simulation-check/v1";
/// The fewest events a set needs before it can speak.
pub const MIN_EVENTS: usize = 30;
/// The fewest events a half-year needs before it counts as a direction sample.
pub const MIN_HALF_YEAR_EVENTS: usize = 5;
/// The fewest positive half-years a surviving rule needs.
pub const MIN_POSITIVE_HALF_YEARS: usize = 3;

/// Where the control set comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Control {
    /// The unfiltered event set.
    Event,
    /// Every bar in the window.
    Bars,
}

impl Control {
    /// The schema word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Bars => "bars",
        }
    }
}

/// The optional filters one side applies after the direction mapping.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SideFilter {
    /// Keep only events whose direction agrees with the side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction_only: Option<bool>,
    /// Keep only events whose direction opposes the side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opposite_direction: Option<bool>,
    /// Require this EMA stack code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ema_stack: Option<i32>,
    /// Require the ATR ratio bucket, `ge1` or `lt1`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub atr_bucket: Option<String>,
    /// Require this RSI zone code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rsi_zone: Option<i32>,
    /// Require at least this relative volume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rvol_min: Option<f64>,
    /// The wick column the minimum applies to.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wick_column: Option<String>,
    /// The wick minimum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wick_min: Option<f64>,
    /// The close-position minimum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_min: Option<f64>,
    /// The close-position maximum.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_max: Option<f64>,
    /// Require the confirmation bar to close back inside the level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_back: Option<bool>,
}

/// The check configuration one run pre-registers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckConfig {
    /// The run identity.
    pub run_id: String,
    /// The event code the rule reads.
    pub event_code: u32,
    /// The first event-bar timestamp in the window.
    pub from: i64,
    /// The last event-bar timestamp in the window.
    pub to: i64,
    /// The event-bar window as recorded, for the record.
    pub from_text: String,
    /// The event-bar window end as recorded, for the record.
    pub to_text: String,
    /// Bars held after entry.
    pub horizon_bars: usize,
    /// Bars between the event and the entry.
    pub confirmation_bars: usize,
    /// The round-trip cost proxy, in basis points.
    pub cost_bps: f64,
    /// Where the control set comes from.
    pub control: Control,
    /// The filters per side, by side name.
    pub sides: BTreeMap<String, SideFilter>,
}

/// One forward-path row, either from an event or from a bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Row {
    /// The event (or control) bar.
    pub event_bar: Bar,
    /// The event direction, absent for a bar control.
    pub direction: Option<i32>,
    /// The event reference price.
    pub reference: Option<f64>,
    /// The confirmation bar, when the rule reads one.
    pub confirmation_bar: Option<Bar>,
    /// The entry index.
    pub entry_index: usize,
    /// The entry time.
    pub entry_time: i64,
    /// The exit index.
    pub exit_index: usize,
    /// The entry open.
    pub entry_open: f64,
    /// The exit close.
    pub exit_close: f64,
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

/// The statistics of one set.
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub positive_half_years: Option<usize>,
}

/// The verdict of one side.
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

/// The result of one side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SideResult {
    /// The side.
    pub side: String,
    /// The filter the side applied.
    pub filter: SideFilter,
    /// The filtered set.
    pub filtered: Stats,
    /// The control set.
    pub control: Stats,
    /// Whether a set was too small.
    pub underpowered: bool,
    /// The screen verdict.
    pub verdict: Verdict,
}

/// The pre-registered thresholds in the record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Thresholds {
    /// The fewest events a set needs.
    pub min_events: usize,
    /// The fewest positive half-years a survivor needs.
    pub min_positive_half_years: usize,
}

/// The complete `check.json` record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckRecord {
    /// The record schema.
    pub schema: String,
    /// The run identity.
    pub run_id: String,
    /// The event name.
    pub event: String,
    /// The event code.
    pub event_code: u32,
    /// Bars held after entry.
    pub horizon_bars: usize,
    /// Bars between the event and the entry.
    pub confirmation_bars: usize,
    /// Where the control set came from.
    pub control: String,
    /// The round-trip cost proxy, in basis points.
    pub cost_bps: f64,
    /// The window start as recorded.
    #[serde(rename = "from")]
    pub from_text: String,
    /// The window end as recorded.
    #[serde(rename = "to")]
    pub to_text: String,
    /// The recorded data version.
    pub data_version: String,
    /// The model identity.
    pub model: String,
    /// The registry identity.
    pub registry_version: String,
    /// The thresholds.
    pub thresholds: Thresholds,
    /// Events that produced a row.
    pub candidate_count: usize,
    /// The per-side results, ordered by side name.
    pub results: Vec<SideResult>,
    /// The overall verdict.
    pub verdict: Verdict,
    /// The claim boundary.
    pub claim: String,
}

/// Why a check cannot be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// The configuration declares no side.
    NoSides,
    /// A value is not finite.
    NotFinite,
    /// A bar index is outside the bundle.
    BarOutside(usize),
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSides => formatter.write_str("the check config must declare at least one side"),
            Self::NotFinite => formatter.write_str("a result is not a finite number"),
            Self::BarOutside(index) => write!(formatter, "bar index {index} is outside the bundle"),
        }
    }
}

impl std::error::Error for CheckError {}

/// Build one forward-path row per event in the window.
#[must_use]
pub fn event_rows(
    bundle: &Bundle,
    event_code: u32,
    from: i64,
    to: i64,
    horizon_bars: usize,
    confirmation_bars: usize,
) -> Vec<Row> {
    let mut rows = Vec::new();
    for event in &bundle.events {
        if event.code != event_code {
            continue;
        }
        let Some(index) = bundle.index_of(event.available_bar) else {
            continue;
        };
        let event_bar = bundle.bars[index];
        if !(from..=to).contains(&event_bar.timestamp) {
            continue;
        }
        let entry_index = index + 1 + confirmation_bars;
        let exit_index = entry_index.saturating_add(horizon_bars);
        if entry_index >= bundle.bars.len() || exit_index >= bundle.bars.len() {
            continue;
        }
        rows.push(Row {
            event_bar,
            direction: Some(event.direction),
            reference: event.reference,
            confirmation_bar: if confirmation_bars > 0 {
                Some(bundle.bars[index + 1])
            } else {
                None
            },
            entry_index,
            entry_time: bundle.bars[entry_index].timestamp,
            exit_index,
            entry_open: bundle.bars[entry_index].open,
            exit_close: bundle.bars[exit_index].close,
        });
    }
    rows.sort_by_key(|row| (row.entry_time, row.entry_index));
    rows
}

/// Build one forward-path row per bar for an unconditional control.
#[must_use]
pub fn bar_rows(bundle: &Bundle, from: i64, to: i64, horizon_bars: usize) -> Vec<Row> {
    let mut rows = Vec::new();
    for (index, bar) in bundle.bars.iter().enumerate() {
        if !(from..=to).contains(&bar.timestamp) {
            continue;
        }
        let entry_index = index + 1;
        let exit_index = entry_index.saturating_add(horizon_bars);
        if entry_index >= bundle.bars.len() || exit_index >= bundle.bars.len() {
            continue;
        }
        rows.push(Row {
            event_bar: *bar,
            direction: Some(0),
            reference: None,
            confirmation_bar: None,
            entry_index,
            entry_time: bundle.bars[entry_index].timestamp,
            exit_index,
            entry_open: bundle.bars[entry_index].open,
            exit_close: bundle.bars[exit_index].close,
        });
    }
    rows
}

/// The net result of one row on one side.
#[must_use]
pub fn net_return(row: &Row, side: &str, cost_bps: f64) -> f64 {
    let gross = if side == "long" {
        (row.exit_close - row.entry_open) / row.entry_open
    } else {
        (row.entry_open - row.exit_close) / row.entry_open
    };
    gross - cost_bps / 10_000.0
}

/// Keep the first row that does not overlap the previous exit.
#[must_use]
pub fn non_overlapping(rows: &[Row], _horizon_bars: usize) -> Vec<Row> {
    let mut selected = Vec::new();
    let mut last_exit: Option<usize> = None;
    for row in rows {
        if last_exit.is_some_and(|last| row.entry_index <= last) {
            continue;
        }
        selected.push(row.clone());
        last_exit = Some(row.exit_index);
    }
    selected
}

/// The direction mapping of one row on one side.
#[must_use]
pub fn passes_direction(row: &Row, side: &str, filter: &SideFilter) -> bool {
    let Some(direction) = row.direction else {
        return true;
    };
    if filter.direction_only == Some(true) {
        if direction == 0 {
            return false;
        }
        return (side == "long" && direction > 0) || (side == "short" && direction < 0);
    }
    if filter.opposite_direction == Some(true) {
        if direction == 0 {
            return false;
        }
        return (side == "long" && direction < 0) || (side == "short" && direction > 0);
    }
    true
}

/// The event filters of one row on one side.
#[must_use]
pub fn passes_filter(row: &Row, side: &str, filter: &SideFilter) -> bool {
    if !passes_direction(row, side, filter) {
        return false;
    }
    if filter
        .ema_stack
        .is_some_and(|stack| row.event_bar.ema_stack != Some(stack))
    {
        return false;
    }
    if let Some(bucket) = &filter.atr_bucket {
        let Some(ratio) = row.event_bar.atr_ratio else {
            return false;
        };
        if (*bucket == "ge1") != (ratio >= 1.0) {
            return false;
        }
    }
    if filter
        .rsi_zone
        .is_some_and(|zone| row.event_bar.rsi_zone != Some(zone))
    {
        return false;
    }
    if let Some(minimum) = filter.rvol_min {
        let Some(rvol) = row.event_bar.rvol else {
            return false;
        };
        if rvol < minimum {
            return false;
        }
    }
    if let Some(minimum) = filter.wick_min {
        let column = filter.wick_column.as_deref().unwrap_or("");
        let Some(value) = row.event_bar.value(column) else {
            return false;
        };
        if value < minimum {
            return false;
        }
    }
    let close_position = row.event_bar.value("close_position");
    if filter
        .close_min
        .is_some_and(|minimum| close_position.is_none_or(|value| value < minimum))
    {
        return false;
    }
    if filter
        .close_max
        .is_some_and(|maximum| close_position.is_none_or(|value| value > maximum))
    {
        return false;
    }
    if filter.close_back == Some(true) {
        let (Some(confirmation), Some(reference), Some(direction)) =
            (row.confirmation_bar.as_ref(), row.reference, row.direction)
        else {
            return false;
        };
        if direction == 0 {
            return false;
        }
        if direction > 0
            && confirmation
                .close
                .partial_cmp(&reference)
                .is_none_or(|order| order != std::cmp::Ordering::Less)
        {
            return false;
        }
        if direction < 0
            && confirmation
                .close
                .partial_cmp(&reference)
                .is_none_or(|order| order != std::cmp::Ordering::Greater)
        {
            return false;
        }
    }
    true
}

/// The statistics of one set of rows.
#[must_use]
pub fn stats(rows: &[Row], side: &str, cost_bps: f64) -> Stats {
    if rows.is_empty() {
        return Stats {
            n: 0,
            mean_net: None,
            hit_rate: None,
            half_years: Vec::new(),
            positive_half_years: None,
        };
    }
    let mut returns = Vec::with_capacity(rows.len());
    let mut buckets: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for row in rows {
        let value = net_return(row, side, cost_bps);
        returns.push(value);
        buckets
            .entry(half_year(row.entry_time))
            .or_default()
            .push(value);
    }
    let mut half_years = Vec::with_capacity(buckets.len());
    let mut positive = 0;
    for (bucket, values) in buckets {
        let bucket_mean = mean(&values).unwrap_or(0.0);
        if values.len() >= MIN_HALF_YEAR_EVENTS && bucket_mean > 0.0 {
            positive += 1;
        }
        half_years.push(HalfYear {
            bucket,
            n: values.len(),
            mean_net: bucket_mean,
        });
    }
    let hits = returns.iter().filter(|value| **value > 0.0).count();
    Stats {
        n: returns.len(),
        mean_net: mean(&returns),
        hit_rate: Some(hits as f64 / returns.len() as f64),
        half_years,
        positive_half_years: Some(positive),
    }
}

/// Check one side and compare the filtered set with the event or bar control.
///
/// # Errors
///
/// Returns [`CheckError`] when a net result is not finite.
pub fn check_side(
    rows: &[Row],
    side: &str,
    filter: &SideFilter,
    horizon_bars: usize,
    cost_bps: f64,
    control_rows: Option<&[Row]>,
) -> Result<SideResult, CheckError> {
    let candidates = non_overlapping(rows, horizon_bars);
    let direction_rows: Vec<Row> = candidates
        .iter()
        .filter(|row| passes_direction(row, side, filter))
        .cloned()
        .collect();
    let filtered_rows: Vec<Row> = direction_rows
        .iter()
        .filter(|row| passes_filter(row, side, filter))
        .cloned()
        .collect();
    let control = match control_rows {
        None => stats(&direction_rows, side, cost_bps),
        Some(control_rows) => stats(&non_overlapping(control_rows, horizon_bars), side, cost_bps),
    };
    let filtered = stats(&filtered_rows, side, cost_bps);
    for value in filtered
        .mean_net
        .into_iter()
        .chain(filtered.hit_rate)
        .chain(control.mean_net)
        .chain(control.hit_rate)
    {
        if !value.is_finite() {
            return Err(CheckError::NotFinite);
        }
    }
    let underpowered = filtered.n < MIN_EVENTS || control.n < MIN_EVENTS;
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
    Ok(SideResult {
        side: side.to_owned(),
        filter: filter.clone(),
        filtered,
        control,
        underpowered,
        verdict,
    })
}

/// The candidate rows and window identity of one check.
#[derive(Debug, Clone, Copy)]
pub struct Rows<'a> {
    /// The candidate rows, already non-overlapping or in bundle order.
    pub rows: &'a [Row],
    /// The bar-control rows, when the run uses one.
    pub control_rows: Option<&'a [Row]>,
    /// Events that produced a row before the non-overlap filter.
    pub candidate_count: usize,
    /// The recorded data version.
    pub data_version: &'a str,
}

/// Run the check over supplied rows and build the record.
///
/// The rows may already be non-overlapping; the non-overlap filter is
/// idempotent, so applying it again is safe. This is the entry point for a
/// recorded run whose row builder is the caller's.
///
/// # Errors
///
/// Returns [`CheckError`] when no side is declared or a result is not finite.
pub fn build_from_rows(
    config: &CheckConfig,
    rows: Rows<'_>,
    catalog: &Catalog,
) -> Result<CheckRecord, CheckError> {
    if config.sides.is_empty() {
        return Err(CheckError::NoSides);
    }
    let mut results = Vec::with_capacity(config.sides.len());
    for (side, filter) in &config.sides {
        results.push(check_side(
            rows.rows,
            side,
            filter,
            config.horizon_bars,
            config.cost_bps,
            rows.control_rows,
        )?);
    }
    let verdict = if results
        .iter()
        .any(|result| result.verdict == Verdict::Survives)
    {
        Verdict::Survives
    } else if results
        .iter()
        .all(|result| result.verdict == Verdict::Underpowered)
    {
        Verdict::Underpowered
    } else {
        Verdict::NoPattern
    };
    Ok(CheckRecord {
        schema: CHECK_SCHEMA.to_owned(),
        run_id: config.run_id.clone(),
        event: catalog
            .get(&config.event_code)
            .cloned()
            .unwrap_or_else(|| config.event_code.to_string()),
        event_code: config.event_code,
        horizon_bars: config.horizon_bars,
        confirmation_bars: config.confirmation_bars,
        control: config.control.as_str().to_owned(),
        cost_bps: config.cost_bps,
        from_text: config.from_text.clone(),
        to_text: config.to_text.clone(),
        data_version: rows.data_version.to_owned(),
        model: "M1.3".to_owned(),
        registry_version: "2026-11-10.2".to_owned(),
        thresholds: Thresholds {
            min_events: MIN_EVENTS,
            min_positive_half_years: MIN_POSITIVE_HALF_YEARS,
        },
        candidate_count: rows.candidate_count,
        results,
        verdict,
        claim: "screen only; not an edge; no validation or holdout access".to_owned(),
    })
}

/// Run the check over a recorded bundle and build the record.
///
/// The bundle is expected to hold only the check window, as the reference
/// loader filters it before the study.
///
/// # Errors
///
/// Returns [`CheckError`] when no side is declared or a result is not finite.
pub fn build(
    bundle: &Bundle,
    config: &CheckConfig,
    catalog: &Catalog,
) -> Result<CheckRecord, CheckError> {
    let rows = event_rows(
        bundle,
        config.event_code,
        config.from,
        config.to,
        config.horizon_bars,
        config.confirmation_bars,
    );
    let control_rows = match config.control {
        Control::Bars => Some(bar_rows(
            bundle,
            config.from,
            config.to,
            config.horizon_bars,
        )),
        Control::Event => None,
    };
    build_from_rows(
        config,
        Rows {
            rows: &rows,
            control_rows: control_rows.as_deref(),
            candidate_count: rows.len(),
            data_version: bundle.data_version(),
        },
        catalog,
    )
}

impl CheckRecord {
    /// Render `check.md`.
    #[must_use]
    pub fn render_markdown(&self) -> String {
        let mut lines = vec![
            format!("# Check — {}", self.run_id),
            String::new(),
            format!("- Event: `{}`", self.event),
            format!("- Window: {} to {}", self.from_text, self.to_text),
            format!(
                "- Horizon: {} bars; cost proxy: {} bps round trip",
                self.horizon_bars,
                python_number(self.cost_bps)
            ),
            "- Model: `M1.3`; registry `2026-11-10.2`".to_owned(),
            format!("- Verdict: **{}**", self.verdict.as_str()),
            "- Claim: screen only; not an edge; no validation or holdout access".to_owned(),
            String::new(),
            "| Side | Filter | Filtered n | Mean net | Hit rate | Control n | Control mean | Control hit | Positive half-years | Verdict |".to_owned(),
            "|---|---|---:|---:|---:|---:|---:|---:|---:|---|".to_owned(),
        ];
        for result in &self.results {
            lines.push(format!(
                "| {} | `{}` | {} | {} | {} | {} | {} | {} | {} | {} |",
                result.side,
                render_filter(&result.filter),
                result.filtered.n,
                format_net(result.filtered.mean_net),
                format_hit(result.filtered.hit_rate),
                result.control.n,
                format_net(result.control.mean_net),
                format_hit(result.control.hit_rate),
                result.filtered.positive_half_years.unwrap_or(0),
                result.verdict.as_str()
            ));
        }
        lines.extend([
            String::new(),
            "## Half-years (filtered)".to_owned(),
            String::new(),
            "| Side | Half-year | n | Mean net |".to_owned(),
            "|---|---|---:|---:|".to_owned(),
        ]);
        for result in &self.results {
            for item in &result.filtered.half_years {
                lines.push(format!(
                    "| {} | {} | {} | {} |",
                    result.side,
                    item.bucket,
                    item.n,
                    format_net(Some(item.mean_net))
                ));
            }
        }
        lines.extend([
            String::new(),
            "The check is a screen, not a backtest. A survivor goes to Strategy".to_owned(),
            "Research with costs, a frozen config, and a pre-registration.".to_owned(),
            String::new(),
        ]);
        lines.join("\n")
    }
}

fn render_filter(filter: &SideFilter) -> String {
    let mut parts = Vec::new();
    if let Some(value) = filter.direction_only {
        parts.push(format!("'direction_only': {}", python_bool(value)));
    }
    if let Some(value) = filter.opposite_direction {
        parts.push(format!("'opposite_direction': {}", python_bool(value)));
    }
    if let Some(value) = filter.ema_stack {
        parts.push(format!("'ema_stack': {value}"));
    }
    if let Some(value) = &filter.atr_bucket {
        parts.push(format!("'atr_bucket': '{value}'"));
    }
    if let Some(value) = filter.rsi_zone {
        parts.push(format!("'rsi_zone': {value}"));
    }
    if let Some(value) = filter.rvol_min {
        parts.push(format!("'rvol_min': {}", python_number(value)));
    }
    if let Some(value) = &filter.wick_column {
        parts.push(format!("'wick_column': '{value}'"));
    }
    if let Some(value) = filter.wick_min {
        parts.push(format!("'wick_min': {}", python_number(value)));
    }
    if let Some(value) = filter.close_min {
        parts.push(format!("'close_min': {}", python_number(value)));
    }
    if let Some(value) = filter.close_max {
        parts.push(format!("'close_max': {}", python_number(value)));
    }
    if let Some(value) = filter.close_back {
        parts.push(format!("'close_back': {}", python_bool(value)));
    }
    format!("{{{}}}", parts.join(", "))
}

fn python_bool(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

fn python_number(value: f64) -> String {
    let text = format!("{value}");
    if text.contains('.') || text.contains('e') {
        text
    } else {
        format!("{text}.0")
    }
}

fn format_net(value: Option<f64>) -> String {
    value.map_or_else(|| "—".to_owned(), |value| format!("{:+.4}%", value * 100.0))
}

fn format_hit(value: Option<f64>) -> String {
    value.map_or_else(|| "—".to_owned(), |value| format!("{:.1}%", value * 100.0))
}

#[cfg(test)]
mod tests;
