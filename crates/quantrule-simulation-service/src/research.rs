//! The research-plane use cases.
//!
//! These functions compose the faithful domain crate with the service records.
//! They add identity, the local bar coordinate the client marks on, the server
//! timestamp, the series projection, and the CSV mirror. They never compute a
//! model reading, never read a strategy, and never write a run artifact.

use std::collections::BTreeMap;
use std::sync::Arc;

use quantrule_api::{Digest, MarkAction, MarkOrigin, MarkQuality, Side, SimulationId};
use quantrule_runtime::adapters::ObjectStore;
use quantrule_simulation::check::{MIN_EVENTS, Thresholds};
use quantrule_simulation::{
    Bar, Chart, ChartBar, CheckError, CheckRecord, CheckVerdict, Mark,
    MarkQuality as DomainQuality, RawMark, Selection, Side as DomainSide, WindowRecord,
    build_check, label, validate_mark, write_marks,
};
use serde::Serialize;

use crate::bundle::{LoadedBundle, bar_time};
use crate::criteria::Criteria;
use crate::records::{MarkRecord, RunRecord, action_text, side_text};

/// Why a research use case cannot answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResearchError {
    /// The run has no selected windows yet.
    NoSelection,
    /// The chart number is outside the selection.
    UnknownChart(u64),
    /// The mark is outside the window or violates the schema.
    Mark(String),
    /// The oracle cannot label a window.
    Oracle(String),
    /// The check cannot run.
    Check(String),
    /// The check produced fewer events than the contract needs.
    CheckIncomplete(usize),
    /// A stored selection cannot be read.
    Selection(String),
}

impl std::fmt::Display for ResearchError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSelection => formatter.write_str("the run has no selected windows"),
            Self::UnknownChart(chart) => write!(formatter, "window {chart} is not selected"),
            Self::Mark(message) => formatter.write_str(message),
            Self::Oracle(message) => formatter.write_str(message),
            Self::Check(message) => formatter.write_str(message),
            Self::CheckIncomplete(events) => write!(
                formatter,
                "the check has {events} events; at least {MIN_EVENTS} are required"
            ),
            Self::Selection(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for ResearchError {}

/// One candle in a window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CandleView {
    /// The bar position inside the window.
    pub bar: u64,
    /// Open.
    pub open: f64,
    /// High.
    pub high: f64,
    /// Low.
    pub low: f64,
    /// Close.
    pub close: f64,
    /// Volume.
    pub volume: f64,
}

/// One published event, in the model's numeric record shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventView {
    /// The event code.
    pub event_code: u32,
    /// The event type code.
    pub event_type: u32,
    /// The event subtype code.
    pub event_subtype_code: u32,
    /// The source code.
    pub source_code: u32,
    /// The bar on which the event occurred, as a window position.
    pub occurred_bar_index: i64,
    /// The first bar on which the event became knowable, as a window position.
    pub available_bar_index: u64,
    /// The direction code.
    pub direction_code: i32,
}

/// One confirmed level.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LevelView {
    /// The reference code.
    pub reference_code: u64,
    /// The scale code.
    pub scale_code: i32,
    /// The kind code.
    pub kind_code: i32,
    /// The price.
    pub price: f64,
    /// The bar on which the level occurred, as a window position.
    pub occurred_bar_index: i64,
    /// The first bar on which it became knowable, as a window position.
    pub available_bar_index: u64,
}

/// The one indicator panel a window carries.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PanelView {
    /// The indicator code.
    pub indicator_code: u32,
    /// The values, one per candle; a missing reading is null.
    pub values: Vec<Option<f64>>,
}

/// The series for one chart window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SeriesView {
    /// The run identity.
    pub sim_id: SimulationId,
    /// The zero-based window number.
    pub chart: u64,
    /// The candles.
    pub candles: Vec<CandleView>,
    /// The fast EMA.
    pub ema_fast: Vec<Option<f64>>,
    /// The mid EMA.
    pub ema_mid: Vec<Option<f64>>,
    /// The slow EMA.
    pub ema_slow: Vec<Option<f64>>,
    /// The one indicator panel.
    pub panel: PanelView,
    /// The events knowable inside the window.
    pub events: Vec<EventView>,
    /// The levels knowable inside the window.
    pub levels: Vec<LevelView>,
}

/// One stored mark the client supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkDraft {
    /// The zero-based window number.
    pub chart: u64,
    /// The bar position inside the window.
    pub bar: u64,
    /// Entry or exit.
    pub action: MarkAction,
    /// Good, bad, or neutral.
    pub quality: MarkQuality,
    /// Long or short.
    pub side: Side,
    /// The reason.
    pub note: String,
}

/// One side result as the check publishes it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckSideView {
    /// The side.
    pub side: String,
    /// The filtered set.
    pub filtered: quantrule_simulation::Stats,
    /// The control set.
    pub control: quantrule_simulation::Stats,
    /// Whether a set was too small.
    pub underpowered: bool,
    /// The screen verdict.
    pub verdict: CheckVerdict,
}

/// The check table and its verdict.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CheckView {
    /// The inputs digest the check was computed over.
    pub inputs_digest: Digest,
    /// Events that produced a row.
    pub events: usize,
    /// The first side's mean net result.
    pub mean_net: f64,
    /// The first side's hit rate.
    pub hit_rate: f64,
    /// The first side's control count.
    pub control: usize,
    /// The first side's control mean net result.
    pub control_mean_net: f64,
    /// The first side's control hit rate.
    pub control_hit_rate: f64,
    /// Positive half-years with enough events.
    pub positive_half_years: usize,
    /// The screen verdict.
    pub passed: bool,
    /// The conditions that failed, in contract order.
    pub failures: Vec<String>,
    /// The overall verdict word.
    pub verdict: String,
    /// The thresholds the screen used.
    pub thresholds: Thresholds,
    /// Every side result, ordered by side name.
    pub sides: Vec<CheckSideView>,
}

/// Recompute the deterministic selection recorded by the run.
///
/// # Errors
///
/// Returns [`ResearchError::Selection`] when the recorded parameters cannot
/// produce a selection.
pub fn selection_for(run: &RunRecord, bundle: &LoadedBundle) -> Result<Selection, ResearchError> {
    quantrule_simulation::select(&bundle.bundle, &run.window_spec, &bundle.catalog)
        .map_err(|error| ResearchError::Selection(error.to_string()))
}

/// Read the recorded selection.
///
/// # Errors
///
/// Returns [`ResearchError::NoSelection`] before the windows are selected.
pub fn recorded_selection(run: &RunRecord) -> Result<&Selection, ResearchError> {
    run.selection.as_ref().ok_or(ResearchError::NoSelection)
}

/// Project one chart window as the client draws it.
///
/// # Errors
///
/// Returns [`ResearchError::NoSelection`] before selection,
/// [`ResearchError::UnknownChart`] for a window outside the selection, and
/// [`ResearchError::Selection`] when the recorded range is outside the bundle.
pub fn series(
    run: &RunRecord,
    bundle: &LoadedBundle,
    criteria: &Criteria,
    chart: u64,
) -> Result<SeriesView, ResearchError> {
    let selection = recorded_selection(run)?;
    let position = usize::try_from(chart).map_err(|_| ResearchError::UnknownChart(chart))?;
    let window = selection
        .windows
        .get(position)
        .ok_or(ResearchError::UnknownChart(chart))?;
    if window.start_index >= window.end_index || window.end_index > bundle.bundle.bars.len() {
        return Err(ResearchError::Selection(format!(
            "window {} is outside the bundle",
            window.window_id
        )));
    }
    let bars = &bundle.bundle.bars[window.start_index..window.end_index];
    let (panel_code, panel_name) = criteria
        .panel()
        .map_err(|error| ResearchError::Selection(error.to_string()))?;
    let candles = bars
        .iter()
        .enumerate()
        .map(|(index, bar)| CandleView {
            bar: index as u64,
            open: bar.open,
            high: bar.high,
            low: bar.low,
            close: bar.close,
            volume: bar.volume,
        })
        .collect();
    let panel = PanelView {
        indicator_code: panel_code,
        values: bars
            .iter()
            .map(|bar| panel_value(bar, panel_name))
            .collect(),
    };
    let mut events = Vec::new();
    for event in &bundle.bundle.events {
        if !run.window_spec.event_codes.contains(&event.code) {
            continue;
        }
        if let Some(offset) = offset_of(bundle, window, event.available_bar) {
            let occurred = offset_of(bundle, window, event.occurred_bar)
                .map_or(-1, |value| i64::try_from(value).unwrap_or(i64::MAX));
            events.push(EventView {
                event_code: event.code,
                event_type: event.type_code,
                event_subtype_code: event.subtype_code,
                source_code: event.source_code,
                occurred_bar_index: occurred,
                available_bar_index: offset as u64,
                direction_code: event.direction,
            });
        }
    }
    events.sort_by_key(|event| (event.available_bar_index, event.event_code));
    let mut levels = Vec::new();
    for level in &bundle.levels {
        if let Some(offset) = offset_of(bundle, window, level.available_bar) {
            let occurred = offset_of(bundle, window, level.occurred_bar)
                .map_or(-1, |value| i64::try_from(value).unwrap_or(i64::MAX));
            levels.push(LevelView {
                reference_code: level.reference_code,
                scale_code: level.scale_code,
                kind_code: level.kind_code,
                price: level.price,
                occurred_bar_index: occurred,
                available_bar_index: offset as u64,
            });
        }
    }
    levels.sort_by_key(|level| (level.available_bar_index, level.reference_code));
    Ok(SeriesView {
        sim_id: SimulationId(run.sim_id.clone()),
        chart,
        candles,
        ema_fast: bars.iter().map(|bar| bar.ema_fast).collect(),
        ema_mid: bars.iter().map(|bar| bar.ema_mid).collect(),
        ema_slow: bars.iter().map(|bar| bar.ema_slow).collect(),
        panel,
        events,
        levels,
    })
}

fn panel_value(bar: &Bar, name: &str) -> Option<f64> {
    match name {
        "rvol" => bar.rvol,
        "adx" => bar.adx,
        _ => bar.rsi,
    }
}

fn offset_of(bundle: &LoadedBundle, window: &WindowRecord, bar: u64) -> Option<usize> {
    let position = bundle.position_of(bar)?;
    if position < window.start_index || position >= window.end_index {
        return None;
    }
    Some(position - window.start_index)
}

/// Build one validated mark from a client draft.
///
/// # Errors
///
/// Returns [`ResearchError::NoSelection`] before selection,
/// [`ResearchError::UnknownChart`] for a chart outside the selection, and
/// [`ResearchError::Mark`] when the bar is outside the window, earlier than the
/// window's first knowable event, or the note is too long.
pub fn build_mark(
    run: &RunRecord,
    bundle: &LoadedBundle,
    draft: MarkDraft,
    origin: MarkOrigin,
) -> Result<MarkRecord, ResearchError> {
    let selection = recorded_selection(run)?;
    let position =
        usize::try_from(draft.chart).map_err(|_| ResearchError::UnknownChart(draft.chart))?;
    let window = selection
        .windows
        .get(position)
        .ok_or(ResearchError::UnknownChart(draft.chart))?;
    let length = window.end_index.saturating_sub(window.start_index);
    if length == 0 || window.end_index > bundle.bundle.bars.len() {
        return Err(ResearchError::Selection(format!(
            "window {} is outside the bundle",
            window.window_id
        )));
    }
    let local = usize::try_from(draft.bar)
        .map_err(|_| ResearchError::Mark(format!("bar {} is not in the window", draft.bar)))?;
    if local >= length {
        return Err(ResearchError::Mark(format!(
            "bar {} is outside window {}",
            draft.bar, window.window_id
        )));
    }
    let first_event = window
        .events
        .iter()
        .filter_map(|event| offset_of(bundle, window, event.available_bar))
        .min();
    if first_event.is_some_and(|minimum| local < minimum) {
        return Err(ResearchError::Mark(format!(
            "bar {} is earlier than the window's first knowable event",
            draft.bar
        )));
    }
    let note = draft.note.trim();
    if note.chars().count() > quantrule_simulation::marks::NOTE_LIMIT {
        return Err(ResearchError::Mark(format!(
            "the note is longer than {} characters",
            quantrule_simulation::marks::NOTE_LIMIT
        )));
    }
    let bar = &bundle.bundle.bars[window.start_index + local];
    let mark_id = mark_id(
        &run.sim_id,
        origin,
        draft.chart,
        draft.bar,
        draft.action,
        draft.side,
    );
    Ok(MarkRecord {
        mark_id,
        chart: draft.chart,
        bar: draft.bar,
        marked_time_utc: bar_time(bar.timestamp),
        action: draft.action,
        quality: draft.quality,
        side: draft.side,
        origin,
        note: note.to_owned(),
        created_at: String::new(),
    })
}

/// The deterministic mark identity.
#[must_use]
pub fn mark_id(
    sim_id: &str,
    origin: MarkOrigin,
    chart: u64,
    bar: u64,
    action: MarkAction,
    side: Side,
) -> String {
    let key = format!(
        "quantrule.simulation-mark/v1|{sim_id}|{}|{chart}|{bar}|{}|{}",
        crate::records::origin_text(origin),
        action_text(action),
        side_text(side)
    );
    format!("m_{}", Digest::sha256(key.as_bytes()).as_hex())
}

/// Run the deterministic oracle over every selected window.
///
/// The same bundle, selection, and criterion produce the same marks and ids.
///
/// # Errors
///
/// Returns [`ResearchError::NoSelection`] before selection and
/// [`ResearchError::Oracle`] when a window cannot be labelled.
pub fn oracle_marks(
    run: &RunRecord,
    bundle: &LoadedBundle,
) -> Result<Vec<MarkRecord>, ResearchError> {
    let selection = recorded_selection(run)?;
    let mut marks: BTreeMap<(u64, u64, &'static str, &'static str), MarkRecord> = BTreeMap::new();
    for (position, window) in selection.windows.iter().enumerate() {
        let chart = u64::try_from(position).unwrap_or(u64::MAX);
        let chart_name = chart_name(chart);
        let chart_bars: Vec<ChartBar> = bundle.bundle.bars[window.start_index..window.end_index]
            .iter()
            .map(|bar| ChartBar {
                bar: bar.bar,
                timestamp: bar.timestamp,
            })
            .collect();
        let manifest = Chart {
            file: chart_name.clone(),
            bars: chart_bars,
        };
        let by_name = BTreeMap::from([(chart_name.as_str(), &manifest)]);
        let labelled = label(
            &bundle.bundle,
            window,
            &chart_name,
            &run.oracle_spec,
            &run.sim_id,
        )
        .map_err(|error| ResearchError::Oracle(error.to_string()))?;
        for mark in labelled.marks {
            let Some(local) = local_of(window, bundle, mark.bar) else {
                // A hindsight rule exit beyond the chart is not representable
                // on this window; the chart cannot display it, so it is not
                // stored as a mark of this window.
                continue;
            };
            let raw = RawMark::from_mark(&mark);
            let validated = validate_mark(&raw, &by_name)
                .map_err(|error| ResearchError::Mark(error.to_string()))?;
            let record = MarkRecord {
                mark_id: mark_id(
                    &run.sim_id,
                    MarkOrigin::Oracle,
                    chart,
                    local,
                    draft_action(validated.action),
                    draft_side(validated.side),
                ),
                chart,
                bar: local,
                marked_time_utc: validated.marked_time_utc,
                action: draft_action(validated.action),
                quality: draft_quality(validated.quality),
                side: draft_side(validated.side),
                origin: MarkOrigin::Oracle,
                note: validated.note,
                created_at: String::new(),
            };
            marks.insert(
                (
                    chart,
                    local,
                    action_text(record.action),
                    side_text(record.side),
                ),
                record,
            );
        }
    }
    Ok(marks.into_values().collect())
}

/// Run the event-study check over the recorded bundle.
///
/// # Errors
///
/// Returns [`ResearchError::CheckIncomplete`] when fewer than
/// [`MIN_EVENTS`] events produced a row, and [`ResearchError::Check`] when the
/// study cannot run.
pub fn check(run: &RunRecord, bundle: &LoadedBundle) -> Result<CheckRecord, ResearchError> {
    let record = build_check(&bundle.bundle, &run.check_config, &bundle.catalog)
        .map_err(|error: CheckError| ResearchError::Check(error.to_string()))?;
    if record.candidate_count < MIN_EVENTS {
        return Err(ResearchError::CheckIncomplete(record.candidate_count));
    }
    Ok(record)
}

/// Project the check record as the screen reads it.
#[must_use]
pub fn check_view(record: &CheckRecord, inputs_digest: &Digest) -> CheckView {
    let first = record.results.first();
    let (mean_net, hit_rate, control, control_mean_net, control_hit_rate, positive) =
        first.map_or((0.0, 0.0, 0, 0.0, 0.0, 0), |side| {
            (
                side.filtered.mean_net.unwrap_or(0.0),
                side.filtered.hit_rate.unwrap_or(0.0),
                side.control.n,
                side.control.mean_net.unwrap_or(0.0),
                side.control.hit_rate.unwrap_or(0.0),
                side.filtered.positive_half_years.unwrap_or(0),
            )
        });
    let passed = record.verdict == CheckVerdict::Survives;
    let mut failures = Vec::new();
    if let Some(side) = first {
        if side.underpowered {
            failures.push("fewer than 30 non-overlapping events".to_owned());
        }
        if side.filtered.mean_net.is_none_or(|value| value <= 0.0) {
            failures.push("the filtered mean net is not positive".to_owned());
        }
        if side.filtered.hit_rate.is_none_or(|value| value <= 0.5) {
            failures.push("the filtered hit rate is not above 50%".to_owned());
        }
        if side
            .filtered
            .mean_net
            .zip(side.control.mean_net)
            .is_none_or(|(filtered, control)| filtered <= control)
        {
            failures.push("the filtered set does not beat the control mean".to_owned());
        }
        if side
            .filtered
            .hit_rate
            .zip(side.control.hit_rate)
            .is_none_or(|(filtered, control)| filtered <= control)
        {
            failures.push("the filtered set does not beat the control hit rate".to_owned());
        }
        if side
            .filtered
            .positive_half_years
            .is_none_or(|value| value < 3)
        {
            failures.push("the direction does not hold in three half-years".to_owned());
        }
    } else {
        failures.push("the check produced no side result".to_owned());
    }
    CheckView {
        inputs_digest: inputs_digest.clone(),
        events: record.candidate_count,
        mean_net,
        hit_rate,
        control,
        control_mean_net,
        control_hit_rate,
        positive_half_years: positive,
        passed,
        failures,
        verdict: record.verdict.as_str().to_owned(),
        thresholds: record.thresholds,
        sides: record
            .results
            .iter()
            .map(|side| CheckSideView {
                side: side.side.clone(),
                filtered: side.filtered.clone(),
                control: side.control.clone(),
                underpowered: side.underpowered,
                verdict: side.verdict,
            })
            .collect(),
    }
}

/// Write the two annotator-schema CSV mirrors for a run.
///
/// The database is authoritative. The mirror is derived: a write failure is
/// logged by the caller and repaired by the next mark write.
///
/// # Errors
///
/// Returns the object-store message.
pub async fn mirror_marks(
    objects: &Arc<dyn ObjectStore>,
    sim_id: &str,
    marks: &[MarkRecord],
) -> Result<(), String> {
    for (origin, suffix) in [
        (MarkOrigin::Human, "marks-human.csv"),
        (MarkOrigin::Oracle, "marks-oracle.csv"),
    ] {
        let rows: Vec<Mark> = marks
            .iter()
            .filter(|mark| mark.origin == origin)
            .map(mark_to_domain)
            .collect();
        let text = write_marks(&rows);
        let key = format!("simulation/runs/{sim_id}/{suffix}");
        objects
            .put(&key, text.into_bytes())
            .await
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn mark_to_domain(mark: &MarkRecord) -> Mark {
    Mark {
        chart: chart_name(mark.chart),
        bar: mark.bar,
        marked_time_utc: mark.marked_time_utc.clone(),
        action: match mark.action {
            MarkAction::Entry => quantrule_simulation::MarkAction::Entry,
            MarkAction::Exit => quantrule_simulation::MarkAction::Exit,
        },
        quality: match mark.quality {
            MarkQuality::Good => DomainQuality::Good,
            MarkQuality::Bad => DomainQuality::Bad,
            MarkQuality::Neutral => DomainQuality::Neutral,
        },
        side: match mark.side {
            Side::Long => DomainSide::Long,
            Side::Short => DomainSide::Short,
        },
        note: mark.note.clone(),
    }
}

fn draft_action(action: quantrule_simulation::MarkAction) -> MarkAction {
    match action {
        quantrule_simulation::MarkAction::Entry => MarkAction::Entry,
        quantrule_simulation::MarkAction::Exit => MarkAction::Exit,
    }
}

fn draft_quality(quality: quantrule_simulation::MarkQuality) -> MarkQuality {
    match quality {
        DomainQuality::Good => MarkQuality::Good,
        DomainQuality::Bad => MarkQuality::Bad,
        DomainQuality::Neutral => MarkQuality::Neutral,
    }
}

fn draft_side(side: DomainSide) -> Side {
    match side {
        DomainSide::Long => Side::Long,
        DomainSide::Short => Side::Short,
    }
}

fn chart_name(chart: u64) -> String {
    format!("w{:03}", chart + 1)
}

fn local_of(window: &WindowRecord, bundle: &LoadedBundle, bar: u64) -> Option<u64> {
    let position = bundle.position_of(bar)?;
    if position < window.start_index || position >= window.end_index {
        return None;
    }
    u64::try_from(position - window.start_index).ok()
}
