//! The data engine.
//!
//! The engine owns candle validation, normalization, the bounded window of
//! recent candles, and construction of M5, M15, and H1 from M1. It is pure: it
//! reads no file, no clock, and no random source. The shell performs source
//! access and hands raw fields to this crate.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod bucket;
mod candle;
mod time;
mod timeframe;
mod timeframe_set;
mod window;

pub use candle::{Candle, CandleError, RawCandle};
pub use time::{TimestampError, format_utc_timestamp, parse_utc_timestamp};
pub use timeframe::Timeframe;
pub use timeframe_set::{Gap, PartialBucket, TimeframeError, TimeframeSeries, TimeframeSet};
pub use window::{CandleValues, CandleWindow, DEFAULT_WINDOW, MAX_SERIES, WindowError};
