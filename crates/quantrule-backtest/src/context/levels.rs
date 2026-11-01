//! The as-of confirmed-level view for the execution timeframe.
//!
//! The view is built from the model's confirmed swing levels. It carries only
//! the nearest support and resistance, their availability, their scale and kind
//! codes, the distance from the execution close, and the retest/broken flags.
//! It never exposes the raw event stream, a future level, an indicator, or a
//! statistic. A level is usable only at or after its `available_at` bar.
//!
//! The view is a strategy context, not a model output. It is computed from the
//! already-published per-timeframe levels, so the model engine is unchanged.

use quantrule_data::Candle;
use quantrule_model::axes::structure::registry::ConfirmedLevel;
use quantrule_model::readings::{PivotKind, PivotScale};
use quantrule_model::timeframe::TimeframeModel;

/// One confirmed level as the strategy may see it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelView {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The confirmed swing price.
    pub price: f64,
    /// The bar on which the extreme occurred.
    pub occurred_at: usize,
    /// The first bar on which the level became knowable.
    pub available_at: usize,
    /// The contract scale code: 1 minor, 2 major.
    pub scale_code: u8,
    /// The kind code: 1 swing high, -1 swing low.
    pub kind_code: i8,
}

/// The nearest confirmed support and resistance at one execution bar.
///
/// `None` support or resistance means no confirmed level was available on that
/// side. A missing value is not replaced by zero.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LevelContext {
    /// The execution bar's close, when the context is complete.
    pub reference_price: Option<f64>,
    /// The nearest confirmed level at or below the execution close.
    pub support: Option<LevelView>,
    /// The nearest confirmed level at or above the execution close.
    pub resistance: Option<LevelView>,
    /// `(close - support.price) / close`, when a support exists.
    pub support_distance_fraction: Option<f64>,
    /// `(resistance.price - close) / close`, when a resistance exists.
    pub resistance_distance_fraction: Option<f64>,
    /// The execution bar's low reached the support and closed at or above it.
    pub retest_support: bool,
    /// The execution bar's high reached the resistance and closed at or below it.
    pub retest_resistance: bool,
    /// The execution close is below the nearest support.
    pub support_broken: bool,
    /// The execution close is above the nearest resistance.
    pub resistance_broken: bool,
    /// The nearest resistance the execution close crossed above on this bar.
    ///
    /// A breakout is knowable only after the execution close, so the level is
    /// reported from the current bar and never from a later one.
    pub broken_resistance: Option<LevelView>,
    /// The nearest support the execution close crossed below on this bar.
    pub broken_support: Option<LevelView>,
}

impl LevelContext {
    /// The unavailable view: no reference price and no levels.
    #[must_use]
    pub const fn unavailable() -> Self {
        Self {
            reference_price: None,
            support: None,
            resistance: None,
            support_distance_fraction: None,
            resistance_distance_fraction: None,
            retest_support: false,
            retest_resistance: false,
            support_broken: false,
            resistance_broken: false,
            broken_resistance: None,
            broken_support: None,
        }
    }

    /// Whether a support or resistance is available.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.support.is_some() || self.resistance.is_some()
    }
}

/// Computes the nearest levels from the selected context models.
///
/// `models[i]` and `pointers[i]` describe one context timeframe. A level is
/// considered only when `level.available_at <= context_bar_index`; future
/// levels are never read. Ties keep the faster context timeframe, then the
/// later confirmation, then the lower `reference_code` through the input order.
pub(crate) fn compute(
    models: &[Option<&TimeframeModel>],
    pointers: &[Option<usize>],
    candle: Candle,
    previous_close: Option<f64>,
    scale: Option<PivotScale>,
) -> LevelContext {
    let close = candle.close();
    if !close.is_finite() || close <= 0.0 {
        return LevelContext::unavailable();
    }

    let mut support: Option<LevelView> = None;
    let mut resistance: Option<LevelView> = None;
    let mut broken_resistance: Option<LevelView> = None;
    let mut broken_support: Option<LevelView> = None;
    for (model, pointer) in models.iter().zip(pointers) {
        let (Some(model), Some(bar_index)) = (model, pointer) else {
            continue;
        };
        for level in model.levels() {
            if level.available_at > *bar_index {
                continue;
            }
            if let Some(scale) = scale
                && level.scale != scale
            {
                continue;
            }
            let view = view_of(model.timeframe_secs(), level);
            if level.price <= close {
                support = nearer_support(support, view);
            }
            if level.price >= close {
                resistance = nearer_resistance(resistance, view);
            }
            if let Some(previous_close) = previous_close.filter(|value| value.is_finite()) {
                if previous_close <= level.price && close > level.price {
                    broken_resistance = nearer_support(broken_resistance, view);
                }
                if previous_close >= level.price && close < level.price {
                    broken_support = nearer_resistance(broken_support, view);
                }
            }
        }
    }

    let support_distance = support.map(|level| (close - level.price) / close);
    let resistance_distance = resistance.map(|level| (level.price - close) / close);
    LevelContext {
        reference_price: Some(close),
        support,
        resistance,
        support_distance_fraction: support_distance,
        resistance_distance_fraction: resistance_distance,
        retest_support: support
            .is_some_and(|level| candle.low() <= level.price && close >= level.price),
        retest_resistance: resistance
            .is_some_and(|level| candle.high() >= level.price && close <= level.price),
        support_broken: support.is_some_and(|level| close < level.price),
        resistance_broken: resistance.is_some_and(|level| close > level.price),
        broken_resistance,
        broken_support,
    }
}

fn view_of(timeframe_secs: u64, level: &ConfirmedLevel) -> LevelView {
    LevelView {
        timeframe_secs,
        price: level.price,
        occurred_at: level.occurred_at,
        available_at: level.available_at,
        scale_code: level.scale.code(),
        kind_code: match level.kind {
            PivotKind::High => 1,
            PivotKind::Low => -1,
        },
    }
}

fn nearer_support(current: Option<LevelView>, candidate: LevelView) -> Option<LevelView> {
    match current {
        None => Some(candidate),
        Some(current) if candidate.price > current.price => Some(candidate),
        Some(current)
            if candidate.price == current.price
                && candidate.available_at > current.available_at =>
        {
            Some(candidate)
        }
        Some(current) => Some(current),
    }
}

fn nearer_resistance(current: Option<LevelView>, candidate: LevelView) -> Option<LevelView> {
    match current {
        None => Some(candidate),
        Some(current) if candidate.price < current.price => Some(candidate),
        Some(current)
            if candidate.price == current.price
                && candidate.available_at > current.available_at =>
        {
            Some(candidate)
        }
        Some(current) => Some(current),
    }
}

#[cfg(test)]
mod tests {
    use super::{LevelView, nearer_resistance, nearer_support};

    fn view(price: f64, available_at: usize) -> LevelView {
        LevelView {
            timeframe_secs: 900,
            price,
            occurred_at: available_at,
            available_at,
            scale_code: 2,
            kind_code: -1,
        }
    }

    #[test]
    fn equal_price_levels_keep_the_later_confirmation() {
        let earlier = view(100.0, 3);
        let later = view(100.0, 5);
        assert_eq!(nearer_support(Some(earlier), later), Some(later));
        assert_eq!(nearer_resistance(Some(earlier), later), Some(later));
    }

    #[test]
    fn the_nearest_level_wins_over_a_farther_one() {
        let near = view(99.0, 3);
        let far = view(95.0, 5);
        assert_eq!(nearer_support(None, near), Some(near));
        assert_eq!(nearer_support(Some(near), far), Some(near));
        assert_eq!(
            nearer_resistance(None, view(101.0, 3)),
            Some(view(101.0, 3))
        );
        assert_eq!(
            nearer_resistance(Some(view(101.0, 3)), view(105.0, 5)),
            Some(view(101.0, 3))
        );
    }
}
