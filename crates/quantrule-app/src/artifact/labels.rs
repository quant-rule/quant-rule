//! Presentation labels for the model's numeric codes.
//!
//! The model publishes numbers; the adapters and artifact renderers map them to
//! words. Every mapping is fixed, so the same numeric output renders as the same
//! text on every run.

/// Maps a timeframe in seconds to its command label.
pub fn timeframe_label(seconds: u64) -> &'static str {
    match seconds {
        60 => "1m",
        300 => "5m",
        900 => "15m",
        3_600 => "1h",
        14_400 => "4h",
        _ => "unknown",
    }
}

/// Maps a progression stage code to its word.
pub fn stage_label(code: i8) -> &'static str {
    match code {
        0 => "against",
        1 => "0-25%",
        2 => "25-50%",
        3 => "50-75%",
        4 => "late",
        5 => "terminal",
        _ => "unknown",
    }
}

/// Maps a context code to its word.
pub fn context_label(code: u8) -> &'static str {
    match code {
        1 => "expanding",
        2 => "coiling",
        3 => "settling",
        4 => "high_range_low_participation",
        _ => "unknown",
    }
}

/// Maps an event type code to its word.
pub fn event_type_label(code: u8) -> &'static str {
    match code {
        1 => "Level",
        2 => "Divergence",
        3 => "Signal",
        _ => "unknown",
    }
}

/// Maps an event code to its registry name.
pub fn event_name(code: u16) -> &'static str {
    match code {
        1 => "level.test",
        2 => "level.cross",
        3 => "level.break",
        4 => "divergence.rsi_regular_bullish",
        5 => "divergence.rsi_regular_bearish",
        6 => "divergence.rsi_hidden_bullish",
        7 => "divergence.rsi_hidden_bearish",
        8 => "signal.rsi_threshold",
        9 => "signal.ema_pair_cross",
        10 => "signal.ema_stack_change",
        11 => "signal.rsi_zone_change",
        12 => "signal.macd_line_signal_cross",
        13 => "signal.macd_histogram_slope_flip",
        14 => "signal.macd_zero_cross",
        15 => "signal.di_cross",
        16 => "signal.trend_continuation",
        17 => "signal.compression_breakout",
        18 => "signal.compression_range",
        19 => "signal.compression_range_onset",
        20 => "level.vwap_test",
        21 => "level.vwap_cross",
        22 => "level.vwap_break",
        23 => "divergence.vwap_regular_bullish",
        24 => "divergence.vwap_regular_bearish",
        25 => "divergence.vwap_hidden_bullish",
        26 => "divergence.vwap_hidden_bearish",
        27 => "signal.vwap_reclaim",
        28 => "signal.vwap_rejection",
        29 => "signal.vwap_slope",
        30 => "level.profile_poc_test",
        31 => "level.profile_vah_test",
        32 => "level.profile_val_test",
        33 => "level.profile_va_break",
        34 => "level.profile_va_reentry",
        35 => "divergence.profile_poc_bullish",
        36 => "divergence.profile_poc_bearish",
        37 => "signal.profile_va_breakout",
        38 => "signal.profile_poc_cross",
        39 => "signal.profile_skew",
        40 => "signal.profile_concentration",
        41 => "divergence.rvol_bullish",
        42 => "divergence.rvol_bearish",
        43 => "signal.rvol_expansion",
        44 => "signal.rvol_contraction",
        45 => "signal.rvol_ratio",
        46 => "divergence.climax_bullish",
        47 => "divergence.climax_bearish",
        48 => "signal.climax_reversal",
        49 => "signal.climax_continuation",
        50 => "level.absorption_test",
        51 => "signal.absorption_break",
        52 => "signal.absorption_rejection",
        53 => "signal.volume_oscillator_cross",
        54 => "divergence.obv_regular_bullish",
        55 => "divergence.obv_regular_bearish",
        56 => "divergence.obv_hidden_bullish",
        57 => "divergence.obv_hidden_bearish",
        58 => "signal.obv_cross",
        59 => "signal.obv_slope",
        60 => "divergence.cmf_regular_bullish",
        61 => "divergence.cmf_regular_bearish",
        62 => "divergence.cmf_hidden_bullish",
        63 => "divergence.cmf_hidden_bearish",
        64 => "signal.cmf_cross",
        65 => "signal.cmf_threshold",
        66 => "divergence.mfi_regular_bullish",
        67 => "divergence.mfi_regular_bearish",
        68 => "divergence.mfi_hidden_bullish",
        69 => "divergence.mfi_hidden_bearish",
        70 => "signal.mfi_threshold",
        71 => "signal.mfi_zone",
        72 => "divergence.ad_regular_bullish",
        73 => "divergence.ad_regular_bearish",
        74 => "signal.ad_cross",
        75 => "level.keltner_test",
        76 => "level.keltner_break",
        77 => "level.donchian_test",
        78 => "level.donchian_break",
        79 => "level.structure_break",
        80 => "level.strength_test",
        81 => "level.wick_test",
        82 => "level.gap_fill",
        83 => "level.gap_test",
        84 => "divergence.atr_bullish",
        85 => "divergence.atr_bearish",
        86 => "divergence.rs_regular_bullish",
        87 => "divergence.rs_regular_bearish",
        88 => "divergence.lead_lag_bullish",
        89 => "divergence.lead_lag_bearish",
        90 => "divergence.breadth_regular_bullish",
        91 => "divergence.breadth_regular_bearish",
        92 => "signal.atr_compression",
        93 => "signal.atr_expansion",
        94 => "signal.atr_regime",
        95 => "signal.range_expansion",
        96 => "signal.range_contraction",
        97 => "signal.mar_regime",
        98 => "signal.keltner_breakout",
        99 => "signal.keltner_position",
        100 => "signal.donchian_breakout",
        101 => "signal.donchian_position",
        102 => "signal.structure_change",
        103 => "signal.strength_change",
        104 => "signal.aroon_cross",
        105 => "signal.aroon_osc",
        106 => "signal.candle_momentum",
        107 => "signal.candle_rejection",
        108 => "signal.candle_doji",
        109 => "signal.candle_marubozu",
        110 => "signal.pattern_engulfing",
        111 => "signal.pattern_inside",
        112 => "signal.pattern_outside",
        113 => "signal.pattern_hammer",
        114 => "signal.pattern_star",
        115 => "signal.pattern_three_soldiers",
        116 => "signal.pattern_three_crows",
        117 => "signal.gap_continuation",
        118 => "signal.gap_fade",
        119 => "signal.path_efficiency",
        120 => "signal.direction_changes",
        121 => "signal.extreme_new",
        122 => "signal.extreme_stale",
        123 => "signal.drawdown_threshold",
        124 => "signal.drawdown_recovery",
        125 => "signal.range_position",
        126 => "signal.relative_strength_cross",
        127 => "signal.lead_lag_cross",
        128 => "signal.breadth_cross",
        129 => "signal.co_movement_cross",
        130 => "signal.dispersion_cross",
        131 => "signal.session_open",
        132 => "signal.session_close",
        133 => "signal.opening_range_breakout",
        134 => "signal.opening_range_fade",
        135 => "signal.overnight_continuation",
        136 => "signal.overnight_fade",
        137 => "signal.session_range_expansion",
        138 => "signal.session_range_contraction",
        139 => "level.bos",
        140 => "level.choch",
        141 => "level.session_vwap_test",
        142 => "level.opening_range_test",
        143 => "level.opening_range_break",
        144 => "signal.opening_range_sweep",
        145 => "signal.opening_range_sweep_control",
        146 => "signal.breakout_retest_hold",
        147 => "signal.lower_high_sequence",
        148 => "signal.lower_low_sequence",
        149 => "level.fib_test",
        150 => "level.band_test",
        _ => "unknown",
    }
}

/// Maps an event type and subtype pair to its word.
pub fn event_subtype_label(type_code: u8, subtype_code: u8) -> &'static str {
    match (type_code, subtype_code) {
        (1, 1) => "test/retest",
        (1, 2) => "cross",
        (1, 3) => "break",
        (1, 4) => "break of structure",
        (1, 5) => "change of character",
        (2, 1) => "regular bullish",
        (2, 2) => "regular bearish",
        (2, 3) => "hidden bullish",
        (2, 4) => "hidden bearish",
        (3, 1) => "threshold crossing",
        (3, 2) => "series crossover",
        (3, 3) => "compact state change",
        (3, 4) => "histogram slope flip",
        (3, 5) => "zero cross",
        (3, 6) => "trend continuation",
        (3, 7) => "compression breakout",
        (3, 8) => "compression range",
        (3, 9) => "compression range onset",
        (3, 10) => "candle pattern",
        (3, 11) => "participation",
        (3, 12) => "absorption",
        (3, 13) => "volume profile",
        (3, 14) => "VWAP",
        (3, 15) => "volume flow",
        (3, 16) => "volatility",
        (3, 17) => "range",
        (3, 18) => "channel",
        (3, 19) => "structure",
        (3, 20) => "gap",
        (3, 21) => "path",
        (3, 22) => "cross-asset",
        (3, 23) => "session",
        (3, 24) => "breakout retest hold",
        _ => "unknown",
    }
}

/// Maps a source code to its word.
pub fn source_label(code: u16) -> &'static str {
    match code {
        1 => "EMA fast",
        2 => "EMA mid",
        3 => "EMA slow",
        4 => "RSI",
        5 => "ATR",
        6 => "RVOL",
        7 => "minor pivots",
        8 => "major pivots",
        12 => "progression",
        13 => "MACD line",
        14 => "MACD signal",
        15 => "MACD histogram",
        16 => "ADX",
        17 => "+DI",
        18 => "-DI",
        19 => "VWAP",
        20 => "volume profile",
        21 => "volume climax",
        22 => "absorption",
        23 => "volume oscillator",
        24 => "OBV",
        25 => "CMF",
        26 => "MFI",
        27 => "A/D line",
        28 => "ATR term",
        29 => "range expansion",
        30 => "MAR",
        31 => "Keltner",
        32 => "Donchian",
        33 => "swing structure",
        34 => "level strength",
        35 => "Aroon",
        36 => "BOS/CHOCH",
        37 => "candle anatomy",
        38 => "candle pattern",
        39 => "gap",
        40 => "path efficiency",
        41 => "direction changes",
        42 => "bars since extreme",
        43 => "drawdown",
        44 => "range position",
        45 => "relative strength",
        46 => "lead-lag",
        47 => "breadth",
        48 => "co-movement",
        49 => "dispersion",
        50 => "session",
        51 => "opening range",
        52 => "overnight/intraday",
        53 => "session range",
        54 => "calendar bucket",
        55 => "pivot sequence",
        56 => "Fibonacci",
        57 => "horizontal band",
        _ => "unknown",
    }
}

/// Maps a progression lifecycle code to its word.
pub fn lifecycle_label(code: u8) -> &'static str {
    match code {
        0 => "forming",
        1 => "active",
        2 => "completed",
        3 => "invalidated",
        4 => "superseded",
        _ => "unknown",
    }
}

/// Maps a pivot scale code to its word.
pub fn scale_label(code: u8) -> &'static str {
    match code {
        1 => "minor",
        2 => "major",
        _ => "unknown",
    }
}

/// Maps a progression leg direction to its word.
pub fn leg_direction_label(code: i8) -> &'static str {
    match code {
        1 => "up",
        -1 => "down",
        _ => "unknown",
    }
}

/// Maps a progression target basis code to its word.
pub fn target_basis_label(code: u8) -> &'static str {
    match code {
        1 => "confirmed structural level",
        2 => "ATR fallback",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::{event_name, event_subtype_label, source_label, timeframe_label};

    #[test]
    fn the_four_hour_label_is_known() {
        assert_eq!(timeframe_label(14_400), "4h");
    }

    #[test]
    fn the_extended_sources_are_labelled() {
        assert_eq!(source_label(19), "VWAP");
        assert_eq!(source_label(54), "calendar bucket");
        assert_eq!(source_label(99), "unknown");
    }

    #[test]
    fn the_event_names_are_labelled() {
        assert_eq!(event_name(1), "level.test");
        assert_eq!(event_name(149), "level.fib_test");
        assert_eq!(event_name(999), "unknown");
    }

    #[test]
    fn the_signal_subtypes_are_labelled() {
        assert_eq!(event_subtype_label(3, 9), "compression range onset");
        assert_eq!(event_subtype_label(3, 23), "session");
    }
}
