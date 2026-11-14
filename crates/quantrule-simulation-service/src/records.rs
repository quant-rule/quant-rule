//! The simulation records the store and the routes exchange.
//!
//! These types are the service's own shape. The domain crate owns the window,
//! mark, oracle, and check rules; these records add the identity, the profile,
//! the server timestamps, and the research-loop status the schema keeps.

use quantrule_api::{Digest, MarkAction, MarkOrigin, MarkQuality, Side, SimulationStatus};
use quantrule_simulation::{CheckConfig, CheckRecord, OracleSpec, Selection, WindowSpec};
use serde::{Deserialize, Serialize};

/// A decision a researcher records; it closes marking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    /// Take the rule to Strategy Research.
    Test,
    /// Change one condition and mark again.
    Refine,
    /// Record that the window showed no edge.
    Stop,
}

impl DecisionKind {
    /// The schema word.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Test => "test",
            Self::Refine => "refine",
            Self::Stop => "stop",
        }
    }

    /// Parse the schema word.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "test" => Some(Self::Test),
            "refine" => Some(Self::Refine),
            "stop" => Some(Self::Stop),
            _ => None,
        }
    }
}

/// One recorded decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionRecord {
    /// Test, refine, or stop.
    pub decision: DecisionKind,
    /// The reason, at most 500 characters.
    pub note: String,
    /// Server-assigned time.
    pub decided_at_utc: String,
}

/// One mark as the store keeps it.
///
/// `bar` is the position inside its window, not the bundle identity. The
/// oracle's bundle identity is translated before the row is written, so a
/// client never sees two coordinate systems.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkRecord {
    /// Server-assigned, deterministic identity.
    pub mark_id: String,
    /// The zero-based window number.
    pub chart: u64,
    /// The bar position inside the window.
    pub bar: u64,
    /// The time derived from the bar, never the client clock.
    pub marked_time_utc: String,
    /// Entry or exit.
    pub action: MarkAction,
    /// Good, bad, or neutral.
    pub quality: MarkQuality,
    /// Long or short.
    pub side: Side,
    /// Human or oracle.
    pub origin: MarkOrigin,
    /// The reason.
    pub note: String,
    /// Server-assigned write time.
    pub created_at: String,
}

/// A new simulation run, before it is stored.
#[derive(Debug, Clone)]
pub struct NewSimulation {
    /// The server-assigned id.
    pub sim_id: String,
    /// The premise, one sentence.
    pub premise: String,
    /// The recorded bundle reference.
    pub bundle: Digest,
    /// The context inputs the run reads.
    pub inputs: String,
    /// The digest over the bundle, seed, criteria, and thresholds.
    pub inputs_digest: Digest,
    /// The window-selection seed.
    pub seed: u64,
    /// The criteria as the researcher wrote them.
    pub criteria: String,
    /// The recorded selection parameters.
    pub window_spec: WindowSpec,
    /// The recorded oracle criterion.
    pub oracle_spec: OracleSpec,
    /// The recorded check thresholds.
    pub check_config: CheckConfig,
    /// Server-assigned creation time.
    pub created_at: String,
}

/// One simulation run as the routes read it.
#[derive(Debug, Clone)]
pub struct RunRecord {
    /// The id.
    pub sim_id: String,
    /// The premise.
    pub premise: String,
    /// The bundle reference.
    pub bundle: Digest,
    /// The context inputs.
    pub inputs: String,
    /// The inputs digest.
    pub inputs_digest: Digest,
    /// The seed.
    pub seed: u64,
    /// The criteria text.
    pub criteria: String,
    /// The selection parameters.
    pub window_spec: WindowSpec,
    /// The oracle criterion.
    pub oracle_spec: OracleSpec,
    /// The check thresholds.
    pub check_config: CheckConfig,
    /// The deterministic selection, once it has been made.
    pub selection: Option<Selection>,
    /// The research-loop stage.
    pub status: SimulationStatus,
    /// The stored check, once it has run.
    pub check_result: Option<CheckRecord>,
    /// The recorded decision, once it exists.
    pub decision: Option<DecisionRecord>,
    /// Server-assigned creation time.
    pub created_at: String,
    /// Human marks recorded.
    pub human_marks: u64,
    /// Oracle marks recorded.
    pub oracle_marks: u64,
    /// Windows carrying at least one human mark.
    pub marked_windows: u64,
}

impl RunRecord {
    /// The windows selected, or an empty slice before selection.
    #[must_use]
    pub fn windows(&self) -> &[quantrule_simulation::WindowRecord] {
        self.selection
            .as_ref()
            .map_or(&[], |selection| selection.windows.as_slice())
    }

    /// Whether the run has recorded a decision.
    #[must_use]
    pub fn is_decided(&self) -> bool {
        self.status == SimulationStatus::Decided
    }
}

/// The counts a list row and a detail payload carry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct SimulationCounts {
    /// Windows selected.
    pub windows: u32,
    /// Windows carrying at least one human mark.
    pub marked_windows: u64,
    /// Human marks.
    pub human_marks: u64,
    /// Oracle marks.
    pub oracle_marks: u64,
}

/// One row of the run list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimulationSummary {
    /// The id.
    pub sim_id: String,
    /// The premise.
    pub premise: String,
    /// The research-loop stage.
    pub status: SimulationStatus,
    /// Windows selected.
    pub windows: u32,
    /// Human marks.
    pub marks: u64,
    /// The bundle reference.
    pub bundle: Digest,
    /// Windows carrying at least one human mark.
    pub marked_windows: u64,
}

/// One simulation run in detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SimulationDetail {
    /// The id.
    pub sim_id: String,
    /// The premise.
    pub premise: String,
    /// The research-loop stage.
    pub status: SimulationStatus,
    /// The bundle reference.
    pub bundle: Digest,
    /// The context inputs.
    pub inputs: String,
    /// The inputs digest.
    pub inputs_digest: Digest,
    /// The seed.
    pub seed: u64,
    /// The criteria text.
    pub criteria: String,
    /// The counts.
    pub counts: SimulationCounts,
    /// The decision, when recorded.
    pub decision: Option<DecisionRecord>,
    /// Server-assigned creation time.
    pub created_at: String,
}

impl RunRecord {
    /// The list projection.
    #[must_use]
    pub fn summary(&self) -> SimulationSummary {
        SimulationSummary {
            sim_id: self.sim_id.clone(),
            premise: self.premise.clone(),
            status: self.status,
            windows: u32::try_from(self.windows().len()).unwrap_or(u32::MAX),
            marks: self.human_marks,
            bundle: self.bundle.clone(),
            marked_windows: self.marked_windows,
        }
    }

    /// The detail projection.
    #[must_use]
    pub fn detail(&self) -> SimulationDetail {
        SimulationDetail {
            sim_id: self.sim_id.clone(),
            premise: self.premise.clone(),
            status: self.status,
            bundle: self.bundle.clone(),
            inputs: self.inputs.clone(),
            inputs_digest: self.inputs_digest.clone(),
            seed: self.seed,
            criteria: self.criteria.clone(),
            counts: SimulationCounts {
                windows: u32::try_from(self.windows().len()).unwrap_or(u32::MAX),
                marked_windows: self.marked_windows,
                human_marks: self.human_marks,
                oracle_marks: self.oracle_marks,
            },
            decision: self.decision.clone(),
            created_at: self.created_at.clone(),
        }
    }
}

/// One selected window as the navigator reads it, a half-open bar range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct WindowView {
    /// The zero-based window number; also the chart number.
    pub index: u64,
    /// The inclusive first bar index.
    pub start: usize,
    /// The exclusive last bar index.
    pub end: usize,
}

/// One mark as the client reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MarkView {
    /// The id.
    pub mark_id: String,
    /// The zero-based window number.
    pub chart: u64,
    /// The bar position inside the window.
    pub bar: u64,
    /// The time derived from the bar.
    pub marked_time_utc: String,
    /// Entry or exit.
    pub action: MarkAction,
    /// Good, bad, or neutral.
    pub quality: MarkQuality,
    /// Long or short.
    pub side: Side,
    /// Human or oracle.
    pub origin: MarkOrigin,
    /// The reason.
    pub note: String,
}

impl From<MarkRecord> for MarkView {
    fn from(record: MarkRecord) -> Self {
        Self {
            mark_id: record.mark_id,
            chart: record.chart,
            bar: record.bar,
            marked_time_utc: record.marked_time_utc,
            action: record.action,
            quality: record.quality,
            side: record.side,
            origin: record.origin,
            note: record.note,
        }
    }
}

/// The acknowledgement a stored mark returns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MarkAck {
    /// The server-assigned id.
    pub mark_id: String,
}

/// Read the status word the schema stores.
#[must_use]
pub fn status_text(status: SimulationStatus) -> &'static str {
    match status {
        SimulationStatus::Draft => "draft",
        SimulationStatus::Marking => "marking",
        SimulationStatus::Reviewed => "reviewed",
        SimulationStatus::Checked => "checked",
        SimulationStatus::Decided => "decided",
    }
}

/// Parse a stored status word.
#[must_use]
pub fn parse_status(text: &str) -> Option<SimulationStatus> {
    match text {
        "draft" => Some(SimulationStatus::Draft),
        "marking" => Some(SimulationStatus::Marking),
        "reviewed" => Some(SimulationStatus::Reviewed),
        "checked" => Some(SimulationStatus::Checked),
        "decided" => Some(SimulationStatus::Decided),
        _ => None,
    }
}

/// The origin word the schema stores.
#[must_use]
pub fn origin_text(origin: MarkOrigin) -> &'static str {
    match origin {
        MarkOrigin::Human => "human",
        MarkOrigin::Oracle => "oracle",
    }
}

/// Parse a stored origin word.
#[must_use]
pub fn parse_origin(text: &str) -> Option<MarkOrigin> {
    match text {
        "human" => Some(MarkOrigin::Human),
        "oracle" => Some(MarkOrigin::Oracle),
        _ => None,
    }
}

/// The action word the schema stores.
#[must_use]
pub fn action_text(action: MarkAction) -> &'static str {
    match action {
        MarkAction::Entry => "entry",
        MarkAction::Exit => "exit",
    }
}

/// Parse a stored action word.
#[must_use]
pub fn parse_action(text: &str) -> Option<MarkAction> {
    match text {
        "entry" => Some(MarkAction::Entry),
        "exit" => Some(MarkAction::Exit),
        _ => None,
    }
}

/// The quality word the schema stores.
#[must_use]
pub fn quality_text(quality: MarkQuality) -> &'static str {
    match quality {
        MarkQuality::Good => "good",
        MarkQuality::Bad => "bad",
        MarkQuality::Neutral => "neutral",
    }
}

/// Parse a stored quality word.
#[must_use]
pub fn parse_quality(text: &str) -> Option<MarkQuality> {
    match text {
        "good" => Some(MarkQuality::Good),
        "bad" => Some(MarkQuality::Bad),
        "neutral" => Some(MarkQuality::Neutral),
        _ => None,
    }
}

/// The side word the schema stores.
#[must_use]
pub fn side_text(side: Side) -> &'static str {
    match side {
        Side::Long => "long",
        Side::Short => "short",
    }
}

/// Parse a stored side word.
#[must_use]
pub fn parse_side(text: &str) -> Option<Side> {
    match text {
        "long" => Some(Side::Long),
        "short" => Some(Side::Short),
        _ => None,
    }
}
