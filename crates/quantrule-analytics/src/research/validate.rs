//! Pre-render validation of the research dataset.
//!
//! The validation runs before any file is returned to the shell. It checks the
//! contract's row counts, bar identity, resolution range and consistency,
//! as-of causality, record availability, dictionary and code completeness, the
//! 100k M1 cap, and the absence of non-finite values in the rendered data.

use std::collections::{BTreeMap, BTreeSet};

use quantrule_data::Candle;

use crate::layers::{LayerSet, M1_SECONDS};
use crate::resolution::{AxisResolution, AxisResolutions};

use super::ExportError;
use super::columns::{AGGREGATES, LAYER1, RAW, RESOLUTIONS, block_columns};
use super::render::prefix;

/// The contract's M1 row cap.
pub(crate) const MAX_M1_ROWS: usize = 100_000;

fn fail(message: &'static str) -> ExportError {
    ExportError::Validation(message)
}

/// Validates the structural invariants of the layers and raw candles.
pub(crate) fn validate_structure(
    layers: &LayerSet,
    raw: &[Candle],
    higher: &[u64],
) -> Result<(), ExportError> {
    if raw.len() > MAX_M1_ROWS {
        return Err(fail("the M1 row count exceeds 100000"));
    }
    if layers.layer1.len() != layers.layer2.len() || layers.layer1.len() != layers.consensus.len() {
        return Err(fail("layer1, layer2, and consensus row counts differ"));
    }
    if layers.layer3.len() != raw.len() {
        return Err(fail("layer3 rows do not match the M1 rows"));
    }
    if higher.contains(&M1_SECONDS) || higher.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(fail(
            "the higher ladder is not strictly ascending or includes M1",
        ));
    }
    if layers.timeframes.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(fail("the configured ladder is not strictly ascending"));
    }

    let mut counts: BTreeMap<u64, usize> = BTreeMap::new();
    let mut group: Option<(u64, usize)> = None;
    for row in &layers.layer1 {
        if !layers.timeframes.contains(&row.timeframe_secs) {
            return Err(fail("a layer1 row names an unconfigured timeframe"));
        }
        match group {
            Some((timeframe, next)) if timeframe == row.timeframe_secs => {
                if row.bar_index != next {
                    return Err(fail("a layer1 bar index is out of order or duplicated"));
                }
                group = Some((timeframe, next + 1));
            }
            Some((timeframe, _)) if row.timeframe_secs < timeframe => {
                return Err(fail("layer1 rows are not grouped by ascending timeframe"));
            }
            _ => {
                if row.bar_index != 0 {
                    return Err(fail("a layer1 timeframe group does not start at bar 0"));
                }
                group = Some((row.timeframe_secs, 1));
            }
        }
        *counts.entry(row.timeframe_secs).or_default() += 1;
    }
    if counts.len() != layers.timeframes.len()
        || layers
            .timeframes
            .iter()
            .any(|timeframe| !counts.contains_key(timeframe))
    {
        return Err(fail("a configured timeframe has no layer1 rows"));
    }

    for (layer1, layer2) in layers.layer1.iter().zip(&layers.layer2) {
        if layer1 != &layer2.layer1 {
            return Err(fail("a layer2 row does not match its layer1 row"));
        }
    }
    for (layer1, consensus) in layers.layer1.iter().zip(&layers.consensus) {
        if layer1.bar_index != consensus.bar_index {
            return Err(fail("consensus rows do not match the layer1 bars"));
        }
    }

    let mut m1_rows: BTreeMap<usize, &crate::layers::Layer2Row> = BTreeMap::new();
    for row in &layers.layer2 {
        if row.layer1.timeframe_secs == M1_SECONDS
            && m1_rows.insert(row.layer1.bar_index, row).is_some()
        {
            return Err(fail("an M1 layer2 bar is duplicated"));
        }
    }
    if layers.layer3.len() > MAX_M1_ROWS {
        return Err(fail("the Layer 3 row count exceeds 100000"));
    }
    for (index, row) in layers.layer3.iter().enumerate() {
        if row.base.layer1.timeframe_secs != M1_SECONDS || row.base.layer1.bar_index != index {
            return Err(fail("a layer3 row does not start at its M1 bar"));
        }
        let Some(expected) = m1_rows.get(&index) else {
            return Err(fail("a layer3 row has no matching M1 layer2 row"));
        };
        if *expected != &row.base {
            return Err(fail("a layer3 row does not match its M1 layer2 row"));
        }
        validate_resolutions(&row.base.resolution)?;
        let Some(base_close) = row.base.layer1.candle.timestamp().checked_add(60) else {
            return Err(fail("an M1 close time overflows"));
        };
        let mut previous: Option<u64> = None;
        for block in &row.higher {
            if !higher.contains(&block.timeframe_secs) {
                return Err(fail("a higher block names an unconfigured timeframe"));
            }
            if previous.is_some_and(|timeframe| timeframe >= block.timeframe_secs) {
                return Err(fail("higher blocks are not strictly ascending"));
            }
            previous = Some(block.timeframe_secs);
            let Ok(seconds) = i64::try_from(block.timeframe_secs) else {
                return Err(fail("a higher timeframe does not fit in i64"));
            };
            let Some(close) = block.candle.timestamp().checked_add(seconds) else {
                return Err(fail("a higher close time overflows"));
            };
            if close > base_close {
                return Err(fail("a higher block closes after its M1 base"));
            }
            let Ok(expected) = u64::try_from(base_close - close) else {
                return Err(fail("a higher staleness is negative"));
            };
            if block.staleness_secs != expected {
                return Err(fail("a higher block staleness is inconsistent"));
            }
            validate_resolutions(&block.resolution)?;
        }
    }

    for event in &layers.events {
        if event.available_bar_index < event.occurred_bar_index {
            return Err(fail("an event is available before it occurred"));
        }
    }
    for level in &layers.levels {
        if level.available_bar_index < level.occurred_bar_index {
            return Err(fail("a level is available before it occurred"));
        }
    }
    for pivot in &layers.pivots {
        if pivot.available_bar_index < pivot.occurred_bar_index {
            return Err(fail("a pivot is available before it occurred"));
        }
    }
    for leg in &layers.legs {
        match (leg.origin_confirmed_code, leg.origin_available_bar_index) {
            (1, Some(available)) if available >= leg.origin_occurred_bar_index => {}
            (0, None) => {}
            (1, _) => {
                return Err(fail(
                    "a confirmed leg's origin availability precedes its occurrence",
                ));
            }
            (0, _) => return Err(fail("a forming leg has an availability bar")),
            _ => return Err(fail("a leg has an invalid origin confirmation code")),
        }
    }

    Ok(())
}

fn validate_resolutions(resolutions: &AxisResolutions) -> Result<(), ExportError> {
    for axis in [
        &resolutions.direction,
        &resolutions.momentum,
        &resolutions.structure,
        &resolutions.progression,
    ] {
        validate_resolution(axis)?;
    }
    Ok(())
}

fn validate_resolution(resolution: &AxisResolution) -> Result<(), ExportError> {
    match (resolution.continuous, resolution.ordinal, resolution.reason) {
        (Some(value), Some(ordinal), None)
            if value.is_finite() && (-1.0..=1.0).contains(&value) && ordinal <= 9 =>
        {
            Ok(())
        }
        (None, None, Some(reason)) if (1..=6).contains(&reason) => Ok(()),
        _ => Err(fail("an axis resolution is out of range or inconsistent")),
    }
}

/// Validates the rendered data files, the dictionary, and the code table.
pub(crate) fn validate_rendered(
    higher: &[u64],
    data_files: &[(&str, &str)],
) -> Result<(), ExportError> {
    let expected = expected_columns(higher);
    let mut contents: BTreeMap<&str, &str> = BTreeMap::new();
    for (name, text) in data_files {
        if contents.insert(*name, *text).is_some() {
            return Err(fail("a data file is rendered twice"));
        }
    }
    let registry = code_registry(&super::metadata::codes_csv());
    if registry.is_empty() {
        return Err(fail("the code table is empty"));
    }

    for (file, columns) in &expected {
        let Some(text) = contents.get(*file) else {
            return Err(fail("a data file is missing"));
        };
        let mut lines = text.lines();
        let Some(header) = lines.next() else {
            return Err(fail("a data file has no header"));
        };
        let headers: Vec<&str> = header.split(',').collect();
        if headers.len() != columns.len() {
            return Err(fail("a data file header does not match the registry"));
        }
        for (actual, (expected, _)) in headers.iter().zip(columns) {
            if actual != expected {
                return Err(fail("a data file header does not match the registry"));
            }
        }
        for line in lines {
            let fields: Vec<&str> = line.split(',').collect();
            if fields.len() != columns.len() {
                return Err(fail("a data row does not match the header"));
            }
            for (field, (_, codes)) in fields.iter().zip(columns) {
                for token in field.split(';') {
                    if token.is_empty() {
                        continue;
                    }
                    if let Ok(value) = token.parse::<f64>()
                        && !value.is_finite()
                    {
                        return Err(fail("a data file writes a non-finite value"));
                    }
                    if !codes.is_empty() {
                        let Some(allowed) = registry.get(*codes) else {
                            return Err(fail("a code registry referenced by the data is missing"));
                        };
                        if !allowed.contains(token) {
                            return Err(fail("a code value has no row in codes.csv"));
                        }
                    }
                }
            }
        }
    }

    let dictionary = super::metadata::dictionary_csv(higher);
    let mut documented: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for line in dictionary.lines().skip(1) {
        let mut parts = line.splitn(4, ',');
        let Some(file) = parts.next() else {
            return Err(fail("a dictionary row has no file"));
        };
        let Some(_position) = parts.next() else {
            return Err(fail("a dictionary row has no position"));
        };
        let Some(column) = parts.next() else {
            return Err(fail("a dictionary row has no column"));
        };
        *documented
            .entry(file.to_owned())
            .or_default()
            .entry(column.to_owned())
            .or_default() += 1;
    }
    for (file, columns) in &expected {
        let Some(documented) = documented.get(*file) else {
            return Err(fail("a data file has no dictionary rows"));
        };
        if documented.len() != columns.len() {
            return Err(fail("the dictionary does not match the data header"));
        }
        for (column, _) in columns {
            if documented.get(column).copied() != Some(1) {
                return Err(fail("the dictionary does not match the data header"));
            }
        }
    }

    Ok(())
}

fn code_registry(csv: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut registry: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for line in csv.lines().skip(1) {
        let mut parts = line.splitn(3, ',');
        let Some(name) = parts.next() else {
            continue;
        };
        let Some(code) = parts.next() else {
            continue;
        };
        registry
            .entry(name.to_owned())
            .or_default()
            .insert(code.to_owned());
    }
    registry
}

fn expected_columns(higher: &[u64]) -> Vec<(&'static str, Vec<(String, &'static str)>)> {
    let raw: Vec<(String, &'static str)> = RAW
        .iter()
        .map(|column| (column.name.to_owned(), column.codes))
        .collect();
    let layer1: Vec<(String, &'static str)> = LAYER1
        .iter()
        .map(|column| (column.name.to_owned(), column.codes))
        .collect();
    let mut layer2 = layer1.clone();
    layer2.extend(
        RESOLUTIONS
            .iter()
            .map(|column| (column.name.to_owned(), column.codes)),
    );
    let mut layer3 = layer2.clone();
    for timeframe in higher {
        let prefix = prefix(*timeframe);
        for column in block_columns() {
            layer3.push((format!("{prefix}{}", column.name), column.codes));
        }
    }
    layer3.extend(
        AGGREGATES
            .iter()
            .map(|column| (column.name.to_owned(), column.codes)),
    );
    vec![
        ("m1.csv", raw),
        ("layer1.csv", layer1),
        ("layer2.csv", layer2),
        ("layer3.csv", layer3),
    ]
}
