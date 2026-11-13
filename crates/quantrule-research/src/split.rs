//! Out-of-sample validation.
//!
//! A hypothesis is found on one period and must be tested on a period it never
//! saw. The rule is structural, not procedural: the two periods cannot overlap,
//! and a validation period can be claimed once per hypothesis.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Why a split or a claim was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitError {
    /// A partition starts at or after its end.
    EmptyRange,
    /// A partition name is blank.
    EmptyName,
    /// The two partitions share a second.
    Overlap,
    /// The partition was already used for validation.
    AlreadyUsed,
}

impl std::fmt::Display for SplitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::EmptyRange => "a partition must end after it starts",
            Self::EmptyName => "a partition needs a name",
            Self::Overlap => "the validation period overlaps the development period",
            Self::AlreadyUsed => "this validation period has already been used",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for SplitError {}

/// A half-open time range, `[start, end)` in Unix seconds UTC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Partition {
    /// A stable name, for example `strategy_dev`.
    pub name: String,
    /// Inclusive start.
    pub start: u64,
    /// Exclusive end.
    pub end: u64,
}

impl Partition {
    /// Build a partition and validate it.
    ///
    /// # Errors
    ///
    /// Returns [`SplitError::EmptyName`] for a blank name and
    /// [`SplitError::EmptyRange`] when `end <= start`.
    pub fn new(name: impl Into<String>, start: u64, end: u64) -> Result<Self, SplitError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(SplitError::EmptyName);
        }
        if end <= start {
            return Err(SplitError::EmptyRange);
        }
        Ok(Self { name, start, end })
    }

    /// Whether a timestamp falls inside the partition.
    #[must_use]
    pub fn contains(&self, timestamp: u64) -> bool {
        (self.start..self.end).contains(&timestamp)
    }

    /// Whether two partitions share any second.
    #[must_use]
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Where a timestamp fell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sample {
    /// The period the hypothesis was developed on.
    Development,
    /// The period held back for validation.
    Validation,
    /// Neither.
    Outside,
}

/// A development period and its held-back validation period.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Split {
    /// The period the hypothesis may be developed on.
    pub development: Partition,
    /// The period that must remain unseen until the hypothesis is frozen.
    pub validation: Partition,
}

impl Split {
    /// Build a split and validate it.
    ///
    /// # Errors
    ///
    /// Returns [`SplitError::Overlap`] when the periods share a second.
    pub fn new(development: Partition, validation: Partition) -> Result<Self, SplitError> {
        if development.overlaps(&validation) {
            return Err(SplitError::Overlap);
        }
        Ok(Self {
            development,
            validation,
        })
    }

    /// Which sample a timestamp belongs to.
    #[must_use]
    pub fn classify(&self, timestamp: u64) -> Sample {
        if self.development.contains(timestamp) {
            Sample::Development
        } else if self.validation.contains(timestamp) {
            Sample::Validation
        } else {
            Sample::Outside
        }
    }
}

/// The validation periods already spent.
///
/// The ledger is persisted by the caller; the crate only enforces the rule. A
/// period is claimed by name and cannot be claimed twice, which is what makes
/// validation one-shot.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValidationLedger {
    used: BTreeSet<String>,
}

impl ValidationLedger {
    /// An empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Claim a validation period.
    ///
    /// # Errors
    ///
    /// Returns [`SplitError::AlreadyUsed`] when the name was already claimed.
    pub fn claim(&mut self, name: &str) -> Result<(), SplitError> {
        if self.used.contains(name) {
            return Err(SplitError::AlreadyUsed);
        }
        self.used.insert(name.to_owned());
        Ok(())
    }

    /// Whether a period was claimed.
    #[must_use]
    pub fn is_used(&self, name: &str) -> bool {
        self.used.contains(name)
    }

    /// How many periods were claimed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.used.len()
    }

    /// Whether nothing was claimed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.used.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn partition(name: &str, start: u64, end: u64) -> Partition {
        Partition::new(name, start, end).expect("partition")
    }

    #[test]
    fn a_partition_is_half_open() {
        let period = partition("dev", 100, 200);
        assert!(period.contains(100));
        assert!(!period.contains(200));
        assert!(!period.contains(99));
    }

    #[test]
    fn adjacent_periods_do_not_overlap() {
        let first = partition("a", 100, 200);
        let second = partition("b", 200, 300);
        assert!(!first.overlaps(&second));
        assert!(!second.overlaps(&first));
    }

    #[test]
    fn a_shared_second_is_an_overlap() {
        let first = partition("a", 100, 201);
        let second = partition("b", 200, 300);
        assert!(first.overlaps(&second));
    }

    #[test]
    fn a_split_refuses_an_overlap() {
        let development = partition("dev", 100, 200);
        let validation = partition("val", 150, 250);
        assert_eq!(
            Split::new(development, validation),
            Err(SplitError::Overlap)
        );
    }

    #[test]
    fn a_timestamp_is_classified() {
        let split =
            Split::new(partition("dev", 100, 200), partition("val", 200, 300)).expect("split");
        assert_eq!(split.classify(150), Sample::Development);
        assert_eq!(split.classify(250), Sample::Validation);
        assert_eq!(split.classify(50), Sample::Outside);
    }

    #[test]
    fn a_validation_period_is_claimed_once() {
        let mut ledger = ValidationLedger::new();
        assert!(ledger.is_empty());
        ledger.claim("strategy_validation").expect("first");
        assert_eq!(
            ledger.claim("strategy_validation"),
            Err(SplitError::AlreadyUsed)
        );
        assert!(ledger.is_used("strategy_validation"));
        assert_eq!(ledger.len(), 1);
    }

    #[test]
    fn an_empty_range_or_name_is_refused() {
        assert_eq!(Partition::new("x", 100, 100), Err(SplitError::EmptyRange));
        assert_eq!(Partition::new(" ", 100, 200), Err(SplitError::EmptyName));
    }

    #[test]
    fn the_ledger_round_trips_through_json() {
        let mut ledger = ValidationLedger::new();
        ledger.claim("a").expect("claim");
        let text = serde_json::to_string(&ledger).expect("encode");
        let back: ValidationLedger = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, ledger);
    }

    #[test]
    fn a_split_round_trips_through_json() {
        let split =
            Split::new(partition("dev", 100, 200), partition("val", 200, 300)).expect("split");
        let text = serde_json::to_string(&split).expect("encode");
        let back: Split = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, split);
    }
}
