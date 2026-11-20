//! The simulation surface: chart-first, hindsight-labeled research.
//!
//! A simulation run is not a run. It has no `run_id`, no canonical digest, and
//! no control-plane message. It records an inputs digest, because the window
//! selection and the oracle are deterministic while the human marks are not.

use serde::{Deserialize, Serialize};

use crate::digest::Digest;

/// The simulation bundle schema token.
pub const BUNDLE_SCHEMA: &str = "quantrule.simulation-bundle/v1";

/// The content-addressed key of one recorded simulation bundle.
#[must_use]
pub fn bundle_key(digest: &Digest) -> String {
    format!("simulation/bundles/{}.json", digest.as_hex())
}

/// The identity of one simulation run.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SimulationId(pub String);

impl std::fmt::Display for SimulationId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Where a simulation run is in the research loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationStatus {
    /// Premise recorded; windows not selected.
    Draft,
    /// Windows selected; marking in progress.
    Marking,
    /// Oracle run and review written.
    Reviewed,
    /// The check has run on the unmarked window.
    Checked,
    /// A decision is recorded; marking is closed.
    Decided,
}

/// What a mark refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkAction {
    /// An entry.
    Entry,
    /// An exit.
    Exit,
}

/// How good the mark is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkQuality {
    /// A good example.
    Good,
    /// A bad example.
    Bad,
    /// Neither; kept for context.
    Neutral,
}

/// The direction of a mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Long.
    Long,
    /// Short.
    Short,
}

/// Who produced a mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkOrigin {
    /// A person clicking the chart.
    Human,
    /// The deterministic hindsight labeler.
    Oracle,
}

/// One mark on a chart window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    /// Server-assigned id.
    pub mark_id: String,
    /// The window this mark belongs to.
    pub chart: String,
    /// The bar index inside the window.
    pub bar: u64,
    /// Server-assigned time; the client clock is untrusted.
    pub marked_time_utc: String,
    /// Entry or exit.
    pub action: MarkAction,
    /// Good, bad, or neutral.
    pub quality: MarkQuality,
    /// Long or short.
    pub side: Side,
    /// Who made it.
    pub origin: MarkOrigin,
    /// The reason, at most 500 characters.
    pub note: String,
}

/// A simulation run in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimulationSummary {
    /// The id.
    pub sim_id: SimulationId,
    /// The premise, one sentence.
    pub premise: String,
    /// The research-loop stage.
    pub status: SimulationStatus,
    /// Windows selected.
    pub windows: u32,
    /// Human marks recorded.
    pub marks: u64,
    /// The recorded model bundle the run reads.
    pub bundle: Digest,
    /// Windows carrying at least one human mark.
    pub marked_windows: u64,
}

/// A request to create a simulation run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateSimulationRequest {
    /// The premise, one sentence.
    pub premise: String,
    /// The recorded model bundle to read.
    pub bundle: Digest,
    /// The context inputs the run reads; absent means unspecified. An empty
    /// value is omitted so the shell's older body keeps its wire shape.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub inputs: String,
    /// The seed for window selection.
    pub seed: u64,
    /// The mark criterion and check thresholds, recorded before the first mark.
    pub criteria: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_loop_stages_serialize_in_snake_case() {
        for (status, text) in [
            (SimulationStatus::Draft, "\"draft\""),
            (SimulationStatus::Marking, "\"marking\""),
            (SimulationStatus::Reviewed, "\"reviewed\""),
            (SimulationStatus::Checked, "\"checked\""),
            (SimulationStatus::Decided, "\"decided\""),
        ] {
            assert_eq!(serde_json::to_string(&status).expect("encode"), text);
        }
    }

    #[test]
    fn the_eight_mark_classes_are_reachable() {
        let mut seen = std::collections::BTreeSet::new();
        for action in [MarkAction::Entry, MarkAction::Exit] {
            for quality in [MarkQuality::Good, MarkQuality::Bad, MarkQuality::Neutral] {
                for side in [Side::Long, Side::Short] {
                    let key = format!("{action:?}/{quality:?}/{side:?}");
                    assert!(seen.insert(key));
                }
            }
        }
        assert_eq!(seen.len(), 12);
    }

    #[test]
    fn a_mark_round_trips() {
        let mark = Mark {
            mark_id: "m1".to_owned(),
            chart: "w1".to_owned(),
            bar: 120,
            marked_time_utc: "2026-11-12T10:00:00Z".to_owned(),
            action: MarkAction::Entry,
            quality: MarkQuality::Good,
            side: Side::Long,
            origin: MarkOrigin::Human,
            note: "higher low into the level".to_owned(),
        };
        let text = serde_json::to_string(&mark).expect("encode");
        let back: Mark = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, mark);
    }

    #[test]
    fn a_create_request_rejects_an_unknown_field() {
        let text = format!(
            "{{\"premise\":\"p\",\"bundle\":\"sha256:{}\",\"seed\":1,\"criteria\":\"c\",\"horizon\":5}}",
            "c".repeat(64)
        );
        assert!(serde_json::from_str::<CreateSimulationRequest>(&text).is_err());
    }

    #[test]
    fn the_browser_create_body_decodes_through_the_api_type() {
        let text = include_str!("../../../tests/fixtures/client-parity/simulation-create.json");
        let request: CreateSimulationRequest =
            serde_json::from_str(text).expect("the browser body decodes");
        assert_eq!(request.premise, "higher lows into a confirmed level");
        assert_eq!(request.inputs, "EMA stack and RSI zone");
        assert_eq!(request.seed, 7);
    }

    #[test]
    fn an_absent_inputs_field_is_unspecified() {
        let text = format!(
            "{{\"premise\":\"p\",\"bundle\":\"sha256:{}\",\"seed\":1,\"criteria\":\"c\"}}",
            "c".repeat(64)
        );
        let request: CreateSimulationRequest =
            serde_json::from_str(&text).expect("the shell body decodes");
        assert_eq!(request.inputs, "");
    }

    #[test]
    fn an_empty_inputs_field_is_omitted_from_the_wire_body() {
        let request = CreateSimulationRequest {
            premise: "p".to_owned(),
            bundle: Digest::sha256(b"bundle"),
            inputs: String::new(),
            seed: 1,
            criteria: "c".to_owned(),
        };
        let text = serde_json::to_string(&request).expect("encode");
        assert!(!text.contains("\"inputs\""));
    }

    #[test]
    fn a_bundle_key_is_the_content_address() {
        let digest = Digest::sha256(b"bundle");
        assert_eq!(
            bundle_key(&digest),
            format!("simulation/bundles/{}.json", digest.as_hex())
        );
    }

    #[test]
    fn a_summary_carries_the_bundle_and_marked_windows() {
        let summary = SimulationSummary {
            sim_id: SimulationId("sim_1".to_owned()),
            premise: "p".to_owned(),
            status: SimulationStatus::Marking,
            windows: 4,
            marks: 3,
            bundle: Digest::sha256(b"bundle"),
            marked_windows: 2,
        };
        let text = serde_json::to_string(&summary).expect("encode");
        let back: SimulationSummary = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, summary);
        assert!(text.contains("\"marked_windows\":2"));
    }

    #[test]
    fn a_summary_without_a_bundle_is_refused() {
        let text = "{\"sim_id\":\"sim\",\"premise\":\"p\",\"status\":\"draft\",\"windows\":0,\"marks\":0,\"marked_windows\":0}";
        assert!(serde_json::from_str::<SimulationSummary>(text).is_err());
    }
}
