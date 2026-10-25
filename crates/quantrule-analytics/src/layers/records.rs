//! The pivot and confirmed-level records.
//!
//! The layer build already computes the model's [`PivotSeries`] and
//! [`ConfirmedLevelRegistry`]. This module turns them into the flat rows the
//! store writes, without recomputing either. A pivot occurrence is one row: a
//! candidate has `confirmed_code = 0` and is available on its occurrence bar;
//! a pivot that later confirms has `confirmed_code = 1` and is available on the
//! confirmation bar.
//!
//! Rows are deterministic. Pivots are ordered by timeframe, occurrence bar,
//! scale, and kind; levels are ordered by timeframe and reference code. The
//! builders never read the clock or an unordered collection.

use quantrule_model::axes::structure::registry::ConfirmedLevel;
use quantrule_model::readings::{Pivot, PivotKind, PivotScale, PivotSeries};

/// One stored pivot occurrence.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PivotRow {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The scale's registry code: 1 minor, 2 major.
    pub scale_code: u8,
    /// The kind's registry code: 1 swing high, 2 swing low.
    pub kind_code: u8,
    /// The swing high or low price.
    pub price: f64,
    /// The bar on which the extreme occurred.
    pub occurred_bar_index: usize,
    /// The first bar on which the pivot was knowable.
    pub available_bar_index: usize,
    /// 1 when the pivot is confirmed, 0 when it is still a candidate.
    pub confirmed_code: u8,
}

/// One stored confirmed level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelRow {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The contract's `1000 + n` reference code.
    pub reference_code: u32,
    /// The scale's registry code: 1 minor, 2 major.
    pub scale_code: u8,
    /// The kind's registry code: 1 swing high, 2 swing low.
    pub kind_code: u8,
    /// The swing high or low price.
    pub price: f64,
    /// The bar on which the extreme occurred.
    pub occurred_bar_index: usize,
    /// The first bar on which the level was knowable.
    pub available_bar_index: usize,
}

/// Builds one row per pivot occurrence for `timeframe_secs`.
pub(crate) fn pivot_rows(timeframe_secs: u64, pivots: &PivotSeries) -> Vec<PivotRow> {
    let len = pivots.len();
    let mut confirmed: [Vec<Option<Pivot>>; 2] = [vec![None; len], vec![None; len]];
    for (index, scale) in PivotScale::ALL.iter().copied().enumerate() {
        for pivot in pivots.all_confirmed(scale) {
            if let Some(slot) = confirmed[index].get_mut(pivot.occurred_at) {
                *slot = Some(*pivot);
            }
        }
    }

    let mut rows = Vec::new();
    for bar in 0..len {
        for (index, scale) in PivotScale::ALL.iter().copied().enumerate() {
            let Some(candidate) = pivots.candidate(scale, bar) else {
                continue;
            };
            let confirmed_pivot = confirmed[index]
                .get(bar)
                .and_then(|pivot| *pivot)
                .filter(|pivot| pivot.kind == candidate.kind);
            rows.push(PivotRow {
                timeframe_secs,
                scale_code: scale.code(),
                kind_code: kind_code(candidate.kind),
                price: candidate.price,
                occurred_bar_index: candidate.occurred_at,
                available_bar_index: confirmed_pivot
                    .map_or(candidate.available_at, |pivot| pivot.available_at),
                confirmed_code: u8::from(confirmed_pivot.is_some()),
            });
        }
    }
    rows
}

/// Builds one row per confirmed level for `timeframe_secs`.
pub(crate) fn level_rows(timeframe_secs: u64, levels: &[ConfirmedLevel]) -> Vec<LevelRow> {
    levels
        .iter()
        .map(|level| LevelRow {
            timeframe_secs,
            reference_code: level.reference_code,
            scale_code: level.scale.code(),
            kind_code: kind_code(level.kind),
            price: level.price,
            occurred_bar_index: level.occurred_at,
            available_bar_index: level.available_at,
        })
        .collect()
}

/// The research contract's pivot-kind code.
const fn kind_code(kind: PivotKind) -> u8 {
    match kind {
        PivotKind::High => 1,
        PivotKind::Low => 2,
    }
}
