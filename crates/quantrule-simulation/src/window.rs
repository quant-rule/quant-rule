//! Deterministic window selection.
//!
//! The span is split into equal strata and one window is drawn from each, so the
//! windows cannot overlap by construction. The same seed gives the same windows
//! on every machine.

// port-parity: PORT-001

use quantrule_research::Pcg32;
use serde::{Deserialize, Serialize};

/// Why a selection cannot be produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowError {
    /// No windows were requested.
    NoWindows,
    /// A window has no bars.
    EmptyWidth,
    /// The span is too small for the requested windows.
    SpanTooSmall,
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::NoWindows => "at least one window is required",
            Self::EmptyWidth => "a window needs at least one bar",
            Self::SpanTooSmall => "the span cannot hold that many windows of that width",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for WindowError {}

/// One window, as a half-open bar range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Window {
    /// Stratum ordinal, from zero.
    pub index: u32,
    /// Inclusive first bar.
    pub start: u64,
    /// Exclusive last bar.
    pub end: u64,
}

impl Window {
    /// Bars in the window.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.end - self.start
    }

    /// Whether the window is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Whether a bar falls inside.
    #[must_use]
    pub fn contains(&self, bar: u64) -> bool {
        (self.start..self.end).contains(&bar)
    }
}

/// Choose `count` non-overlapping windows of `width` bars from a span.
///
/// # Errors
///
/// Returns [`WindowError`] when the count is zero, the width is zero, or the span
/// cannot hold the requested windows.
pub fn select(span: u64, count: u32, width: u64, seed: u64) -> Result<Vec<Window>, WindowError> {
    if count == 0 {
        return Err(WindowError::NoWindows);
    }
    if width == 0 {
        return Err(WindowError::EmptyWidth);
    }
    let stratum = span / u64::from(count);
    if stratum < width {
        return Err(WindowError::SpanTooSmall);
    }

    let mut generator = Pcg32::new(seed);
    let mut windows = Vec::with_capacity(count as usize);
    for index in 0..count {
        let low = u64::from(index) * stratum;
        let high = low + stratum - width;
        let start = low + generator.next_f64().mul_add((high - low + 1) as f64, 0.0) as u64;
        let start = start.min(high);
        windows.push(Window {
            index,
            start,
            end: start + width,
        });
    }
    Ok(windows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_windows_do_not_overlap_and_stay_in_bounds() {
        let windows = select(1_000, 5, 100, 7).expect("select");
        assert_eq!(windows.len(), 5);
        for pair in windows.windows(2) {
            assert!(pair[0].end <= pair[1].start, "{:?}", pair);
        }
        for window in &windows {
            assert!(window.end <= 1_000);
            assert_eq!(window.len(), 100);
        }
    }

    #[test]
    fn the_same_seed_gives_the_same_windows() {
        let first = select(10_000, 6, 120, 11).expect("first");
        let second = select(10_000, 6, 120, 11).expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn a_different_seed_moves_the_windows() {
        let first = select(10_000, 6, 120, 11).expect("first");
        let second = select(10_000, 6, 120, 12).expect("second");
        assert_ne!(first, second);
    }

    #[test]
    fn a_span_that_cannot_hold_the_windows_is_refused() {
        assert_eq!(select(100, 5, 30, 1), Err(WindowError::SpanTooSmall));
    }

    #[test]
    fn a_single_bar_span_is_one_window() {
        let windows = select(1, 1, 1, 3).expect("select");
        assert_eq!(
            windows,
            vec![Window {
                index: 0,
                start: 0,
                end: 1,
            }]
        );
    }

    #[test]
    fn empty_inputs_are_refused() {
        assert_eq!(select(100, 0, 10, 1), Err(WindowError::NoWindows));
        assert_eq!(select(100, 1, 0, 1), Err(WindowError::EmptyWidth));
    }

    #[test]
    fn a_window_contains_its_start_and_not_its_end() {
        let window = Window {
            index: 0,
            start: 10,
            end: 20,
        };
        assert!(window.contains(10));
        assert!(!window.contains(20));
        assert!(!window.is_empty());
    }

    #[test]
    fn windows_round_trip_through_json() {
        let windows = select(500, 3, 50, 5).expect("select");
        let text = serde_json::to_string(&windows).expect("encode");
        let back: Vec<Window> = serde_json::from_str(&text).expect("decode");
        assert_eq!(back, windows);
    }
}
