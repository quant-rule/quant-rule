//! The three research layers.
//!
//! Layer 1 is the candle, readings, and metrics per bar; Layer 2 adds the four
//! axis resolutions; Layer 3 is the M1 base with the latest closed
//! higher-timeframe blocks and the cross-timeframe aggregates. The layers are
//! derived only from the model's public per-bar output and the research
//! contract in `docs/analytics/`. The set also carries the model's pivot,
//! confirmed-level, event, and retained-leg records, which the store writes but
//! the CSV export does not.
//!
//! The module is pure: it performs no I/O, reads no clock, and uses no
//! randomness. Rows are ordered by timeframe and then by bar, so reordering the
//! inputs changes nothing. Layer 3 requires an M1 base; a dataset without one
//! has no Layer 3 rows.
//!
//! The column widths fixed by the contract are [`LAYER1_COLUMNS`],
//! [`LAYER2_COLUMNS`], [`HIGHER_BLOCK_COLUMNS`], and [`AGGREGATE_COLUMNS`]. The
//! export asserts them against the CSV header.

mod consensus;
mod metrics;
mod records;
mod series;
#[cfg(test)]
mod tests;

pub use consensus::ConsensusRow;
pub use metrics::{
    ContextMetrics, DirectionMetrics, Metrics, MomentumMetrics, ProgressionMetrics,
    ReadingsSnapshot, StructureMetrics,
};
pub use records::{EventRow, LegRow, LevelRow, PivotRow};

use std::fmt;

use quantrule_data::Candle;
use quantrule_model::aggregate::CrossTimeframeAggregates;
use quantrule_model::input::{MarketStateConfig, TimeframeInput};
use quantrule_model::readings::PivotScale;
use quantrule_model::run::ModelRun;
use quantrule_model::timeframe::TimeframeModel;

use crate::alignment;
use crate::resolution::{AxisResolution, AxisResolutions, clamp_unit};
use series::TimeframeSeries;

/// M1 in seconds: the Layer 3 base timeframe.
pub const M1_SECONDS: u64 = 60;

/// The number of columns in `layer1.csv`.
pub const LAYER1_COLUMNS: usize = 61;

/// The number of columns in `layer2.csv`.
pub const LAYER2_COLUMNS: usize = 73;

/// The number of columns in one higher-timeframe block of `layer3.csv`.
pub const HIGHER_BLOCK_COLUMNS: usize = 67;

/// The number of aggregate columns in `layer3.csv`.
pub const AGGREGATE_COLUMNS: usize = 82;

/// The number of columns in `layer3.csv` for a number of higher timeframes.
#[must_use]
pub const fn layer3_columns(higher_timeframes: usize) -> usize {
    LAYER2_COLUMNS + higher_timeframes * HIGHER_BLOCK_COLUMNS + AGGREGATE_COLUMNS
}

/// The configuration of a layer build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerConfig {
    /// The pivot scale used by Structure and Divergence.
    pub structure_scale: PivotScale,
}

impl LayerConfig {
    /// Builds the configuration for a structure scale.
    #[must_use]
    pub const fn new(structure_scale: PivotScale) -> Self {
        Self { structure_scale }
    }
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            structure_scale: PivotScale::Major,
        }
    }
}

/// One Layer 1 row: a candle, its readings, and its published metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layer1Row {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The 0-based closed-bar position in the timeframe series.
    pub bar_index: usize,
    /// The candle.
    pub candle: Candle,
    /// The scalar readings.
    pub readings: ReadingsSnapshot,
    /// The published metrics.
    pub metrics: Metrics,
}

/// One Layer 2 row: a Layer 1 row plus the four axis resolutions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layer2Row {
    /// The Layer 1 fields.
    pub layer1: Layer1Row,
    /// The four axis resolutions.
    pub resolution: AxisResolutions,
}

impl Layer2Row {
    /// Adds the four axis resolutions to a Layer 1 row.
    ///
    /// The resolutions are derived from the row's own published metrics by the
    /// same mapping the layer build uses. Progression's raw `progress` is
    /// clamped for the resolution; the Layer 1 field keeps the raw value.
    #[must_use]
    pub fn from_layer1(layer1: Layer1Row) -> Self {
        Self {
            resolution: resolutions(&layer1.metrics),
            layer1,
        }
    }
}

/// One as-of higher-timeframe block of a Layer 3 row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HigherBlock {
    /// The higher timeframe in seconds.
    pub timeframe_secs: u64,
    /// The as-of bar's 0-based position in its timeframe series.
    pub bar_index: usize,
    /// The as-of bar's candle.
    pub candle: Candle,
    /// The as-of bar's published metrics.
    pub metrics: Metrics,
    /// The as-of bar's four axis resolutions.
    pub resolution: AxisResolutions,
    /// Seconds between the higher bar's close and the M1 bar's close.
    pub staleness_secs: u64,
}

/// One Layer 3 row: the M1 base, the as-of blocks, and the aggregates.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer3Row {
    /// The M1 base row.
    pub base: Layer2Row,
    /// The as-of blocks, fastest first. A timeframe with no closed bar yet is
    /// absent; the export writes its block as nulls.
    pub higher: Vec<HigherBlock>,
    /// The cross-timeframe aggregates over the base and the present blocks.
    pub aggregates: CrossTimeframeAggregates,
}

/// The three layers and the model records for one dataset.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerSet {
    /// The configured non-empty timeframes in seconds, fastest first.
    pub timeframes: Vec<u64>,
    /// Layer 1 rows, ordered by timeframe and then bar.
    pub layer1: Vec<Layer1Row>,
    /// Layer 2 rows, in the same order as [`Self::layer1`].
    pub layer2: Vec<Layer2Row>,
    /// Per-bar consensus, ordered by timeframe and then bar, in the same order
    /// as [`Self::layer1`].
    pub consensus: Vec<ConsensusRow>,
    /// Layer 3 rows, one per M1 bar.
    pub layer3: Vec<Layer3Row>,
    /// Pivot occurrences, ordered by timeframe, occurrence bar, scale, and kind.
    pub pivots: Vec<PivotRow>,
    /// Confirmed levels, ordered by timeframe and reference code.
    pub levels: Vec<LevelRow>,
    /// Event records, ordered by timeframe, availability bar, type, subtype,
    /// source, occurrence bar, reference, and reference value.
    pub events: Vec<EventRow>,
    /// Retained progression legs, ordered by timeframe and `leg_id`.
    pub legs: Vec<LegRow>,
}

/// A layer build could not be performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerError {
    /// A timeframe is zero seconds.
    ZeroTimeframe,
    /// Two non-empty inputs name the same timeframe.
    DuplicateTimeframe {
        /// The repeated timeframe in seconds.
        timeframe_secs: u64,
    },
}

impl fmt::Display for LayerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroTimeframe => {
                write!(formatter, "a timeframe must be greater than zero seconds")
            }
            Self::DuplicateTimeframe { timeframe_secs } => {
                write!(
                    formatter,
                    "timeframe {timeframe_secs} seconds appears more than once"
                )
            }
        }
    }
}

impl std::error::Error for LayerError {}

/// Builds Layers 1, 2, and 3 from the model's per-timeframe candles.
///
/// Empty inputs are omitted and do not appear in [`LayerSet::timeframes`].
/// Layer 3 uses only bars closed at or before the M1 bar's close, so a later
/// bar can never change an earlier row.
///
/// # Errors
///
/// Returns [`LayerError::ZeroTimeframe`] when an input names a zero timeframe,
/// and [`LayerError::DuplicateTimeframe`] when two non-empty inputs name the
/// same timeframe.
pub fn build_layers(
    inputs: &[TimeframeInput<'_>],
    config: &LayerConfig,
) -> Result<LayerSet, LayerError> {
    let mut ordered: Vec<&TimeframeInput<'_>> = inputs
        .iter()
        .filter(|input| !input.candles.is_empty())
        .collect();
    ordered.sort_by_key(|input| input.timeframe_secs);

    if let Some(pair) = ordered
        .windows(2)
        .find(|pair| pair[0].timeframe_secs == pair[1].timeframe_secs)
    {
        return Err(LayerError::DuplicateTimeframe {
            timeframe_secs: pair[0].timeframe_secs,
        });
    }

    let timeframes: Vec<u64> = ordered.iter().map(|input| input.timeframe_secs).collect();
    let model_config = MarketStateConfig::new(config.structure_scale);

    let mut models: Vec<TimeframeModel> = Vec::with_capacity(ordered.len());
    for input in &ordered {
        let model =
            TimeframeModel::compute(input, &model_config).ok_or(LayerError::ZeroTimeframe)?;
        models.push(model);
    }

    let mut series = Vec::with_capacity(models.len());
    for model in &models {
        series.push(TimeframeSeries::new(model)?);
    }

    Ok(assemble(timeframes, series))
}

/// Builds the research layers from an already computed model run.
///
/// The run's per-timeframe models supply every bar; the layers do not recompute
/// the readings or the axes, and they borrow the run instead of cloning it. The
/// run's structure scale must match `config`; the shell's cross-config
/// validation enforces that.
///
/// `timeframes` selects the subset to publish, in the run's ascending order. A
/// timeframe that the run does not have is omitted.
///
/// # Errors
///
/// Returns [`LayerError::ZeroTimeframe`] when a selected run timeframe is zero.
pub fn build_layers_from_run(
    run: &ModelRun,
    timeframes: &[u64],
    _config: &LayerConfig,
) -> Result<LayerSet, LayerError> {
    let mut series = Vec::with_capacity(run.timeframes().len());
    for timeframe in run.timeframes() {
        if !timeframes.contains(&timeframe.timeframe_secs()) {
            continue;
        }
        series.push(TimeframeSeries::new(timeframe)?);
    }
    let selected = series.iter().map(TimeframeSeries::timeframe_secs).collect();
    Ok(assemble(selected, series))
}

/// Summarizes a model run for the selected timeframes without building the
/// layers.
///
/// The report needs the axis summaries, the correlations, and the layer row
/// counts, not the rows themselves. Building them for a full year costs several
/// gigabytes; this path streams one bar at a time and keeps only the
/// accumulators. The numbers are the same as [`crate::summary::summarize`] over
/// [`build_layers_from_run`] for the same run and timeframes.
///
/// Layer 3 is published only for an M1 base. Its row count is the number of M1
/// bars that publish a snapshot, which is the condition the layer build uses.
#[must_use]
pub fn summarize_model_run(run: &ModelRun, timeframes: &[u64]) -> crate::summary::DatasetSummary {
    let mut builder = crate::summary::DatasetSummaryBuilder::new();
    let mut layer3_rows = 0;
    for model in run.timeframes() {
        if !timeframes.contains(&model.timeframe_secs()) {
            continue;
        }
        builder.add_timeframe(
            model.timeframe_secs(),
            (0..model.len()).filter_map(|bar| {
                let bar = model.bar(bar)?;
                let metrics = Metrics::from_bars(
                    &bar.direction,
                    &bar.momentum,
                    &bar.structure,
                    &bar.progression,
                    &bar.context,
                );
                Some(resolutions(&metrics))
            }),
        );
        if model.timeframe_secs() == M1_SECONDS {
            layer3_rows = (0..model.len())
                .filter(|bar| model.snapshot_at(*bar).is_some())
                .count();
        }
    }
    builder.set_layer3_rows(layer3_rows);
    builder.build()
}

fn assemble(timeframes: Vec<u64>, mut series: Vec<TimeframeSeries<'_>>) -> LayerSet {
    let mut layer1 = Vec::new();
    let mut layer2 = Vec::new();
    let mut consensus = Vec::new();
    let mut pivots = Vec::new();
    let mut levels = Vec::new();
    let mut events = Vec::new();
    let mut legs = Vec::new();
    for timeframe in &series {
        consensus.extend(ConsensusRow::rows(timeframe));
        for bar in 0..timeframe.len() {
            let Some(candle) = timeframe.candle(bar) else {
                continue;
            };
            let Some(readings) = timeframe.readings_at(bar) else {
                continue;
            };
            let Some(metrics) = timeframe.metrics_at(bar) else {
                continue;
            };
            let row = Layer1Row {
                timeframe_secs: timeframe.timeframe_secs(),
                bar_index: bar,
                candle,
                readings,
                metrics,
            };
            layer1.push(row);
            layer2.push(Layer2Row::from_layer1(row));
        }
        pivots.extend(records::pivot_rows(
            timeframe.timeframe_secs(),
            timeframe.pivots(),
        ));
        levels.extend(records::level_rows(
            timeframe.timeframe_secs(),
            timeframe.levels(),
        ));
        events.extend(records::event_rows(
            timeframe.timeframe_secs(),
            timeframe.events(),
        ));
        legs.extend(records::leg_rows(
            timeframe.timeframe_secs(),
            timeframe.legs(),
        ));
    }

    let mut layer3 = Vec::new();
    if let Some(position) = series
        .iter()
        .position(|timeframe| timeframe.timeframe_secs() == M1_SECONDS)
    {
        series.swap(0, position);
        let m1_rows: Vec<&Layer2Row> = layer2
            .iter()
            .filter(|row| row.layer1.timeframe_secs == M1_SECONDS)
            .collect();

        if let Some((m1, higher)) = series.split_first_mut() {
            for base in m1_rows {
                let bar = base.layer1.bar_index;
                let Some(base_snapshot) = m1.snapshot_at(bar) else {
                    continue;
                };
                let mut snapshots = Vec::with_capacity(1 + higher.len());
                snapshots.push(base_snapshot);
                let mut blocks = Vec::with_capacity(higher.len());

                for timeframe in higher.iter_mut() {
                    let Some(index) = alignment::latest_closed_index(
                        base.layer1.candle.timestamp(),
                        M1_SECONDS,
                        timeframe.open_times(),
                        timeframe.timeframe_secs(),
                    ) else {
                        continue;
                    };
                    let Some(open_time) = timeframe.open_times().get(index) else {
                        continue;
                    };
                    let Some(staleness_secs) = alignment::staleness_secs(
                        base.layer1.candle.timestamp(),
                        M1_SECONDS,
                        *open_time,
                        timeframe.timeframe_secs(),
                    ) else {
                        continue;
                    };
                    let Some(candle) = timeframe.candle(index) else {
                        continue;
                    };
                    let Some(metrics) = timeframe.metrics_at(index) else {
                        continue;
                    };
                    let Some(snapshot) = timeframe.snapshot_at(index) else {
                        continue;
                    };
                    blocks.push(HigherBlock {
                        timeframe_secs: timeframe.timeframe_secs(),
                        bar_index: index,
                        candle,
                        metrics,
                        resolution: resolutions(&metrics),
                        staleness_secs,
                    });
                    snapshots.push(snapshot);
                }

                let aggregates = CrossTimeframeAggregates::compute(&snapshots);
                layer3.push(Layer3Row {
                    base: *base,
                    higher: blocks,
                    aggregates,
                });
            }
        }
    }

    LayerSet {
        timeframes,
        layer1,
        layer2,
        consensus,
        layer3,
        pivots,
        levels,
        events,
        legs,
    }
}

/// Derives the four resolutions from a Layer 1 row's own metrics.
fn resolutions(metrics: &Metrics) -> AxisResolutions {
    AxisResolutions {
        direction: AxisResolution::new(
            metrics.direction.direction,
            metrics.direction.unavailable_reason_code,
        ),
        momentum: AxisResolution::new(
            metrics.momentum.force,
            metrics.momentum.unavailable_reason_code,
        ),
        structure: AxisResolution::new(
            metrics.structure.score,
            metrics.structure.unavailable_reason_code,
        ),
        progression: AxisResolution::new(
            metrics.progression.progress.map(clamp_unit),
            metrics.progression.unavailable_reason_code,
        ),
    }
}
