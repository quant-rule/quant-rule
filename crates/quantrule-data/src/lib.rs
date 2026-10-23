//! The data engine.
//!
//! The engine owns candle validation, normalization, and the bounded window of
//! recent candles. It is pure: it reads no file, no clock, and no random
//! source. The shell performs source access and hands raw fields to this crate.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod candle;
mod time;
mod timeframe;
mod window;

pub use candle::{Candle, CandleError, RawCandle};
pub use time::{TimestampError, format_utc_timestamp, parse_utc_timestamp};
pub use timeframe::Timeframe;
pub use window::{CandleValues, CandleWindow, DEFAULT_WINDOW, WindowError};
