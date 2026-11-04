//! The model's input contract.
//!
//! The configuration and the per-timeframe candle slice are consumed by both
//! the run orchestration and the assembled state. They live here so neither
//! module has to import the other, and so the input shape has one owner.

use quantrule_data::{Candle, Timeframe};

use crate::readings::{PivotScale, ReadingsConfig};

/// Level-event settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelEventSettings {
    /// Whether the EMA references are active.
    pub ema: bool,
    /// Whether the confirmed-pivot references are active.
    pub pivots: bool,
    /// The proximity zone in ATR units.
    pub test_atr: f64,
    /// The close-beyond distance in ATR units for a break.
    pub break_atr: f64,
}

impl LevelEventSettings {
    /// The contract defaults.
    pub const DEFAULT: Self = Self {
        ema: true,
        pivots: true,
        test_atr: 0.2,
        break_atr: 0.2,
    };
}

impl Default for LevelEventSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Divergence-event settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DivergenceSettings {
    /// The confirmed-pivot scale.
    pub scale: PivotScale,
    /// The RSI window on either side of the pivot.
    pub window: usize,
}

impl DivergenceSettings {
    /// The contract defaults.
    pub const DEFAULT: Self = Self {
        scale: PivotScale::Major,
        window: 2,
    };
}

impl Default for DivergenceSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Signal-event settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalSettings {
    /// The RSI threshold levels.
    pub rsi_levels: [f64; 3],
    /// The EMA pairs, fastest first.
    pub ema_pairs: [(usize, usize); 3],
    /// Whether the MACD line/signal crossover is active.
    pub macd_line_signal_cross: bool,
    /// Whether the MACD histogram slope flip is active.
    pub macd_histogram_slope_flip: bool,
    /// Whether the MACD zero cross is active.
    pub macd_zero_cross: bool,
    /// Whether the DI crossover is active.
    pub di_cross: bool,
    /// Whether the trend-continuation conjunction is active.
    pub trend_continuation: bool,
    /// The ADX threshold for the trend-continuation conjunction.
    pub trend_adx_threshold: f64,
    /// The ADX slope lookback for the trend-continuation conjunction.
    pub trend_adx_slope_bars: usize,
}

impl SignalSettings {
    /// The contract defaults.
    pub const DEFAULT: Self = Self {
        rsi_levels: [30.0, 50.0, 70.0],
        ema_pairs: [(20, 50), (50, 200), (20, 200)],
        macd_line_signal_cross: true,
        macd_histogram_slope_flip: true,
        macd_zero_cross: true,
        di_cross: true,
        trend_continuation: true,
        trend_adx_threshold: 25.0,
        trend_adx_slope_bars: 3,
    };
}

impl Default for SignalSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// All event settings for one timeframe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventSettings {
    /// Level settings.
    pub level: LevelEventSettings,
    /// Divergence settings.
    pub divergence: DivergenceSettings,
    /// Signal settings.
    pub signal: SignalSettings,
}

impl EventSettings {
    /// The contract defaults.
    pub const DEFAULT: Self = Self {
        level: LevelEventSettings::DEFAULT,
        divergence: DivergenceSettings::DEFAULT,
        signal: SignalSettings::DEFAULT,
    };
}

impl Default for EventSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Progression settings for one timeframe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressionSettings {
    /// The fallback target distance in ATR units.
    pub target_atr: f64,
    /// The stability window in bars.
    pub stability_bars: usize,
    /// The invalidation distance in ATR units.
    pub invalidation_atr: f64,
    /// The minimum target distance in ATR units.
    pub min_target_atr: f64,
}

impl ProgressionSettings {
    /// The contract defaults.
    pub const DEFAULT: Self = Self {
        target_atr: 2.0,
        stability_bars: 3,
        invalidation_atr: 0.2,
        min_target_atr: 0.2,
    };
}

impl Default for ProgressionSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The configuration of a market-state run.
///
/// The readings are indexed by [`Timeframe::ALL`] order. A timeframe without an
/// explicit override uses the default reading configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarketStateConfig {
    /// The pivot scale used by Structure and Divergence.
    pub structure_scale: PivotScale,
    readings: [ReadingsConfig; Timeframe::ALL.len()],
    events: [EventSettings; Timeframe::ALL.len()],
    progression: [ProgressionSettings; Timeframe::ALL.len()],
    overridden: [bool; Timeframe::ALL.len()],
    macd_enabled: [bool; Timeframe::ALL.len()],
    max_timeframe_secs: u64,
}

impl MarketStateConfig {
    /// Builds the configuration with the default readings on every timeframe.
    #[must_use]
    pub const fn new(structure_scale: PivotScale) -> Self {
        Self {
            structure_scale,
            readings: [ReadingsConfig::DEFAULT; Timeframe::ALL.len()],
            events: [EventSettings::DEFAULT; Timeframe::ALL.len()],
            progression: [ProgressionSettings::DEFAULT; Timeframe::ALL.len()],
            overridden: [false; Timeframe::ALL.len()],
            macd_enabled: [false; Timeframe::ALL.len()],
            max_timeframe_secs: 14_400,
        }
    }

    /// The reading configuration for one timeframe.
    #[must_use]
    pub fn readings_for(&self, timeframe_secs: u64) -> ReadingsConfig {
        Timeframe::ALL
            .iter()
            .position(|timeframe| u64::from(timeframe.seconds()) == timeframe_secs)
            .map_or_else(ReadingsConfig::default, |index| self.readings[index])
    }

    /// Sets the default readings for every timeframe.
    pub fn set_readings_default(&mut self, readings: ReadingsConfig) {
        self.readings = [readings; Timeframe::ALL.len()];
    }

    /// Sets one timeframe's readings and marks them as an explicit override.
    ///
    /// A timeframe outside [`Timeframe::ALL`] is ignored; the config loader
    /// rejects it before this point.
    pub fn set_readings(&mut self, timeframe: Timeframe, readings: ReadingsConfig) {
        if let Some(index) = Timeframe::ALL
            .iter()
            .position(|candidate| *candidate == timeframe)
        {
            self.readings[index] = readings;
            self.overridden[index] = true;
        }
    }

    /// The event settings for one timeframe.
    #[must_use]
    pub fn events_for(&self, timeframe_secs: u64) -> EventSettings {
        Timeframe::ALL
            .iter()
            .position(|timeframe| u64::from(timeframe.seconds()) == timeframe_secs)
            .map_or_else(EventSettings::default, |index| self.events[index])
    }

    /// The progression settings for one timeframe.
    #[must_use]
    pub fn progression_for(&self, timeframe_secs: u64) -> ProgressionSettings {
        Timeframe::ALL
            .iter()
            .position(|timeframe| u64::from(timeframe.seconds()) == timeframe_secs)
            .map_or_else(ProgressionSettings::default, |index| {
                self.progression[index]
            })
    }

    /// Sets the default events for every timeframe.
    pub fn set_events_default(&mut self, events: EventSettings) {
        self.events = [events; Timeframe::ALL.len()];
    }

    /// Sets one timeframe's events.
    pub fn set_events(&mut self, timeframe: Timeframe, events: EventSettings) {
        if let Some(index) = Timeframe::ALL
            .iter()
            .position(|candidate| *candidate == timeframe)
        {
            self.events[index] = events;
            self.overridden[index] = true;
        }
    }

    /// Sets the default progression settings for every timeframe.
    pub fn set_progression_default(&mut self, progression: ProgressionSettings) {
        self.progression = [progression; Timeframe::ALL.len()];
    }

    /// Sets one timeframe's progression settings.
    pub fn set_progression(&mut self, timeframe: Timeframe, progression: ProgressionSettings) {
        if let Some(index) = Timeframe::ALL
            .iter()
            .position(|candidate| *candidate == timeframe)
        {
            self.progression[index] = progression;
            self.overridden[index] = true;
        }
    }

    /// Enables or disables MACD on every timeframe.
    pub fn set_macd_default(&mut self, enabled: bool) {
        self.macd_enabled = [enabled; Timeframe::ALL.len()];
    }

    /// Enables or disables MACD on one timeframe.
    pub fn set_macd_enabled(&mut self, timeframe: Timeframe, enabled: bool) {
        if let Some(index) = Timeframe::ALL
            .iter()
            .position(|candidate| *candidate == timeframe)
        {
            self.macd_enabled[index] = enabled;
        }
    }

    /// Whether MACD is enabled for one timeframe.
    #[must_use]
    pub fn macd_enabled_for(&self, timeframe_secs: u64) -> bool {
        Timeframe::ALL
            .iter()
            .position(|timeframe| u64::from(timeframe.seconds()) == timeframe_secs)
            .is_some_and(|index| self.macd_enabled[index])
    }

    /// The highest timeframe the model accepts.
    #[must_use]
    pub const fn max_timeframe_secs(&self) -> u64 {
        self.max_timeframe_secs
    }

    /// Sets the highest accepted timeframe.
    pub const fn set_max_timeframe_secs(&mut self, seconds: u64) {
        self.max_timeframe_secs = seconds;
    }

    /// Whether the timeframe's readings were explicitly overridden.
    #[must_use]
    pub fn is_overridden(&self, timeframe: Timeframe) -> bool {
        Timeframe::ALL
            .iter()
            .position(|candidate| *candidate == timeframe)
            .is_some_and(|index| self.overridden[index])
    }
}

impl Default for MarketStateConfig {
    fn default() -> Self {
        Self::new(PivotScale::Major)
    }
}

/// One timeframe's candles, paired with its timeframe in seconds.
#[derive(Debug, Clone, Copy)]
pub struct TimeframeInput<'a> {
    /// The source timeframe in seconds.
    pub timeframe_secs: u64,
    /// The closed candles for this timeframe.
    pub candles: &'a [Candle],
}

impl<'a> TimeframeInput<'a> {
    /// Pairs one timeframe with its closed candles.
    #[must_use]
    pub const fn new(timeframe_secs: u64, candles: &'a [Candle]) -> Self {
        Self {
            timeframe_secs,
            candles,
        }
    }
}
