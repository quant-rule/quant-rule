//! Deterministic summary and relation analysis over the research layers.
//!
//! The summary counts every published continuous value and every ordinal
//! bucket per timeframe, and reports pairwise Pearson correlations between the
//! four continuous axis values. An ordinal code is counted, never averaged, and
//! no score combines the axes.
//!
//! The module is pure: it reads only the layer set it is given. The
//! accumulation order is the row order, so the same layers produce the same
//! numbers.

use crate::layers::LayerSet;
use crate::resolution::{AxisResolution, AxisResolutions};

/// The four axis names, in contract order.
pub const AXES: [&str; 4] = ["direction", "momentum", "structure", "progression"];

/// The six unordered axis pairs, as index pairs into [`AXES`].
const PAIRS: [(usize, usize); 6] = [(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];

/// The summary of one axis at one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisSummary {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The axis name.
    pub axis: &'static str,
    /// The number of present continuous values.
    pub count: usize,
    /// The number of null continuous values.
    pub nulls: usize,
    /// The minimum present value.
    pub min: Option<f64>,
    /// The maximum present value.
    pub max: Option<f64>,
    /// The mean of the present values.
    pub mean: Option<f64>,
    /// Counts for ordinal buckets 0..9.
    pub ordinal_counts: [usize; 10],
    /// The number of null ordinals.
    pub ordinal_nulls: usize,
}

/// The correlation of one axis pair at one timeframe.
#[derive(Debug, Clone, PartialEq)]
pub struct RelationSummary {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The left axis.
    pub left: &'static str,
    /// The right axis.
    pub right: &'static str,
    /// The number of rows where both values are present.
    pub count: usize,
    /// The Pearson correlation, or `None` when it is undefined.
    pub correlation: Option<f64>,
}

/// The dataset summary.
#[derive(Debug, Clone, PartialEq)]
pub struct DatasetSummary {
    /// The number of Layer 1 rows.
    pub layer1_rows: usize,
    /// The number of Layer 2 rows.
    pub layer2_rows: usize,
    /// The number of Layer 3 rows.
    pub layer3_rows: usize,
    /// One summary per timeframe and axis, in timeframe then axis order.
    pub axes: Vec<AxisSummary>,
    /// One correlation per timeframe and axis pair, in timeframe then pair order.
    pub relations: Vec<RelationSummary>,
}

/// Summarizes the layers.
#[must_use]
pub fn summarize(layers: &LayerSet) -> DatasetSummary {
    let mut builder = DatasetSummaryBuilder::new();
    for timeframe in &layers.timeframes {
        let rows = layers
            .layer2
            .iter()
            .filter(|row| row.layer1.timeframe_secs == *timeframe);
        builder.add_timeframe(*timeframe, rows.map(|row| row.resolution));
    }
    builder.set_layer3_rows(layers.layer3.len());
    builder.build()
}

/// Accumulates a [`DatasetSummary`] one timeframe at a time.
///
/// The full-layer summary and the streaming report path share this accumulator,
/// so a long run can be summarized without holding every row at once. Rows must
/// arrive in bar order, which is the order the layer build uses.
#[derive(Debug, Default)]
pub struct DatasetSummaryBuilder {
    axes: Vec<AxisSummary>,
    relations: Vec<RelationSummary>,
    layer1_rows: usize,
    layer2_rows: usize,
    layer3_rows: usize,
}

impl DatasetSummaryBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one timeframe's rows, in bar order.
    ///
    /// One row is one Layer 1 and one Layer 2 row, so both counts grow by the
    /// number of rows added.
    pub fn add_timeframe<I>(&mut self, timeframe_secs: u64, rows: I)
    where
        I: IntoIterator<Item = AxisResolutions>,
    {
        let mut axis_accumulators = [AxisAccumulator::default(); 4];
        let mut pair_accumulators = [PairAccumulator::default(); 6];
        let mut count = 0_usize;

        for row in rows {
            count += 1;
            let values = [row.direction, row.momentum, row.structure, row.progression];
            for (index, value) in values.iter().enumerate() {
                axis_accumulators[index].add(*value);
            }
            for (index, (left, right)) in PAIRS.iter().enumerate() {
                pair_accumulators[index].add(values[*left].continuous, values[*right].continuous);
            }
        }

        for (index, axis) in AXES.iter().enumerate() {
            self.axes
                .push(axis_accumulators[index].summary(timeframe_secs, axis, count));
        }
        for (index, (left, right)) in PAIRS.iter().enumerate() {
            self.relations.push(RelationSummary {
                timeframe_secs,
                left: AXES[*left],
                right: AXES[*right],
                count: pair_accumulators[index].count,
                correlation: pair_accumulators[index].correlation(),
            });
        }
        self.layer1_rows += count;
        self.layer2_rows += count;
    }

    /// Records the Layer 3 row count.
    pub fn set_layer3_rows(&mut self, rows: usize) {
        self.layer3_rows = rows;
    }

    /// Finishes the summary.
    #[must_use]
    pub fn build(self) -> DatasetSummary {
        DatasetSummary {
            layer1_rows: self.layer1_rows,
            layer2_rows: self.layer2_rows,
            layer3_rows: self.layer3_rows,
            axes: self.axes,
            relations: self.relations,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct AxisAccumulator {
    count: usize,
    nulls: usize,
    min: Option<f64>,
    max: Option<f64>,
    sum: f64,
    ordinal_counts: [usize; 10],
    ordinal_nulls: usize,
}

impl AxisAccumulator {
    fn add(&mut self, resolution: AxisResolution) {
        match resolution.continuous {
            Some(value) => {
                self.count += 1;
                self.sum += value;
                self.min = Some(self.min.map_or(value, |current| current.min(value)));
                self.max = Some(self.max.map_or(value, |current| current.max(value)));
            }
            None => self.nulls += 1,
        }
        match resolution.ordinal {
            Some(bucket) => self.ordinal_counts[usize::from(bucket)] += 1,
            None => self.ordinal_nulls += 1,
        }
    }

    fn summary(self, timeframe_secs: u64, axis: &'static str, total: usize) -> AxisSummary {
        AxisSummary {
            timeframe_secs,
            axis,
            count: self.count,
            nulls: total.saturating_sub(self.count),
            min: self.min,
            max: self.max,
            mean: (self.count > 0).then(|| self.sum / self.count as f64),
            ordinal_counts: self.ordinal_counts,
            ordinal_nulls: self.ordinal_nulls,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct PairAccumulator {
    count: usize,
    sum_x: f64,
    sum_y: f64,
    sum_xy: f64,
    sum_xx: f64,
    sum_yy: f64,
}

impl PairAccumulator {
    fn add(&mut self, left: Option<f64>, right: Option<f64>) {
        let (Some(left), Some(right)) = (left, right) else {
            return;
        };
        self.count += 1;
        self.sum_x += left;
        self.sum_y += right;
        self.sum_xy += left * right;
        self.sum_xx += left * left;
        self.sum_yy += right * right;
    }

    fn correlation(self) -> Option<f64> {
        if self.count < 2 {
            return None;
        }
        let count = self.count as f64;
        let numerator = count * self.sum_xy - self.sum_x * self.sum_y;
        let left = count * self.sum_xx - self.sum_x * self.sum_x;
        let right = count * self.sum_yy - self.sum_y * self.sum_y;
        let denominator = (left * right).sqrt();
        if denominator <= 0.0 || !denominator.is_finite() {
            return None;
        }
        Some((numerator / denominator).clamp(-1.0, 1.0))
    }
}

/// A crafted resolution used by the tests.
#[cfg(test)]
fn resolution(value: Option<f64>) -> AxisResolution {
    AxisResolution::new(value, if value.is_none() { Some(1) } else { None })
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;
    use quantrule_model::state::TimeframeInput;

    use super::{AXES, summarize};
    use crate::layers::{LayerConfig, LayerSet, build_layers};
    use crate::resolution::AxisResolutions;

    fn layer_set(rows: [[Option<f64>; 4]; 3]) -> LayerSet {
        let candles: Vec<Candle> = (0..3)
            .map(|index| {
                Candle::new(
                    index as u64,
                    index as i64 * 60,
                    1.0,
                    1.0,
                    1.0,
                    1.0,
                    Some(1.0),
                )
                .expect("the test candle is valid")
            })
            .collect();
        let mut set = build_layers(
            &[TimeframeInput {
                timeframe_secs: 60,
                candles: &candles,
            }],
            &LayerConfig::default(),
        )
        .expect("the fixture builds");

        for (row, values) in set.layer2.iter_mut().zip(rows) {
            row.resolution = AxisResolutions {
                direction: super::resolution(values[0]),
                momentum: super::resolution(values[1]),
                structure: super::resolution(values[2]),
                progression: super::resolution(values[3]),
            };
        }
        set
    }

    #[test]
    fn the_summary_reports_counts_ranges_means_and_ordinal_histograms() {
        let set = layer_set([
            [Some(0.1), Some(0.2), Some(0.3), Some(0.0)],
            [Some(0.1), Some(0.4), Some(0.2), Some(0.0)],
            [Some(0.9), Some(0.6), Some(0.1), Some(0.0)],
        ]);
        let summary = summarize(&set);

        assert_eq!(summary.layer1_rows, 3);
        assert_eq!(summary.layer2_rows, 3);
        assert_eq!(summary.axes.len(), 4);
        let direction = summary
            .axes
            .iter()
            .find(|axis| axis.axis == AXES[0])
            .expect("the direction summary");
        assert_eq!(direction.timeframe_secs, 60);
        assert_eq!(direction.count, 3);
        assert_eq!(direction.nulls, 0);
        assert_eq!(direction.min, Some(0.1));
        assert_eq!(direction.max, Some(0.9));
        assert!((direction.mean.expect("a mean") - 1.1 / 3.0).abs() < 1e-12);
        assert_eq!(direction.ordinal_counts[5], 2);
        assert_eq!(direction.ordinal_counts[9], 1);
        assert_eq!(direction.ordinal_counts.iter().sum::<usize>(), 3);
    }

    #[test]
    fn a_constant_series_has_no_correlation() {
        let set = layer_set([
            [Some(0.1), Some(0.2), Some(0.3), Some(0.0)],
            [Some(0.2), Some(0.3), Some(0.3), Some(0.0)],
            [Some(0.3), Some(0.4), Some(0.3), Some(0.0)],
        ]);
        let summary = summarize(&set);
        for relation in &summary.relations {
            if relation.left == "direction" && relation.right == "momentum" {
                assert_eq!(relation.count, 3);
                let correlation = relation.correlation.expect("a correlation");
                assert!(
                    (correlation - 1.0).abs() < 1e-12,
                    "expected a perfect positive relation, got {correlation}"
                );
            }
            if relation.left == "structure" {
                assert_eq!(relation.correlation, None);
            }
        }
    }

    #[test]
    fn a_perfect_negative_relation_is_minus_one() {
        let set = layer_set([
            [Some(0.1), Some(0.3), Some(0.0), Some(0.0)],
            [Some(0.2), Some(0.2), Some(0.0), Some(0.0)],
            [Some(0.3), Some(0.1), Some(0.0), Some(0.0)],
        ]);
        let summary = summarize(&set);
        let relation = summary
            .relations
            .iter()
            .find(|relation| relation.left == "direction" && relation.right == "momentum")
            .expect("the pair");
        assert_eq!(relation.correlation, Some(-1.0));
    }

    #[test]
    fn a_null_value_is_absent_from_the_pair_count() {
        let set = layer_set([
            [Some(0.1), Some(0.2), Some(0.0), Some(0.0)],
            [Some(0.2), None, Some(0.0), Some(0.0)],
            [Some(0.3), Some(0.4), Some(0.0), Some(0.0)],
        ]);
        let summary = summarize(&set);
        let momentum = summary
            .axes
            .iter()
            .find(|axis| axis.axis == "momentum")
            .expect("the momentum summary");
        assert_eq!(momentum.count, 2);
        assert_eq!(momentum.nulls, 1);
        let relation = summary
            .relations
            .iter()
            .find(|relation| relation.left == "direction" && relation.right == "momentum")
            .expect("the pair");
        assert_eq!(relation.count, 2);
    }

    #[test]
    fn ordinal_codes_are_counted_and_never_averaged() {
        let set = layer_set([
            [Some(-1.0), Some(0.0), Some(0.0), Some(0.0)],
            [Some(0.0), Some(0.0), Some(0.0), Some(0.0)],
            [Some(1.0), Some(0.0), Some(0.0), Some(0.0)],
        ]);
        let direction = summarize(&set)
            .axes
            .into_iter()
            .find(|axis| axis.axis == "direction")
            .expect("the direction summary");
        assert_eq!(direction.count, 3);
        assert_eq!(direction.ordinal_counts[0], 1);
        assert_eq!(direction.ordinal_counts[5], 1);
        assert_eq!(direction.ordinal_counts[9], 1);
        assert_eq!(direction.mean, Some(0.0));
    }

    #[test]
    fn an_empty_layer_set_has_no_axes_or_relations() {
        let set = LayerSet {
            timeframes: Vec::new(),
            layer1: Vec::new(),
            layer2: Vec::new(),
            consensus: Vec::new(),
            layer3: Vec::new(),
            pivots: Vec::new(),
            levels: Vec::new(),
            events: Vec::new(),
            legs: Vec::new(),
        };
        let summary = summarize(&set);
        assert_eq!(summary.layer1_rows, 0);
        assert!(summary.axes.is_empty());
        assert!(summary.relations.is_empty());
    }

    #[test]
    fn repeated_runs_are_identical() {
        let set = layer_set([
            [Some(0.1), Some(0.2), Some(0.3), Some(0.4)],
            [Some(-0.2), Some(-0.1), Some(0.0), Some(0.1)],
            [Some(0.7), Some(0.6), Some(0.5), Some(0.4)],
        ]);
        assert_eq!(summarize(&set), summarize(&set));
    }
}
