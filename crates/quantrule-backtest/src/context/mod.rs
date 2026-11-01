//! The as-of ladder context for the execution timeframe.
//!
//! The builder reads a completed [`ModelRun`] and a strategy-owned active
//! ladder. For every execution bar it selects the latest context bar whose
//! close is at or before the execution bar's close, then computes the weighted
//! per-axis context. It never reads a forming higher-timeframe bar and never
//! recomputes a model reading.
//!
//! The context is descriptive: it publishes weighted values, shares, counts,
//! and ordinal histograms. Ordinal codes are counted, never averaged. The
//! strategy later decides what the context means.

use std::fmt;

use quantrule_model::kernel::COMPACT_STATE_THRESHOLD;
use quantrule_model::run::ModelRun;
use quantrule_model::timeframe::{ModelBar, TimeframeModel};

/// One higher-timeframe bar selected for an execution bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextTimeframe {
    /// The context timeframe in seconds.
    pub timeframe_secs: u64,
    /// The zero-based bar position in the context series.
    pub bar_index: usize,
    /// The bar's open time as UTC Unix seconds.
    pub open_time: i64,
    /// The bar's close time as UTC Unix seconds.
    pub close_time: i64,
}

/// The agreement code for one axis's weighted context.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgreementCode {
    /// No valid context value.
    Unavailable = 0,
    /// Weighted bullish share is at least one half and exceeds bearish.
    Bullish = 1,
    /// Weighted bearish share is at least one half and exceeds bullish.
    Bearish = 2,
    /// Valid values exist but neither side has a weighted majority.
    Mixed = 3,
}

impl AgreementCode {
    /// The stable numeric code.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }
}

/// The weighted context for one continuous axis.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisContext {
    /// Context timeframes that contributed a valid value, fastest first.
    pub timeframes: Vec<u64>,
    /// Number of valid bullish readings.
    pub bullish_count: u32,
    /// Number of valid bearish readings.
    pub bearish_count: u32,
    /// Number of valid neutral readings.
    pub neutral_count: u32,
    /// The weighted mean of valid readings, or `None`.
    pub weighted_value: Option<f64>,
    /// Weighted bullish share of valid readings, or `None`.
    pub bullish_share: Option<f64>,
    /// Weighted bearish share of valid readings, or `None`.
    pub bearish_share: Option<f64>,
    /// Weighted neutral share of valid readings, or `None`.
    pub neutral_share: Option<f64>,
    /// The weighted agreement code.
    pub agreement: AgreementCode,
}

impl AxisContext {
    /// The unavailable context: no valid values.
    #[must_use]
    pub const fn unavailable() -> Self {
        Self {
            timeframes: Vec::new(),
            bullish_count: 0,
            bearish_count: 0,
            neutral_count: 0,
            weighted_value: None,
            bullish_share: None,
            bearish_share: None,
            neutral_share: None,
            agreement: AgreementCode::Unavailable,
        }
    }
}

/// The weighted context for Structure.
#[derive(Debug, Clone, PartialEq)]
pub struct StructureContext {
    /// The weighted score context.
    pub score: AxisContext,
    /// Number of context timeframes whose high and low relations oppose.
    pub conflict_count: u32,
}

impl StructureContext {
    /// The unavailable context: no valid score and no conflict.
    #[must_use]
    pub const fn unavailable() -> Self {
        Self {
            score: AxisContext::unavailable(),
            conflict_count: 0,
        }
    }
}

/// The weighted context for Progression.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressionContext {
    /// The weighted leg-direction context.
    pub direction: AxisContext,
    /// Weighted shares of published stage codes `0..5`.
    pub stage_histogram: [f64; 6],
    /// Number of context timeframes whose published stage is late or terminal.
    pub late_or_terminal_count: u32,
}

impl ProgressionContext {
    /// The unavailable context: no valid leg or stage.
    #[must_use]
    pub const fn unavailable() -> Self {
        Self {
            direction: AxisContext::unavailable(),
            stage_histogram: [0.0; 6],
            late_or_terminal_count: 0,
        }
    }
}

/// The complete as-of context for one execution bar.
#[derive(Debug, Clone, PartialEq)]
pub struct LadderContext {
    /// The execution timeframe in seconds.
    pub execution_timeframe_secs: u64,
    /// The zero-based execution bar.
    pub execution_bar_index: usize,
    /// The execution bar's open time.
    pub execution_open_time: i64,
    /// The execution bar's close time.
    pub execution_close_time: i64,
    /// The selected context bars, fastest first.
    pub context: Vec<ContextTimeframe>,
    /// Context timeframes with no closed bar at this execution bar.
    pub missing_timeframes: Vec<u64>,
    /// Direction context.
    pub direction: AxisContext,
    /// Momentum context.
    pub momentum: AxisContext,
    /// Structure context.
    pub structure: StructureContext,
    /// Progression context.
    pub progression: ProgressionContext,
}

impl LadderContext {
    /// Whether every configured context timeframe had a closed bar.
    #[must_use]
    pub fn context_complete(&self) -> bool {
        self.missing_timeframes.is_empty()
    }
}

/// The strategy-owned active ladder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LadderConfig {
    execution_timeframe_secs: u64,
    context_timeframes: Vec<u64>,
}

impl Default for LadderConfig {
    /// The control active ladder: 5m execution with 15m context.
    fn default() -> Self {
        Self {
            execution_timeframe_secs: 300,
            context_timeframes: vec![900],
        }
    }
}

impl LadderConfig {
    /// Builds an active ladder.
    ///
    /// The execution timeframe is the fastest member; context timeframes are
    /// strictly slower, ascending, and unique.
    ///
    /// # Errors
    ///
    /// Returns [`LadderContextError`] when the execution timeframe is zero, the
    /// context list is empty, or the contexts are not strictly ascending and
    /// slower than execution.
    pub fn new(
        execution_timeframe_secs: u64,
        context_timeframes: Vec<u64>,
    ) -> Result<Self, LadderContextError> {
        if execution_timeframe_secs == 0 {
            return Err(LadderContextError::ZeroExecutionTimeframe);
        }
        if context_timeframes.is_empty() {
            return Err(LadderContextError::EmptyContext);
        }
        let mut previous = execution_timeframe_secs;
        for timeframe in &context_timeframes {
            if *timeframe <= previous {
                return Err(LadderContextError::ContextNotAscending {
                    previous,
                    current: *timeframe,
                });
            }
            previous = *timeframe;
        }
        Ok(Self {
            execution_timeframe_secs,
            context_timeframes,
        })
    }

    /// The execution timeframe in seconds.
    #[must_use]
    pub const fn execution_timeframe_secs(&self) -> u64 {
        self.execution_timeframe_secs
    }

    /// The context timeframes, fastest first.
    #[must_use]
    pub fn context_timeframes(&self) -> &[u64] {
        &self.context_timeframes
    }
}

/// The per-axis, per-context-timeframe weights.
///
/// Each vector has one positive finite weight per context timeframe, in active
/// ladder order. The default is equal weight.
#[derive(Debug, Clone, PartialEq)]
pub struct LadderWeights {
    weights: [Vec<f64>; 4],
}

impl LadderWeights {
    /// Builds equal weights for `context_len` context timeframes.
    #[must_use]
    pub fn equal(context_len: usize) -> Self {
        Self {
            weights: std::array::from_fn(|_| vec![1.0; context_len]),
        }
    }

    /// Builds explicit weights for Direction, Momentum, Structure, and
    /// Progression, in that order.
    ///
    /// # Errors
    ///
    /// Returns [`LadderContextError`] when the vectors differ in length, are
    /// empty, contain a non-finite or non-positive value, or overflow the
    /// finite range when summed per axis.
    pub fn new(
        direction: Vec<f64>,
        momentum: Vec<f64>,
        structure: Vec<f64>,
        progression: Vec<f64>,
    ) -> Result<Self, LadderContextError> {
        let expected = direction.len();
        if expected == 0 {
            return Err(LadderContextError::EmptyWeights);
        }
        for weights in [&momentum, &structure, &progression] {
            if weights.len() != expected {
                return Err(LadderContextError::WeightsLength {
                    expected,
                    actual: weights.len(),
                });
            }
        }
        let weights = [direction, momentum, structure, progression];
        for (axis, values) in weights.iter().enumerate() {
            for (index, value) in values.iter().enumerate() {
                if !value.is_finite() || *value <= 0.0 {
                    return Err(LadderContextError::InvalidWeight { axis, index });
                }
            }
            let sum: f64 = values.iter().sum();
            if !sum.is_finite() {
                return Err(LadderContextError::InvalidWeightSum { axis });
            }
        }
        Ok(Self { weights })
    }

    /// The number of context timeframes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.weights[0].len()
    }

    /// Whether the weight set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Direction weights, in active-ladder context order.
    #[must_use]
    pub fn direction(&self) -> &[f64] {
        &self.weights[0]
    }

    /// Momentum weights, in active-ladder context order.
    #[must_use]
    pub fn momentum(&self) -> &[f64] {
        &self.weights[1]
    }

    /// Structure weights, in active-ladder context order.
    #[must_use]
    pub fn structure(&self) -> &[f64] {
        &self.weights[2]
    }

    /// Progression weights, in active-ladder context order.
    #[must_use]
    pub fn progression(&self) -> &[f64] {
        &self.weights[3]
    }

    fn axis(&self, axis: usize) -> &[f64] {
        &self.weights[axis]
    }
}

/// The context builder could not complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LadderContextError {
    /// The execution timeframe is zero.
    ZeroExecutionTimeframe,
    /// No context timeframe was supplied.
    EmptyContext,
    /// Context timeframes are not strictly ascending and slower than execution.
    ContextNotAscending {
        /// The previous timeframe in seconds.
        previous: u64,
        /// The offending timeframe in seconds.
        current: u64,
    },
    /// No weights were supplied.
    EmptyWeights,
    /// Weight vectors differ in length.
    WeightsLength {
        /// The expected length.
        expected: usize,
        /// The observed length.
        actual: usize,
    },
    /// A weight is not finite or not positive.
    InvalidWeight {
        /// The axis index: 0 Direction, 1 Momentum, 2 Structure, 3 Progression.
        axis: usize,
        /// The context-timeframe index.
        index: usize,
    },
    /// The weights do not match the context list.
    WeightCount {
        /// The number of context timeframes.
        expected: usize,
        /// The number of weights.
        actual: usize,
    },
    /// The weights for one axis overflow the finite range when summed.
    InvalidWeightSum {
        /// The axis index: 0 Direction, 1 Momentum, 2 Structure, 3 Progression.
        axis: usize,
    },
    /// The run does not contain the execution timeframe.
    MissingExecutionTimeframe {
        /// The missing timeframe in seconds.
        timeframe_secs: u64,
    },
    /// A model bar was missing from a non-empty series.
    MissingBar {
        /// The timeframe in seconds.
        timeframe_secs: u64,
        /// The missing bar index.
        bar: usize,
    },
    /// A bar close time overflowed the supported arithmetic.
    TimestampOverflow {
        /// The bar open time as UTC Unix seconds.
        timestamp: i64,
        /// The timeframe in seconds that was added.
        seconds: i64,
    },
}

impl fmt::Display for LadderContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroExecutionTimeframe => {
                formatter.write_str("the execution timeframe must be positive")
            }
            Self::EmptyContext => formatter.write_str("the context ladder must not be empty"),
            Self::ContextNotAscending { previous, current } => write!(
                formatter,
                "context timeframe {current}s must be slower than {previous}s"
            ),
            Self::EmptyWeights => formatter.write_str("the ladder weights must not be empty"),
            Self::WeightsLength { expected, actual } => write!(
                formatter,
                "the ladder weights need {expected} entries, found {actual}"
            ),
            Self::InvalidWeight { axis, index } => write!(
                formatter,
                "ladder weight axis {axis} index {index} must be finite and positive"
            ),
            Self::WeightCount { expected, actual } => write!(
                formatter,
                "the ladder has {expected} context timeframes but {actual} weights"
            ),
            Self::InvalidWeightSum { axis } => write!(
                formatter,
                "ladder weight axis {axis} must have a finite sum"
            ),
            Self::MissingExecutionTimeframe { timeframe_secs } => write!(
                formatter,
                "the model run has no execution timeframe {timeframe_secs}s"
            ),
            Self::MissingBar {
                timeframe_secs,
                bar,
            } => write!(
                formatter,
                "the model run has no bar {bar} for timeframe {timeframe_secs}s"
            ),
            Self::TimestampOverflow { timestamp, seconds } => write!(
                formatter,
                "adding {seconds}s to timestamp {timestamp} overflows the supported range"
            ),
        }
    }
}

impl std::error::Error for LadderContextError {}

/// Builds one [`LadderContext`] per execution bar.
///
/// # Errors
///
/// Returns [`LadderContextError`] when the ladder or weights are invalid, or
/// when a configured timeframe is absent from the model run.
pub fn build(
    run: &ModelRun,
    ladder: &LadderConfig,
    weights: &LadderWeights,
) -> Result<Vec<LadderContext>, LadderContextError> {
    let mut builder = LadderContextBuilder::new(run, ladder, weights)?;
    let mut contexts = Vec::with_capacity(builder.execution_len());
    while let Some(context) = builder.next_context()? {
        contexts.push(context);
    }
    Ok(contexts)
}

/// A streaming builder for the as-of context.
///
/// The builder keeps one pointer per context timeframe, so a full pass over an
/// execution series is linear in the number of bars. It borrows the model run
/// and owns the ladder and weights.
pub struct LadderContextBuilder<'a> {
    execution: &'a TimeframeModel,
    execution_secs: i64,
    context_models: Vec<Option<&'a TimeframeModel>>,
    context_secs: Vec<i64>,
    weights: LadderWeights,
    pointers: Vec<Option<usize>>,
    next: Vec<usize>,
    bar_index: usize,
}

impl<'a> LadderContextBuilder<'a> {
    /// Prepares a builder for one run, ladder, and weight set.
    ///
    /// # Errors
    ///
    /// Returns [`LadderContextError`] when the weights do not match the context
    /// list, or when the run does not contain the execution timeframe. A context
    /// timeframe absent from the run is not an error: it contributes no bar and
    /// is reported in [`LadderContext::missing_timeframes`].
    pub fn new(
        run: &'a ModelRun,
        ladder: &LadderConfig,
        weights: &LadderWeights,
    ) -> Result<Self, LadderContextError> {
        if weights.len() != ladder.context_timeframes().len() {
            return Err(LadderContextError::WeightCount {
                expected: ladder.context_timeframes().len(),
                actual: weights.len(),
            });
        }

        let execution = run.timeframe(ladder.execution_timeframe_secs()).ok_or(
            LadderContextError::MissingExecutionTimeframe {
                timeframe_secs: ladder.execution_timeframe_secs(),
            },
        )?;
        let context_secs: Vec<i64> = ladder
            .context_timeframes()
            .iter()
            .map(|seconds| *seconds as i64)
            .collect();
        let context_models: Vec<Option<&TimeframeModel>> = ladder
            .context_timeframes()
            .iter()
            .map(|seconds| run.timeframe(*seconds))
            .collect();

        Ok(Self {
            execution,
            execution_secs: execution.timeframe_secs() as i64,
            context_models,
            context_secs,
            weights: weights.clone(),
            pointers: vec![None; ladder.context_timeframes().len()],
            next: vec![0; ladder.context_timeframes().len()],
            bar_index: 0,
        })
    }

    /// The number of execution bars the builder will yield.
    #[must_use]
    pub fn execution_len(&self) -> usize {
        self.execution.len()
    }

    /// Builds the next execution bar's context, or `None` at the end.
    ///
    /// # Errors
    ///
    /// Returns [`LadderContextError::MissingBar`] when a non-empty model series
    /// omits a bar.
    pub fn next_context(&mut self) -> Result<Option<LadderContext>, LadderContextError> {
        if self.bar_index >= self.execution.len() {
            return Ok(None);
        }
        let bar_index = self.bar_index;
        self.bar_index += 1;
        let bar = self
            .execution
            .bar(bar_index)
            .ok_or(LadderContextError::MissingBar {
                timeframe_secs: self.execution.timeframe_secs(),
                bar: bar_index,
            })?;
        let execution_open_time = bar.candle.timestamp();
        let execution_close_time = execution_open_time.checked_add(self.execution_secs).ok_or(
            LadderContextError::TimestampOverflow {
                timestamp: execution_open_time,
                seconds: self.execution_secs,
            },
        )?;
        let mut selected = Vec::with_capacity(self.context_models.len());
        let mut missing = Vec::new();

        for (context_index, model) in self.context_models.iter().enumerate() {
            let Some(model) = model else {
                missing.push(self.context_secs[context_index] as u64);
                continue;
            };
            let timeframe_secs = model.timeframe_secs();
            let open_times = model.open_times();
            let context_secs = self.context_secs[context_index];
            while self.next[context_index] < open_times.len()
                && open_times[self.next[context_index]]
                    .checked_add(context_secs)
                    .is_some_and(|close_time| close_time <= execution_close_time)
            {
                self.pointers[context_index] = Some(self.next[context_index]);
                self.next[context_index] += 1;
            }

            let Some(context_bar_index) = self.pointers[context_index] else {
                missing.push(timeframe_secs);
                continue;
            };
            let context_bar =
                model
                    .bar(context_bar_index)
                    .ok_or(LadderContextError::MissingBar {
                        timeframe_secs,
                        bar: context_bar_index,
                    })?;
            let open_time = context_bar.candle.timestamp();
            let close_time = open_time.checked_add(context_secs).ok_or(
                LadderContextError::TimestampOverflow {
                    timestamp: open_time,
                    seconds: context_secs,
                },
            )?;
            selected.push((
                ContextTimeframe {
                    timeframe_secs,
                    bar_index: context_bar_index,
                    open_time,
                    close_time,
                },
                context_bar,
            ));
        }

        let (direction, momentum, structure, progression) = if missing.is_empty() {
            compute_axes(&selected, &self.weights)
        } else {
            (
                AxisContext::unavailable(),
                AxisContext::unavailable(),
                StructureContext::unavailable(),
                ProgressionContext::unavailable(),
            )
        };

        Ok(Some(LadderContext {
            execution_timeframe_secs: self.execution.timeframe_secs(),
            execution_bar_index: bar_index,
            execution_open_time,
            execution_close_time,
            context: selected.iter().map(|(identity, _)| *identity).collect(),
            missing_timeframes: missing,
            direction,
            momentum,
            structure,
            progression,
        }))
    }
}

type SelectedBars<'a> = [(ContextTimeframe, ModelBar)];
type ComputedAxes = (
    AxisContext,
    AxisContext,
    StructureContext,
    ProgressionContext,
);

fn compute_axes(bars: &SelectedBars<'_>, weights: &LadderWeights) -> ComputedAxes {
    let mut direction = AxisAccumulator::default();
    let mut momentum = AxisAccumulator::default();
    let mut structure = AxisAccumulator::default();
    let mut conflict_count = 0u32;
    let mut progression_direction = AxisAccumulator::default();
    let mut stage_histogram = [0.0; 6];
    let mut stage_weight = 0.0;
    let mut late_or_terminal_count = 0u32;

    for (index, (identity, bar)) in bars.iter().enumerate() {
        direction.push(
            identity.timeframe_secs,
            bar.direction.direction,
            weights.axis(0)[index],
        );
        momentum.push(
            identity.timeframe_secs,
            bar.momentum.force,
            weights.axis(1)[index],
        );
        structure.push(
            identity.timeframe_secs,
            bar.structure.score,
            weights.axis(2)[index],
        );
        if let (Some(high), Some(low)) = (
            bar.structure.high_relation_code,
            bar.structure.low_relation_code,
        ) && high != 0
            && low != 0
            && high.signum() != low.signum()
        {
            conflict_count += 1;
        }

        let weight = weights.axis(3)[index];
        progression_direction.push(
            identity.timeframe_secs,
            bar.progression.leg_direction.map(f64::from),
            weight,
        );
        if let Some(stage) = bar.progression.stage_code
            && (0..=5).contains(&stage)
        {
            stage_histogram[stage as usize] += weight;
            stage_weight += weight;
            if stage >= 4 {
                late_or_terminal_count += 1;
            }
        }
    }

    if stage_weight > 0.0 {
        for share in &mut stage_histogram {
            *share /= stage_weight;
        }
    }

    (
        direction.finish(),
        momentum.finish(),
        StructureContext {
            score: structure.finish(),
            conflict_count,
        },
        ProgressionContext {
            direction: progression_direction.finish(),
            stage_histogram,
            late_or_terminal_count,
        },
    )
}

#[derive(Default)]
struct AxisAccumulator {
    timeframes: Vec<u64>,
    bullish_count: u32,
    bearish_count: u32,
    neutral_count: u32,
    weighted_sum: f64,
    weight_sum: f64,
    bullish_weight: f64,
    bearish_weight: f64,
    neutral_weight: f64,
}

impl AxisAccumulator {
    fn push(&mut self, timeframe_secs: u64, value: Option<f64>, weight: f64) {
        let Some(value) = value.filter(|value| value.is_finite()) else {
            return;
        };
        self.timeframes.push(timeframe_secs);
        self.weighted_sum += value * weight;
        self.weight_sum += weight;
        if value > COMPACT_STATE_THRESHOLD {
            self.bullish_count += 1;
            self.bullish_weight += weight;
        } else if value < -COMPACT_STATE_THRESHOLD {
            self.bearish_count += 1;
            self.bearish_weight += weight;
        } else {
            self.neutral_count += 1;
            self.neutral_weight += weight;
        }
    }

    fn finish(self) -> AxisContext {
        if self.weight_sum == 0.0 {
            return AxisContext::unavailable();
        }
        let bullish_share = self.bullish_weight / self.weight_sum;
        let bearish_share = self.bearish_weight / self.weight_sum;
        let neutral_share = self.neutral_weight / self.weight_sum;
        AxisContext {
            timeframes: self.timeframes,
            bullish_count: self.bullish_count,
            bearish_count: self.bearish_count,
            neutral_count: self.neutral_count,
            weighted_value: Some(self.weighted_sum / self.weight_sum),
            bullish_share: Some(bullish_share),
            bearish_share: Some(bearish_share),
            neutral_share: Some(neutral_share),
            agreement: agreement(bullish_share, bearish_share),
        }
    }
}

fn agreement(bullish_share: f64, bearish_share: f64) -> AgreementCode {
    if bullish_share >= 0.5 && bullish_share > bearish_share {
        AgreementCode::Bullish
    } else if bearish_share >= 0.5 && bearish_share > bullish_share {
        AgreementCode::Bearish
    } else {
        AgreementCode::Mixed
    }
}

#[cfg(test)]
mod tests;
