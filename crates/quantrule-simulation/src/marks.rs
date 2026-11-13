//! The mark schema.
//!
//! A mark is one judgment: a good or bad entry or exit, on one bar of one
//! window. The same schema serves the human and the oracle, so the two can be
//! compared row by row.

use serde::{Deserialize, Serialize};

use crate::window::Window;

/// The longest note a mark may carry.
pub const NOTE_LIMIT: usize = 500;

/// Whether a mark is an entry or an exit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkAction {
    /// An entry.
    Entry,
    /// An exit.
    Exit,
}

/// Whether the mark is a good or a bad example.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkQuality {
    /// A good example.
    Good,
    /// A bad example.
    Bad,
    /// Neither; kept for context.
    Neutral,
}

/// The direction of the mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Long.
    Long,
    /// Short.
    Short,
}

/// Who produced the mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkOrigin {
    /// A person.
    Human,
    /// The deterministic labeler.
    Oracle,
}

/// One mark on one window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mark {
    /// The window index the mark belongs to.
    pub chart: u32,
    /// The bar inside the window.
    pub bar: u64,
    /// Entry or exit.
    pub action: MarkAction,
    /// Good, bad, or neutral.
    pub quality: MarkQuality,
    /// Long or short.
    pub side: Side,
    /// Who made it.
    pub origin: MarkOrigin,
    /// Why, at most [`NOTE_LIMIT`] characters.
    pub note: String,
}

/// Why a set of marks is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkError {
    /// No marks were supplied.
    Empty,
    /// A mark names a window that does not exist.
    UnknownChart(u32),
    /// A mark falls outside its window.
    BarOutside(u32),
    /// A note is too long.
    NoteTooLong,
    /// The set has no good or no bad example.
    NoContrast,
}

impl std::fmt::Display for MarkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("a marking set needs at least one mark"),
            Self::UnknownChart(index) => write!(formatter, "window {index} does not exist"),
            Self::BarOutside(index) => write!(formatter, "a mark is outside window {index}"),
            Self::NoteTooLong => write!(formatter, "a note is longer than {NOTE_LIMIT} characters"),
            Self::NoContrast => {
                formatter.write_str("a marking set needs both a good and a bad example")
            }
        }
    }
}

impl std::error::Error for MarkError {}

impl Mark {
    /// Validate the whole set against the windows it refers to.
    ///
    /// A set with only good examples cannot teach the boundary, so both
    /// qualities must appear.
    ///
    /// # Errors
    ///
    /// Returns [`MarkError`] when the set is empty, a mark names an unknown
    /// window or falls outside it, a note is too long, or one quality is absent.
    pub fn validate(marks: &[Mark], windows: &[Window]) -> Result<(), MarkError> {
        if marks.is_empty() {
            return Err(MarkError::Empty);
        }
        let mut good = false;
        let mut bad = false;
        for mark in marks {
            let Some(window) = windows.iter().find(|window| window.index == mark.chart) else {
                return Err(MarkError::UnknownChart(mark.chart));
            };
            if !window.contains(mark.bar) {
                return Err(MarkError::BarOutside(mark.chart));
            }
            if mark.note.chars().count() > NOTE_LIMIT {
                return Err(MarkError::NoteTooLong);
            }
            match mark.quality {
                MarkQuality::Good => good = true,
                MarkQuality::Bad => bad = true,
                MarkQuality::Neutral => {}
            }
        }
        if good && bad {
            Ok(())
        } else {
            Err(MarkError::NoContrast)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> Window {
        Window {
            index: 0,
            start: 100,
            end: 200,
        }
    }

    fn mark(bar: u64, quality: MarkQuality) -> Mark {
        Mark {
            chart: 0,
            bar,
            action: MarkAction::Entry,
            quality,
            side: Side::Long,
            origin: MarkOrigin::Human,
            note: "higher low into the level".to_owned(),
        }
    }

    #[test]
    fn a_balanced_set_passes() {
        let marks = [mark(110, MarkQuality::Good), mark(150, MarkQuality::Bad)];
        assert_eq!(Mark::validate(&marks, &[window()]), Ok(()));
    }

    #[test]
    fn an_empty_set_is_refused() {
        assert_eq!(Mark::validate(&[], &[window()]), Err(MarkError::Empty));
    }

    #[test]
    fn an_unknown_window_is_refused() {
        let mut mark = mark(110, MarkQuality::Good);
        mark.chart = 4;
        assert_eq!(
            Mark::validate(&[mark], &[window()]),
            Err(MarkError::UnknownChart(4))
        );
    }

    #[test]
    fn a_bar_outside_the_window_is_refused() {
        let marks = [mark(90, MarkQuality::Good), mark(150, MarkQuality::Bad)];
        assert_eq!(
            Mark::validate(&marks, &[window()]),
            Err(MarkError::BarOutside(0))
        );
    }

    #[test]
    fn a_long_note_is_refused() {
        let mut mark = mark(110, MarkQuality::Good);
        mark.note = "x".repeat(NOTE_LIMIT + 1);
        let marks = [mark, super::tests::mark(150, MarkQuality::Bad)];
        assert_eq!(
            Mark::validate(&marks, &[window()]),
            Err(MarkError::NoteTooLong)
        );
    }

    #[test]
    fn a_set_without_bad_examples_is_refused() {
        let marks = [mark(110, MarkQuality::Good), mark(150, MarkQuality::Good)];
        assert_eq!(
            Mark::validate(&marks, &[window()]),
            Err(MarkError::NoContrast)
        );
    }

    #[test]
    fn a_mark_round_trips_through_json() {
        let mark = mark(110, MarkQuality::Good);
        let text = serde_json::to_string(&mark).expect("encode");
        let back: Mark = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, mark);
    }
}
