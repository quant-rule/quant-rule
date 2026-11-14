//! The deterministic hindsight labeler.
//!
//! The oracle looks at a window with the future visible and marks the best entry
//! and the most tempting bad entry, per side. It is the mechanical half of the
//! marking loop: a human marks the same window, and the two sets are compared.
//!
//! "Best" is the highest forward return for a long and the lowest for a short.
//! "Most tempting" is the highest caller-supplied temptation among the bars whose
//! forward return went the other way. Ties break on the lowest bar, so the result
//! never depends on iteration order.

// port-parity: PORT-001

use crate::marks::{Mark, MarkAction, MarkOrigin, MarkQuality, Side};
use crate::window::Window;

/// Why the oracle cannot label a window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleError {
    /// No bars were supplied.
    Empty,
    /// A bar falls outside the window.
    BarOutside(u64),
    /// A feature is not a finite number.
    NotFinite,
}

impl std::fmt::Display for OracleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => formatter.write_str("a window needs at least one bar"),
            Self::BarOutside(bar) => write!(formatter, "bar {bar} is outside the window"),
            Self::NotFinite => formatter.write_str("a feature is not a finite number"),
        }
    }
}

impl std::error::Error for OracleError {}

/// What the oracle sees for one bar.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Bar {
    /// The bar index, inside the window.
    pub index: u64,
    /// The result of entering here and exiting at the horizon, in R.
    pub forward_return: f64,
    /// How much the setup looked like a good entry at the time.
    pub temptation: f64,
}

/// Label one window for one side.
///
/// # Errors
///
/// Returns [`OracleError`] when no bars are supplied, a bar is outside the
/// window, or a feature is not finite.
pub fn label(bars: &[Bar], window: Window, side: Side) -> Result<Vec<Mark>, OracleError> {
    if bars.is_empty() {
        return Err(OracleError::Empty);
    }
    if bars.iter().any(|bar| !window.contains(bar.index)) {
        let outside = bars
            .iter()
            .find(|bar| !window.contains(bar.index))
            .map_or(0, |bar| bar.index);
        return Err(OracleError::BarOutside(outside));
    }
    if bars
        .iter()
        .any(|bar| !bar.forward_return.is_finite() || !bar.temptation.is_finite())
    {
        return Err(OracleError::NotFinite);
    }

    let better = |candidate: &Bar, current: &Bar| match side {
        Side::Long => {
            candidate.forward_return > current.forward_return
                || (candidate.forward_return == current.forward_return
                    && candidate.index < current.index)
        }
        Side::Short => {
            candidate.forward_return < current.forward_return
                || (candidate.forward_return == current.forward_return
                    && candidate.index < current.index)
        }
    };

    let went_the_other_way = |bar: &Bar| match side {
        Side::Long => bar.forward_return <= 0.0,
        Side::Short => bar.forward_return >= 0.0,
    };

    let favourite = bars
        .iter()
        .fold(None::<&Bar>, |best, bar| match best {
            Some(current) if !better(bar, current) => Some(current),
            _ => Some(bar),
        })
        .expect("the sample is not empty");

    let mut marks = vec![mark(favourite, window, side, MarkQuality::Good)];

    if let Some(trap) =
        bars.iter()
            .filter(|bar| went_the_other_way(bar))
            .fold(None::<&Bar>, |best, bar| match best {
                Some(current)
                    if bar.temptation < current.temptation
                        || (bar.temptation == current.temptation && bar.index > current.index) =>
                {
                    Some(current)
                }
                _ => Some(bar),
            })
    {
        marks.push(mark(trap, window, side, MarkQuality::Bad));
    }

    Ok(marks)
}

fn mark(bar: &Bar, window: Window, side: Side, quality: MarkQuality) -> Mark {
    let note = match quality {
        MarkQuality::Good => "oracle: best forward result in the window",
        MarkQuality::Bad => "oracle: tempting setup with the worst forward result",
        MarkQuality::Neutral => "oracle",
    };
    Mark {
        chart: window.index,
        bar: bar.index,
        action: MarkAction::Entry,
        quality,
        side,
        origin: MarkOrigin::Oracle,
        note: note.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> Window {
        Window {
            index: 2,
            start: 100,
            end: 110,
        }
    }

    fn bars() -> Vec<Bar> {
        vec![
            Bar {
                index: 100,
                forward_return: -1.0,
                temptation: 0.9,
            },
            Bar {
                index: 101,
                forward_return: 2.0,
                temptation: 0.3,
            },
            Bar {
                index: 102,
                forward_return: -2.0,
                temptation: 0.8,
            },
            Bar {
                index: 103,
                forward_return: 1.0,
                temptation: 0.5,
            },
        ]
    }

    #[test]
    fn a_long_marks_the_best_and_the_worst_tempting() {
        let marks = label(&bars(), window(), Side::Long).expect("label");
        assert_eq!(marks.len(), 2);
        assert_eq!(marks[0].bar, 101);
        assert_eq!(marks[0].quality, MarkQuality::Good);
        assert_eq!(marks[1].bar, 100);
        assert_eq!(marks[1].quality, MarkQuality::Bad);
    }

    #[test]
    fn a_short_marks_the_lowest_result() {
        let marks = label(&bars(), window(), Side::Short).expect("label");
        assert_eq!(marks[0].bar, 102);
        assert_eq!(marks[0].quality, MarkQuality::Good);
        assert_eq!(marks[1].bar, 103);
        assert_eq!(marks[1].quality, MarkQuality::Bad);
    }

    #[test]
    fn ties_break_on_the_lowest_bar() {
        let bars = vec![
            Bar {
                index: 105,
                forward_return: 2.0,
                temptation: 0.1,
            },
            Bar {
                index: 102,
                forward_return: 2.0,
                temptation: 0.1,
            },
        ];
        let marks = label(&bars, window(), Side::Long).expect("label");
        assert_eq!(marks[0].bar, 102);
    }

    #[test]
    fn no_bad_mark_when_everything_worked() {
        let bars = vec![
            Bar {
                index: 100,
                forward_return: 1.0,
                temptation: 0.2,
            },
            Bar {
                index: 101,
                forward_return: 2.0,
                temptation: 0.4,
            },
        ];
        let marks = label(&bars, window(), Side::Long).expect("label");
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].quality, MarkQuality::Good);
    }

    #[test]
    fn a_single_bar_window_marks_one_candidate() {
        let bars = vec![Bar {
            index: 100,
            forward_return: 1.0,
            temptation: 0.5,
        }];
        let marks = label(&bars, window(), Side::Long).expect("label");
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].bar, 100);
        assert_eq!(marks[0].quality, MarkQuality::Good);
    }

    #[test]
    fn an_empty_window_is_refused() {
        assert_eq!(label(&[], window(), Side::Long), Err(OracleError::Empty));
    }

    #[test]
    fn a_bar_outside_the_window_is_refused() {
        let bars = vec![Bar {
            index: 200,
            forward_return: 1.0,
            temptation: 0.1,
        }];
        assert_eq!(
            label(&bars, window(), Side::Long),
            Err(OracleError::BarOutside(200))
        );
    }

    #[test]
    fn the_same_bars_give_the_same_marks() {
        let first = label(&bars(), window(), Side::Long).expect("first");
        let second = label(&bars(), window(), Side::Long).expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn the_marks_are_oracle_origin() {
        let marks = label(&bars(), window(), Side::Long).expect("label");
        assert!(marks.iter().all(|mark| mark.origin == MarkOrigin::Oracle));
    }
}
