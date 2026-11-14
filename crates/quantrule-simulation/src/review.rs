//! Reviewing the two marking sets.
//!
//! The human and the oracle mark the same window with the same schema. The
//! review counts what each found, pairs the marks that refer to the same bar and
//! side, and lists where the two disagreed about quality.
//!
//! A disagreement is a finding, not a defect to hide: the rule under test comes
//! from what the marks share, not from what either side alone prefers.

// port-parity: PORT-001

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::marks::{Mark, MarkAction, MarkQuality, Side};

/// Marks per quality and side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Counts {
    /// Good examples.
    pub good: usize,
    /// Bad examples.
    pub bad: usize,
    /// Neutral examples.
    pub neutral: usize,
    /// Long marks.
    pub long: usize,
    /// Short marks.
    pub short: usize,
    /// Every mark.
    pub total: usize,
}

impl Counts {
    fn add(&mut self, mark: &Mark) {
        self.total += 1;
        match mark.quality {
            MarkQuality::Good => self.good += 1,
            MarkQuality::Bad => self.bad += 1,
            MarkQuality::Neutral => self.neutral += 1,
        }
        match mark.side {
            Side::Long => self.long += 1,
            Side::Short => self.short += 1,
        }
    }
}

/// How the two sets line up.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agreement {
    /// Same bar, same side, same quality.
    pub matched: usize,
    /// Marked by the human only.
    pub human_only: usize,
    /// Marked by the oracle only.
    pub oracle_only: usize,
}

/// Two marks on the same bar and side that disagree about quality.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disagreement {
    /// The window.
    pub chart: u32,
    /// The bar.
    pub bar: u64,
    /// The side.
    pub side: Side,
    /// Entry or exit.
    pub action: MarkAction,
    /// What the human said.
    pub human: MarkQuality,
    /// What the oracle said.
    pub oracle: MarkQuality,
}

/// The review of one window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Review {
    /// Human marks.
    pub human: Counts,
    /// Oracle marks.
    pub oracle: Counts,
    /// How they line up.
    pub agreement: Agreement,
    /// Where they disagree, ordered by window, bar, side, action.
    pub disagreements: Vec<Disagreement>,
}

type Key = (u32, u64, u8, u8);

fn key(mark: &Mark) -> Key {
    let side = match mark.side {
        Side::Long => 0,
        Side::Short => 1,
    };
    let action = match mark.action {
        MarkAction::Entry => 0,
        MarkAction::Exit => 1,
    };
    (mark.chart, mark.bar, side, action)
}

fn side_of(code: u8) -> Side {
    if code == 0 { Side::Long } else { Side::Short }
}

fn action_of(code: u8) -> MarkAction {
    if code == 0 {
        MarkAction::Entry
    } else {
        MarkAction::Exit
    }
}

/// Compare the human marks with the oracle marks.
#[must_use]
pub fn review(human: &[Mark], oracle: &[Mark]) -> Review {
    let mut human_counts = Counts::default();
    let mut oracle_counts = Counts::default();
    let mut human_by_key: BTreeMap<Key, &Mark> = BTreeMap::new();
    let mut oracle_by_key: BTreeMap<Key, &Mark> = BTreeMap::new();

    for mark in human {
        human_counts.add(mark);
        human_by_key.insert(key(mark), mark);
    }
    for mark in oracle {
        oracle_counts.add(mark);
        oracle_by_key.insert(key(mark), mark);
    }

    let mut agreement = Agreement::default();
    let mut disagreements = Vec::new();

    for (key, human_mark) in &human_by_key {
        match oracle_by_key.get(key) {
            Some(oracle_mark) if human_mark.quality == oracle_mark.quality => {
                agreement.matched += 1;
            }
            Some(oracle_mark) => {
                agreement.matched += 1;
                disagreements.push(Disagreement {
                    chart: key.0,
                    bar: key.1,
                    side: side_of(key.2),
                    action: action_of(key.3),
                    human: human_mark.quality,
                    oracle: oracle_mark.quality,
                });
            }
            None => agreement.human_only += 1,
        }
    }

    for key in oracle_by_key.keys() {
        if !human_by_key.contains_key(key) {
            agreement.oracle_only += 1;
        }
    }

    Review {
        human: human_counts,
        oracle: oracle_counts,
        agreement,
        disagreements,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::marks::MarkOrigin;

    fn mark(bar: u64, quality: MarkQuality, side: Side, origin: MarkOrigin) -> Mark {
        Mark {
            chart: 0,
            bar,
            action: MarkAction::Entry,
            quality,
            side,
            origin,
            note: String::new(),
        }
    }

    #[test]
    fn identical_sets_match_completely() {
        let marks = vec![
            mark(10, MarkQuality::Good, Side::Long, MarkOrigin::Human),
            mark(20, MarkQuality::Bad, Side::Long, MarkOrigin::Human),
        ];
        let review = review(&marks, &marks);
        assert_eq!(review.agreement.matched, 2);
        assert_eq!(review.agreement.human_only, 0);
        assert_eq!(review.agreement.oracle_only, 0);
        assert!(review.disagreements.is_empty());
    }

    #[test]
    fn a_quality_disagreement_is_listed() {
        let human = vec![mark(10, MarkQuality::Good, Side::Long, MarkOrigin::Human)];
        let oracle = vec![mark(10, MarkQuality::Bad, Side::Long, MarkOrigin::Oracle)];
        let review = review(&human, &oracle);
        assert_eq!(review.agreement.matched, 1);
        assert_eq!(review.disagreements.len(), 1);
        assert_eq!(review.disagreements[0].human, MarkQuality::Good);
        assert_eq!(review.disagreements[0].oracle, MarkQuality::Bad);
    }

    #[test]
    fn marks_on_different_bars_are_unpaired() {
        let human = vec![mark(10, MarkQuality::Good, Side::Long, MarkOrigin::Human)];
        let oracle = vec![mark(11, MarkQuality::Good, Side::Long, MarkOrigin::Oracle)];
        let review = review(&human, &oracle);
        assert_eq!(review.agreement.matched, 0);
        assert_eq!(review.agreement.human_only, 1);
        assert_eq!(review.agreement.oracle_only, 1);
    }

    #[test]
    fn a_different_side_does_not_pair() {
        let human = vec![mark(10, MarkQuality::Good, Side::Long, MarkOrigin::Human)];
        let oracle = vec![mark(10, MarkQuality::Good, Side::Short, MarkOrigin::Oracle)];
        let review = review(&human, &oracle);
        assert_eq!(review.agreement.matched, 0);
    }

    #[test]
    fn the_counts_are_by_quality_and_side() {
        let marks = vec![
            mark(10, MarkQuality::Good, Side::Long, MarkOrigin::Human),
            mark(11, MarkQuality::Bad, Side::Short, MarkOrigin::Human),
            mark(12, MarkQuality::Neutral, Side::Long, MarkOrigin::Human),
        ];
        let review = review(&marks, &[]);
        assert_eq!(review.human.total, 3);
        assert_eq!(review.human.good, 1);
        assert_eq!(review.human.bad, 1);
        assert_eq!(review.human.neutral, 1);
        assert_eq!(review.human.long, 2);
        assert_eq!(review.human.short, 1);
    }

    #[test]
    fn empty_sets_have_zero_counts() {
        let review = review(&[], &[]);
        assert_eq!(review.human, Counts::default());
        assert_eq!(review.oracle, Counts::default());
        assert!(review.disagreements.is_empty());
    }

    #[test]
    fn the_review_is_deterministic() {
        let human = vec![mark(10, MarkQuality::Good, Side::Long, MarkOrigin::Human)];
        let oracle = vec![mark(10, MarkQuality::Bad, Side::Long, MarkOrigin::Oracle)];
        assert_eq!(review(&human, &oracle), review(&human, &oracle));
    }

    #[test]
    fn a_review_round_trips_through_json() {
        let human = vec![mark(10, MarkQuality::Good, Side::Long, MarkOrigin::Human)];
        let oracle = vec![mark(10, MarkQuality::Bad, Side::Long, MarkOrigin::Oracle)];
        let review = review(&human, &oracle);
        let text = serde_json::to_string(&review).expect("encode");
        let back: Review = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, review);
    }
}
