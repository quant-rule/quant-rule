//! Raw source columns.

use super::{Column, c};

/// The raw file's six columns.
pub(crate) const RAW: [Column; 6] = [
    c(
        "Date",
        "candle",
        "string",
        "UTC timestamp",
        false,
        "",
        "",
        "Bar open time as a UTC timestamp.",
    ),
    c(
        "Open",
        "candle",
        "float",
        "finite",
        false,
        "",
        "",
        "Open price.",
    ),
    c(
        "High",
        "candle",
        "float",
        "finite",
        false,
        "",
        "",
        "High price.",
    ),
    c(
        "Low",
        "candle",
        "float",
        "finite",
        false,
        "",
        "",
        "Low price.",
    ),
    c(
        "Close",
        "candle",
        "float",
        "finite",
        false,
        "",
        "",
        "Close price.",
    ),
    c(
        "Volume",
        "candle",
        "float",
        ">= 0",
        true,
        "",
        "",
        "Volume; empty when the source supplied none.",
    ),
];
