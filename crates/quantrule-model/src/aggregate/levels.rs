//! Level clustering across timeframes.
//!
//! The eligible levels are the bounded A6 reference set: the three EMAs and the
//! latest confirmed swing high and low at each scale. Two levels are linked
//! when their price separation is no greater than `0.5 * max(ATR_i, ATR_j)`;
//! clusters are the connected sets of links (single linkage). The span of the
//! largest cluster is reported in ATR so chaining is visible.

use std::collections::{BTreeMap, BTreeSet};

use super::TimeframeSnapshot;

/// The maximum number of references in the A6 set.
pub const MAX_REFERENCES: usize = 7;

/// The bounded set of price references for one timeframe.
///
/// The constructor keeps at most seven prices. The contract's A6 set is the
/// three EMAs plus the latest confirmed swing high and low at each scale.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceSet {
    prices: Vec<f64>,
}

impl ReferenceSet {
    /// Builds a set, keeping at most [`MAX_REFERENCES`] prices.
    #[must_use]
    pub fn new(prices: Vec<f64>) -> Self {
        let mut prices = prices;
        if prices.len() > MAX_REFERENCES {
            prices.truncate(MAX_REFERENCES);
        }
        Self { prices }
    }

    /// Builds the A6 set from the three EMAs and the four latest swing levels.
    ///
    /// Missing values are omitted; the order is EMA 20, EMA 50, EMA 200, minor
    /// high, minor low, major high, major low.
    #[must_use]
    pub fn from_parts(
        ema20: Option<f64>,
        ema50: Option<f64>,
        ema200: Option<f64>,
        minor_high: Option<f64>,
        minor_low: Option<f64>,
        major_high: Option<f64>,
        major_low: Option<f64>,
    ) -> Self {
        let prices = [
            ema20, ema50, ema200, minor_high, minor_low, major_high, major_low,
        ]
        .into_iter()
        .flatten()
        .collect();
        Self { prices }
    }

    /// The reference prices.
    #[must_use]
    pub fn prices(&self) -> &[f64] {
        &self.prices
    }

    /// The number of references.
    #[must_use]
    pub fn len(&self) -> usize {
        self.prices.len()
    }

    /// Whether the set has no references.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }
}

/// The level-cluster aggregates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelClusterAggregates {
    /// Number of single-linkage clusters, including singletons.
    pub level_cluster_count: u32,
    /// Distinct timeframes in the largest cluster.
    pub largest_level_cluster_timeframe_count: u32,
    /// Price span of the largest cluster divided by its maximum ATR.
    pub largest_level_cluster_span_atr: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct LevelPoint {
    timeframe_index: usize,
    price: f64,
    atr: f64,
}

/// Clusters the bounded reference sets across the ladder.
pub(crate) fn cluster(ordered: &[&TimeframeSnapshot]) -> LevelClusterAggregates {
    let mut points = Vec::new();
    for (timeframe_index, snapshot) in ordered.iter().enumerate() {
        let Some(atr) = snapshot.atr.filter(|atr| atr.is_finite() && *atr > 0.0) else {
            continue;
        };
        for price in snapshot.references.prices() {
            points.push(LevelPoint {
                timeframe_index,
                price: *price,
                atr,
            });
        }
    }

    if points.is_empty() {
        return LevelClusterAggregates {
            level_cluster_count: 0,
            largest_level_cluster_timeframe_count: 0,
            largest_level_cluster_span_atr: None,
        };
    }

    let mut parent: Vec<usize> = (0..points.len()).collect();
    for left in 0..points.len() {
        for right in (left + 1)..points.len() {
            let separation = (points[left].price - points[right].price).abs();
            let tolerance = 0.5 * points[left].atr.max(points[right].atr);
            if separation <= tolerance {
                union(&mut parent, left, right);
            }
        }
    }

    let mut clusters: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for index in 0..points.len() {
        let root = find(&mut parent, index);
        clusters.entry(root).or_default().push(index);
    }

    let level_cluster_count = clusters.len() as u32;
    let mut largest_timeframes = 0_u32;
    let mut largest_span: Option<f64> = None;

    for indices in clusters.values() {
        let timeframes: BTreeSet<usize> = indices
            .iter()
            .map(|index| points[*index].timeframe_index)
            .collect();
        let minimum = indices
            .iter()
            .map(|index| points[*index].price)
            .fold(f64::INFINITY, f64::min);
        let maximum = indices
            .iter()
            .map(|index| points[*index].price)
            .fold(f64::NEG_INFINITY, f64::max);
        let max_atr = indices
            .iter()
            .map(|index| points[*index].atr)
            .fold(0.0_f64, f64::max);
        let span = if max_atr > 0.0 {
            Some((maximum - minimum) / max_atr)
        } else {
            None
        };
        let timeframe_count = timeframes.len() as u32;

        if timeframe_count > largest_timeframes
            || (timeframe_count == largest_timeframes
                && span.unwrap_or(0.0) > largest_span.unwrap_or(0.0))
        {
            largest_timeframes = timeframe_count;
            largest_span = span;
        }
    }

    LevelClusterAggregates {
        level_cluster_count,
        largest_level_cluster_timeframe_count: largest_timeframes,
        largest_level_cluster_span_atr: largest_span,
    }
}

fn find(parent: &mut [usize], index: usize) -> usize {
    let mut root = index;
    while parent[root] != root {
        root = parent[root];
    }
    let mut current = index;
    while parent[current] != current {
        let next = parent[current];
        parent[current] = root;
        current = next;
    }
    root
}

fn union(parent: &mut [usize], left: usize, right: usize) {
    let left_root = find(parent, left);
    let right_root = find(parent, right);
    if left_root != right_root {
        parent[right_root] = left_root;
    }
}
