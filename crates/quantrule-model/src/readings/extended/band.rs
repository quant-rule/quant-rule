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

#[derive(Clone, Copy)]
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
        let mut points = active.clone();
        if points.len() < 2 {
            continue;
        }
        points.sort_by(|left, right| left.price.total_cmp(&right.price));
        let mut clusters: Vec<Vec<LevelPoint>> = Vec::new();
        for point in points {
            match clusters.last_mut() {
                Some(cluster)
                    if (point.price - cluster.last().expect("cluster is non-empty").price)
                        .abs()
                        <= tolerance =>
                {
                    cluster.push(point);
                }
                _ => clusters.push(vec![point]),
            }
        }
        let close = candle.close();
        let mut best: Option<Vec<LevelPoint>> = None;
        for cluster in clusters {
            if cluster.len() < 2 {
                continue;
            }
            let center =
                cluster.iter().map(|point| point.price).sum::<f64>() / cluster.len() as f64;
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
        let Some(band) = best else {
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
        let youngest = band
            .iter()
            .map(|point| point.available_at)
            .max()
            .expect("band is non-empty");
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
