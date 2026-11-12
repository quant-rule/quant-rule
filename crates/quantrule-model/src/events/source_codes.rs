//! The contract's source codes, defined once.
//!
//! Every event record names the reading or comparison that produced it with a
//! stable `source_code`. The code is numeric because the model publishes
//! numbers; the registry in `docs/registry/indicators.md` carries the words.
//! A code is stable once published and is never reused. The event modules
//! re-export the constants they own so existing paths keep working.
//!
//! The registry is the source of truth for the mapping; `tools/check_registry.py`
//! compares this module's numeric set with `docs/registry/indicators.json`.

/// EMA 20, the fast EMA.
pub const EMA20_SOURCE_CODE: u16 = 1;
/// EMA 50, the middle EMA.
pub const EMA50_SOURCE_CODE: u16 = 2;
/// EMA 200, the slow EMA.
pub const EMA200_SOURCE_CODE: u16 = 3;
/// RSI 14.
pub const RSI_SOURCE_CODE: u16 = 4;
/// ATR 14, used by the base level and compression events.
pub const ATR_SOURCE_CODE: u16 = 5;
/// RVOL 20.
pub const RVOL_SOURCE_CODE: u16 = 6;
/// The latest confirmed minor pivot.
pub const MINOR_PIVOT_SOURCE_CODE: u16 = 7;
/// The latest confirmed major pivot.
pub const MAJOR_PIVOT_SOURCE_CODE: u16 = 8;
/// The MACD line.
pub const MACD_LINE_SOURCE_CODE: u16 = 13;
/// The MACD signal line.
pub const MACD_SIGNAL_SOURCE_CODE: u16 = 14;
/// The MACD histogram.
pub const MACD_HISTOGRAM_SOURCE_CODE: u16 = 15;
/// Average Directional Index.
pub const ADX_SOURCE_CODE: u16 = 16;
/// Positive directional indicator.
pub const DI_PLUS_SOURCE_CODE: u16 = 17;
/// Negative directional indicator.
pub const DI_MINUS_SOURCE_CODE: u16 = 18;
/// Rolling VWAP.
pub const VWAP_SOURCE_CODE: u16 = 19;
/// Rolling volume profile.
pub const PROFILE_SOURCE_CODE: u16 = 20;
/// Volume climax.
pub const CLIMAX_SOURCE_CODE: u16 = 21;
/// Absorption.
pub const ABSORPTION_SOURCE_CODE: u16 = 22;
/// Volume oscillator.
pub const VOLUME_OSCILLATOR_SOURCE_CODE: u16 = 23;
/// On-balance volume.
pub const OBV_SOURCE_CODE: u16 = 24;
/// Chaikin money flow.
pub const CMF_SOURCE_CODE: u16 = 25;
/// Money flow index.
pub const MFI_SOURCE_CODE: u16 = 26;
/// Accumulation/distribution line.
pub const AD_SOURCE_CODE: u16 = 27;
/// ATR term structure.
pub const ATR_TERM_SOURCE_CODE: u16 = 28;
/// Range expansion.
pub const RANGE_SOURCE_CODE: u16 = 29;
/// Mean absolute return.
pub const MAR_SOURCE_CODE: u16 = 30;
/// Keltner channel.
pub const KELTNER_SOURCE_CODE: u16 = 31;
/// Donchian channel.
pub const DONCHIAN_SOURCE_CODE: u16 = 32;
/// Swing structure.
pub const SWING_SOURCE_CODE: u16 = 33;
/// Level strength.
pub const STRENGTH_SOURCE_CODE: u16 = 34;
/// Aroon.
pub const AROON_SOURCE_CODE: u16 = 35;
/// Break of structure / change of character.
pub const BOS_CHOCH_SOURCE_CODE: u16 = 36;
/// Candle anatomy.
pub const CANDLE_SOURCE_CODE: u16 = 37;
/// Candle pattern.
pub const PATTERN_SOURCE_CODE: u16 = 38;
/// Price gap.
pub const GAP_SOURCE_CODE: u16 = 39;
/// Path efficiency.
pub const PATH_SOURCE_CODE: u16 = 40;
/// Direction changes.
pub const DIRECTION_SOURCE_CODE: u16 = 41;
/// Bars since extreme.
pub const EXTREME_SOURCE_CODE: u16 = 42;
/// Drawdown.
pub const DRAWDOWN_SOURCE_CODE: u16 = 43;
/// Range position.
pub const RANGE_POSITION_SOURCE_CODE: u16 = 44;
/// Relative strength.
pub const RELATIVE_STRENGTH_SOURCE_CODE: u16 = 45;
/// Lead-lag.
pub const LEAD_LAG_SOURCE_CODE: u16 = 46;
/// Breadth.
pub const BREADTH_SOURCE_CODE: u16 = 47;
/// Co-movement.
pub const CO_MOVEMENT_SOURCE_CODE: u16 = 48;
/// Dispersion.
pub const DISPERSION_SOURCE_CODE: u16 = 49;
/// Session.
pub const SESSION_SOURCE_CODE: u16 = 50;
/// Opening range.
pub const OPENING_RANGE_SOURCE_CODE: u16 = 51;
/// Overnight/intraday split.
pub const OVERNIGHT_SOURCE_CODE: u16 = 52;
/// Session range.
pub const SESSION_RANGE_SOURCE_CODE: u16 = 53;
/// Confirmed pivot sequence.
pub const PIVOT_SEQUENCE_SOURCE_CODE: u16 = 55;
/// Fibonacci retracement.
pub const FIBONACCI_SOURCE_CODE: u16 = 56;
/// Horizontal support/resistance band.
pub const BAND_SOURCE_CODE: u16 = 57;

/// Every source code the model currently emits.
pub const ALL: &[u16] = &[
    EMA20_SOURCE_CODE,
    EMA50_SOURCE_CODE,
    EMA200_SOURCE_CODE,
    RSI_SOURCE_CODE,
    ATR_SOURCE_CODE,
    RVOL_SOURCE_CODE,
    MINOR_PIVOT_SOURCE_CODE,
    MAJOR_PIVOT_SOURCE_CODE,
    MACD_LINE_SOURCE_CODE,
    MACD_SIGNAL_SOURCE_CODE,
    MACD_HISTOGRAM_SOURCE_CODE,
    ADX_SOURCE_CODE,
    DI_PLUS_SOURCE_CODE,
    DI_MINUS_SOURCE_CODE,
    VWAP_SOURCE_CODE,
    PROFILE_SOURCE_CODE,
    CLIMAX_SOURCE_CODE,
    ABSORPTION_SOURCE_CODE,
    VOLUME_OSCILLATOR_SOURCE_CODE,
    OBV_SOURCE_CODE,
    CMF_SOURCE_CODE,
    MFI_SOURCE_CODE,
    AD_SOURCE_CODE,
    ATR_TERM_SOURCE_CODE,
    RANGE_SOURCE_CODE,
    MAR_SOURCE_CODE,
    KELTNER_SOURCE_CODE,
    DONCHIAN_SOURCE_CODE,
    SWING_SOURCE_CODE,
    STRENGTH_SOURCE_CODE,
    AROON_SOURCE_CODE,
    BOS_CHOCH_SOURCE_CODE,
    CANDLE_SOURCE_CODE,
    PATTERN_SOURCE_CODE,
    GAP_SOURCE_CODE,
    PATH_SOURCE_CODE,
    DIRECTION_SOURCE_CODE,
    EXTREME_SOURCE_CODE,
    DRAWDOWN_SOURCE_CODE,
    RANGE_POSITION_SOURCE_CODE,
    RELATIVE_STRENGTH_SOURCE_CODE,
    LEAD_LAG_SOURCE_CODE,
    BREADTH_SOURCE_CODE,
    CO_MOVEMENT_SOURCE_CODE,
    DISPERSION_SOURCE_CODE,
    SESSION_SOURCE_CODE,
    OPENING_RANGE_SOURCE_CODE,
    OVERNIGHT_SOURCE_CODE,
    SESSION_RANGE_SOURCE_CODE,
    PIVOT_SEQUENCE_SOURCE_CODE,
    FIBONACCI_SOURCE_CODE,
    BAND_SOURCE_CODE,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_codes_are_unique_and_in_range() {
        let mut seen = std::collections::BTreeSet::new();
        for code in ALL.iter().copied() {
            assert!(
                (1..=57).contains(&code),
                "source code {code} is out of range"
            );
            assert!(seen.insert(code), "source code {code} is duplicated");
        }
    }
}
