//! Presentation labels for the model's numeric codes.
//!
//! The model publishes numbers; the shell maps them to words. Every mapping is
//! fixed, so the same numeric output renders as the same text on every run.

pub(crate) fn timeframe_label(seconds: u64) -> &'static str {
    match seconds {
        60 => "1m",
        300 => "5m",
        900 => "15m",
        3_600 => "1h",
        14_400 => "4h",
        _ => "unknown",
    }
}

pub(crate) fn stage_label(code: i8) -> &'static str {
    match code {
        0 => "against",
        1 => "under_25",
        2 => "25_to_50",
        3 => "50_to_75",
        4 => "late",
        5 => "terminal",
        _ => "unknown",
    }
}

pub(crate) fn context_label(code: u8) -> &'static str {
    match code {
        1 => "expanding",
        2 => "coiling",
        3 => "settling",
        4 => "high_range_low_participation",
        _ => "unknown",
    }
}

pub(crate) fn event_type_label(code: u8) -> &'static str {
    match code {
        1 => "level",
        2 => "divergence",
        3 => "signal",
        _ => "unknown",
    }
}

pub(crate) fn event_subtype_label(type_code: u8, subtype_code: u8) -> &'static str {
    match (type_code, subtype_code) {
        (1, 1) => "test",
        (1, 2) => "cross",
        (1, 3) => "break",
        (1, 4) => "break_of_structure",
        (1, 5) => "change_of_character",
        (2, 1) => "regular_bullish",
        (2, 2) => "regular_bearish",
        (2, 3) => "hidden_bullish",
        (2, 4) => "hidden_bearish",
        (3, 1) => "threshold_crossing",
        (3, 2) => "series_crossover",
        (3, 3) => "compact_state_change",
        (3, 4) => "histogram_slope_flip",
        (3, 5) => "zero_cross",
        (3, 6) => "trend_continuation",
        (3, 7) => "compression_breakout",
        (3, 8) => "compression_range",
        (3, 9) => "compression_range_onset",
        (3, 10) => "candle_pattern",
        (3, 11) => "participation",
        (3, 12) => "absorption",
        (3, 13) => "volume_profile",
        (3, 14) => "vwap",
        (3, 15) => "volume_flow",
        (3, 16) => "volatility",
        (3, 17) => "range",
        (3, 18) => "channel",
        (3, 19) => "structure",
        (3, 20) => "gap",
        (3, 21) => "path",
        (3, 22) => "cross_asset",
        (3, 23) => "session",
        _ => "unknown",
    }
}

pub(crate) fn source_label(code: u16) -> &'static str {
    match code {
        1 => "ema_fast",
        2 => "ema_mid",
        3 => "ema_slow",
        4 => "rsi",
        5 => "atr",
        6 => "rvol",
        7 => "minor_pivots",
        8 => "major_pivots",
        12 => "progression",
        13 => "macd_line",
        14 => "macd_signal",
        15 => "macd_histogram",
        16 => "adx",
        17 => "di_plus",
        18 => "di_minus",
        19 => "vwap",
        20 => "volume_profile",
        21 => "volume_climax",
        22 => "absorption",
        23 => "volume_oscillator",
        24 => "obv",
        25 => "cmf",
        26 => "mfi",
        27 => "ad_line",
        28 => "atr_term",
        29 => "range_expansion",
        30 => "mar",
        31 => "keltner",
        32 => "donchian",
        33 => "swing_structure",
        34 => "level_strength",
        35 => "aroon",
        36 => "bos_choch",
        37 => "candle_anatomy",
        38 => "candle_pattern",
        39 => "gap",
        40 => "path_efficiency",
        41 => "direction_changes",
        42 => "bars_since_extreme",
        43 => "drawdown",
        44 => "range_position",
        45 => "relative_strength",
        46 => "lead_lag",
        47 => "breadth",
        48 => "co_movement",
        49 => "dispersion",
        50 => "session",
        51 => "opening_range",
        52 => "overnight_intraday",
        53 => "session_range",
        54 => "calendar_bucket",
        _ => "unknown",
    }
}

pub(crate) fn lifecycle_label(code: u8) -> &'static str {
    match code {
        0 => "forming",
        1 => "active",
        2 => "completed",
        3 => "invalidated",
        4 => "superseded",
        _ => "unknown",
    }
}

pub(crate) fn scale_label(code: u8) -> &'static str {
    match code {
        1 => "minor",
        2 => "major",
        _ => "unknown",
    }
}

pub(crate) fn leg_direction_label(code: i8) -> &'static str {
    match code {
        1 => "up",
        -1 => "down",
        _ => "unknown",
    }
}

pub(crate) fn target_basis_label(code: u8) -> &'static str {
    match code {
        1 => "confirmed_level",
        2 => "atr_fallback",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::{event_subtype_label, source_label, timeframe_label};

    #[test]
    fn the_four_hour_label_is_known() {
        assert_eq!(timeframe_label(14_400), "4h");
    }

    #[test]
    fn the_extended_sources_are_labelled() {
        assert_eq!(source_label(19), "vwap");
        assert_eq!(source_label(54), "calendar_bucket");
        assert_eq!(source_label(99), "unknown");
    }

    #[test]
    fn the_signal_subtypes_are_labelled() {
        assert_eq!(event_subtype_label(3, 9), "compression_range_onset");
        assert_eq!(event_subtype_label(3, 23), "session");
    }
}
