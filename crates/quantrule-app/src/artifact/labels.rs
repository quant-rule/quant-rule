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
    use super::{event_subtype_label, source_label, timeframe_label};

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
    fn the_signal_subtypes_are_labelled() {
        assert_eq!(event_subtype_label(3, 9), "compression range onset");
        assert_eq!(event_subtype_label(3, 23), "session");
    }
}
