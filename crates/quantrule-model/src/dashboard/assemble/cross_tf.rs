//! The cross-timeframe agreement and churn summaries.
//!
//! Agreement is a count over the published per-axis positions for Direction,
//! Momentum, and Structure. Churn is a reduction over the published
//! `state_changes_50` counters and the code-bearing history of the four axes.
//! Neither summary recomputes an axis, changes a per-timeframe value, or feeds
//! back into the model.
//!
//! Both summaries are limited to the configured ladder. A missing timeframe is
//! absent, never a neutral vote. A missing axis value lowers the valid count
//! instead of becoming neutral. Churn sources are sorted by timeframe and then
//! axis, so neither the caller's ladder order nor an unordered collection can
//! change the output.
//!
//! This module is pure: it reads the assembled view and the model run it is
//! given, and performs no I/O, reads no clock, and uses no randomness.

use crate::readings::roles::AxisCode;
use crate::run::{ModelRun, TimeframeModel};

use super::super::{
    AxisAgreement, AxisValue, ChurnClass, ChurnSource, Coverage, CrossTfAgreement, CrossTfChurn,
    DashboardConfig, MajorityClass, TimeframeState,
};

/// The three axes that vote in the cross-timeframe agreement.
const VOTING_AXES: [AxisCode; 3] = [AxisCode::Direction, AxisCode::Momentum, AxisCode::Structure];

/// The contract's fixed churn window in code-bearing bars.
const CHURN_WINDOW_BARS: u32 = 50;

/// The number of code-bearing bars required before churn history is complete.
const HISTORY_REQUIRED_CODE_BARS: usize = CHURN_WINDOW_BARS as usize;

/// The inclusive half-width of the neutral corridor.
const POSITION_THRESHOLD: f64 = 0.1;

/// Derives the agreement summary from the assembled per-axis positions.
///
/// `timeframes` is the configured ladder's assembled rows. Only present rows
/// are read, and only a finite signed position is a valid vote. An undefined
/// position is excluded rather than counted as neutral.
pub(super) fn agreement(
    timeframes: &[TimeframeState],
    config: &DashboardConfig,
    coverage: Coverage,
) -> CrossTfAgreement {
    let axes: Vec<AxisAgreement> = VOTING_AXES
        .into_iter()
        .map(|axis| {
            let positions = timeframes
                .iter()
                .filter(|timeframe| timeframe.present)
                .map(|timeframe| published_position(timeframe, axis));
            axis_agreement(axis, positions, config.min_cross_tf_members)
        })
        .collect();
    let agreeing_axes = axes.iter().filter(|axis| axis.agrees).count() as u8;

    CrossTfAgreement {
        of: VOTING_AXES.len() as u8,
        agreeing_axes,
        axes,
        coverage,
        min_valid_timeframes: config.min_cross_tf_members,
    }
}

/// Derives the churn summary from the published model series.
///
/// `ladder` is the normalized configured ladder. A ladder member without a
/// closed bar contributes no evidence, which leaves history incomplete rather
/// than inventing a zero measurement.
pub(super) fn churn(
    run: &ModelRun,
    ladder: &[u64],
    config: &DashboardConfig,
    coverage: Coverage,
) -> CrossTfChurn {
    let mut evidence = Vec::with_capacity(ladder.len() * AxisCode::ALL.len());

    for &timeframe_secs in ladder {
        let Some(timeframe) = run.timeframe(timeframe_secs) else {
            continue;
        };
        let Some(view) = timeframe.len().checked_sub(1) else {
            continue;
        };
        let Some(axes) = churn_evidence(timeframe, view) else {
            continue;
        };
        evidence.extend_from_slice(&axes);
    }

    summarize_churn(&evidence, ladder, config, coverage)
}

/// Builds one agreement entry from a sequence of candidate positions.
///
/// `None` is an absent or non-finite position and is excluded from the valid
/// count. A strict majority requires one class to hold more than half of the
/// valid positions; an exact half or a three-way split has no majority.
fn axis_agreement(
    axis: AxisCode,
    positions: impl IntoIterator<Item = Option<f64>>,
    min_valid_timeframes: u8,
) -> AxisAgreement {
    let mut bullish = 0_u32;
    let mut bearish = 0_u32;
    let mut neutral = 0_u32;
    let mut valid = 0_u32;

    for position in positions.into_iter().flatten() {
        if !position.is_finite() {
            continue;
        }
        valid += 1;
        if position > POSITION_THRESHOLD {
            bullish += 1;
        } else if position < -POSITION_THRESHOLD {
            bearish += 1;
        } else {
            neutral += 1;
        }
    }

    let insufficient_coverage = valid < u32::from(min_valid_timeframes);
    let majority = if insufficient_coverage {
        None
    } else {
        strict_majority(bullish, bearish, neutral, valid)
    };
    let majority_share = majority.and_then(|class| {
        let count = match class {
            MajorityClass::Bullish => bullish,
            MajorityClass::Bearish => bearish,
            MajorityClass::Neutral => neutral,
        };
        (valid > 0).then(|| f64::from(count) / f64::from(valid))
    });

    AxisAgreement {
        axis,
        valid_timeframes: to_u8(valid),
        bullish_count: to_u8(bullish),
        bearish_count: to_u8(bearish),
        neutral_count: to_u8(neutral),
        majority,
        majority_share,
        agrees: majority.is_some(),
        insufficient_coverage,
    }
}

/// Returns the class whose count is a strict majority, or `None`.
fn strict_majority(bullish: u32, bearish: u32, neutral: u32, valid: u32) -> Option<MajorityClass> {
    let maximum = bullish.max(bearish).max(neutral);
    if u64::from(maximum) * 2 <= u64::from(valid) {
        return None;
    }

    // A tie cannot reach this point: a tied maximum is at most half of the
    // valid count. The branch order therefore only makes the unique winner
    // explicit.
    if bullish == maximum {
        Some(MajorityClass::Bullish)
    } else if bearish == maximum {
        Some(MajorityClass::Bearish)
    } else {
        Some(MajorityClass::Neutral)
    }
}

/// The finite signed position published for `axis`, if there is one.
fn published_position(timeframe: &TimeframeState, axis: AxisCode) -> Option<f64> {
    let state = timeframe.axes.iter().find(|state| state.axis == axis)?;
    match state.position {
        AxisValue::Signed { value, .. } if value.is_finite() => Some(value),
        _ => None,
    }
}

/// One axis's churn evidence at one closed bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChurnEvidence {
    /// The source timeframe in seconds.
    timeframe_secs: u64,
    /// The source axis.
    axis: AxisCode,
    /// Reported changes in the last 50 code-bearing bars.
    changes_50: u64,
    /// Code-bearing bars in the loaded history, capped at the required window.
    code_bearing_bars: usize,
}

/// Reads the four axes' churn evidence at `view`.
///
/// Returns `None` when `view` is not a bar of the timeframe. Only bars up to
/// and including `view` are read, so a later bar cannot influence an earlier
/// summary.
fn churn_evidence(timeframe: &TimeframeModel, view: usize) -> Option<[ChurnEvidence; 4]> {
    let latest = timeframe.bar(view)?;
    let counts = code_bearing_counts(timeframe, view)?;
    let changes = [
        latest.direction.state_changes_50,
        latest.momentum.state_changes_50,
        latest.structure.state_changes_50,
        latest.progression.state_changes_50,
    ];
    let mut evidence = [ChurnEvidence {
        timeframe_secs: timeframe.timeframe_secs(),
        axis: AxisCode::Direction,
        changes_50: 0,
        code_bearing_bars: 0,
    }; 4];

    for (index, axis) in AxisCode::ALL.into_iter().enumerate() {
        evidence[index] = ChurnEvidence {
            timeframe_secs: timeframe.timeframe_secs(),
            axis,
            changes_50: changes[index],
            code_bearing_bars: counts[index],
        };
    }

    Some(evidence)
}

/// Counts code-bearing bars per axis up to and including `view`.
///
/// The count stops once every axis has reached the required window, because a
/// larger count has no additional meaning for completeness.
fn code_bearing_counts(timeframe: &TimeframeModel, view: usize) -> Option<[usize; 4]> {
    if view >= timeframe.len() {
        return None;
    }

    let mut counts = [0_usize; 4];
    for index in 0..=view {
        let Some(bar) = timeframe.bar(index) else {
            continue;
        };
        let code_bearing = [
            bar.direction.state_code.is_some(),
            bar.momentum.state_code.is_some(),
            bar.structure.state_code.is_some(),
            bar.progression.stage_code.is_some(),
        ];
        for (count, present) in counts.iter_mut().zip(code_bearing) {
            if present {
                *count += 1;
            }
        }
        if counts
            .iter()
            .all(|count| *count >= HISTORY_REQUIRED_CODE_BARS)
        {
            break;
        }
    }

    Some(counts)
}

/// Reduces the per-axis evidence to the published churn summary.
///
/// `configured_timeframes` is the normalized ladder. History is complete only
/// when every configured timeframe is represented for every compared axis with
/// at least the full code-bearing window.
fn summarize_churn(
    evidence: &[ChurnEvidence],
    configured_timeframes: &[u64],
    config: &DashboardConfig,
    coverage: Coverage,
) -> CrossTfChurn {
    let mut max_changes = 0_u64;
    let mut sources = Vec::new();

    for item in evidence {
        // An axis with no code-bearing bar has no measured churn. Counting it
        // as a zero would turn a missing history into a stable-looking source.
        if item.code_bearing_bars == 0 {
            continue;
        }
        if item.changes_50 > max_changes {
            max_changes = item.changes_50;
            sources.clear();
            sources.push(ChurnSource {
                timeframe_secs: item.timeframe_secs,
                axis: item.axis,
            });
        } else if item.changes_50 == max_changes {
            sources.push(ChurnSource {
                timeframe_secs: item.timeframe_secs,
                axis: item.axis,
            });
        }
    }

    sources.sort_by_key(|source| (source.timeframe_secs, source.axis));

    let history_complete = !configured_timeframes.is_empty()
        && configured_timeframes.iter().all(|timeframe_secs| {
            AxisCode::ALL.into_iter().all(|axis| {
                evidence.iter().any(|item| {
                    item.timeframe_secs == *timeframe_secs
                        && item.axis == axis
                        && item.code_bearing_bars >= HISTORY_REQUIRED_CODE_BARS
                })
            })
        });
    let churn_class = churn_class(history_complete, max_changes, config.churn_threshold);

    CrossTfChurn {
        max_changes_50: to_u32(max_changes),
        sources,
        threshold: config.churn_threshold,
        window: CHURN_WINDOW_BARS,
        history_complete,
        churn_class,
        coverage,
    }
}

/// Maps the history state and threshold comparison to the churn class.
///
/// Incomplete history always wins: an `Unstable` claim needs a full window, and
/// a `Holding` claim would overstate what an incomplete window shows.
fn churn_class(history_complete: bool, max_changes: u64, threshold: u32) -> ChurnClass {
    if !history_complete {
        ChurnClass::Building
    } else if max_changes >= u64::from(threshold) {
        ChurnClass::Unstable
    } else {
        ChurnClass::Holding
    }
}

/// Converts a count to the contract's `u8` field width.
///
/// A ladder above 255 members is outside the published field width; clamping
/// keeps the output numeric instead of truncating it to a different count.
fn to_u8(value: u32) -> u8 {
    u8::try_from(value).unwrap_or(u8::MAX)
}

/// Converts a change count to the contract's `u32` field width; the counter is
/// bounded by the 50-bar window, so the clamp is defensive only.
fn to_u32(value: u64) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
