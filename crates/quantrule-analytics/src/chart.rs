//! A pure chart view of the Layer 2 bars.
//!
//! The shell renders a PNG from this view. The module selects the last bars of
//! one timeframe and copies the candle and the four continuous axis resolutions
//! from [`LayerSet`]; it does not compute a value,
//! clamp a value, or inspect a later bar.

use quantrule_data::Candle;

use crate::layers::LayerSet;

/// One bar in a chart view.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChartBar {
    /// The 0-based bar position in the timeframe series.
    pub bar_index: usize,
    /// The candle.
    pub candle: Candle,
    /// Direction's continuous resolution in `[-1, +1]`, or `None`.
    pub direction: Option<f64>,
    /// Momentum's continuous resolution in `[-1, +1]`, or `None`.
    pub momentum: Option<f64>,
    /// Structure's continuous resolution in `[-1, +1]`, or `None`.
    pub structure: Option<f64>,
    /// Progression's clamped continuous resolution in `[-1, +1]`, or `None`.
    pub progression: Option<f64>,
}

/// The last bars of one timeframe, oldest first.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeries {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The selected bars, oldest first.
    pub bars: Vec<ChartBar>,
}

impl ChartSeries {
    /// Selects the last `limit` bars of `timeframe_secs`.
    ///
    /// A timeframe with no bars, or a zero limit, produces an empty series. A
    /// limit larger than the series returns the whole series. The rows keep the
    /// Layer 2 order, so the first drawn bar is the oldest selected bar.
    #[must_use]
    pub fn from_layers(layers: &LayerSet, timeframe_secs: u64, limit: usize) -> Self {
        let rows: Vec<&crate::layers::Layer2Row> = layers
            .layer2
            .iter()
            .filter(|row| row.layer1.timeframe_secs == timeframe_secs)
            .collect();
        let start = rows.len().saturating_sub(limit);
        let bars = rows[start..]
            .iter()
            .map(|row| ChartBar {
                bar_index: row.layer1.bar_index,
                candle: row.layer1.candle,
                direction: row.resolution.direction.continuous,
                momentum: row.resolution.momentum.continuous,
                structure: row.resolution.structure.continuous,
                progression: row.resolution.progression.continuous,
            })
            .collect();

        Self {
            timeframe_secs,
            bars,
        }
    }

    /// The number of selected bars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.bars.len()
    }

    /// Whether the view has no bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bars.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use quantrule_data::Candle;
    use quantrule_model::input::TimeframeInput;

    use super::ChartSeries;
    use crate::layers::{LayerConfig, build_layers};

    const M1: u64 = 60;
    const M5: u64 = 300;

    fn candles(count: usize) -> Vec<Candle> {
        (0..count)
            .map(|index| {
                let price = 100.0 + (index % 17) as f64 * 0.1;
                Candle::new(
                    index as u64,
                    1_700_000_000 + index as i64 * 60,
                    price,
                    price + 1.0,
                    price - 1.0,
                    price + 0.5,
                    Some(10.0),
                )
                .expect("the test candle is valid")
            })
            .collect()
    }

    fn input(timeframe_secs: u64, candles: &[Candle]) -> TimeframeInput<'_> {
        TimeframeInput {
            timeframe_secs,
            candles,
        }
    }

    #[test]
    fn selects_the_last_bars_in_oldest_first_order() {
        let m1 = candles(20);
        let set =
            build_layers(&[input(M1, &m1)], &LayerConfig::default()).expect("the fixture builds");
        let series = ChartSeries::from_layers(&set, M1, 5);

        assert_eq!(series.len(), 5);
        assert_eq!(
            series
                .bars
                .iter()
                .map(|bar| bar.bar_index)
                .collect::<Vec<_>>(),
            vec![15, 16, 17, 18, 19]
        );
        for bar in &series.bars {
            let row = set
                .layer2
                .iter()
                .find(|row| row.layer1.bar_index == bar.bar_index)
                .expect("the source row");
            assert_eq!(bar.candle, row.layer1.candle);
            assert_eq!(bar.direction, row.resolution.direction.continuous);
            assert_eq!(bar.momentum, row.resolution.momentum.continuous);
            assert_eq!(bar.structure, row.resolution.structure.continuous);
            assert_eq!(bar.progression, row.resolution.progression.continuous);
        }
    }

    #[test]
    fn a_limit_larger_than_the_series_returns_every_bar() {
        let m1 = candles(3);
        let set =
            build_layers(&[input(M1, &m1)], &LayerConfig::default()).expect("the fixture builds");
        let series = ChartSeries::from_layers(&set, M1, 1_000);
        assert_eq!(series.len(), 3);
    }

    #[test]
    fn a_zero_limit_is_empty() {
        let m1 = candles(3);
        let set =
            build_layers(&[input(M1, &m1)], &LayerConfig::default()).expect("the fixture builds");
        let series = ChartSeries::from_layers(&set, M1, 0);
        assert!(series.is_empty());
    }

    #[test]
    fn an_absent_timeframe_is_empty() {
        let m1 = candles(3);
        let set =
            build_layers(&[input(M1, &m1)], &LayerConfig::default()).expect("the fixture builds");
        let series = ChartSeries::from_layers(&set, M5, 10);
        assert!(series.is_empty());
        assert_eq!(series.timeframe_secs, M5);
    }

    #[test]
    fn repeated_requests_are_identical() {
        let m1 = candles(30);
        let set =
            build_layers(&[input(M1, &m1)], &LayerConfig::default()).expect("the fixture builds");
        assert_eq!(
            ChartSeries::from_layers(&set, M1, 10),
            ChartSeries::from_layers(&set, M1, 10)
        );
    }

    #[test]
    fn a_short_series_keeps_missing_axis_values_null() {
        let m1 = candles(1);
        let set =
            build_layers(&[input(M1, &m1)], &LayerConfig::default()).expect("the fixture builds");
        let series = ChartSeries::from_layers(&set, M1, 1);
        let bar = series.bars.first().expect("one bar");
        assert_eq!(bar.direction, None);
        assert_eq!(bar.momentum, None);
        assert_eq!(bar.structure, None);
        assert_eq!(bar.progression, None);
    }
}
