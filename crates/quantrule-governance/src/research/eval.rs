//! Shared evaluation statistics for the research harnesses.

use super::stats::inverse_normal_cdf;

/// Cluster-robust hit-rate statistics.
pub struct ClusterStats {
    /// Number of events.
    pub n: usize,
    /// Number of clusters.
    pub clusters: usize,
    /// Hit rate.
    pub hit_rate: Option<f64>,
    /// Cluster-robust standard error.
    pub se: Option<f64>,
    /// Bundles that contributed at most one cluster.
    pub singleton_bundles: usize,
}

/// Compute cluster-robust statistics with clusters formed inside each bundle.
#[must_use]
pub fn cluster_stats(entries: &[(usize, i64, bool)], horizon: i64) -> ClusterStats {
    let n = entries.len();
    if n == 0 {
        return ClusterStats {
            n: 0,
            clusters: 0,
            hit_rate: None,
            se: None,
            singleton_bundles: 0,
        };
    }
    let hits = entries.iter().filter(|(_, _, hit)| *hit).count();
    let probability = hits as f64 / n as f64;
    let mut grouped: std::collections::BTreeMap<usize, Vec<(i64, bool)>> =
        std::collections::BTreeMap::new();
    for (bundle, bar, hit) in entries {
        grouped.entry(*bundle).or_default().push((*bar, *hit));
    }
    let mut variance = 0.0;
    let mut clusters = 0;
    let mut singleton = 0;
    for values in grouped.values_mut() {
        values.sort_by_key(|(bar, _)| *bar);
        let mut groups: Vec<Vec<(i64, bool)>> = Vec::new();
        let mut current: Vec<(i64, bool)> = Vec::new();
        for (bar, hit) in values.iter() {
            if (current.is_empty()
                || *bar - current.last().map_or(*bar, |value| value.0) >= horizon)
                && !current.is_empty()
            {
                groups.push(std::mem::take(&mut current));
            }
            current.push((*bar, *hit));
        }
        if !current.is_empty() {
            groups.push(current);
        }
        let j = groups.len();
        clusters += j;
        if j <= 1 {
            singleton += 1;
            continue;
        }
        let residual: f64 = groups
            .iter()
            .map(|group| {
                let group_hits = group.iter().filter(|(_, hit)| *hit).count() as f64;
                (group_hits - probability * group.len() as f64).powi(2)
            })
            .sum();
        variance += (j as f64 / (j as f64 - 1.0)) * residual / (n as f64 * n as f64);
    }
    ClusterStats {
        n,
        clusters,
        hit_rate: Some(probability),
        se: (variance > 0.0).then(|| variance.sqrt()),
        singleton_bundles: singleton,
    }
}

/// One-sided control p-value.
#[must_use]
pub fn control_p(observed: f64, draws: &[Option<f64>]) -> Option<f64> {
    let valid: Vec<f64> = draws.iter().filter_map(|draw| *draw).collect();
    if valid.is_empty() {
        return None;
    }
    let at_least = valid.iter().filter(|draw| **draw >= observed).count();
    Some((1 + at_least) as f64 / (valid.len() + 1) as f64)
}

/// Lower-index percentile over the valid draws.
#[must_use]
pub fn percentile(draws: &[Option<f64>], fraction: f64) -> Option<f64> {
    let mut valid: Vec<f64> = draws.iter().filter_map(|draw| *draw).collect();
    if valid.is_empty() {
        return None;
    }
    valid.sort_by(|left, right| left.total_cmp(right));
    let index = ((fraction * (valid.len() - 1) as f64) as usize).min(valid.len() - 1);
    Some(valid[index])
}

/// Lower confidence bound.
#[must_use]
pub fn lower(value: Option<f64>, se: Option<f64>, z: f64) -> Option<f64> {
    value.zip(se).map(|(value, se)| value - z * se)
}

/// Bonferroni-corrected z for the primary cells.
#[must_use]
pub fn primary_z(cells: usize) -> f64 {
    inverse_normal_cdf(1.0 - 0.05 / (2.0 * cells as f64))
}

/// Two-sided validation z.
#[must_use]
pub fn validation_z() -> f64 {
    inverse_normal_cdf(1.0 - 0.05 / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clusters_form_within_bundles() {
        let stats = cluster_stats(
            &[
                (0, 0, true),
                (0, 1, false),
                (0, 10, true),
                (0, 11, true),
                (1, 0, false),
            ],
            5,
        );
        assert_eq!(stats.n, 5);
        assert_eq!(stats.clusters, 3);
        assert_eq!(stats.singleton_bundles, 1);
        assert!((stats.hit_rate.expect("rate") - 0.6).abs() < 1e-12);
    }

    #[test]
    fn control_p_and_percentile_ignore_missing_draws() {
        let draws = vec![Some(0.1), None, Some(0.5), Some(0.9)];
        assert_eq!(percentile(&draws, 0.5), Some(0.5));
        assert_eq!(control_p(0.5, &draws), Some(3.0 / 4.0));
    }

    #[test]
    fn z_values_match_the_contract() {
        assert!((primary_z(12) - 2.865_260_238_532_133_3).abs() < 1e-14);
        assert!((validation_z() - 1.959_963_984_540_053_6).abs() < 1e-14);
        assert_eq!(lower(Some(0.1), Some(0.01), 2.0), Some(0.08));
    }
}
