//! One timeframe's complete model series.
//!
//! The module owns the per-bar readings, context, events, confirmed levels, and
//! progression. It reads only the candles and configuration it is given; it
//! performs no I/O, reads no clock, and uses no randomness.

use std::num::NonZeroU64;

use quantrule_data::Candle;

use crate::context::ContextBar;
use crate::context::ContextSeries;
use crate::events::EventRecord;
use crate::events::compression::{
    CompressionBreakoutConfig, CompressionBreakoutSeries, CompressionRangeConfig,
    CompressionRangeOnsetSeries, CompressionRangeSeries,
};
use crate::events::divergence::{DivergenceConfig, DivergenceEventSeries};
use crate::events::dmi::{DmiConfig, DmiEventSeries};
use crate::events::extended::{ExtendedEventConfig, ExtendedEventSeries};
use crate::events::level::{LevelEventConfig, LevelEventSeries};
use crate::events::signal::{SignalConfig, SignalEventSeries};
use crate::events::trend::{TrendContinuationConfig, TrendContinuationSeries};
use crate::input::{MarketStateConfig, TimeframeInput};
use crate::levels::{ConfirmedLevel, ConfirmedLevelRegistry};
use crate::progression::{ProgressionBar, ProgressionConfig, ProgressionLeg, ProgressionSeries};
use crate::readings::{AdxSeries, ExtendedReadings, MacdSeries, PivotSeries, Readings};
use crate::retention::{EventBatch, EventHistory};

/// The six scalar readings for one closed bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelReadings {
    /// EMA 20.
    pub ema20: Option<f64>,
    /// EMA 50.
    pub ema50: Option<f64>,
    /// EMA 200.
    pub ema200: Option<f64>,
    /// RSI 14.
    pub rsi14: Option<f64>,
    /// ATR 14.
    pub atr14: Option<f64>,
    /// Relative volume.
    pub rvol20: Option<f64>,
    /// Average Directional Index.
    pub adx: Option<f64>,
    /// Positive directional indicator.
    pub di_plus: Option<f64>,
    /// Negative directional indicator.
    pub di_minus: Option<f64>,
}

impl ModelReadings {
    fn at(readings: &Readings, adx: &AdxSeries, bar: usize) -> Self {
        Self {
            ema20: readings.ema20.get(bar),
            ema50: readings.ema50.get(bar),
            ema200: readings.ema200.get(bar),
            rsi14: readings.rsi14.get(bar),
            atr14: readings.atr14.get(bar),
            rvol20: readings.rvol20.get(bar),
            adx: adx.adx.get(bar),
            di_plus: adx.di_plus.get(bar),
            di_minus: adx.di_minus.get(bar),
        }
    }
}

/// The v3 discrete indicator states for one closed bar.
///
/// The states are derived from the published readings, never from a legacy
/// axis score. A strategy may read them; it never recomputes an indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelStates {
    /// +1 fast > mid > slow, -1 fast < mid < slow, 0 otherwise.
    pub ema_stack: Option<i8>,
    /// -1 rsi < 30, +1 rsi > 70, 0 otherwise.
    pub rsi_zone: Option<i8>,
    /// +1 line > signal, -1 line < signal, 0 equal.
    pub macd_state: Option<i8>,
    /// +1 histogram > 0, -1 histogram < 0, 0 equal.
    pub macd_histogram_state: Option<i8>,
    /// 1 ADX at or above its threshold, 0 below.
    pub adx_regime: Option<i8>,
    /// Sign of the ADX change over the configured lookback.
    pub adx_slope: Option<i8>,
    /// Sign of `+DI - -DI`.
    pub di_state: Option<i8>,
    /// Relative-volume regime: -1 below 1.4, 0 from 1.4 to 2.0, +1 at or above 2.0.
    pub rvol_regime: Option<i8>,
}

/// The complete published model state for one closed bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelBar {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The zero-based bar position in the timeframe series.
    pub bar_index: usize,
    /// The candle.
    pub candle: Candle,
    /// The six scalar readings.
    pub readings: ModelReadings,
    /// The v3 discrete indicator states.
    pub states: ModelStates,
    /// Progression.
    pub progression: ProgressionBar,
    /// Context.
    pub context: ContextBar,
}

/// One timeframe's complete model series and records.
#[derive(Debug, Clone, PartialEq)]
pub struct TimeframeModel {
    timeframe_secs: u64,
    candles: Vec<Candle>,
    open_times: Vec<i64>,
    readings: Readings,
    extended: ExtendedReadings,
    macd: MacdSeries,
    adx: AdxSeries,
    adx_threshold: f64,
    adx_slope_bars: usize,
    progression: ProgressionSeries,
    context: ContextSeries,
    events: EventBatch,
    extended_events: ExtendedEventSeries,
    merged_events: Vec<EventRecord>,
    event_history: EventHistory,
    levels: Vec<ConfirmedLevel>,
}

impl TimeframeModel {
    /// Computes the model for one non-empty timeframe.
    ///
    /// Returns `None` when the timeframe is zero or the candle series is empty.
    #[must_use]
    pub fn compute(input: &TimeframeInput<'_>, config: &MarketStateConfig) -> Option<Self> {
        let timeframe_secs = NonZeroU64::new(input.timeframe_secs)?;
        if input.candles.is_empty() {
            return None;
        }

        let readings_config = config.readings_for(input.timeframe_secs);
        let readings = Readings::compute_with_config(input.candles, &readings_config);
        let extended = ExtendedReadings::compute(input.candles, &readings, &config.extended());
        let event_settings = config.events_for(input.timeframe_secs);
        let progression_settings = config.progression_for(input.timeframe_secs);
        let macd = if config.macd_enabled_for(input.timeframe_secs) {
            MacdSeries::compute(
                input.candles,
                readings_config.macd_fast,
                readings_config.macd_slow,
                readings_config.macd_signal,
            )
        } else {
            MacdSeries::empty(input.candles.len())
        };
        let adx = if readings_config.adx_active {
            AdxSeries::compute(input.candles, readings_config.adx)
        } else {
            AdxSeries::empty(input.candles.len())
        };
        let progression = ProgressionSeries::compute_with_config(
            input.candles,
            &readings,
            ProgressionConfig::with_settings(
                input.timeframe_secs,
                progression_settings.target_atr,
                progression_settings.stability_bars,
                progression_settings.invalidation_atr,
                progression_settings.min_target_atr,
            ),
        );
        let context = ContextSeries::compute(&readings);

        let level_events = LevelEventSeries::compute(
            input.candles,
            &readings,
            &LevelEventConfig::with_settings(timeframe_secs, event_settings.level),
        );
        let divergence_events = DivergenceEventSeries::compute(
            &readings,
            &DivergenceConfig::with_window(
                timeframe_secs,
                event_settings.divergence.scale,
                event_settings.divergence.window,
            ),
        );
        let signal_events = SignalEventSeries::compute(
            &readings,
            &macd,
            &SignalConfig::with_settings(timeframe_secs, event_settings.signal).with_ema_periods([
                readings_config.ema_fast,
                readings_config.ema_mid,
                readings_config.ema_slow,
            ]),
        );
        let dmi_events = if event_settings.signal.di_cross {
            DmiEventSeries::compute(&adx, &DmiConfig::new(timeframe_secs))
        } else {
            DmiEventSeries::empty()
        };
        let trend_events = if event_settings.signal.trend_continuation {
            TrendContinuationSeries::compute(
                &readings,
                &macd,
                &adx,
                &TrendContinuationConfig::new(
                    timeframe_secs,
                    event_settings.signal.trend_adx_threshold,
                    event_settings.signal.trend_adx_slope_bars,
                ),
            )
        } else {
            TrendContinuationSeries::empty()
        };
        let compression_events = if event_settings.signal.compression_breakout {
            CompressionBreakoutSeries::compute(
                input.candles,
                &readings,
                &context,
                &CompressionBreakoutConfig::with_settings(
                    timeframe_secs,
                    event_settings.signal.compression_coiling_bars,
                    event_settings.signal.compression_break_atr,
                    event_settings.signal.compression_rvol_min,
                ),
            )
        } else {
            CompressionBreakoutSeries::empty()
        };

        let range_events = if event_settings.signal.compression_range {
            CompressionRangeSeries::compute(
                input.candles,
                &context,
                &CompressionRangeConfig::with_coiling_bars(
                    timeframe_secs,
                    event_settings.signal.compression_range_coiling_bars,
                ),
            )
        } else {
            CompressionRangeSeries::empty()
        };

        let onset_events = if event_settings.signal.compression_range_onset {
            CompressionRangeOnsetSeries::compute(
                input.candles,
                &context,
                &CompressionRangeConfig::with_coiling_bars(
                    timeframe_secs,
                    event_settings.signal.compression_range_onset_coiling_bars,
                ),
            )
        } else {
            CompressionRangeOnsetSeries::empty()
        };
        let extended_events = ExtendedEventSeries::compute(
            input.candles,
            &readings,
            &extended,
            &ExtendedEventConfig::new(timeframe_secs)
                .with_path_window(config.extended().path_window),
        );

        let mut events: Vec<EventRecord> = level_events
            .events()
            .iter()
            .chain(divergence_events.events())
            .chain(signal_events.events())
            .chain(dmi_events.events())
            .chain(trend_events.events())
            .chain(compression_events.events())
            .chain(range_events.events())
            .chain(onset_events.events())
            .copied()
            .collect();
        // Keep the detector append order within one availability/occurrence
        // bar; the contract fixes the level reference order and the signal
        // detector order.
        events.sort_by_key(|event| (event.available_bar_index, event.occurred_bar_index));

        let mut merged_events = Vec::with_capacity(events.len() + extended_events.len());
        merged_events.extend_from_slice(&events);
        merged_events.extend_from_slice(extended_events.events());
        merged_events.sort_by_key(|event| (event.available_bar_index, event.occurred_bar_index));

        let events = EventBatch::new(events);
        let event_history = EventHistory::from_batch(&events);
        let registry = ConfirmedLevelRegistry::from_pivots(&readings.pivots);
        let levels: Vec<ConfirmedLevel> = registry.levels().to_vec();

        let open_times = input.candles.iter().map(Candle::timestamp).collect();

        Some(Self {
            timeframe_secs: input.timeframe_secs,
            candles: input.candles.to_vec(),
            open_times,
            readings,
            extended,
            macd,
            adx,
            adx_threshold: readings_config.adx_threshold,
            adx_slope_bars: readings_config.adx_slope_bars,
            progression,
            context,
            events,
            extended_events,
            merged_events,
            event_history,
            levels,
        })
    }

    /// The source timeframe in seconds.
    #[must_use]
    pub const fn timeframe_secs(&self) -> u64 {
        self.timeframe_secs
    }

    /// The number of closed bars.
    #[must_use]
    pub fn len(&self) -> usize {
        self.candles.len()
    }

    /// Whether the timeframe has no bars.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.candles.is_empty()
    }

    /// The candle at `bar`, or `None` when out of range.
    #[must_use]
    pub fn candle(&self, bar: usize) -> Option<Candle> {
        self.candles.get(bar).copied()
    }

    /// The closed candles for this timeframe.
    ///
    /// The slice is read-only and in timestamp order. The spec run needs it to
    /// compute session readings and session events over the evaluated series.
    #[must_use]
    pub fn candles(&self) -> &[Candle] {
        &self.candles
    }

    /// The scalar readings computed for this timeframe.
    ///
    /// The flat report reads the published series; it never recomputes them.
    #[must_use]
    pub const fn readings(&self) -> &Readings {
        &self.readings
    }

    /// The extended OHLCV readings computed for this timeframe.
    #[must_use]
    pub const fn extended(&self) -> &ExtendedReadings {
        &self.extended
    }

    /// The MACD series computed for this timeframe.
    #[must_use]
    pub const fn macd(&self) -> &MacdSeries {
        &self.macd
    }

    /// The ADX/DI series computed for this timeframe.
    #[must_use]
    pub const fn adx(&self) -> &AdxSeries {
        &self.adx
    }

    /// The configured ADX state at `bar`.
    #[must_use]
    pub fn adx_state(&self, bar: usize) -> crate::readings::AdxState {
        self.adx.state(bar, self.adx_threshold, self.adx_slope_bars)
    }

    /// The pivot series computed for this timeframe.
    #[must_use]
    pub fn pivots(&self) -> &PivotSeries {
        &self.readings.pivots
    }

    /// The Progression series for this timeframe.
    #[must_use]
    pub const fn progression(&self) -> &ProgressionSeries {
        &self.progression
    }

    /// The context series for this timeframe.
    #[must_use]
    pub const fn context(&self) -> &ContextSeries {
        &self.context
    }

    /// The retained progression legs for this timeframe.
    #[must_use]
    pub fn legs(&self) -> &[ProgressionLeg] {
        self.progression.legs()
    }

    /// The bar open times, for the as-of search.
    #[must_use]
    pub fn open_times(&self) -> &[i64] {
        &self.open_times
    }

    /// The v3 discrete states at `bar`, or `None` when out of range.
    ///
    /// This is the cheap accessor for a consumer that reads a state without
    /// assembling the complete model bar.
    #[must_use]
    pub fn states_at(&self, bar: usize) -> Option<ModelStates> {
        if bar >= self.candles.len() {
            return None;
        }
        Some(model_states(
            &self.readings,
            &self.macd,
            &self.adx,
            bar,
            self.adx_threshold,
            self.adx_slope_bars,
        ))
    }

    /// The complete model bar at `bar`, or `None` when out of range.
    #[must_use]
    pub fn bar(&self, bar: usize) -> Option<ModelBar> {
        let candle = self.candles.get(bar).copied()?;
        let progression = self.progression.get(bar)?;
        let context = self.context.get(bar)?;
        Some(ModelBar {
            timeframe_secs: self.timeframe_secs,
            bar_index: bar,
            candle,
            readings: ModelReadings::at(&self.readings, &self.adx, bar),
            states: model_states(
                &self.readings,
                &self.macd,
                &self.adx,
                bar,
                self.adx_threshold,
                self.adx_slope_bars,
            ),
            progression,
            context,
        })
    }

    /// The latest complete model bar.
    #[must_use]
    pub fn latest_bar(&self) -> Option<ModelBar> {
        self.len().checked_sub(1).and_then(|bar| self.bar(bar))
    }

    /// The full event batch, in availability order.
    #[must_use]
    pub fn events(&self) -> &[EventRecord] {
        self.events.as_slice()
    }

    /// The extended events detected for this timeframe.
    ///
    /// These are additive to the v4 event batch; [`Self::events`] carries the
    /// registered event batch. The shell's event CSV publishes both batches.
    #[must_use]
    pub const fn extended_events(&self) -> &ExtendedEventSeries {
        &self.extended_events
    }

    /// The core and extended event batches as one availability-ordered view.
    ///
    /// The detectors append to two separate batches. A reporting consumer needs
    /// one ordered stream, so this method merges them by availability bar and
    /// then occurrence bar. The sort is stable, so records that share both keys
    /// keep the detector append order.
    #[must_use]
    pub fn all_events(&self) -> &[EventRecord] {
        self.merged_events.as_slice()
    }

    /// The full event batch wrapper.
    #[must_use]
    pub const fn event_batch(&self) -> &EventBatch {
        &self.events
    }

    /// The bounded event history.
    #[must_use]
    pub const fn event_history(&self) -> &EventHistory {
        &self.event_history
    }

    /// The confirmed levels, in confirmation order.
    #[must_use]
    pub fn levels(&self) -> &[ConfirmedLevel] {
        &self.levels
    }

    /// The last `count` complete model bars, oldest first.
    #[must_use]
    pub fn tail(&self, count: usize) -> Vec<ModelBar> {
        let start = self.len().saturating_sub(count);
        (start..self.len())
            .filter_map(|bar| self.bar(bar))
            .collect()
    }
}

fn model_states(
    readings: &Readings,
    macd: &MacdSeries,
    adx: &AdxSeries,
    bar: usize,
    adx_threshold: f64,
    adx_slope_bars: usize,
) -> ModelStates {
    let ema_stack = ema_stack(
        readings.ema20.get(bar),
        readings.ema50.get(bar),
        readings.ema200.get(bar),
    );
    let rsi_zone = readings.rsi14.get(bar).map(|rsi| {
        if rsi < 30.0 {
            -1
        } else if rsi > 70.0 {
            1
        } else {
            0
        }
    });
    let adx_state = adx.state(bar, adx_threshold, adx_slope_bars);
    ModelStates {
        ema_stack,
        rsi_zone,
        macd_state: macd_state(macd.line.get(bar), macd.signal.get(bar)),
        macd_histogram_state: macd.histogram.get(bar).map(sign_state),
        adx_regime: adx_state.adx_regime,
        adx_slope: adx_state.adx_slope,
        di_state: adx_state.di_state,
        rvol_regime: readings.rvol20.get(bar).map(rvol_regime_code),
    }
}

fn rvol_regime_code(value: f64) -> i8 {
    if value < 1.4 {
        -1
    } else if value >= 2.0 {
        1
    } else {
        0
    }
}

fn ema_stack(fast: Option<f64>, mid: Option<f64>, slow: Option<f64>) -> Option<i8> {
    let (fast, mid, slow) = (fast?, mid?, slow?);
    if fast > mid && mid > slow {
        Some(1)
    } else if fast < mid && mid < slow {
        Some(-1)
    } else {
        Some(0)
    }
}

fn macd_state(line: Option<f64>, signal: Option<f64>) -> Option<i8> {
    let (line, signal) = (line?, signal?);
    if line > signal {
        Some(1)
    } else if line < signal {
        Some(-1)
    } else {
        Some(0)
    }
}

fn sign_state(value: f64) -> i8 {
    if value > 0.0 {
        1
    } else if value < 0.0 {
        -1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::rvol_regime_code;

    #[test]
    fn rvol_regime_uses_the_contract_thresholds() {
        assert_eq!(rvol_regime_code(1.39), -1);
        assert_eq!(rvol_regime_code(1.4), 0);
        assert_eq!(rvol_regime_code(1.99), 0);
        assert_eq!(rvol_regime_code(2.0), 1);
    }
}
