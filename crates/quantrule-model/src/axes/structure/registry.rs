//! The confirmed-level registry.
//!
//! The registry is the ordered record of every confirmed swing pivot at both
//! scales. The Structure axis publishes it for the Progression axis, which
//! needs same-scale structural targets, and for the Level events, which select
//! their bounded reference set from it. The registry itself does not select
//! that set; it holds the full confirmed history.
//!
//! A level is appended once, at the bar on which its pivot became knowable, so
//! an earlier level is never renumbered by a later confirmation. The module is
//! pure: it reads only the pivot series it is given.

use crate::readings::{PivotKind, PivotScale, PivotSeries};

/// The first confirmed-level reference code.
///
/// Contract §14 gives a confirmed swing level the code `1000 + n`, where `n` is
/// the level's 1-based sequence number in confirmation-bar order.
pub const FIRST_REFERENCE_CODE: u32 = 1001;

/// One confirmed swing level.
///
/// The level records both when the pivot occurred and when it became knowable,
/// so a consumer cannot use it before [`ConfirmedLevel::available_at`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfirmedLevel {
    /// The contract's Level-event reference code, starting at
    /// [`FIRST_REFERENCE_CODE`].
    pub reference_code: u32,
    /// The pivot scale.
    pub scale: PivotScale,
    /// Whether the level is a swing high or a swing low.
    pub kind: PivotKind,
    /// The swing high or low price.
    pub price: f64,
    /// The bar on which the extreme occurred.
    pub occurred_at: usize,
    /// The first bar on which the level became knowable.
    pub available_at: usize,
}

/// Every confirmed swing level, ordered by confirmation bar.
///
/// At one confirmation bar the minor scale precedes the major scale. Within one
/// scale at most one pivot confirms per bar, so that tie rule is the only one
/// the ordering needs.
///
/// # Examples
///
/// ```
/// use quantrule_model::axes::structure::registry::ConfirmedLevelRegistry;
/// use quantrule_model::readings::PivotSeries;
///
/// let registry = ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&[]));
///
/// assert!(registry.is_empty());
/// assert_eq!(registry.levels(), []);
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConfirmedLevelRegistry {
    levels: Vec<ConfirmedLevel>,
}

impl ConfirmedLevelRegistry {
    /// Builds the registry from both confirmed pivot scales.
    ///
    /// Levels are appended in confirmation-bar order. Reference codes are
    /// assigned in that order and never change afterwards.
    #[must_use]
    pub fn from_pivots(pivots: &PivotSeries) -> Self {
        let mut levels = Vec::with_capacity(
            pivots.confirmed_count(PivotScale::Minor) + pivots.confirmed_count(PivotScale::Major),
        );

        for bar in 0..pivots.len() {
            for scale in PivotScale::ALL {
                let Some(pivot) = pivots.confirmed_at(scale, bar) else {
                    continue;
                };
                let sequence = levels.len() as u32 + 1;
                levels.push(ConfirmedLevel {
                    reference_code: 1000 + sequence,
                    scale,
                    kind: pivot.kind,
                    price: pivot.price,
                    occurred_at: pivot.occurred_at,
                    available_at: pivot.available_at,
                });
            }
        }

        Self { levels }
    }

    /// Every level, in confirmation-bar order.
    #[must_use]
    pub fn levels(&self) -> &[ConfirmedLevel] {
        &self.levels
    }

    /// The number of levels.
    #[must_use]
    pub fn len(&self) -> usize {
        self.levels.len()
    }

    /// Whether the registry has no levels.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }

    /// The level at `position`, zero-based, or `None` when out of range.
    #[must_use]
    pub fn get(&self, position: usize) -> Option<ConfirmedLevel> {
        self.levels.get(position).copied()
    }

    /// Every level knowable at or before `bar`.
    ///
    /// A level whose `available_at` is later than `bar` is never yielded.
    pub fn as_of(&self, bar: usize) -> impl Iterator<Item = &ConfirmedLevel> + '_ {
        self.levels
            .iter()
            .take_while(move |level| level.available_at <= bar)
    }
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;

    use super::{ConfirmedLevel, ConfirmedLevelRegistry, FIRST_REFERENCE_CODE};
    use crate::readings::{PivotKind, PivotScale, PivotSeries};

    /// Candles with one major high at bar 5 and a minor low at bar 8; both
    /// confirm at bar 10, while the minor high at bar 5 confirms at bar 7.
    fn shared_confirmation_candles() -> Vec<Candle> {
        let mids = [
            100.0, 101.0, 102.0, 103.0, 104.0, 105.0, 104.0, 103.0, 102.0, 103.0, 104.0,
        ];
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

    #[test]
    fn levels_are_ordered_by_confirmation_bar_and_minor_comes_first() {
        let pivots = PivotSeries::compute(&shared_confirmation_candles());
        let registry = ConfirmedLevelRegistry::from_pivots(&pivots);

        assert_eq!(registry.len(), 3);
        assert_eq!(registry.levels()[0].reference_code, FIRST_REFERENCE_CODE);
        assert_eq!(registry.levels()[0].scale, PivotScale::Minor);
        assert_eq!(registry.levels()[0].kind, PivotKind::High);
        assert_eq!(registry.levels()[0].occurred_at, 5);
        assert_eq!(registry.levels()[0].available_at, 7);

        // The minor low and the major high confirm together at bar 10. The
        // minor scale must be appended first.
        assert_eq!(registry.levels()[1].reference_code, 1002);
        assert_eq!(registry.levels()[1].scale, PivotScale::Minor);
        assert_eq!(registry.levels()[1].kind, PivotKind::Low);
        assert_eq!(registry.levels()[1].occurred_at, 8);
        assert_eq!(registry.levels()[1].available_at, 10);

        assert_eq!(registry.levels()[2].reference_code, 1003);
        assert_eq!(registry.levels()[2].scale, PivotScale::Major);
        assert_eq!(registry.levels()[2].kind, PivotKind::High);
        assert_eq!(registry.levels()[2].occurred_at, 5);
        assert_eq!(registry.levels()[2].available_at, 10);
    }

    #[test]
    fn as_of_never_exposes_a_later_level() {
        let pivots = PivotSeries::compute(&shared_confirmation_candles());
        let registry = ConfirmedLevelRegistry::from_pivots(&pivots);

        assert_eq!(registry.as_of(6).count(), 0);
        assert_eq!(registry.as_of(7).count(), 1);
        assert_eq!(registry.as_of(9).count(), 1);
        assert_eq!(registry.as_of(10).count(), 3);
        assert_eq!(registry.as_of(usize::MAX).count(), 3);

        let codes: Vec<u32> = registry
            .as_of(10)
            .map(|level| level.reference_code)
            .collect();
        assert_eq!(codes, [1001, 1002, 1003]);
    }

    #[test]
    fn a_prefix_never_renumbers_an_earlier_level() {
        let candles = shared_confirmation_candles();
        let full = ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&candles));

        for length in 0..=candles.len() {
            let prefix =
                ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&candles[..length]));
            let expected = full.as_of(length.saturating_sub(1));
            let expected: Vec<ConfirmedLevel> = expected.copied().collect();
            assert_eq!(prefix.levels(), expected.as_slice());
        }
    }

    #[test]
    fn reference_codes_are_contiguous_from_one_thousand_and_one() {
        let candles = shared_confirmation_candles();
        let registry = ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&candles));

        for (index, level) in registry.levels().iter().enumerate() {
            assert_eq!(level.reference_code, FIRST_REFERENCE_CODE + index as u32);
        }
        assert_eq!(
            registry.get(0).map(|level| level.reference_code),
            Some(1001)
        );
        assert_eq!(registry.get(3), None);
    }

    #[test]
    fn an_empty_pivot_series_produces_an_empty_registry() {
        let registry = ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&[]));

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert_eq!(registry.get(0), None);
        assert_eq!(registry.as_of(0).count(), 0);
    }
}
