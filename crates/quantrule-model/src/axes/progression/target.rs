//! Target selection for a progression leg.
//!
//! A leg's target is the nearest confirmed structural level of the same scale
//! beyond its origin, or the ATR fallback when no such level is knowable. The
//! module reads only the confirmed-level registry built from the pivot
//! readings; it never reads another axis's output.
//!
//! The registry is append-only, so only a nearer structural level can revise a
//! target as new levels confirm. A fallback target is replaced by the first
//! structural level that qualifies, even when that level is farther away,
//! because §8.4 gives a structural level precedence over the fallback.

use crate::axes::structure::registry::ConfirmedLevelRegistry;
use crate::readings::{PivotKind, PivotScale};

/// Contract §14 code for a confirmed structural level.
pub(crate) const STRUCTURAL_BASIS: u8 = 1;

/// Contract §14 code for the ATR fallback.
pub(crate) const ATR_FALLBACK_BASIS: u8 = 2;

/// A selected target price and the basis it came from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Target {
    /// The target price.
    pub(crate) price: f64,
    /// The contract's target basis code: 1 for a level, 2 for the fallback.
    pub(crate) basis_code: u8,
}

/// Selects the target for a leg at `bar`.
///
/// `direction` is +1 for an upward leg and -1 for a downward leg. The
/// structural candidates are the confirmed levels of `scale` knowable at
/// `bar`: highs strictly above the origin for an upward leg, lows strictly
/// below it for a downward leg. An exact price tie goes to the earliest
/// confirmed level. The fallback requires a usable `atr_at_origin`; without
/// one, and without a structural level, the leg has no target.
pub(crate) fn select(
    registry: &ConfirmedLevelRegistry,
    bar: usize,
    scale: PivotScale,
    direction: i8,
    origin_price: f64,
    atr_at_origin: Option<f64>,
) -> Option<Target> {
    if let Some(price) = nearest_level(registry, bar, scale, direction, origin_price) {
        return Some(Target {
            price,
            basis_code: STRUCTURAL_BASIS,
        });
    }

    let atr = atr_at_origin?;
    let price = origin_price + f64::from(direction) * 2.0 * atr;
    if !price.is_finite() {
        return None;
    }
    Some(Target {
        price,
        basis_code: ATR_FALLBACK_BASIS,
    })
}

/// The nearest confirmed level beyond `origin_price` in `direction`.
///
/// Upward legs target highs above the origin, so the nearest is the lowest
/// qualifying price. Downward legs target lows below it, so the nearest is the
/// highest qualifying price. The strict comparison keeps the earliest
/// confirmed level when two levels share a price.
fn nearest_level(
    registry: &ConfirmedLevelRegistry,
    bar: usize,
    scale: PivotScale,
    direction: i8,
    origin_price: f64,
) -> Option<f64> {
    let kind = if direction > 0 {
        PivotKind::High
    } else {
        PivotKind::Low
    };
    let mut best: Option<f64> = None;

    for level in registry.as_of(bar) {
        if level.scale != scale || level.kind != kind || !level.price.is_finite() {
            continue;
        }

        let qualifies = if direction > 0 {
            level.price > origin_price
        } else {
            level.price < origin_price
        };
        if !qualifies {
            continue;
        }

        let nearer = match best {
            None => true,
            Some(price) if direction > 0 => level.price < price,
            Some(price) => level.price > price,
        };
        if nearer {
            best = Some(level.price);
        }
    }

    best
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;

    use super::{ATR_FALLBACK_BASIS, STRUCTURAL_BASIS, Target, nearest_level, select};
    use crate::axes::structure::registry::ConfirmedLevelRegistry;
    use crate::readings::{PivotScale, PivotSeries};

    fn candle(high: f64, low: f64) -> Candle {
        Candle::new(
            0,
            0,
            (high + low) / 2.0,
            high,
            low,
            (high + low) / 2.0,
            None,
        )
        .expect("the test candle is valid")
    }

    /// A major high at bar 5 and a major low at bar 11, confirming at 10 and
    /// 16. Both are strict against five bars on each side.
    fn level_candles() -> Vec<Candle> {
        let mids = [
            100.0, 101.0, 102.0, 103.0, 104.0, 110.0, 104.0, 103.0, 102.0, 101.0, 100.0, 90.0,
            100.0, 101.0, 102.0, 103.0, 104.0,
        ];
        mids.iter()
            .map(|mid| candle(mid + 0.5, mid - 0.5))
            .collect()
    }

    fn registry() -> ConfirmedLevelRegistry {
        ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&level_candles()))
    }

    #[test]
    fn the_nearest_high_above_the_origin_is_the_up_target() {
        let registry = registry();
        let target = select(&registry, 10, PivotScale::Major, 1, 100.0, None);
        assert_eq!(
            target,
            Some(Target {
                price: 110.5,
                basis_code: STRUCTURAL_BASIS,
            })
        );
    }

    #[test]
    fn the_nearest_low_below_the_origin_is_the_down_target() {
        let registry = registry();
        let target = select(&registry, 16, PivotScale::Major, -1, 105.0, None);
        assert_eq!(
            target,
            Some(Target {
                price: 89.5,
                basis_code: STRUCTURAL_BASIS,
            })
        );
    }

    #[test]
    fn a_level_before_its_availability_bar_is_not_used() {
        let registry = registry();
        assert_eq!(
            nearest_level(&registry, 9, PivotScale::Major, 1, 100.0),
            None
        );
        assert_eq!(
            nearest_level(&registry, 10, PivotScale::Major, 1, 100.0),
            Some(110.5)
        );
    }

    #[test]
    fn a_level_at_the_origin_is_not_beyond_it() {
        let registry = registry();
        assert_eq!(
            nearest_level(&registry, 10, PivotScale::Major, 1, 110.5),
            None
        );
        assert_eq!(
            nearest_level(&registry, 16, PivotScale::Major, -1, 89.5),
            None
        );
    }

    #[test]
    fn the_fallback_uses_two_atr_in_the_leg_direction() {
        let registry = ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&[]));
        assert_eq!(
            select(&registry, 0, PivotScale::Minor, 1, 100.0, Some(2.0)),
            Some(Target {
                price: 104.0,
                basis_code: ATR_FALLBACK_BASIS,
            })
        );
        assert_eq!(
            select(&registry, 0, PivotScale::Minor, -1, 100.0, Some(2.0)),
            Some(Target {
                price: 96.0,
                basis_code: ATR_FALLBACK_BASIS,
            })
        );
    }

    #[test]
    fn no_level_and_no_atr_means_no_target() {
        let registry = ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&[]));
        assert_eq!(
            select(&registry, 0, PivotScale::Minor, 1, 100.0, None),
            None
        );
    }

    #[test]
    fn a_structural_target_wins_over_the_fallback() {
        let registry = registry();
        let target = select(&registry, 10, PivotScale::Major, 1, 100.0, Some(1.0));
        assert_eq!(
            target,
            Some(Target {
                price: 110.5,
                basis_code: STRUCTURAL_BASIS,
            })
        );
    }

    #[test]
    fn a_farther_confirmed_high_does_not_replace_the_nearest() {
        // A nearer high at 110.5 confirms at 10 and a farther high at 120.5
        // confirms at 22. The nearest target is unchanged by the later,
        // farther level, so the leg's revision pass sees no new price.
        let mids = [
            100.0, 101.0, 102.0, 103.0, 104.0, 110.0, 104.0, 103.0, 102.0, 101.0, 100.0, 101.0,
            102.0, 103.0, 104.0, 105.0, 106.0, 120.0, 106.0, 105.0, 104.0, 103.0, 102.0, 101.0,
            100.0, 99.0, 98.0, 97.0,
        ];
        let candles: Vec<Candle> = mids
            .iter()
            .map(|mid| candle(mid + 0.5, mid - 0.5))
            .collect();
        let registry = ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&candles));

        assert_eq!(
            nearest_level(&registry, 10, PivotScale::Major, 1, 100.0),
            Some(110.5)
        );
        assert_eq!(
            nearest_level(&registry, 22, PivotScale::Major, 1, 100.0),
            Some(110.5)
        );
    }

    #[test]
    fn two_levels_at_the_same_price_yield_that_price() {
        // Two confirmed highs at the same price are the same target: the
        // record stores the price, not the level's identity, so the tie is
        // resolved to the same numeric target either way.
        let mids = [
            100.0, 101.0, 102.0, 103.0, 104.0, 110.0, 104.0, 103.0, 102.0, 101.0, 100.0, 101.0,
            102.0, 103.0, 104.0, 105.0, 106.0, 110.0, 106.0, 105.0, 104.0, 103.0, 102.0, 101.0,
        ];
        let candles: Vec<Candle> = mids
            .iter()
            .map(|mid| candle(mid + 0.5, mid - 0.5))
            .collect();
        let registry = ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&candles));

        assert_eq!(
            nearest_level(&registry, 22, PivotScale::Major, 1, 100.0),
            Some(110.5)
        );
    }

    #[test]
    fn an_overflowing_fallback_is_not_a_target() {
        let registry = ConfirmedLevelRegistry::from_pivots(&PivotSeries::compute(&[]));
        assert_eq!(
            select(&registry, 0, PivotScale::Minor, 1, f64::MAX, Some(f64::MAX)),
            None
        );
    }
}
