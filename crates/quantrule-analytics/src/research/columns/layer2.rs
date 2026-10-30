//! Layer 2 columns and the higher-timeframe block.

use super::{Column, LAYER1, c, layer1_names};

/// Layer 2's 12 appended resolution columns, in file order.
pub(crate) const RESOLUTIONS: [Column; 12] = [
    c(
        "direction_continuous",
        "resolution",
        "float",
        "-1..+1",
        true,
        "direction_resolution_reason_code",
        "",
        "Direction resolution value.",
    ),
    c(
        "direction_ordinal",
        "resolution",
        "int",
        "0..9",
        true,
        "",
        "resolution_bucket",
        "Direction equal-width bucket.",
    ),
    c(
        "direction_resolution_reason_code",
        "resolution",
        "int",
        "1..6",
        true,
        "",
        "unavailable_reason",
        "Why the Direction resolution is null.",
    ),
    c(
        "momentum_continuous",
        "resolution",
        "float",
        "-1..+1",
        true,
        "momentum_resolution_reason_code",
        "",
        "Momentum resolution value.",
    ),
    c(
        "momentum_ordinal",
        "resolution",
        "int",
        "0..9",
        true,
        "",
        "resolution_bucket",
        "Momentum equal-width bucket.",
    ),
    c(
        "momentum_resolution_reason_code",
        "resolution",
        "int",
        "1..6",
        true,
        "",
        "unavailable_reason",
        "Why the Momentum resolution is null.",
    ),
    c(
        "structure_continuous",
        "resolution",
        "float",
        "-1..+1",
        true,
        "structure_resolution_reason_code",
        "",
        "Structure resolution value.",
    ),
    c(
        "structure_ordinal",
        "resolution",
        "int",
        "0..9",
        true,
        "",
        "resolution_bucket",
        "Structure equal-width bucket.",
    ),
    c(
        "structure_resolution_reason_code",
        "resolution",
        "int",
        "1..6",
        true,
        "",
        "unavailable_reason",
        "Why the Structure resolution is null.",
    ),
    c(
        "progression_continuous",
        "resolution",
        "float",
        "-1..+1",
        true,
        "progression_resolution_reason_code",
        "",
        "Clamped Progression resolution value.",
    ),
    c(
        "progression_ordinal",
        "resolution",
        "int",
        "0..9",
        true,
        "",
        "resolution_bucket",
        "Progression equal-width bucket.",
    ),
    c(
        "progression_resolution_reason_code",
        "resolution",
        "int",
        "1..6",
        true,
        "",
        "unavailable_reason",
        "Why the Progression resolution is null.",
    ),
];

/// Layer 2's column names, in file order.
#[must_use]
pub fn layer2_names() -> Vec<&'static str> {
    let mut names = layer1_names();
    names.extend(RESOLUTIONS.iter().map(|column| column.name));
    names
}

/// The block's trailing staleness column.
pub(crate) const STALENESS: Column = c(
    "staleness_secs",
    "meta",
    "int",
    ">= 0",
    false,
    "",
    "",
    "Seconds between the higher bar's close and the M1 bar's close.",
);

/// One higher-timeframe block's 67 columns, before the prefix.
pub(crate) fn block_columns() -> Vec<Column> {
    let mut columns = Vec::with_capacity(67);
    columns.push(LAYER1[1]);
    columns.extend_from_slice(&LAYER1[2..8]);
    columns.extend_from_slice(&LAYER1[14..61]);
    columns.extend_from_slice(&RESOLUTIONS);
    columns.push(STALENESS);
    columns
}
