//! The Structure axis.
//!
//! Structure measures how price is organised. At the configured pivot scale it
//! compares the latest two confirmed swing highs and the latest two confirmed
//! swing lows, publishes the resulting score and compact swing code, and adds
//! the EMA distance, cross, and retest features for EMA 20/50/200. It also
//! publishes the confirmed-level registry that the Progression axis and the
//! Level events consume.
//!
//! The swing score reads only confirmed pivots. The EMA features are additional
//! context and never enter the score, so a correlated measurement cannot become
//! a second vote. The module is pure: it performs no I/O, reads no clock, and
//! uses no randomness.

mod ema;
pub mod registry;

use std::cmp::Ordering;

use crate::kernel::{
    PERCENTILE_WINDOW, Readiness, StateCounters, UnavailableReason, empirical_percentile,
};
use crate::readings::{Pivot, PivotKind, PivotScale, Readings};
use quantrule_data::Candle;

use self::ema::EmaFeatures;
use self::registry::ConfirmedLevelRegistry;

/// The published Structure values for one closed bar.
///
/// `score` is the primary value. The relation codes are resolved independently:
/// when one is available and the other is not, the available code is retained
/// while `score`, `state_code`, and the percentile are null. The EMA features
/// are auxiliary context; their nulls do not make the axis unavailable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StructureBar {
    /// Relation of the latest two confirmed swing highs at the configured
    /// scale: +1 when the newer high is higher, 0 when the two are equal, -1
    /// when it is lower, or `None` when fewer than two are knowable.
    pub high_relation_code: Option<i8>,
    /// Relation of the latest two confirmed swing lows at the configured
    /// scale: +1 when the newer low is higher, 0 when the two are equal, -1
    /// when it is lower, or `None` when fewer than two are knowable.
    pub low_relation_code: Option<i8>,
    /// `(high_relation_code + low_relation_code) / 2`, a discrete-valued
    /// continuous number in `[-1, +1]`, or `None` when either relation is
    /// unavailable.
    pub score: Option<f64>,
    /// Compact swing state: `round(2 * score)`, an integer in `[-2, +2]`, or
    /// `None` when the score is unavailable.
    pub state_code: Option<i8>,
    /// Pivot scale the swing fields were computed at: 1 = minor, 2 = major.
    pub scale_code: u8,
    /// `(Close - EMA 20) / ATR 14`, or `None` when an input is unavailable or
    /// ATR 14 is not a valid denominator.
    pub ema_distance_atr_20: Option<f64>,
    /// `(Close - EMA 50) / ATR 14`, or `None` when an input is unavailable or
    /// ATR 14 is not a valid denominator.
    pub ema_distance_atr_50: Option<f64>,
    /// `(Close - EMA 200) / ATR 14`, or `None` when an input is unavailable
    /// or ATR 14 is not a valid denominator.
    pub ema_distance_atr_200: Option<f64>,
    /// Close/EMA 20 cross: +1 when the close crossed from at or below to
    /// above, -1 from at or above to below, 0 when no cross occurred, or
    /// `None` when a required value is unavailable.
    pub ema_cross_code_20: Option<i8>,
    /// Close/EMA 50 cross, with the same codes as [`Self::ema_cross_code_20`].
    pub ema_cross_code_50: Option<i8>,
    /// Close/EMA 200 cross, with the same codes as [`Self::ema_cross_code_20`].
    pub ema_cross_code_200: Option<i8>,
    /// EMA 20 zone retest: +1 when the zone was tested and the close finished
    /// above the EMA, -1 when it finished below, 0 when no qualifying test
    /// occurred, or `None` when an input is unavailable or ATR 14 is invalid.
    pub ema_retest_code_20: Option<i8>,
    /// EMA 50 zone retest, with the same codes as
    /// [`Self::ema_retest_code_20`].
    pub ema_retest_code_50: Option<i8>,
    /// EMA 200 zone retest, with the same codes as
    /// [`Self::ema_retest_code_20`].
    pub ema_retest_code_200: Option<i8>,
    /// Empirical percentile of the absolute score magnitude, in `0..=100`, or
    /// `None` when fewer than 50 valid prior scores exist.
    pub size_percentile: Option<f64>,
    /// Mechanical completeness: unavailable when the score is null,
    /// provisional until the percentile has 50 valid prior samples, ready
    /// thereafter. An auxiliary EMA null is not a readiness downgrade.
    pub readiness: Readiness,
    /// Why Structure is unavailable, or `None` when the score is available.
    pub unavailable_reason: Option<UnavailableReason>,
    /// Closed bars the compact swing state has held; zero before the first
    /// code.
    pub state_age_bars: u64,
    /// Compact swing state changes in the last 50 code-bearing closed bars.
    pub state_changes_50: u64,
}

/// The scale the Structure swing fields are computed at.
///
/// The default is [`PivotScale::Major`], because the major swing sequence is
/// the structural one. The registry always holds both scales; the configuration
/// applies to the per-bar swing fields.
///
/// # Examples
///
/// ```
/// use quantrule_model::axes::structure::StructureConfig;
/// use quantrule_model::readings::PivotScale;
///
/// assert_eq!(StructureConfig::default().scale, PivotScale::Major);
/// assert_eq!(StructureConfig::new(PivotScale::Minor).scale, PivotScale::Minor);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructureConfig {
    /// The pivot scale used for the swing relations and score.
    pub scale: PivotScale,
}

impl StructureConfig {
    /// Builds a configuration for `scale`.
    #[must_use]
    pub const fn new(scale: PivotScale) -> Self {
        Self { scale }
    }
}

impl Default for StructureConfig {
    fn default() -> Self {
        Self {
            scale: PivotScale::Major,
        }
    }
}

/// The per-bar Structure series and its confirmed-level registry.
///
/// The series has exactly one [`StructureBar`] per input bar of the readings.
/// Its length always equals the readings length, so a consumer indexes it by
/// bar position. The registry holds every confirmed pivot at both scales,
/// independent of the configured swing scale.
///
/// # Examples
///
/// ```
/// use quantrule_data::Candle;
/// use quantrule_model::axes::structure::StructureSeries;
/// use quantrule_model::kernel::{Readiness, UnavailableReason};
/// use quantrule_model::readings::Readings;
///
/// let candle = Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0))
///     .expect("the example candle is valid");
/// let readings = Readings::compute(&[candle]);
/// let structure = StructureSeries::compute(&[candle], &readings);
///
/// assert_eq!(structure.len(), 1);
/// let bar = structure.get(0).expect("one bar");
/// assert_eq!(bar.score, None);
/// assert_eq!(bar.scale_code, 2);
/// assert_eq!(bar.readiness, Readiness::Unavailable);
/// assert_eq!(
///     bar.unavailable_reason,
///     Some(UnavailableReason::InsufficientConfirmedPivots),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct StructureSeries {
    bars: Vec<StructureBar>,
    registry: ConfirmedLevelRegistry,
}

impl StructureSeries {
    /// Computes Structure with the default major scale.
    ///
    /// `candles` supplies the close, high, and low needed by the EMA features;
    /// `readings` supplies the EMA, ATR, and confirmed pivots. The two are
    /// expected to describe the same series. A missing candle at a bar makes
    /// that bar's EMA features null rather than panicking.
    #[must_use]
    pub fn compute(candles: &[Candle], readings: &Readings) -> Self {
        Self::compute_with_config(candles, readings, StructureConfig::default())
    }

    /// Computes Structure with an explicit configuration.
    ///
    /// The result is aligned to `readings.len()`. Each bar is a pure function
    /// of the readings knowable at that bar, the candle at that bar, and the
    /// valid score magnitudes strictly before it; no later bar is read.
    #[must_use]
    pub fn compute_with_config(
        candles: &[Candle],
        readings: &Readings,
        config: StructureConfig,
    ) -> Self {
        let registry = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
        let mut bars = Vec::with_capacity(readings.len());
        let mut counters = StateCounters::new();
        // Absolute valid score magnitudes in bar order. The current bar is
        // appended only after its percentile is computed, so a bar never
        // samples itself. The kernel reads at most 252 samples, so the buffer
        // is bounded rather than sized to the input length.
        let mut magnitudes = Vec::with_capacity(PERCENTILE_WINDOW);
        let mut latest_high: Option<Pivot> = None;
        let mut previous_high: Option<Pivot> = None;
        let mut latest_low: Option<Pivot> = None;
        let mut previous_low: Option<Pivot> = None;

        for bar in 0..readings.len() {
            // A pivot is usable on the bar it becomes knowable, never earlier.
            if let Some(pivot) = readings.pivots.confirmed_at(config.scale, bar) {
                match pivot.kind {
                    PivotKind::High => {
                        previous_high = latest_high;
                        latest_high = Some(pivot);
                    }
                    PivotKind::Low => {
                        previous_low = latest_low;
                        latest_low = Some(pivot);
                    }
                }
            }

            let high_relation_code = relation(latest_high, previous_high);
            let low_relation_code = relation(latest_low, previous_low);
            let score = structure_score(high_relation_code, low_relation_code);
            let features = EmaFeatures::compute(candles, readings, bar);

            match score {
                Some(score) => {
                    let magnitude = score.abs();
                    let size_percentile = empirical_percentile(&magnitudes, magnitude);
                    let state_code = structure_code(score);
                    let stats = counters.observe(state_code);
                    let readiness = if size_percentile.is_some() {
                        Readiness::Ready
                    } else {
                        Readiness::Provisional
                    };

                    bars.push(StructureBar {
                        high_relation_code,
                        low_relation_code,
                        score: Some(score),
                        state_code,
                        scale_code: config.scale.code(),
                        ema_distance_atr_20: features.distance_atr[0],
                        ema_distance_atr_50: features.distance_atr[1],
                        ema_distance_atr_200: features.distance_atr[2],
                        ema_cross_code_20: features.cross_code[0],
                        ema_cross_code_50: features.cross_code[1],
                        ema_cross_code_200: features.cross_code[2],
                        ema_retest_code_20: features.retest_code[0],
                        ema_retest_code_50: features.retest_code[1],
                        ema_retest_code_200: features.retest_code[2],
                        size_percentile,
                        readiness,
                        unavailable_reason: None,
                        state_age_bars: stats.age_bars,
                        state_changes_50: stats.changes_50,
                    });

                    if magnitudes.len() == PERCENTILE_WINDOW {
                        magnitudes.remove(0);
                    }
                    magnitudes.push(magnitude);
                }
                None => {
                    let stats = counters.observe(None);
                    bars.push(StructureBar {
                        high_relation_code,
                        low_relation_code,
                        score: None,
                        state_code: None,
                        scale_code: config.scale.code(),
                        ema_distance_atr_20: features.distance_atr[0],
                        ema_distance_atr_50: features.distance_atr[1],
                        ema_distance_atr_200: features.distance_atr[2],
                        ema_cross_code_20: features.cross_code[0],
                        ema_cross_code_50: features.cross_code[1],
                        ema_cross_code_200: features.cross_code[2],
                        ema_retest_code_20: features.retest_code[0],
                        ema_retest_code_50: features.retest_code[1],
                        ema_retest_code_200: features.retest_code[2],
                        size_percentile: None,
                        readiness: Readiness::Unavailable,
                        // The missing input is a confirmed pivot, not a bar,
                        // so an incomplete pair always reports reason 2.
                        unavailable_reason: Some(UnavailableReason::InsufficientConfirmedPivots),
                        state_age_bars: stats.age_bars,
                        state_changes_50: stats.changes_50,
                    });
                }
            }
        }

        Self { bars, registry }
    }

    /// The number of bars in the series.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Whether the series has no bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }

    /// The record at `bar`, or `None` when the bar is out of range.
    #[must_use]
    pub fn get(&self, bar: usize) -> Option<StructureBar> {
        self.bars.get(bar).copied()
    }

    /// Every bar record, in bar order.
    #[must_use]
    pub fn bars(&self) -> &[StructureBar] {
        &self.bars
    }

    /// The latest bar record, or `None` for an empty series.
    #[must_use]
    pub fn latest(&self) -> Option<StructureBar> {
        self.bars.last().copied()
    }

    /// The confirmed-level registry built from both pivot scales.
    #[must_use]
    pub fn registry(&self) -> &ConfirmedLevelRegistry {
        &self.registry
    }
}

/// The relation of the newer pivot to the older one.
fn relation(newer: Option<Pivot>, older: Option<Pivot>) -> Option<i8> {
    let (newer, older) = (newer?, older?);
    match newer.price.partial_cmp(&older.price)? {
        Ordering::Greater => Some(1),
        Ordering::Equal => Some(0),
        Ordering::Less => Some(-1),
    }
}

/// The contract's score for a complete relation pair.
fn structure_score(high_relation_code: Option<i8>, low_relation_code: Option<i8>) -> Option<f64> {
    let high = high_relation_code?;
    let low = low_relation_code?;
    Some(f64::from(high + low) / 2.0)
}

/// The contract's compact swing code for a score.
fn structure_code(score: f64) -> Option<i8> {
    let doubled = 2.0 * score;
    if !doubled.is_finite() {
        return None;
    }
    Some(doubled.round() as i8)
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;

    use super::{StructureConfig, StructureSeries, relation, structure_code, structure_score};
    use crate::kernel::{PERCENTILE_WINDOW, Readiness, UnavailableReason};
    use crate::readings::{Pivot, PivotKind, PivotScale, PivotSeries, ReadingSeries, Readings};

    fn assert_close(actual: Option<f64>, expected: Option<f64>) {
        match (actual, expected) {
            (None, None) => {}
            (Some(actual), Some(expected)) => assert!(
                (actual - expected).abs() < 1e-12,
                "expected {expected}, got {actual}"
            ),
            _ => panic!("expected {expected:?}, got {actual:?}"),
        }
    }

    fn pivot(kind: PivotKind, price: f64, occurred_at: usize) -> Pivot {
        Pivot {
            scale: PivotScale::Major,
            kind,
            price,
            occurred_at,
            available_at: occurred_at + PivotScale::Major.lookback(),
        }
    }

    /// Candles from a piecewise-linear mid-price path.
    ///
    /// A path whose turning points are at least five bars apart produces one
    /// major pivot per turn. The high and low are half a point either side of
    /// the mid, so a turn is strict against its neighbours.
    fn zigzag_candles(points: &[(usize, f64)]) -> Vec<Candle> {
        let last_bar = points.last().expect("the path has a last point").0;
        let mut mids = vec![0.0; last_bar + 1];

        for pair in points.windows(2) {
            let (start_bar, start_mid) = pair[0];
            let (end_bar, end_mid) = pair[1];
            let span = (end_bar - start_bar) as f64;
            for (bar, mid) in mids
                .iter_mut()
                .enumerate()
                .take(end_bar + 1)
                .skip(start_bar)
            {
                let offset = (bar - start_bar) as f64;
                *mid = start_mid + (end_mid - start_mid) * offset / span;
            }
        }

        mids.iter()
            .enumerate()
            .map(|(bar, mid)| {
                Candle::new(
                    bar as u64,
                    bar as i64,
                    *mid,
                    mid + 0.5,
                    mid - 0.5,
                    *mid,
                    Some(10.0),
                )
                .expect("the test candle is valid")
            })
            .collect()
    }

    /// A major-scale path with every relation code and every compact code.
    ///
    /// Turning points: 106 (bar 5), 100 (12), 108 (19), 100 (26), 104 (33),
    /// 98 (40), 106 (47), 100 (54), 106 (61), 100 (68), then a rise to 104
    /// (73). The flat tail used by the percentile test is appended separately.
    fn relation_path() -> Vec<Candle> {
        zigzag_candles(&[
            (0, 101.0),
            (5, 106.0),
            (12, 100.0),
            (19, 108.0),
            (26, 100.0),
            (33, 104.0),
            (40, 98.0),
            (47, 106.0),
            (54, 100.0),
            (61, 106.0),
            (68, 100.0),
            (73, 104.0),
        ])
    }

    fn flat_tail(candles: &[Candle], count: usize) -> Vec<Candle> {
        let mut extended = candles.to_vec();
        let start = extended.len();
        for bar in start..start + count {
            extended.push(
                Candle::new(
                    bar as u64,
                    bar as i64,
                    104.0,
                    104.5,
                    103.5,
                    104.0,
                    Some(10.0),
                )
                .expect("the flat tail candle is valid"),
            );
        }
        extended
    }

    fn series(candles: &[Candle]) -> StructureSeries {
        let readings = Readings::compute(candles);
        StructureSeries::compute(candles, &readings)
    }

    #[test]
    fn relation_codes_compare_the_latest_two_prices() {
        let high = |price| pivot(PivotKind::High, price, 0);
        assert_eq!(relation(Some(high(2.0)), Some(high(1.0))), Some(1));
        assert_eq!(relation(Some(high(1.0)), Some(high(1.0))), Some(0));
        assert_eq!(relation(Some(high(1.0)), Some(high(2.0))), Some(-1));
        assert_eq!(relation(Some(high(1.0)), None), None);
        assert_eq!(relation(None, Some(high(1.0))), None);
    }

    #[test]
    fn score_and_code_follow_the_contract_table() {
        let cases = [
            (1, 1, 1.0, 2),
            (1, 0, 0.5, 1),
            (1, -1, 0.0, 0),
            (0, 1, 0.5, 1),
            (0, 0, 0.0, 0),
            (0, -1, -0.5, -1),
            (-1, 1, 0.0, 0),
            (-1, 0, -0.5, -1),
            (-1, -1, -1.0, -2),
        ];

        for (high, low, expected_score, expected_code) in cases {
            let score = structure_score(Some(high), Some(low));
            assert_eq!(score, Some(expected_score));
            assert_eq!(structure_code(expected_score), Some(expected_code));
        }

        assert_eq!(structure_score(Some(1), None), None);
        assert_eq!(structure_score(None, Some(1)), None);
        assert_eq!(structure_score(None, None), None);
    }

    #[test]
    fn the_reference_path_covers_every_relation_and_compact_code() {
        let series = series(&relation_path());

        // Bar 30: the second low has not confirmed yet, so the pair is
        // incomplete but the high relation is already available.
        let bar = series.get(30).expect("bar 30");
        assert_eq!(bar.high_relation_code, Some(1));
        assert_eq!(bar.low_relation_code, None);
        assert_eq!(bar.score, None);
        assert_eq!(bar.state_code, None);
        assert_eq!(bar.readiness, Readiness::Unavailable);
        assert_eq!(
            bar.unavailable_reason,
            Some(UnavailableReason::InsufficientConfirmedPivots)
        );
        assert_eq!(bar.state_age_bars, 0);
        assert_eq!(bar.state_changes_50, 0);

        // Bar 31: the second low confirms, giving a higher high and an equal
        // low: score +0.5 and code +1.
        let bar = series.get(31).expect("bar 31");
        assert_eq!(bar.high_relation_code, Some(1));
        assert_eq!(bar.low_relation_code, Some(0));
        assert_close(bar.score, Some(0.5));
        assert_eq!(bar.state_code, Some(1));
        assert_eq!(bar.readiness, Readiness::Provisional);
        assert_eq!(bar.unavailable_reason, None);
        assert_eq!(bar.state_age_bars, 1);
        assert_eq!(bar.state_changes_50, 0);

        // Bar 38: lower high, equal low: -0.5 and -1.
        let bar = series.get(38).expect("bar 38");
        assert_eq!(bar.high_relation_code, Some(-1));
        assert_eq!(bar.low_relation_code, Some(0));
        assert_close(bar.score, Some(-0.5));
        assert_eq!(bar.state_code, Some(-1));
        assert_eq!(bar.state_age_bars, 1);
        assert_eq!(bar.state_changes_50, 1);

        // Bar 45: lower high and lower low: -1 and -2.
        let bar = series.get(45).expect("bar 45");
        assert_eq!(bar.high_relation_code, Some(-1));
        assert_eq!(bar.low_relation_code, Some(-1));
        assert_close(bar.score, Some(-1.0));
        assert_eq!(bar.state_code, Some(-2));
        assert_eq!(bar.state_changes_50, 2);

        // Bar 52: higher high and lower low: offsetting relations, score 0.
        let bar = series.get(52).expect("bar 52");
        assert_eq!(bar.high_relation_code, Some(1));
        assert_eq!(bar.low_relation_code, Some(-1));
        assert_close(bar.score, Some(0.0));
        assert_eq!(bar.state_code, Some(0));
        assert_eq!(bar.state_changes_50, 3);

        // Bar 59: higher high and higher low: +1 and +2.
        let bar = series.get(59).expect("bar 59");
        assert_eq!(bar.high_relation_code, Some(1));
        assert_eq!(bar.low_relation_code, Some(1));
        assert_close(bar.score, Some(1.0));
        assert_eq!(bar.state_code, Some(2));
        assert_eq!(bar.state_changes_50, 4);

        // Bar 66: equal high and higher low: +0.5 and +1.
        let bar = series.get(66).expect("bar 66");
        assert_eq!(bar.high_relation_code, Some(0));
        assert_eq!(bar.low_relation_code, Some(1));
        assert_close(bar.score, Some(0.5));
        assert_eq!(bar.state_code, Some(1));
        assert_eq!(bar.state_changes_50, 5);

        // Bar 73: equal high and equal low: score 0 and code 0.
        let bar = series.get(73).expect("bar 73");
        assert_eq!(bar.high_relation_code, Some(0));
        assert_eq!(bar.low_relation_code, Some(0));
        assert_close(bar.score, Some(0.0));
        assert_eq!(bar.state_code, Some(0));
        assert_eq!(bar.state_changes_50, 6);
    }

    #[test]
    fn the_scale_code_defaults_to_major_and_can_be_minor() {
        let candles = relation_path();
        let readings = Readings::compute(&candles);

        let default = StructureSeries::compute(&candles, &readings);
        assert_eq!(default.get(0).expect("bar 0").scale_code, 2);
        assert_eq!(
            default,
            StructureSeries::compute_with_config(
                &candles,
                &readings,
                StructureConfig::new(PivotScale::Major),
            )
        );

        let minor = StructureSeries::compute_with_config(
            &candles,
            &readings,
            StructureConfig::new(PivotScale::Minor),
        );
        assert_eq!(minor.get(0).expect("bar 0").scale_code, 1);

        // The minor scale confirms the same turns two bars earlier, so its
        // first complete pair exists before the major scale's does.
        assert!(minor.get(28).expect("bar 28").score.is_some());
        assert!(default.get(28).expect("bar 28").score.is_none());
        assert_eq!(minor.get(28).expect("bar 28").state_code, Some(1));
    }

    #[test]
    fn readiness_reaches_ready_after_fifty_score_samples() {
        let candles = flat_tail(&relation_path(), 20);
        let series = series(&candles);

        // The first valid score is at bar 31, so bar 80 has 49 prior samples
        // and bar 81 has 50.
        let provisional = series.get(80).expect("bar 80");
        assert_eq!(provisional.readiness, Readiness::Provisional);
        assert_eq!(provisional.size_percentile, None);

        let ready = series.get(81).expect("bar 81");
        assert_eq!(ready.readiness, Readiness::Ready);
        let percentile = ready.size_percentile.expect("50 prior samples");
        assert!((0.0..=100.0).contains(&percentile));
    }

    #[test]
    fn state_counters_follow_the_compact_code_and_freeze_on_null() {
        let series = series(&relation_path());

        // A same-code run: bar 31 is the first code 1 and bar 32 holds it.
        assert_eq!(series.get(31).expect("bar 31").state_age_bars, 1);
        assert_eq!(series.get(32).expect("bar 32").state_age_bars, 2);
        assert_eq!(series.get(32).expect("bar 32").state_changes_50, 0);

        // The first score is at bar 31, so the null prefix freezes at zero.
        assert_eq!(series.get(30).expect("bar 30").state_age_bars, 0);
        assert_eq!(series.get(30).expect("bar 30").state_changes_50, 0);
    }

    #[test]
    fn the_percentile_uses_only_the_most_recent_252_scores() {
        // The reference path already has a valid score from bar 31. Append a
        // long, varied zigzag so the last bar has more than 252 valid prior
        // scores and the early scores fall outside the window.
        let mut points = vec![
            (0, 101.0),
            (5, 106.0),
            (12, 100.0),
            (19, 108.0),
            (26, 100.0),
            (33, 104.0),
            (40, 98.0),
            (47, 106.0),
            (54, 100.0),
            (61, 106.0),
            (68, 100.0),
            (73, 104.0),
        ];
        let peaks = [
            110.0, 108.0, 112.0, 106.0, 111.0, 109.0, 113.0, 107.0, 110.0, 108.0, 112.0, 106.0,
            111.0, 109.0, 113.0, 107.0,
        ];
        let troughs = [
            104.0, 102.0, 106.0, 100.0, 105.0, 103.0, 107.0, 101.0, 104.0, 102.0, 106.0, 100.0,
            105.0, 103.0, 107.0, 101.0,
        ];
        let mut bar = 73;
        for (peak, trough) in peaks.into_iter().zip(troughs) {
            bar += 7;
            points.push((bar, peak));
            bar += 7;
            points.push((bar, trough));
        }

        let candles = zigzag_candles(&points);
        let series = series(&candles);
        assert!(series.len() > 284);

        let mut cap_exercised = false;
        for bar in 31..series.len() {
            let Some(current) = series.get(bar).and_then(|record| record.score) else {
                continue;
            };
            let prior: Vec<f64> = series.bars()[..bar]
                .iter()
                .filter_map(|record| record.score)
                .map(f64::abs)
                .collect();
            if prior.len() <= PERCENTILE_WINDOW {
                continue;
            }

            let capped = &prior[prior.len() - PERCENTILE_WINDOW..];
            let expected_capped = 100.0
                * capped
                    .iter()
                    .filter(|sample| **sample <= current.abs())
                    .count() as f64
                / capped.len() as f64;
            let expected_full = 100.0
                * prior
                    .iter()
                    .filter(|sample| **sample <= current.abs())
                    .count() as f64
                / prior.len() as f64;

            // The test is only meaningful where dropping the oldest samples
            // changes the answer.
            if (expected_capped - expected_full).abs() > 1e-12 {
                cap_exercised = true;
                assert_eq!(
                    series.get(bar).expect("a bar").size_percentile,
                    Some(expected_capped),
                    "bar {bar} must use the most recent {PERCENTILE_WINDOW} samples"
                );
            }
        }
        assert!(cap_exercised, "the series must exercise the 252-sample cap");
    }

    #[test]
    fn ema_features_never_enter_the_swing_fields() {
        let candles = relation_path();
        let readings = Readings::compute(&candles);
        let baseline = StructureSeries::compute(&candles, &readings);

        let mut changed = readings.clone();
        changed.ema20 = ReadingSeries::new(vec![Some(1.0); changed.len()]);
        changed.ema50 = ReadingSeries::new(vec![Some(2.0); changed.len()]);
        changed.ema200 = ReadingSeries::new(vec![Some(3.0); changed.len()]);
        changed.atr14 = ReadingSeries::new(vec![Some(4.0); changed.len()]);

        let mutated = StructureSeries::compute(&candles, &changed);
        assert_eq!(baseline.registry(), mutated.registry());

        for bar in 0..baseline.len() {
            let before = baseline.get(bar).expect("a bar");
            let after = mutated.get(bar).expect("a bar");
            assert_eq!(before.high_relation_code, after.high_relation_code);
            assert_eq!(before.low_relation_code, after.low_relation_code);
            assert_eq!(before.score, after.score);
            assert_eq!(before.state_code, after.state_code);
            assert_eq!(before.scale_code, after.scale_code);
            assert_eq!(before.size_percentile, after.size_percentile);
            assert_eq!(before.readiness, after.readiness);
            assert_eq!(before.unavailable_reason, after.unavailable_reason);
            assert_eq!(before.state_age_bars, after.state_age_bars);
            assert_eq!(before.state_changes_50, after.state_changes_50);
        }
    }

    #[test]
    fn changing_the_pivots_does_not_change_the_ema_features() {
        let candles = relation_path();
        let readings = Readings::compute(&candles);
        let baseline = StructureSeries::compute(&candles, &readings);

        let mut changed = readings.clone();
        changed.pivots = PivotSeries::compute(&zigzag_candles(&[(0, 50.0), (6, 60.0), (12, 50.0)]));

        let mutated = StructureSeries::compute(&candles, &changed);
        for bar in 0..baseline.len() {
            let before = baseline.get(bar).expect("a bar");
            let after = mutated.get(bar).expect("a bar");
            assert_eq!(before.ema_distance_atr_20, after.ema_distance_atr_20);
            assert_eq!(before.ema_distance_atr_50, after.ema_distance_atr_50);
            assert_eq!(before.ema_distance_atr_200, after.ema_distance_atr_200);
            assert_eq!(before.ema_cross_code_20, after.ema_cross_code_20);
            assert_eq!(before.ema_cross_code_50, after.ema_cross_code_50);
            assert_eq!(before.ema_cross_code_200, after.ema_cross_code_200);
            assert_eq!(before.ema_retest_code_20, after.ema_retest_code_20);
            assert_eq!(before.ema_retest_code_50, after.ema_retest_code_50);
            assert_eq!(before.ema_retest_code_200, after.ema_retest_code_200);
        }
    }

    #[test]
    fn changing_rsi_and_rvol_does_not_change_structure() {
        let candles = relation_path();
        let readings = Readings::compute(&candles);
        let baseline = StructureSeries::compute(&candles, &readings);

        let mut changed = readings.clone();
        changed.rsi14 = ReadingSeries::new(vec![Some(1.0); changed.len()]);
        changed.rvol20 = ReadingSeries::new(vec![Some(2.0); changed.len()]);

        assert_eq!(baseline, StructureSeries::compute(&candles, &changed));
    }

    #[test]
    fn a_missing_candle_series_nulls_only_the_ema_features() {
        let candles = relation_path();
        let readings = Readings::compute(&candles);
        let series = StructureSeries::compute(&[], &readings);

        let bar = series.get(59).expect("bar 59");
        assert_close(bar.score, Some(1.0));
        assert_eq!(bar.state_code, Some(2));
        assert_eq!(bar.ema_distance_atr_20, None);
        assert_eq!(bar.ema_cross_code_20, None);
        assert_eq!(bar.ema_retest_code_20, None);
    }

    #[test]
    fn an_empty_series_has_no_bars() {
        let readings = Readings::compute(&[]);
        let series = StructureSeries::compute(&[], &readings);

        assert!(series.is_empty());
        assert_eq!(series.len(), 0);
        assert_eq!(series.get(0), None);
        assert_eq!(series.latest(), None);
        assert!(series.registry().is_empty());
    }

    #[test]
    fn a_single_candle_is_unavailable() {
        let candle =
            Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(10.0)).expect("the test candle is valid");
        let readings = Readings::compute(&[candle]);
        let series = StructureSeries::compute(&[candle], &readings);
        let bar = series.latest().expect("one bar");

        assert_eq!(bar.score, None);
        assert_eq!(bar.state_code, None);
        assert_eq!(bar.scale_code, 2);
        assert_eq!(bar.readiness, Readiness::Unavailable);
        assert_eq!(
            bar.unavailable_reason,
            Some(UnavailableReason::InsufficientConfirmedPivots)
        );
        assert_eq!(bar.state_age_bars, 0);
        assert_eq!(bar.state_changes_50, 0);
    }

    #[test]
    fn the_series_length_matches_the_readings_length() {
        let candles = relation_path();
        let readings = Readings::compute(&candles);
        let series = StructureSeries::compute(&candles, &readings);

        assert_eq!(series.len(), readings.len());
        assert_eq!(series.bars().len(), readings.len());
    }
}
