//! One open bucket while a higher timeframe is being built.
//!
//! A bucket is fed only by validated M1 candles that fall in the same
//! timeframe interval. It is complete only when every expected M1 candle is
//! present; an incomplete bucket is dropped rather than published with a
//! misleading open or close.

use std::fmt;

use crate::{CandleValues, Timeframe, format_utc_timestamp};

/// A bucket could not accept a source candle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BucketError {
    /// The summed volume exceeded the finite range of `f64`.
    VolumeOverflow {
        /// The UTC start timestamp of the bucket that overflowed.
        bucket_start: i64,
    },
}

impl fmt::Display for BucketError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VolumeOverflow { bucket_start } => write!(
                formatter,
                "aggregated volume overflowed in the bucket starting {}",
                format_utc_timestamp(*bucket_start)
            ),
        }
    }
}

impl std::error::Error for BucketError {}

/// An in-progress higher-timeframe bucket.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Bucket {
    timeframe: Timeframe,
    start: i64,
    count: u32,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<f64>,
}

impl Bucket {
    /// Starts a bucket with its first candle.
    pub(crate) fn new(timeframe: Timeframe, values: CandleValues) -> Self {
        Self {
            timeframe,
            start: timeframe.bucket_start(values.timestamp),
            count: 1,
            open: values.open,
            high: values.high,
            low: values.low,
            close: values.close,
            volume: values.volume,
        }
    }

    /// The bucket's UTC start timestamp.
    pub(crate) const fn start(&self) -> i64 {
        self.start
    }

    /// The number of M1 candles accepted into the bucket.
    pub(crate) const fn present(&self) -> u32 {
        self.count
    }

    /// The number of M1 candles a complete bucket must contain.
    pub(crate) const fn expected(&self) -> u32 {
        self.timeframe.seconds() / 60
    }

    /// Whether every expected M1 candle is present.
    pub(crate) const fn is_complete(&self) -> bool {
        self.count == self.expected()
    }

    /// Adds one M1 candle that belongs to this bucket.
    ///
    /// # Errors
    ///
    /// Returns [`BucketError::VolumeOverflow`] when the summed volume is no
    /// longer finite. The bucket is not modified in that case.
    pub(crate) fn push(&mut self, values: CandleValues) -> Result<(), BucketError> {
        let volume = match (self.volume, values.volume) {
            (Some(total), Some(volume)) => {
                let sum = total + volume;
                if !sum.is_finite() {
                    return Err(BucketError::VolumeOverflow {
                        bucket_start: self.start,
                    });
                }
                Some(sum)
            }
            _ => None,
        };

        self.count += 1;
        self.high = self.high.max(values.high);
        self.low = self.low.min(values.low);
        self.close = values.close;
        self.volume = volume;
        Ok(())
    }

    /// Converts a complete bucket into the values of one candle.
    ///
    /// The caller must check [`Bucket::is_complete`] first. A partial bucket
    /// has no close for the full interval and must not reach this method.
    pub(crate) fn into_values(self) -> CandleValues {
        CandleValues {
            timestamp: self.start,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Bucket;
    use crate::{CandleValues, Timeframe};

    const BASE: i64 = 1_735_689_600;

    fn values_at(seconds: i64) -> CandleValues {
        CandleValues {
            timestamp: seconds,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: Some(10.0),
        }
    }

    #[test]
    fn a_new_bucket_has_one_candle_and_is_partial() {
        let bucket = Bucket::new(Timeframe::M5, values_at(BASE));

        assert_eq!(bucket.start(), BASE);
        assert_eq!(bucket.present(), 1);
        assert_eq!(bucket.expected(), 5);
        assert!(!bucket.is_complete());
    }

    #[test]
    fn complete_bucket_keeps_ohlcv_and_sums_volume() {
        let mut bucket = Bucket::new(Timeframe::M5, values_at(BASE));
        bucket
            .push(CandleValues {
                timestamp: BASE + 60,
                open: 3.0,
                high: 5.0,
                low: 2.0,
                close: 4.0,
                volume: Some(20.0),
            })
            .expect("the bucket accepts the candle");
        for offset in 2..5 {
            bucket
                .push(CandleValues {
                    timestamp: BASE + offset * 60,
                    open: 4.0,
                    high: 4.0,
                    low: 4.0,
                    close: 4.0,
                    volume: Some(1.0),
                })
                .expect("the bucket accepts the candle");
        }

        assert!(bucket.is_complete());
        let candle = bucket.into_values();
        assert_eq!(candle.timestamp, BASE);
        assert_eq!(candle.open, 1.0);
        assert_eq!(candle.high, 5.0);
        assert_eq!(candle.low, 0.5);
        assert_eq!(candle.close, 4.0);
        assert_eq!(candle.volume, Some(33.0));
    }

    #[test]
    fn missing_volume_makes_the_aggregate_volume_unknown() {
        let mut bucket = Bucket::new(Timeframe::M5, values_at(BASE));
        bucket
            .push(CandleValues {
                volume: None,
                ..values_at(BASE + 60)
            })
            .expect("the bucket accepts the candle");

        assert_eq!(bucket.into_values().volume, None);
    }

    #[test]
    fn volume_overflow_reports_the_bucket_and_leaves_it_unchanged() {
        let mut bucket = Bucket::new(
            Timeframe::M5,
            CandleValues {
                volume: Some(1e308),
                ..values_at(BASE)
            },
        );
        let error = bucket
            .push(CandleValues {
                volume: Some(1e308),
                timestamp: BASE + 60,
                ..values_at(BASE + 60)
            })
            .expect_err("the sum overflows");
        assert_eq!(
            error,
            super::BucketError::VolumeOverflow { bucket_start: BASE }
        );
        assert_eq!(bucket.present(), 1);
    }

    #[test]
    fn a_zero_volume_is_kept_as_a_value() {
        let mut bucket = Bucket::new(
            Timeframe::M5,
            CandleValues {
                volume: Some(0.0),
                ..values_at(BASE)
            },
        );
        for offset in 1..5 {
            bucket
                .push(CandleValues {
                    volume: Some(0.0),
                    ..values_at(BASE + offset * 60)
                })
                .expect("the bucket accepts the candle");
        }

        assert_eq!(bucket.into_values().volume, Some(0.0));
    }
}
