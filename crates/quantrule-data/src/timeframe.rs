//! Timeframes supported by the data engine.

/// A candle timeframe.
///
/// The source slice fetches these intervals natively from Dukascopy. Building
/// higher timeframes from an M1 series remains issue #50.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeframe {
    seconds: u32,
    code: &'static str,
    file_code: &'static str,
}

impl Timeframe {
    /// One minute.
    pub const M1: Self = Self {
        seconds: 60,
        code: "1m",
        file_code: "M1",
    };

    /// Five minutes.
    pub const M5: Self = Self {
        seconds: 300,
        code: "5m",
        file_code: "M5",
    };

    /// Fifteen minutes.
    pub const M15: Self = Self {
        seconds: 900,
        code: "15m",
        file_code: "M15",
    };

    /// One hour.
    pub const H1: Self = Self {
        seconds: 3_600,
        code: "1h",
        file_code: "H1",
    };

    /// The timeframe length in seconds.
    #[must_use]
    pub const fn seconds(self) -> u32 {
        self.seconds
    }

    /// The short user-facing code, such as `5m`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        self.code
    }

    /// The uppercase code used in a CSV file name, such as `M5`.
    #[must_use]
    pub const fn file_code(self) -> &'static str {
        self.file_code
    }

    /// Resolves a short timeframe code.
    ///
    /// Codes are case-insensitive and accept either the user form (`5m`) or
    /// the file form (`M5`).
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        if code.eq_ignore_ascii_case("1m") || code.eq_ignore_ascii_case("m1") {
            Some(Self::M1)
        } else if code.eq_ignore_ascii_case("5m") || code.eq_ignore_ascii_case("m5") {
            Some(Self::M5)
        } else if code.eq_ignore_ascii_case("15m") || code.eq_ignore_ascii_case("m15") {
            Some(Self::M15)
        } else if code.eq_ignore_ascii_case("1h") || code.eq_ignore_ascii_case("h1") {
            Some(Self::H1)
        } else {
            None
        }
    }
}

impl Default for Timeframe {
    fn default() -> Self {
        Self::M1
    }
}

#[cfg(test)]
mod tests {
    use super::Timeframe;

    #[test]
    fn m1_is_sixty_seconds() {
        assert_eq!(Timeframe::M1.seconds(), 60);
    }

    #[test]
    fn native_timeframes_have_their_expected_lengths() {
        assert_eq!(Timeframe::M5.seconds(), 300);
        assert_eq!(Timeframe::M15.seconds(), 900);
        assert_eq!(Timeframe::H1.seconds(), 3_600);
    }

    #[test]
    fn codes_are_stable() {
        assert_eq!(Timeframe::M1.code(), "1m");
        assert_eq!(Timeframe::M5.code(), "5m");
        assert_eq!(Timeframe::M15.code(), "15m");
        assert_eq!(Timeframe::H1.code(), "1h");
        assert_eq!(Timeframe::M1.file_code(), "M1");
        assert_eq!(Timeframe::M5.file_code(), "M5");
        assert_eq!(Timeframe::M15.file_code(), "M15");
        assert_eq!(Timeframe::H1.file_code(), "H1");
    }

    #[test]
    fn codes_parse_case_insensitively() {
        for code in ["1m", "M1", "5m", "m5", "15M", "h1"] {
            assert!(Timeframe::from_code(code).is_some(), "{code} is known");
        }
        assert_eq!(Timeframe::from_code("15m"), Some(Timeframe::M15));
        assert_eq!(Timeframe::from_code("1H"), Some(Timeframe::H1));
    }

    #[test]
    fn unknown_codes_are_rejected() {
        assert_eq!(Timeframe::from_code("30m"), None);
        assert_eq!(Timeframe::from_code("1d"), None);
        assert_eq!(Timeframe::from_code(""), None);
    }

    #[test]
    fn the_default_is_m1() {
        assert_eq!(Timeframe::default(), Timeframe::M1);
    }
}
