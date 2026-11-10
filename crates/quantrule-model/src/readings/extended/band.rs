//! Horizontal support/resistance bands from confirmed major pivot levels.
//!
//! Levels are clustered by price proximity using one ATR-scaled tolerance. The
//! selected band is the cluster with the most levels; ties go to the cluster
//! nearest the close, then to the lower center. The computation only reads
//! confirmed levels available at or before the bar.

use super::*;

pub(super) struct BandReadings {
    pub(super) upper: Vec<Option<f64>>,
    pub(super) lower: Vec<Option<f64>>,
    pub(super) center: Vec<Option<f64>>,
    pub(super) touches: Vec<Option<f64>>,
    pub(super) age: Vec<Option<f64>>,
    pub(super) width_atr: Vec<Option<f64>>,
    pub(super) position: Vec<Option<f64>>,
}

#[derive(Clone, Copy, PartialEq)]
struct LevelPoint {
    price: f64,
    available_at: usize,
}

pub(super) fn band_series(
    candles: &[Candle],
    readings: &Readings,
    config: &ExtendedConfig,
) -> BandReadings {
    let len = candles.len();
    let null = || vec![None; len];
    let mut result = BandReadings {
        upper: null(),
        lower: null(),
        center: null(),
        touches: null(),
        age: null(),
        width_atr: null(),
        position: null(),
    };
    if len == 0 {
        return result;
    }
    let registry = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
    let all_levels = registry.levels();
    let mut active: Vec<LevelPoint> = Vec::new();
    let mut next_level = 0_usize;
    for (bar, candle) in candles.iter().enumerate() {
        while next_level < all_levels.len() && all_levels[next_level].available_at <= bar {
            let level = all_levels[next_level];
            if level.scale == PivotScale::Major {
                active.push(LevelPoint {
                    price: level.price,
                    available_at: level.available_at,
                });
            }
            next_level += 1;
        }
        active.retain(|level| bar - level.available_at <= config.level_lookback);
        let Some(atr) = readings.atr14.get(bar).filter(|atr| *atr > 0.0) else {
            continue;
        };
        let tolerance = config.band_atr * atr;
        let close = candle.close();
        let Some(band) = select_band(&active, close, tolerance) else {
            continue;
        };
        let upper = band
            .iter()
            .map(|point| point.price)
            .fold(f64::NEG_INFINITY, f64::max);
        let lower = band
            .iter()
            .map(|point| point.price)
            .fold(f64::INFINITY, f64::min);
        let center = band.iter().map(|point| point.price).sum::<f64>() / band.len() as f64;
        let Some(youngest) = band.iter().map(|point| point.available_at).max() else {
            continue;
        };
        let width = upper - lower;
        result.upper[bar] = Some(upper);
        result.lower[bar] = Some(lower);
        result.center[bar] = Some(center);
        result.touches[bar] = Some(band.len() as f64);
        result.age[bar] = Some((bar - youngest) as f64);
        result.width_atr[bar] = Some(width / atr);
        if width > 0.0 {
            result.position[bar] = Some((close - lower) / width);
        }
    }
    result
}

/// Selects the band cluster: largest first, then nearest center to close,
/// then the lower center. A cluster needs at least two levels.
fn select_band(points: &[LevelPoint], close: f64, tolerance: f64) -> Option<Vec<LevelPoint>> {
    if points.len() < 2 {
        return None;
    }
    let mut sorted = points.to_vec();
    sorted.sort_by(|left, right| left.price.total_cmp(&right.price));
    let mut clusters: Vec<Vec<LevelPoint>> = Vec::new();
    for point in sorted {
        let joins = clusters.last().is_some_and(|cluster| {
            cluster
                .last()
                .is_some_and(|last| (point.price - last.price).abs() <= tolerance)
        });
        if joins {
            if let Some(cluster) = clusters.last_mut() {
                cluster.push(point);
            }
        } else {
            clusters.push(vec![point]);
        }
    }
    let mut best: Option<Vec<LevelPoint>> = None;
    for cluster in clusters {
        if cluster.len() < 2 {
            continue;
        }
        let center = cluster.iter().map(|point| point.price).sum::<f64>() / cluster.len() as f64;
        let better = match &best {
            None => true,
            Some(current) => {
                let current_center =
                    current.iter().map(|point| point.price).sum::<f64>() / current.len() as f64;
                cluster.len() > current.len()
                    || (cluster.len() == current.len()
                        && ((center - close).abs() < (current_center - close).abs()
                            || ((center - close).abs() == (current_center - close).abs()
                                && center < current_center)))
            }
        };
        if better {
            best = Some(cluster);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::{LevelPoint, select_band};

    fn points(prices: &[f64]) -> Vec<LevelPoint> {
        prices
            .iter()
            .map(|price| LevelPoint {
                price: *price,
                available_at: 0,
            })
            .collect()
    }

    fn prices(band: &[LevelPoint]) -> Vec<f64> {
        band.iter().map(|point| point.price).collect()
    }

    #[test]
    fn horizontal_band_largest_cluster_wins() {
        let band =
            select_band(&points(&[100.0, 100.1, 100.2, 90.0]), 100.0, 0.25).expect("a band exists");
        assert_eq!(prices(&band), [100.0, 100.1, 100.2]);
    }

    #[test]
    fn horizontal_band_ties_go_to_nearest_close() {
        let band =
            select_band(&points(&[90.0, 90.1, 100.0, 100.1]), 100.0, 0.25).expect("a band exists");
        assert_eq!(prices(&band), [100.0, 100.1]);
    }

    #[test]
    fn horizontal_band_equal_centers_go_to_lower() {
        let band =
            select_band(&points(&[90.0, 90.2, 100.0, 100.2]), 95.1, 0.25).expect("a band exists");
        assert_eq!(prices(&band), [90.0, 90.2]);
    }

    #[test]
    fn horizontal_band_requires_two_levels() {
        assert!(select_band(&points(&[100.0]), 100.0, 0.25).is_none());
        assert!(select_band(&points(&[100.0, 110.0]), 100.0, 0.25).is_none());
    }

    #[test]
    fn horizontal_band_equal_levels_form_a_zero_width_cluster() {
        let band =
            select_band(&points(&[90.0, 90.0, 100.0, 100.0]), 100.0, 0.25).expect("a band exists");
        assert_eq!(prices(&band), [100.0, 100.0]);
    }
}
