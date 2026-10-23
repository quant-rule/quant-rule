//! The data engine.
//!
//! The engine owns candle validation and normalization. It is pure: it reads no
//! file, no clock, and no random source. The shell performs source access and
//! hands raw fields to this crate.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod candle;
mod time;
mod timeframe;

pub use candle::{Candle, CandleError, RawCandle};
pub use time::{TimestampError, format_utc_timestamp, parse_utc_timestamp};
pub use timeframe::Timeframe;
