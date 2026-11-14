//! The deterministic hindsight labeler.
//!
//! The oracle looks at one window with the future visible and proposes the
//! cleanest good entry and the most tempting bad entry on each side under the
//! run's pre-registered criterion. It is the mechanical half of the marking
//! loop: a human marks the same window and the two sets are compared.
//!
//! "Good" is a target reached before a stop; "bad" is a stop reached before a
//! target. Both are measured in ATR from the entry close over the forward
//! horizon. The perfect exit is the best excursion for a good path, and the
//! best exit before the adverse move for a bad path. Ties break on the earliest
//! entry, so the result never depends on iteration order.
//!
//! A simulation run is a hindsight label, not a causal signal. The oracle never
//! writes a strategy decision.
//!
//! `port-parity: PORT-001` names the golden fixture this module is compared
//! against.

// port-parity: PORT-001

use serde::{Deserialize, Serialize};

use crate::bundle::{Bar, Bundle};
use crate::marks::{Mark, MarkAction, MarkQuality, NOTE_LIMIT, Side};
use crate::window::WindowRecord;

/// The pre-registered hindsight criterion for one run.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OracleSpec {
    /// Bars the forward path may span.
    pub horizon_bars: usize,
    /// The favorable move, in ATR, that makes a good entry.
    pub target_atr: f64,
    /// The adverse move, in ATR, that fails a good entry.
    pub stop_atr: f64,
    /// The adverse move, in ATR, that makes a bad entry.
    pub bad_stop_atr: f64,
    /// The favorable move, in ATR, that rescues a bad entry.
    pub bad_target_atr: f64,
    /// The latest bar offset after the candidate the oracle may enter on.
    pub entry_offset_max: usize,
}

impl OracleSpec {
    /// Reject an impossible criterion.
    ///
    /// # Errors
    ///
    /// Returns [`OracleError::InvalidSpec`] when the horizon is shorter than two
    /// bars, a stop and target are not ordered, or the entry offset is negative
    /// (which the unsigned type already excludes).
    pub fn validate(&self) -> Result<(), OracleError> {
        if self.horizon_bars < 2 {
            return Err(OracleError::InvalidSpec("horizon_bars must be at least 2"));
        }
        if !(0.0 < self.stop_atr && self.stop_atr < self.target_atr) {
            return Err(OracleError::InvalidSpec(
                "require 0 < stop_atr < target_atr",
            ));
        }
        if !(0.0 < self.bad_target_atr && self.bad_target_atr < self.bad_stop_atr) {
            return Err(OracleError::InvalidSpec(
                "require 0 < bad_target_atr < bad_stop_atr",
            ));
        }
        Ok(())
    }
}

/// Why the oracle cannot label a window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OracleError {
    /// The bundle has no bars, or the window is outside them.
    EmptyWindow,
    /// A bar index is outside the bundle.
    BarOutside(usize),
    /// The criterion is impossible.
    InvalidSpec(&'static str),
}

impl std::fmt::Display for OracleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyWindow => formatter.write_str("the window has no bars"),
            Self::BarOutside(index) => write!(formatter, "bar index {index} is outside the bundle"),
            Self::InvalidSpec(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for OracleError {}

/// The forward path of one candidate entry.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PathResult {
    /// The side.
    pub side: Side,
    /// The entry index.
    pub entry_index: usize,
    /// The entry bar identity.
    pub entry_bar: u64,
    /// The entry price: the entry bar close.
    pub entry_price: f64,
    /// The ATR at the entry.
    pub atr: f64,
    /// The best favorable excursion, in ATR.
    pub mfe_atr: f64,
    /// The worst adverse excursion, in ATR.
    pub mae_atr: f64,
    /// The index of the best favorable excursion.
    pub mfe_index: usize,
    /// The index of the worst adverse excursion.
    pub mae_index: usize,
    /// The first bar that reached the good target.
    pub good_target_index: Option<usize>,
    /// The first bar that reached the good stop.
    pub good_stop_index: Option<usize>,
    /// The first bar that reached the bad stop.
    pub bad_stop_index: Option<usize>,
    /// The first bar that reached the bad target.
    pub bad_target_index: Option<usize>,
    /// Whether the target came before the stop.
    pub good: bool,
    /// Whether the stop came before the target.
    pub bad: bool,
    /// The best exit with hindsight.
    pub perfect_exit_index: Option<usize>,
    /// The rule exit.
    pub rule_exit_index: Option<usize>,
    /// The best favorable excursion before the bad stop, in ATR.
    pub mfe_before_bad_stop: f64,
    /// The best favorable excursion up to the rule exit, in ATR.
    pub mfe_to_rule_atr: f64,
    /// The worst adverse excursion up to the rule exit, in ATR.
    pub mae_to_rule_atr: f64,
}

impl PathResult {
    /// A ranking score; larger is a cleaner good mark.
    #[must_use]
    pub fn reward_risk(&self) -> f64 {
        self.mfe_atr / self.mae_atr.max(0.1)
    }
}

/// One detail row for the review sidecar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Detail {
    /// The chart file name.
    pub chart: String,
    /// The run identity.
    pub run_id: String,
    /// The window identity.
    pub window_id: String,
    /// The side.
    pub side: Side,
    /// The quality.
    pub quality: MarkQuality,
    /// The entry bar identity.
    pub entry_bar: u64,
    /// The entry index.
    pub entry_index: usize,
    /// The entry price.
    pub entry_price: f64,
    /// The ATR at the entry.
    pub atr: f64,
    /// The rule-exit bar identity.
    pub rule_exit_bar: Option<u64>,
    /// The perfect-exit bar identity.
    pub perfect_exit_bar: Option<u64>,
    /// The best favorable excursion, in ATR.
    pub mfe_atr: f64,
    /// The worst adverse excursion, in ATR.
    pub mae_atr: f64,
    /// The best favorable excursion to the rule exit, in ATR.
    pub mfe_to_rule_atr: f64,
    /// The worst adverse excursion to the rule exit, in ATR.
    pub mae_to_rule_atr: f64,
    /// The best favorable excursion before the bad stop, in ATR.
    pub mfe_before_bad_stop_atr: f64,
    /// Whether the path was a good example.
    pub good: bool,
    /// Whether the path was a bad example.
    pub bad: bool,
}

/// The marks and detail rows the oracle proposes for one window.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    /// The proposed marks, in proposal order; their times are filled at
    /// validation.
    pub marks: Vec<Mark>,
    /// The sidecar detail rows.
    pub details: Vec<Detail>,
}

/// Label one window for both sides.
///
/// # Errors
///
/// Returns [`OracleError`] when the criterion is invalid or the window falls
/// outside the bundle.
pub fn label(
    bundle: &Bundle,
    window: &WindowRecord,
    chart: &str,
    spec: &OracleSpec,
    run_id: &str,
) -> Result<Label, OracleError> {
    spec.validate()?;
    if bundle.bars.is_empty() || window.start_index >= window.end_index {
        return Err(OracleError::EmptyWindow);
    }
    if window.end_index > bundle.bars.len() || window.candidate_index >= bundle.bars.len() {
        return Err(OracleError::BarOutside(window.end_index));
    }
    let candidates = entry_candidates(window, bundle, spec);
    let mut marks = Vec::new();
    let mut details = Vec::new();
    if candidates.is_empty() {
        return Ok(Label { marks, details });
    }
    for side in [Side::Long, Side::Short] {
        let results: Vec<PathResult> = candidates
            .iter()
            .filter_map(|index| forward_path(bundle, *index, side, spec))
            .collect();
        let mut good = select_good(&results);
        let mut bad = select_bad(&results);
        if let (Some(good_path), Some(bad_path)) = (good.as_ref(), bad.as_ref())
            && (bad_path.entry_bar == good_path.entry_bar
                || bad_path.rule_exit_index == good_path.rule_exit_index)
        {
            bad = None;
        }
        for (result, quality) in [
            (good.take(), MarkQuality::Good),
            (bad.take(), MarkQuality::Bad),
        ] {
            let Some(result) = result else {
                continue;
            };
            let Some(rule_exit) = result.rule_exit_index else {
                continue;
            };
            let perfect = result.perfect_exit_index.map_or_else(
                || "?".to_owned(),
                |index| bundle.bars[index].bar.to_string(),
            );
            let hint = format!(
                "{run_id} {} {} hint mfe={:.2}atr mae={:.2}atr perfect_exit={perfect}; hindsight label, not a signal",
                quality.as_str(),
                side.as_str(),
                result.mfe_atr,
                result.mae_atr
            );
            marks.push(mark_row(
                chart,
                result.entry_bar,
                MarkAction::Entry,
                quality,
                side,
                truncate(&hint),
            ));
            let exit_bar = bundle.bars[rule_exit].bar;
            marks.push(mark_row(
                chart,
                exit_bar,
                MarkAction::Exit,
                quality,
                side,
                truncate(&format!(
                    "{run_id} rule exit {} {}; {hint}",
                    quality.as_str(),
                    side.as_str()
                )),
            ));
            details.push(detail_row(chart, window, &result, bundle, quality, run_id));
        }
    }
    Ok(Label { marks, details })
}

fn entry_candidates(window: &WindowRecord, bundle: &Bundle, spec: &OracleSpec) -> Vec<usize> {
    let mut candidates = Vec::new();
    for offset in 0..=spec.entry_offset_max {
        let index = window.candidate_index + offset;
        if window.start_index <= index && index < window.end_index && index < bundle.bars.len() - 1
        {
            candidates.push(index);
        }
    }
    candidates
}

fn forward_path(
    bundle: &Bundle,
    entry_index: usize,
    side: Side,
    spec: &OracleSpec,
) -> Option<PathResult> {
    let entry = bundle.bars.get(entry_index)?;
    let atr = entry.atr?;
    if !atr.is_finite() || atr <= 0.0 {
        return None;
    }
    let entry_price = entry.close;
    if !entry_price.is_finite() {
        return None;
    }
    let end = (entry_index + spec.horizon_bars).min(bundle.bars.len().checked_sub(1)?);
    if end <= entry_index {
        return None;
    }
    let mut best_fav = 0.0_f64;
    let mut best_fav_index = entry_index;
    let mut worst_adv = 0.0_f64;
    let mut worst_adv_index = entry_index;
    let mut good_target = None;
    let mut good_stop = None;
    let mut bad_stop = None;
    let mut bad_target = None;
    let mut pre_bad_stop_best = 0.0_f64;
    let mut pre_bad_stop_index = None;
    for index in entry_index + 1..=end {
        let bar = &bundle.bars[index];
        let (favorable, adverse) = excursion(bar, entry_price, side);
        let favorable_atr = favorable / atr;
        let adverse_atr = adverse / atr;
        if bad_stop.is_none() && favorable_atr > pre_bad_stop_best {
            pre_bad_stop_best = favorable_atr;
            pre_bad_stop_index = Some(index);
        }
        if favorable_atr > best_fav {
            best_fav = favorable_atr;
            best_fav_index = index;
        }
        if adverse_atr > worst_adv {
            worst_adv = adverse_atr;
            worst_adv_index = index;
        }
        if good_target.is_none() && favorable_atr >= spec.target_atr {
            good_target = Some(index);
        }
        if good_stop.is_none() && adverse_atr >= spec.stop_atr {
            good_stop = Some(index);
        }
        if bad_target.is_none() && favorable_atr >= spec.bad_target_atr {
            bad_target = Some(index);
        }
        if bad_stop.is_none() && adverse_atr >= spec.bad_stop_atr {
            bad_stop = Some(index);
        }
    }
    let good = good_target.is_some() && (good_stop.is_none() || good_target < good_stop);
    let bad = bad_stop.is_some() && (bad_target.is_none() || bad_stop < bad_target);
    let (perfect, rule) = if good {
        (Some(best_fav_index), good_target)
    } else if bad {
        (pre_bad_stop_index, bad_stop)
    } else {
        (None, None)
    };
    let mut mfe_to_rule = 0.0_f64;
    let mut mae_to_rule = 0.0_f64;
    if let Some(rule) = rule {
        for index in entry_index + 1..=rule {
            let bar = &bundle.bars[index];
            let (favorable, adverse) = excursion(bar, entry_price, side);
            mfe_to_rule = mfe_to_rule.max(favorable / atr);
            mae_to_rule = mae_to_rule.max(adverse / atr);
        }
    }
    Some(PathResult {
        side,
        entry_index,
        entry_bar: entry.bar,
        entry_price,
        atr,
        mfe_atr: best_fav,
        mae_atr: worst_adv,
        mfe_index: best_fav_index,
        mae_index: worst_adv_index,
        good_target_index: good_target,
        good_stop_index: good_stop,
        bad_stop_index: bad_stop,
        bad_target_index: bad_target,
        good,
        bad,
        perfect_exit_index: perfect,
        rule_exit_index: rule,
        mfe_before_bad_stop: pre_bad_stop_best,
        mfe_to_rule_atr: mfe_to_rule,
        mae_to_rule_atr: mae_to_rule,
    })
}

fn excursion(bar: &Bar, entry: f64, side: Side) -> (f64, f64) {
    match side {
        Side::Long => (bar.high - entry, entry - bar.low),
        Side::Short => (entry - bar.low, bar.high - entry),
    }
}

fn select_good(results: &[PathResult]) -> Option<PathResult> {
    let mut good: Vec<&PathResult> = results.iter().filter(|result| result.good).collect();
    if good.is_empty() {
        return None;
    }
    good.sort_by(|left, right| {
        right
            .reward_risk()
            .partial_cmp(&left.reward_risk())
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.entry_index.cmp(&right.entry_index))
    });
    good.first().map(|result| **result)
}

fn select_bad(results: &[PathResult]) -> Option<PathResult> {
    let mut bad: Vec<&PathResult> = results.iter().filter(|result| result.bad).collect();
    if bad.is_empty() {
        return None;
    }
    bad.sort_by(|left, right| {
        left.entry_index.cmp(&right.entry_index).then_with(|| {
            right
                .mae_atr
                .partial_cmp(&left.mae_atr)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
    });
    bad.first().map(|result| **result)
}

fn mark_row(
    chart: &str,
    bar: u64,
    action: MarkAction,
    quality: MarkQuality,
    side: Side,
    note: String,
) -> Mark {
    Mark {
        chart: chart.to_owned(),
        bar,
        marked_time_utc: String::new(),
        action,
        quality,
        side,
        note,
    }
}

fn truncate(note: &str) -> String {
    note.chars().take(NOTE_LIMIT).collect()
}

fn detail_row(
    chart: &str,
    window: &WindowRecord,
    result: &PathResult,
    bundle: &Bundle,
    quality: MarkQuality,
    run_id: &str,
) -> Detail {
    Detail {
        chart: chart.to_owned(),
        run_id: run_id.to_owned(),
        window_id: window.window_id.clone(),
        side: result.side,
        quality,
        entry_bar: result.entry_bar,
        entry_index: result.entry_index,
        entry_price: result.entry_price,
        atr: result.atr,
        rule_exit_bar: result.rule_exit_index.map(|index| bundle.bars[index].bar),
        perfect_exit_bar: result
            .perfect_exit_index
            .map(|index| bundle.bars[index].bar),
        mfe_atr: result.mfe_atr,
        mae_atr: result.mae_atr,
        mfe_to_rule_atr: result.mfe_to_rule_atr,
        mae_to_rule_atr: result.mae_to_rule_atr,
        mfe_before_bad_stop_atr: result.mfe_before_bad_stop,
        good: result.good,
        bad: result.bad,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::Bar;

    fn bundle(highs: &[f64], lows: &[f64], closes: &[f64]) -> Bundle {
        let bars = highs
            .iter()
            .zip(lows)
            .zip(closes)
            .enumerate()
            .map(|(index, ((high, low), close))| Bar {
                atr: Some(1.0),
                ..Bar::from_prices(
                    index as u64,
                    1_514_764_800 + index as i64 * 900,
                    *close,
                    *high,
                    *low,
                    *close,
                )
            })
            .collect();
        Bundle::new(900, bars, Vec::new())
    }

    fn window(bundle: &Bundle) -> WindowRecord {
        WindowRecord {
            window_id: "w001".to_owned(),
            start_index: 0,
            end_index: bundle.bars.len(),
            start_bar: 0,
            end_bar: bundle.bars.len() as u64 - 1,
            start_time: String::new(),
            end_time: String::new(),
            candidate_index: 0,
            candidate_bar: 0,
            candidate_time: String::new(),
            events: Vec::new(),
        }
    }

    fn spec() -> OracleSpec {
        OracleSpec {
            horizon_bars: 60,
            target_atr: 2.0,
            stop_atr: 1.0,
            bad_stop_atr: 1.5,
            bad_target_atr: 1.0,
            entry_offset_max: 0,
        }
    }

    fn marks_for(bundle: &Bundle, side: Side, quality: MarkQuality) -> (Vec<Mark>, Vec<Mark>) {
        let label =
            label(bundle, &window(bundle), "sim-w001.png", &spec(), "SIM-T").expect("label");
        let entries = label
            .marks
            .iter()
            .filter(|mark| {
                mark.side == side && mark.quality == quality && mark.action == MarkAction::Entry
            })
            .cloned()
            .collect();
        let exits = label
            .marks
            .iter()
            .filter(|mark| {
                mark.side == side && mark.quality == quality && mark.action == MarkAction::Exit
            })
            .cloned()
            .collect();
        (entries, exits)
    }

    #[test]
    fn a_long_target_first_is_good() {
        let bundle = bundle(
            &[100.0, 102.1, 102.5],
            &[99.5, 99.9, 99.8],
            &[100.0, 100.0, 100.0],
        );
        let (entries, exits) = marks_for(&bundle, Side::Long, MarkQuality::Good);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].bar, 0);
        assert_eq!(exits.len(), 1);
        assert_eq!(exits[0].bar, 1);
        let label =
            label(&bundle, &window(&bundle), "sim-w001.png", &spec(), "SIM-T").expect("label");
        let detail = label
            .details
            .iter()
            .find(|row| row.side == Side::Long && row.quality == MarkQuality::Good)
            .expect("detail");
        assert!((detail.mfe_atr - 2.5).abs() < 1e-12);
        assert_eq!(detail.perfect_exit_bar, Some(2));
    }

    #[test]
    fn a_long_stop_first_is_bad() {
        let bundle = bundle(
            &[100.0, 100.2, 100.4],
            &[99.5, 98.4, 98.9],
            &[100.0, 100.0, 100.0],
        );
        let (entries, exits) = marks_for(&bundle, Side::Long, MarkQuality::Bad);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].bar, 0);
        assert_eq!(exits.len(), 1);
        assert_eq!(exits[0].bar, 1);
        let label =
            label(&bundle, &window(&bundle), "sim-w001.png", &spec(), "SIM-T").expect("label");
        let detail = label
            .details
            .iter()
            .find(|row| row.side == Side::Long && row.quality == MarkQuality::Bad)
            .expect("detail");
        assert!(detail.bad);
        assert!((detail.mae_atr - 1.6).abs() < 1e-12);
    }

    #[test]
    fn a_short_target_first_is_good() {
        let bundle = bundle(
            &[100.0, 99.9, 99.8],
            &[99.5, 97.9, 97.5],
            &[100.0, 100.0, 100.0],
        );
        let (entries, exits) = marks_for(&bundle, Side::Short, MarkQuality::Good);
        assert_eq!(entries.len(), 1);
        assert_eq!(exits.len(), 1);
        assert_eq!(entries[0].bar, 0);
        assert_eq!(exits[0].bar, 1);
    }

    #[test]
    fn a_short_stop_first_is_bad() {
        let bundle = bundle(
            &[100.0, 101.8, 102.0],
            &[99.5, 99.8, 99.0],
            &[100.0, 100.0, 100.0],
        );
        let (entries, exits) = marks_for(&bundle, Side::Short, MarkQuality::Bad);
        assert_eq!(entries.len(), 1);
        assert_eq!(exits.len(), 1);
    }

    #[test]
    fn an_entry_offset_can_wait_for_confirmation() {
        let bundle = bundle(
            &[100.0, 100.0, 102.1],
            &[99.5, 99.5, 99.9],
            &[100.0, 100.0, 100.0],
        );
        let mut spec = spec();
        spec.entry_offset_max = 1;
        let label =
            label(&bundle, &window(&bundle), "sim-w001.png", &spec, "SIM-T").expect("label");
        let entry = label
            .marks
            .iter()
            .find(|mark| mark.action == MarkAction::Entry && mark.side == Side::Long)
            .expect("entry");
        assert_eq!(entry.bar, 1);
    }

    #[test]
    fn the_same_bars_give_the_same_marks() {
        let bundle = bundle(
            &[100.0, 102.1, 102.5],
            &[99.5, 99.9, 99.8],
            &[100.0, 100.0, 100.0],
        );
        let first =
            label(&bundle, &window(&bundle), "sim-w001.png", &spec(), "SIM-T").expect("first");
        let second =
            label(&bundle, &window(&bundle), "sim-w001.png", &spec(), "SIM-T").expect("second");
        assert_eq!(first, second);
    }

    #[test]
    fn an_invalid_criterion_is_refused() {
        let bundle = bundle(&[100.0, 100.0], &[99.0, 99.0], &[100.0, 100.0]);
        let mut spec = spec();
        spec.stop_atr = 3.0;
        assert!(matches!(
            label(&bundle, &window(&bundle), "chart", &spec, "SIM-T"),
            Err(OracleError::InvalidSpec(_))
        ));
    }
}
