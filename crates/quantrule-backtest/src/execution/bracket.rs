//! Resolution of a strategy's stop and target specifications.
//!
//! The strategy emits a [`crate::intent::TradeIntent`] and execution resolves
//! it here. The module is private: callers see the resolved prices only through
//! the position the engine opens.

use crate::intent::{StopSpec, TargetSpec};
use crate::side::Side;

use super::RejectReason;

/// Resolves the strategy's stop and target specifications against a fill.
///
/// The ATR is read only when the stop is ATR-based. A reward-to-risk target uses
/// the stop distance, so an absolute stop plus a reward-to-risk target needs no
/// ATR either. The caller validates the returned prices.
pub(super) fn resolve(
    side: Side,
    entry: f64,
    atr14: Option<f64>,
    stop: StopSpec,
    target: TargetSpec,
) -> Result<(f64, f64), RejectReason> {
    let stop_price = match stop {
        StopSpec::AtrMultiple { multiple } => {
            if !multiple.is_finite() || multiple <= 0.0 {
                return Err(RejectReason::InvalidBracket);
            }
            let atr = usable_atr(atr14).ok_or(RejectReason::MissingAtr)?;
            match side {
                Side::Long => entry - multiple * atr,
                Side::Short => entry + multiple * atr,
            }
        }
        StopSpec::Price { price } => price,
    };

    let target_price = match target {
        TargetSpec::RewardToRisk { multiple } => {
            if !multiple.is_finite() || multiple <= 0.0 {
                return Err(RejectReason::InvalidBracket);
            }
            let stop_distance = (entry - stop_price).abs();
            match side {
                Side::Long => entry + stop_distance * multiple,
                Side::Short => entry - stop_distance * multiple,
            }
        }
        TargetSpec::Price { price } => price,
    };

    Ok((stop_price, target_price))
}

/// Whether the resolved levels bracket the fill on the correct side.
pub(super) fn is_valid(side: Side, entry: f64, stop: f64, target: f64) -> bool {
    if !stop.is_finite() || stop <= 0.0 || !target.is_finite() || target <= 0.0 {
        return false;
    }
    match side {
        Side::Long => stop < entry && entry < target,
        Side::Short => target < entry && entry < stop,
    }
}

fn usable_atr(atr14: Option<f64>) -> Option<f64> {
    atr14.filter(|value| value.is_finite() && *value > 0.0)
}
