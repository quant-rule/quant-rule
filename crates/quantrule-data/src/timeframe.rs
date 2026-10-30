//! Timeframes supported by the data engine.
//!
//! The source slice can fetch M1, M5, M15, and H1 natively from Dukascopy; the
//! data engine derives M5, M15, H1, and H4 from an M1 series. A derived series
//! uses the same candle shape and bounded window as the source M1 series.

/// A candle timeframe.
///
/// The source slice fetches M1, M5, M15, and H1 natively from Dukascopy. The
/// data engine derives [`Timeframe::M5`], [`Timeframe::M15`],
/// [`Timeframe::H1`], and [`Timeframe::H4`] from an M1 series. D1 is not
/// supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timeframe {
    seconds: u32,
    code: &'static str,
    file_code: &'static str,
}

impl Timeframe {
    /// Every supported timeframe in ascending duration order.
    pub const ALL: [Self; 5] = [Self::M1, Self::M5, Self::M15, Self::H1, Self::H4];

    /// The default ladder used by commands that do not read a run config.
    ///
    /// It is the historical four-timeframe ladder. The configured `data.json`
    /// v3 available ladder may include H4 or omit members.
    pub const DEFAULT_LADDER: [Self; 4] = [Self::M1, Self::M5, Self::M15, Self::H1];

    /// The timeframes derived from M1 by the data engine.
    pub const DERIVED: [Self; 4] = [Self::M5, Self::M15, Self::H1, Self::H4];

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

    /// Four hours.
    pub const H4: Self = Self {
        seconds: 14_400,
        code: "4h",
        file_code: "H4",
    };

    /// The timeframe length in seconds.
    #[must_use]
    pub const fn seconds(self) -> u32 {
        self.seconds
    }

    /// The UTC start of the bucket containing `timestamp`.
    ///
    /// Timestamps are Unix seconds. Floor division keeps every bucket aligned
    /// to the Unix epoch, which is a UTC boundary for M5, M15, and H1.
    #[must_use]
    pub const fn bucket_start(self, timestamp: i64) -> i64 {
        let seconds = self.seconds as i64;
        timestamp.div_euclid(seconds) * seconds
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
        } else if code.eq_ignore_ascii_case("4h") || code.eq_ignore_ascii_case("h4") {
            Some(Self::H4)
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
        assert_eq!(Timeframe::H4.seconds(), 14_400);
    }

    #[test]
    fn codes_are_stable() {
        assert_eq!(Timeframe::M1.code(), "1m");
        assert_eq!(Timeframe::M5.code(), "5m");
        assert_eq!(Timeframe::M15.code(), "15m");
        assert_eq!(Timeframe::H1.code(), "1h");
        assert_eq!(Timeframe::H4.code(), "4h");
        assert_eq!(Timeframe::M1.file_code(), "M1");
        assert_eq!(Timeframe::M5.file_code(), "M5");
        assert_eq!(Timeframe::M15.file_code(), "M15");
        assert_eq!(Timeframe::H1.file_code(), "H1");
        assert_eq!(Timeframe::H4.file_code(), "H4");
    }

    #[test]
    fn the_timeframe_order_is_fixed() {
        assert_eq!(
            Timeframe::ALL,
            [
                Timeframe::M1,
                Timeframe::M5,
                Timeframe::M15,
                Timeframe::H1,
                Timeframe::H4
            ]
        );
        assert_eq!(
            Timeframe::DEFAULT_LADDER,
            [Timeframe::M1, Timeframe::M5, Timeframe::M15, Timeframe::H1]
        );
        assert_eq!(
            Timeframe::DERIVED,
            [Timeframe::M5, Timeframe::M15, Timeframe::H1, Timeframe::H4]
        );
    }

    #[test]
    fn buckets_start_on_utc_boundaries() {
        let base = 1_735_689_600;

        assert_eq!(Timeframe::M1.bucket_start(base + 59), base);
        assert_eq!(Timeframe::M5.bucket_start(base + 299), base);
        assert_eq!(Timeframe::M5.bucket_start(base + 300), base + 300);
        assert_eq!(Timeframe::M15.bucket_start(base + 899), base);
        assert_eq!(Timeframe::M15.bucket_start(base + 900), base + 900);
        assert_eq!(Timeframe::H1.bucket_start(base + 3_599), base);
        assert_eq!(Timeframe::H1.bucket_start(base + 3_600), base + 3_600);
        assert_eq!(Timeframe::H4.bucket_start(base + 14_399), base);
        assert_eq!(Timeframe::H4.bucket_start(base + 14_400), base + 14_400);
    }

    #[test]
    fn bucket_start_handles_timestamps_before_the_epoch() {
        assert_eq!(Timeframe::M5.bucket_start(-1), -300);
        assert_eq!(Timeframe::M5.bucket_start(-300), -300);
    }

    #[test]
    fn codes_parse_case_insensitively() {
        for code in ["1m", "M1", "5m", "m5", "15M", "h1"] {
            assert!(Timeframe::from_code(code).is_some(), "{code} is known");
        }
        assert_eq!(Timeframe::from_code("15m"), Some(Timeframe::M15));
        assert_eq!(Timeframe::from_code("1H"), Some(Timeframe::H1));
        assert_eq!(Timeframe::from_code("4h"), Some(Timeframe::H4));
        assert_eq!(Timeframe::from_code("H4"), Some(Timeframe::H4));
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
