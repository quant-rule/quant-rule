//! Recording a hypothesis.
//!
//! The kata's first requirement: define the hypothesis before testing it. The
//! type enforces that by consuming the draft: once sealed, the record has no
//! setters, and its fingerprint is computed over the exact content that will be
//! tested.

use serde::{Deserialize, Serialize};

use crate::costs::CostModel;
use crate::split::Split;

/// Why a hypothesis cannot be built or sealed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HypothesisError {
    /// The premise is blank.
    EmptyPremise,
    /// The rule reference is blank.
    EmptyRule,
    /// The dataset reference is blank.
    EmptyDataset,
    /// The pre-registered criteria are blank.
    EmptyCriteria,
    /// The seal time is blank.
    EmptySealTime,
    /// The record could not be serialized for fingerprinting.
    Serialize,
}

impl std::fmt::Display for HypothesisError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmptyPremise => "a hypothesis needs a premise",
            Self::EmptyRule => "a hypothesis needs a rule reference",
            Self::EmptyDataset => "a hypothesis needs a dataset reference",
            Self::EmptyCriteria => "a hypothesis needs pre-registered criteria",
            Self::EmptySealTime => "sealing a hypothesis needs a time",
            Self::Serialize => "the hypothesis could not be fingerprinted",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for HypothesisError {}

/// A hypothesis being written down.
///
/// It has no identity until it is sealed: the fingerprint is computed from the
/// final content, so two drafts that differ in one cost are two hypotheses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hypothesis {
    /// One sentence: what is expected and why.
    pub premise: String,
    /// The rule under test, as a configuration reference.
    pub rule: String,
    /// The dataset version the test will consume.
    pub dataset: String,
    /// The costs the result must survive.
    pub costs: CostModel,
    /// The development and validation periods.
    pub split: Split,
    /// The criteria recorded before the run, for example "expectancy > 0 after costs".
    pub criteria: String,
}

impl Hypothesis {
    /// Build a draft.
    ///
    /// # Errors
    ///
    /// Returns [`HypothesisError`] when a required field is blank.
    pub fn new(
        premise: impl Into<String>,
        rule: impl Into<String>,
        dataset: impl Into<String>,
        costs: CostModel,
        split: Split,
        criteria: impl Into<String>,
    ) -> Result<Self, HypothesisError> {
        let draft = Self {
            premise: premise.into(),
            rule: rule.into(),
            dataset: dataset.into(),
            costs,
            split,
            criteria: criteria.into(),
        };
        draft.validate()?;
        Ok(draft)
    }

    /// Seal the hypothesis for execution.
    ///
    /// Sealing consumes the draft, so the content cannot change afterwards. The
    /// caller supplies the time; this crate never reads a clock.
    ///
    /// # Errors
    ///
    /// Returns [`HypothesisError`] when a required field is blank or the record
    /// cannot be serialized.
    pub fn seal(self, sealed_at: impl Into<String>) -> Result<SealedHypothesis, HypothesisError> {
        let sealed_at = sealed_at.into();
        if sealed_at.trim().is_empty() {
            return Err(HypothesisError::EmptySealTime);
        }
        self.validate()?;
        let fingerprint = fingerprint(&self)?;
        Ok(SealedHypothesis {
            hypothesis: self,
            sealed_at,
            fingerprint,
        })
    }

    fn validate(&self) -> Result<(), HypothesisError> {
        if self.premise.trim().is_empty() {
            return Err(HypothesisError::EmptyPremise);
        }
        if self.rule.trim().is_empty() {
            return Err(HypothesisError::EmptyRule);
        }
        if self.dataset.trim().is_empty() {
            return Err(HypothesisError::EmptyDataset);
        }
        if self.criteria.trim().is_empty() {
            return Err(HypothesisError::EmptyCriteria);
        }
        Ok(())
    }
}

/// A hypothesis that may be tested and may not be edited.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SealedHypothesis {
    hypothesis: Hypothesis,
    sealed_at: String,
    fingerprint: String,
}

impl SealedHypothesis {
    /// The content.
    #[must_use]
    pub fn hypothesis(&self) -> &Hypothesis {
        &self.hypothesis
    }

    /// When the record was sealed, RFC 3339 UTC as supplied by the caller.
    #[must_use]
    pub fn sealed_at(&self) -> &str {
        &self.sealed_at
    }

    /// The identity of the exact content under test.
    #[must_use]
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Whether another sealed record carries the same content.
    #[must_use]
    pub fn matches(&self, other: &Self) -> bool {
        self.fingerprint == other.fingerprint
    }
}

/// FNV-1a over the canonical JSON of the draft, in lowercase hexadecimal.
fn fingerprint(draft: &Hypothesis) -> Result<String, HypothesisError> {
    let bytes = serde_json::to_vec(draft).map_err(|_| HypothesisError::Serialize)?;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Ok(format!("{hash:016x}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::split::Partition;

    fn split() -> Split {
        Split::new(
            Partition::new("dev", 100, 200).expect("dev"),
            Partition::new("val", 200, 300).expect("val"),
        )
        .expect("split")
    }

    fn draft() -> Hypothesis {
        Hypothesis::new(
            "a retest of a confirmed level holds",
            "strategy/retest.toml",
            "sha256:abc",
            CostModel::new(2.0, 1.0, 2.0).expect("costs"),
            split(),
            "expectancy above zero after costs",
        )
        .expect("draft")
    }

    #[test]
    fn a_draft_needs_every_field() {
        assert_eq!(
            Hypothesis::new("", "r", "d", CostModel::default(), split(), "c"),
            Err(HypothesisError::EmptyPremise)
        );
        assert_eq!(
            Hypothesis::new("p", "r", "d", CostModel::default(), split(), " "),
            Err(HypothesisError::EmptyCriteria)
        );
    }

    #[test]
    fn sealing_needs_a_time() {
        assert_eq!(draft().seal(""), Err(HypothesisError::EmptySealTime));
    }

    #[test]
    fn the_same_content_gives_the_same_fingerprint() {
        let first = draft().seal("2026-11-13T00:00:00Z").expect("first");
        let second = draft().seal("2026-11-13T00:00:00Z").expect("second");
        assert!(first.matches(&second));
    }

    #[test]
    fn a_changed_cost_is_a_different_hypothesis() {
        let first = draft().seal("2026-11-13T00:00:00Z").expect("first");
        let mut other = draft();
        other.costs = CostModel::new(5.0, 0.0, 0.0).expect("costs");
        let second = other.seal("2026-11-13T00:00:00Z").expect("second");
        assert!(!first.matches(&second));
    }

    #[test]
    fn the_sealed_record_round_trips_through_json() {
        let sealed = draft().seal("2026-11-13T00:00:00Z").expect("sealed");
        let text = serde_json::to_string(&sealed).expect("encode");
        let back: SealedHypothesis = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, sealed);
    }

    #[test]
    fn a_sealed_record_keeps_its_content() {
        let sealed = draft().seal("2026-11-13T00:00:00Z").expect("sealed");
        assert_eq!(sealed.hypothesis().rule, "strategy/retest.toml");
        assert_eq!(sealed.sealed_at(), "2026-11-13T00:00:00Z");
        assert_eq!(sealed.fingerprint().len(), 16);
    }
}
