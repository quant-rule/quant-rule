//! The candle entity and its validation rules.

use std::fmt;

use crate::time::{TimestampError, parse_utc_timestamp};

/// A validated OHLCV candle.
///
/// The fields are private so that an invalid candle cannot be constructed by
/// struct literal. Use [`Candle::parse`] at the source boundary or
/// [`Candle::new`] when the fields are already numeric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candle {
    series_index: u64,
    timestamp: i64,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<f64>,
}

/// The raw string fields a source provides for one candle.
///
/// The shell reads a source and hands these fields to
/// [`Candle::parse`]. Keeping the boundary as borrowed strings means the data
/// engine does not need to know the source format beyond the timestamp shape
/// documented on each field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawCandle<'a> {
    /// UTC timestamp in `YYYY-MM-DD HH:MM:SS+00:00` form.
    pub date: &'a str,
    /// Open price.
    pub open: &'a str,
    /// High price.
    pub high: &'a str,
    /// Low price.
    pub low: &'a str,
    /// Close price.
    pub close: &'a str,
    /// Volume, when the source supplies it. An empty string means absent.
    pub volume: Option<&'a str>,
}

/// A candle could not be parsed or validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandleError {
    /// A numeric field could not be parsed as a number.
    InvalidNumber {
        /// The field that failed.
        field: &'static str,
    },
    /// A numeric field was not finite.
    NonFinite {
        /// The field that failed.
        field: &'static str,
    },
    /// The high was below the low.
    HighBelowLow,
    /// The open was above the high.
    OpenAboveHigh,
    /// The close was above the high.
    CloseAboveHigh,
    /// The open was below the low.
    OpenBelowLow,
    /// The close was below the low.
    CloseBelowLow,
    /// The volume was negative.
    NegativeVolume,
    /// The timestamp could not be parsed.
    Timestamp(TimestampError),
}

impl From<TimestampError> for CandleError {
    fn from(source: TimestampError) -> Self {
        Self::Timestamp(source)
    }
}

impl fmt::Display for CandleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNumber { field } => write!(formatter, "{field} is not a number"),
            Self::NonFinite { field } => write!(formatter, "{field} is not finite"),
            Self::HighBelowLow => write!(formatter, "high is below low"),
            Self::OpenAboveHigh => write!(formatter, "open is above high"),
            Self::CloseAboveHigh => write!(formatter, "close is above high"),
            Self::OpenBelowLow => write!(formatter, "open is below low"),
            Self::CloseBelowLow => write!(formatter, "close is below low"),
            Self::NegativeVolume => write!(formatter, "volume is negative"),
            Self::Timestamp(source) => write!(formatter, "invalid timestamp: {source}"),
        }
    }
}

impl std::error::Error for CandleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Timestamp(source) => Some(source),
            _ => None,
        }
    }
}

impl Candle {
    /// Builds a candle from already parsed values.
    ///
    /// # Errors
    ///
    /// Returns an error when a price or the volume is not finite, when the
    /// OHLC values are inconsistent, or when the volume is negative.
    pub fn new(
        series_index: u64,
        timestamp: i64,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: Option<f64>,
    ) -> Result<Self, CandleError> {
        check_finite("open", open)?;
        check_finite("high", high)?;
        check_finite("low", low)?;
        check_finite("close", close)?;
        if let Some(volume) = volume {
            check_finite("volume", volume)?;
            if volume < 0.0 {
                return Err(CandleError::NegativeVolume);
            }
        }

        if high < low {
            return Err(CandleError::HighBelowLow);
        }
        if open > high {
            return Err(CandleError::OpenAboveHigh);
        }
        if close > high {
            return Err(CandleError::CloseAboveHigh);
        }
        if open < low {
            return Err(CandleError::OpenBelowLow);
        }
        if close < low {
            return Err(CandleError::CloseBelowLow);
        }

        Ok(Self {
            series_index,
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        })
    }

    /// Parses and validates one source candle.
    ///
    /// # Errors
    ///
    /// Returns an error when the timestamp, a price, or the volume cannot be
    /// parsed, or when the parsed values violate the candle invariants.
    pub fn parse(series_index: u64, raw: RawCandle<'_>) -> Result<Self, CandleError> {
        let timestamp = parse_utc_timestamp(raw.date)?;
        let open = parse_number("open", raw.open)?;
        let high = parse_number("high", raw.high)?;
        let low = parse_number("low", raw.low)?;
        let close = parse_number("close", raw.close)?;
        let volume = match raw.volume {
            Some(field) if !field.trim().is_empty() => Some(parse_number("volume", field)?),
            _ => None,
        };

        Self::new(series_index, timestamp, open, high, low, close, volume)
    }

    /// The stable index of the candle in its series.
    #[must_use]
    pub const fn series_index(&self) -> u64 {
        self.series_index
    }

    /// The candle timestamp as UTC Unix seconds.
    #[must_use]
    pub const fn timestamp(&self) -> i64 {
        self.timestamp
    }

    /// The open price.
    #[must_use]
    pub const fn open(&self) -> f64 {
        self.open
    }

    /// The high price.
    #[must_use]
    pub const fn high(&self) -> f64 {
        self.high
    }

    /// The low price.
    #[must_use]
    pub const fn low(&self) -> f64 {
        self.low
    }

    /// The close price.
    #[must_use]
    pub const fn close(&self) -> f64 {
        self.close
    }

    /// The volume, or `None` when the source did not supply one.
    #[must_use]
    pub const fn volume(&self) -> Option<f64> {
        self.volume
    }
}

fn parse_number(field: &'static str, input: &str) -> Result<f64, CandleError> {
    input
        .trim()
        .parse::<f64>()
        .map_err(|_| CandleError::InvalidNumber { field })
}

fn check_finite(field: &'static str, value: f64) -> Result<(), CandleError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(CandleError::NonFinite { field })
    }
}

#[cfg(test)]
mod tests {
    use super::{Candle, CandleError, RawCandle};

    fn valid_raw() -> RawCandle<'static> {
        RawCandle {
            date: "2025-01-01 22:00:00+00:00",
            open: "1.03503",
            high: "1.03514",
            low: "1.03503",
            close: "1.03514",
            volume: Some("5.97"),
        }
    }

    #[test]
    fn parses_a_valid_candle() {
        let candle = Candle::parse(42, valid_raw()).expect("the fixture candle is valid");

        assert_eq!(candle.series_index(), 42);
        assert_eq!(candle.timestamp(), 1_735_768_800);
        assert_eq!(candle.open(), 1.035_03);
        assert_eq!(candle.high(), 1.035_14);
        assert_eq!(candle.low(), 1.035_03);
        assert_eq!(candle.close(), 1.035_14);
        assert_eq!(candle.volume(), Some(5.97));
    }

    #[test]
    fn treats_a_missing_volume_as_none() {
        let raw = RawCandle {
            volume: None,
            ..valid_raw()
        };

        let candle = Candle::parse(0, raw).expect("volume is optional");

        assert_eq!(candle.volume(), None);
    }

    #[test]
    fn treats_an_empty_volume_as_none() {
        let raw = RawCandle {
            volume: Some("  "),
            ..valid_raw()
        };

        let candle = Candle::parse(0, raw).expect("blank volume means absent");

        assert_eq!(candle.volume(), None);
    }

    #[test]
    fn preserves_a_real_zero_volume() {
        let raw = RawCandle {
            volume: Some("0"),
            ..valid_raw()
        };

        let candle = Candle::parse(0, raw).expect("zero volume is a value");

        assert_eq!(candle.volume(), Some(0.0));
    }

    #[test]
    fn accepts_negative_prices() {
        let candle =
            Candle::new(0, 0, -1.5, 0.0, -2.0, -1.0, None).expect("negative prices are valid");

        assert_eq!(candle.open(), -1.5);
        assert_eq!(candle.high(), 0.0);
        assert_eq!(candle.low(), -2.0);
        assert_eq!(candle.close(), -1.0);
    }

    #[test]
    fn accepts_a_zero_range_candle() {
        let candle =
            Candle::new(0, 0, 1.0, 1.0, 1.0, 1.0, Some(0.0)).expect("a zero range is valid");

        assert_eq!(candle.high(), candle.low());
    }

    #[test]
    fn rejects_a_high_below_the_low() {
        let result = Candle::new(0, 0, 1.0, 0.5, 1.0, 1.0, None);

        assert_eq!(result, Err(CandleError::HighBelowLow));
    }

    #[test]
    fn rejects_an_open_above_the_high() {
        let result = Candle::new(0, 0, 2.0, 1.0, 0.0, 1.0, None);

        assert_eq!(result, Err(CandleError::OpenAboveHigh));
    }

    #[test]
    fn rejects_a_close_above_the_high() {
        let result = Candle::new(0, 0, 1.0, 1.0, 0.0, 2.0, None);

        assert_eq!(result, Err(CandleError::CloseAboveHigh));
    }

    #[test]
    fn rejects_an_open_below_the_low() {
        let result = Candle::new(0, 0, -1.0, 1.0, 0.0, 1.0, None);

        assert_eq!(result, Err(CandleError::OpenBelowLow));
    }

    #[test]
    fn rejects_a_close_below_the_low() {
        let result = Candle::new(0, 0, 1.0, 1.0, 0.0, -1.0, None);

        assert_eq!(result, Err(CandleError::CloseBelowLow));
    }

    #[test]
    fn rejects_non_finite_prices() {
        let result = Candle::new(0, 0, f64::NAN, 1.0, 0.0, 1.0, None);

        assert_eq!(result, Err(CandleError::NonFinite { field: "open" }));
    }

    #[test]
    fn rejects_an_infinite_volume() {
        let result = Candle::new(0, 0, 1.0, 1.0, 0.0, 1.0, Some(f64::INFINITY));

        assert_eq!(result, Err(CandleError::NonFinite { field: "volume" }));
    }

    #[test]
    fn rejects_a_negative_volume() {
        let result = Candle::new(0, 0, 1.0, 1.0, 0.0, 1.0, Some(-0.1));

        assert_eq!(result, Err(CandleError::NegativeVolume));
    }

    #[test]
    fn rejects_a_non_numeric_price() {
        let raw = RawCandle {
            open: "not-a-number",
            ..valid_raw()
        };

        let result = Candle::parse(0, raw);

        assert_eq!(result, Err(CandleError::InvalidNumber { field: "open" }));
    }

    #[test]
    fn maps_a_bad_timestamp_to_the_timestamp_error() {
        let raw = RawCandle {
            date: "2025-02-29 00:00:00+00:00",
            ..valid_raw()
        };

        let result = Candle::parse(0, raw);

        assert!(matches!(
            result,
            Err(CandleError::Timestamp(crate::TimestampError::InvalidDate))
        ));
    }
}
