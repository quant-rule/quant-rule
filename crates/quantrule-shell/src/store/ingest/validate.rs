//! Pre-transaction validation for the dataset writer.
//!
//! The writer validates the whole record set before it opens the transaction,
//! so an invalid row cannot leave a partial dataset behind. Validation checks
//! shape and contract ranges only; it does not recompute a resolution, a
//! consensus count, or an aggregate.

use std::collections::{BTreeMap, BTreeSet};

use quantrule_analytics::layers::{
    ConsensusRow, Layer2Row, Layer3Row, LevelRow, M1_SECONDS, PivotRow,
};
use quantrule_analytics::resolution::{AxisResolution, ordinal_bucket};
use quantrule_model::aggregate::CrossTimeframeAggregates;
use quantrule_model::axes::structure::registry::FIRST_REFERENCE_CODE;

use super::{DatasetRecord, IngestRows, MAX_M1_BARS};
use crate::store::StoreError;

pub(super) fn validate(dataset: &DatasetRecord, rows: &IngestRows<'_>) -> Result<(), StoreError> {
    if dataset.name.is_empty() {
        return Err(StoreError::InvalidDataset {
            detail: "the dataset name is empty".to_string(),
        });
    }
    if dataset.data_version.is_empty() {
        return Err(StoreError::InvalidDataset {
            detail: "the data version is empty".to_string(),
        });
    }
    if dataset.m1_bars > MAX_M1_BARS {
        return Err(StoreError::InvalidDataset {
            detail: format!(
                "the M1 bar count {} exceeds the {MAX_M1_BARS} cap",
                dataset.m1_bars
            ),
        });
    }

    let mut bar_counts = BTreeMap::new();
    for row in rows.bars {
        *bar_counts
            .entry(row.layer1.timeframe_secs)
            .or_insert(0_usize) += 1;
        validate_resolutions(row)?;
    }
    let m1_rows = bar_counts.get(&M1_SECONDS).copied().unwrap_or(0);
    if m1_rows != dataset.m1_bars {
        return Err(StoreError::InvalidDataset {
            detail: format!(
                "the dataset declares {} M1 bars but the layer has {m1_rows}",
                dataset.m1_bars
            ),
        });
    }

    validate_pivots(rows.pivots, &bar_counts)?;
    validate_levels(rows.levels, &bar_counts)?;
    validate_consensus(rows.consensus, rows.bars, &bar_counts)?;
    validate_layer3(rows.layer3, rows.bars, dataset.m1_bars)?;
    Ok(())
}

fn validate_pivots(rows: &[PivotRow], bar_counts: &BTreeMap<u64, usize>) -> Result<(), StoreError> {
    let mut seen = BTreeSet::new();
    for row in rows {
        let bar_count = timeframe_bar_count(bar_counts, row.timeframe_secs, "pivot")?;
        if !(1..=2).contains(&row.scale_code) {
            return Err(invalid(format!(
                "the pivot scale code {} is not 1 or 2",
                row.scale_code
            )));
        }
        if !(1..=2).contains(&row.kind_code) {
            return Err(invalid(format!(
                "the pivot kind code {} is not 1 or 2",
                row.kind_code
            )));
        }
        if !row.price.is_finite() {
            return Err(invalid(format!(
                "the pivot at bar {} has a non-finite price",
                row.occurred_bar_index
            )));
        }
        check_bar_range(
            bar_count,
            row.occurred_bar_index,
            row.available_bar_index,
            "pivot",
        )?;
        if row.available_bar_index < row.occurred_bar_index {
            return Err(invalid(format!(
                "the pivot at bar {} is available at bar {}, before it occurred",
                row.occurred_bar_index, row.available_bar_index
            )));
        }
        if row.confirmed_code > 1 {
            return Err(invalid(format!(
                "the pivot at bar {} has confirmed code {}",
                row.occurred_bar_index, row.confirmed_code
            )));
        }
        if row.confirmed_code == 0 && row.available_bar_index != row.occurred_bar_index {
            return Err(invalid(format!(
                "the candidate at bar {} is available at bar {}",
                row.occurred_bar_index, row.available_bar_index
            )));
        }
        let key = (
            row.timeframe_secs,
            row.scale_code,
            row.kind_code,
            row.occurred_bar_index,
        );
        if !seen.insert(key) {
            return Err(invalid(format!(
                "the pivot at timeframe {} bar {} is duplicated",
                row.timeframe_secs, row.occurred_bar_index
            )));
        }
    }
    Ok(())
}

fn validate_levels(rows: &[LevelRow], bar_counts: &BTreeMap<u64, usize>) -> Result<(), StoreError> {
    let mut seen = BTreeSet::new();
    for row in rows {
        let bar_count = timeframe_bar_count(bar_counts, row.timeframe_secs, "level")?;
        if row.reference_code < FIRST_REFERENCE_CODE {
            return Err(invalid(format!(
                "the level reference code {} is below {FIRST_REFERENCE_CODE}",
                row.reference_code
            )));
        }
        if !(1..=2).contains(&row.scale_code) {
            return Err(invalid(format!(
                "the level scale code {} is not 1 or 2",
                row.scale_code
            )));
        }
        if !(1..=2).contains(&row.kind_code) {
            return Err(invalid(format!(
                "the level kind code {} is not 1 or 2",
                row.kind_code
            )));
        }
        if !row.price.is_finite() {
            return Err(invalid(format!(
                "the level at bar {} has a non-finite price",
                row.occurred_bar_index
            )));
        }
        check_bar_range(
            bar_count,
            row.occurred_bar_index,
            row.available_bar_index,
            "level",
        )?;
        if row.available_bar_index < row.occurred_bar_index {
            return Err(invalid(format!(
                "the level at bar {} is available at bar {}, before it occurred",
                row.occurred_bar_index, row.available_bar_index
            )));
        }
        let key = (row.timeframe_secs, row.reference_code);
        if !seen.insert(key) {
            return Err(invalid(format!(
                "the level reference code {} is duplicated for timeframe {}",
                row.reference_code, row.timeframe_secs
            )));
        }
    }
    Ok(())
}

fn validate_consensus(
    rows: &[ConsensusRow],
    bars: &[Layer2Row],
    bar_counts: &BTreeMap<u64, usize>,
) -> Result<(), StoreError> {
    let expected: BTreeSet<(u64, usize)> = bars
        .iter()
        .map(|row| (row.layer1.timeframe_secs, row.layer1.bar_index))
        .collect();
    let mut seen = BTreeSet::new();
    for row in rows {
        let timeframe_secs = row.consensus.timeframe_secs;
        let bar_count = timeframe_bar_count(bar_counts, timeframe_secs, "consensus")?;
        if row.bar_index >= bar_count {
            return Err(invalid(format!(
                "the consensus at timeframe {timeframe_secs} bar {} is outside the {bar_count} bars",
                row.bar_index
            )));
        }
        if !seen.insert((timeframe_secs, row.bar_index)) {
            return Err(invalid(format!(
                "the consensus at timeframe {timeframe_secs} bar {} is duplicated",
                row.bar_index
            )));
        }
        let counts = [
            row.consensus.bullish_axis_count,
            row.consensus.bearish_axis_count,
            row.consensus.neutral_axis_count,
        ];
        if counts.iter().any(|count| *count > 3) {
            return Err(invalid(format!(
                "the consensus at timeframe {timeframe_secs} bar {} has a count outside 0..=3",
                row.bar_index
            )));
        }
        let sum: u16 = counts.iter().map(|count| u16::from(*count)).sum();
        if sum > 3 {
            return Err(invalid(format!(
                "the consensus at timeframe {timeframe_secs} bar {} counts {sum} axes",
                row.bar_index
            )));
        }
        if let Some(code) = row.consensus.weakest_directional_axis_code
            && !(1..=3).contains(&code)
        {
            return Err(invalid(format!(
                "the consensus at timeframe {timeframe_secs} bar {} has weakest axis code {code}",
                row.bar_index
            )));
        }
    }
    if seen != expected {
        return Err(invalid(format!(
            "the consensus keys do not match the {} bars",
            expected.len()
        )));
    }
    Ok(())
}

fn validate_layer3(
    rows: &[Layer3Row],
    bars: &[Layer2Row],
    m1_bars: usize,
) -> Result<(), StoreError> {
    let expected: BTreeSet<usize> = bars
        .iter()
        .filter(|row| row.layer1.timeframe_secs == M1_SECONDS)
        .map(|row| row.layer1.bar_index)
        .collect();
    let configured: BTreeSet<u64> = bars.iter().map(|row| row.layer1.timeframe_secs).collect();
    let mut seen = BTreeSet::new();
    for row in rows {
        if row.base.layer1.timeframe_secs != M1_SECONDS {
            return Err(invalid(format!(
                "the Layer 3 row at bar {} has base timeframe {}",
                row.base.layer1.bar_index, row.base.layer1.timeframe_secs
            )));
        }
        let bar_index = row.base.layer1.bar_index;
        if bar_index >= m1_bars {
            return Err(invalid(format!(
                "the Layer 3 bar {bar_index} is outside the {m1_bars} M1 bars"
            )));
        }
        if !seen.insert(bar_index) {
            return Err(invalid(format!(
                "the Layer 3 bar {bar_index} is duplicated"
            )));
        }
        validate_aggregates(&row.aggregates, &configured, bar_index)?;
    }
    if seen != expected {
        return Err(invalid(format!(
            "the Layer 3 keys do not match the {} M1 bars",
            expected.len()
        )));
    }
    Ok(())
}

fn validate_aggregates(
    aggregates: &CrossTimeframeAggregates,
    configured: &BTreeSet<u64>,
    bar_index: usize,
) -> Result<(), StoreError> {
    let location = format!("the Layer 3 bar {bar_index}");
    validate_timeframe_list(
        &aggregates.ladder,
        configured,
        &aggregates.ladder,
        &location,
        "ladder",
        false,
    )?;
    if aggregates.ladder.first() != Some(&M1_SECONDS) {
        return Err(invalid(format!("{location} ladder does not start with M1")));
    }
    for (list, name) in [
        (&aggregates.direction.timeframes, "direction"),
        (&aggregates.momentum.timeframes, "momentum"),
        (&aggregates.structure.timeframes, "structure"),
        (&aggregates.progression.timeframes, "progression"),
    ] {
        validate_timeframe_list(list, configured, &aggregates.ladder, &location, name, true)?;
    }
    validate_aggregate_floats(aggregates, &location)
}

fn validate_timeframe_list(
    list: &[u64],
    configured: &BTreeSet<u64>,
    ladder: &[u64],
    location: &str,
    name: &str,
    allow_empty: bool,
) -> Result<(), StoreError> {
    if list.is_empty() {
        if allow_empty {
            return Ok(());
        }
        return Err(invalid(format!("{location} has an empty {name}")));
    }
    if list.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(invalid(format!(
            "{location} has an unsorted or duplicate {name} list"
        )));
    }
    for timeframe in list {
        if !configured.contains(timeframe) {
            return Err(invalid(format!(
                "{location} has {name} timeframe {timeframe} with no bars"
            )));
        }
        if !ladder.contains(timeframe) {
            return Err(invalid(format!(
                "{location} has {name} timeframe {timeframe} outside the ladder"
            )));
        }
    }
    Ok(())
}

fn validate_aggregate_floats(
    aggregates: &CrossTimeframeAggregates,
    location: &str,
) -> Result<(), StoreError> {
    let direction = &aggregates.direction;
    let momentum = &aggregates.momentum;
    let structure = &aggregates.structure;
    let progression = &aggregates.progression;
    let events = &aggregates.events;
    let values = [
        ("direction mean", direction.direction_mean),
        ("direction bullish share", direction.direction_bullish_share),
        ("direction bearish share", direction.direction_bearish_share),
        ("direction neutral share", direction.direction_neutral_share),
        ("direction gradient", direction.direction_gradient),
        ("momentum force mean", momentum.momentum_force_mean),
        (
            "momentum acceleration mean",
            momentum.momentum_acceleration_mean,
        ),
        ("momentum positive share", momentum.momentum_positive_share),
        ("momentum negative share", momentum.momentum_negative_share),
        (
            "momentum force fast-slow gap",
            momentum.momentum_force_fast_slow_gap,
        ),
        (
            "momentum acceleration fast-slow gap",
            momentum.momentum_acceleration_fast_slow_gap,
        ),
        ("momentum gradient", momentum.momentum_gradient),
        ("structure bullish share", structure.structure_bullish_share),
        ("structure bearish share", structure.structure_bearish_share),
        ("structure neutral share", structure.structure_neutral_share),
        ("structure gradient", structure.structure_gradient),
        ("structure ready share", structure.structure_ready_share),
        (
            "largest level cluster span",
            structure.largest_level_cluster_span_atr,
        ),
        (
            "minor modal stage share",
            progression.minor.modal_stage_share,
        ),
        (
            "major modal stage share",
            progression.major.modal_stage_share,
        ),
        (
            "progression ready share",
            progression.progression_ready_share,
        ),
        (
            "leg direction agreement share",
            progression.leg_direction_agreement_share,
        ),
        (
            "divergence direction agreement share",
            events.divergence_direction_agreement_share,
        ),
    ];
    for (name, value) in values {
        if let Some(value) = value
            && !value.is_finite()
        {
            return Err(invalid(format!("{location} has non-finite {name}")));
        }
    }
    Ok(())
}

fn timeframe_bar_count(
    bar_counts: &BTreeMap<u64, usize>,
    timeframe_secs: u64,
    record: &str,
) -> Result<usize, StoreError> {
    bar_counts.get(&timeframe_secs).copied().ok_or_else(|| {
        invalid(format!(
            "the {record} timeframe {timeframe_secs} has no bars"
        ))
    })
}

fn check_bar_range(
    bar_count: usize,
    occurred_bar_index: usize,
    available_bar_index: usize,
    record: &str,
) -> Result<(), StoreError> {
    if occurred_bar_index >= bar_count {
        return Err(invalid(format!(
            "the {record} occurrence bar {occurred_bar_index} is outside the {bar_count} bars"
        )));
    }
    if available_bar_index >= bar_count {
        return Err(invalid(format!(
            "the {record} availability bar {available_bar_index} is outside the {bar_count} bars"
        )));
    }
    Ok(())
}

fn invalid(detail: impl Into<String>) -> StoreError {
    StoreError::InvalidDataset {
        detail: detail.into(),
    }
}

fn validate_resolutions(row: &Layer2Row) -> Result<(), StoreError> {
    let layer1 = &row.layer1;
    for (axis, resolution) in [
        ("direction", row.resolution.direction),
        ("momentum", row.resolution.momentum),
        ("structure", row.resolution.structure),
        ("progression", row.resolution.progression),
    ] {
        validate_resolution(axis, layer1.timeframe_secs, layer1.bar_index, resolution)?;
    }
    Ok(())
}

fn validate_resolution(
    axis: &str,
    timeframe_secs: u64,
    bar_index: usize,
    resolution: AxisResolution,
) -> Result<(), StoreError> {
    let location = format!("the {axis} resolution at timeframe {timeframe_secs} bar {bar_index}");
    match resolution.continuous {
        Some(value) => {
            if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                return Err(invalid(format!("{location} has continuous value {value}")));
            }
            if let Some(reason) = resolution.reason {
                return Err(invalid(format!(
                    "{location} carries reason {reason} with a value"
                )));
            }
            let Some(ordinal) = resolution.ordinal else {
                return Err(invalid(format!(
                    "{location} has a continuous value but no ordinal"
                )));
            };
            if ordinal > 9 {
                return Err(invalid(format!(
                    "{location} has ordinal {ordinal}, outside 0..=9"
                )));
            }
            let expected = ordinal_bucket(value);
            if ordinal != expected {
                return Err(invalid(format!(
                    "{location} has ordinal {ordinal} but bucket {expected}"
                )));
            }
        }
        None => {
            if resolution.ordinal.is_some() {
                return Err(invalid(format!(
                    "{location} has an ordinal without a continuous value"
                )));
            }
            let Some(reason) = resolution.reason else {
                return Err(invalid(format!(
                    "{location} has no continuous value and no reason"
                )));
            };
            if !(1..=6).contains(&reason) {
                return Err(invalid(format!(
                    "{location} has reason {reason}, outside 1..=6"
                )));
            }
        }
    }
    Ok(())
}
