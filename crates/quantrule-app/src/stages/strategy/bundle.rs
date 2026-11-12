//! Read a model bundle into the strategy's published snapshot.
//!
//! The reader consumes only `readings.csv`, `events.csv`, and
//! `provenance.json`. It never reads `result.json`, a candle, a fill, an order,
//! cash, equity, or P&L. The model bundle is the handoff, and the strategy stage
//! does not recompute an upstream value.

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::artifact::provenance::{ProvenanceError, StageProvenance};
use quantrule_backtest::strategy::{
    EventStrategyInput, EventTimeframeView, EventView, context_window_bars,
};
use quantrule_model::context::ContextBar;
use quantrule_model::timeframe::ModelStates;

/// The model bundle could not be read.
#[derive(Debug)]
pub enum StrategyBundleError {
    /// A required artifact could not be read.
    Read {
        /// The artifact path.
        path: PathBuf,
        /// The underlying I/O error.
        source: io::Error,
    },
    /// A required artifact is not shaped like a model bundle.
    Invalid {
        /// The artifact path.
        path: PathBuf,
        /// What is wrong, with the artifact's own field names.
        message: String,
    },
}

impl fmt::Display for StrategyBundleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(formatter, "cannot read {}: {source}", path.display())
            }
            Self::Invalid { path, message } => {
                write!(
                    formatter,
                    "invalid model bundle {}: {message}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for StrategyBundleError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Invalid { .. } => None,
        }
    }
}

/// One published model bar, reduced to the values a strategy may read.
#[derive(Debug, Clone)]
pub struct BarRow {
    /// The timeframe in seconds.
    pub timeframe_secs: u64,
    /// The zero-based bar position.
    pub bar: usize,
    /// The bar's UTC open time.
    pub timestamp: i64,
    /// The published discrete states.
    pub states: ModelStates,
    /// The published progression stage, or `None`.
    pub progression_stage: Option<i8>,
    /// The published progression leg direction, or `None`.
    pub progression_leg_direction: Option<i8>,
    /// The published context values, or `None`.
    pub context: Option<ContextBar>,
    /// The published ATR(14), or `None`.
    pub atr: Option<f64>,
}

/// A completed model bundle ready for the strategy stage.
#[derive(Debug, Clone)]
pub struct ModelBundle {
    /// The input candle identity.
    pub data_version: String,
    /// The model configuration identity.
    pub config_id: String,
    /// The requested period start, as written.
    pub period_from: Option<String>,
    /// The requested period end, as written.
    pub period_to: Option<String>,
    bars: BTreeMap<u64, Vec<BarRow>>,
    events: BTreeMap<u64, Vec<EventView>>,
}

impl ModelBundle {
    /// Loads a model bundle from its directory.
    ///
    /// # Errors
    ///
    /// Returns [`StrategyBundleError`] when a required artifact is missing,
    /// unreadable, or not shaped like a model bundle.
    pub fn load(path: &Path) -> Result<Self, StrategyBundleError> {
        let provenance = parse_provenance(&path.join("provenance.json"))?;
        let bars = parse_readings(&path.join("readings.csv"))?;
        let events = parse_events(&path.join("events.csv"))?;
        if bars.is_empty() {
            return Err(StrategyBundleError::Invalid {
                path: path.join("readings.csv"),
                message: "the bundle has no readings".to_owned(),
            });
        }
        for timeframe in events.keys() {
            if !bars.contains_key(timeframe) {
                return Err(StrategyBundleError::Invalid {
                    path: path.join("events.csv"),
                    message: format!("events name timeframe {timeframe}s, but readings do not"),
                });
            }
        }
        Ok(Self {
            data_version: provenance.data_version,
            config_id: provenance.config_id,
            period_from: provenance.period_from,
            period_to: provenance.period_to,
            bars,
            events,
        })
    }

    /// Whether the bundle publishes one timeframe.
    #[must_use]
    pub fn contains_timeframe(&self, timeframe_secs: u64) -> bool {
        self.bars.contains_key(&timeframe_secs)
    }

    /// One timeframe's published bars, or `None`.
    #[must_use]
    pub fn bars(&self, timeframe_secs: u64) -> Option<&[BarRow]> {
        self.bars.get(&timeframe_secs).map(Vec::as_slice)
    }

    /// The closed execution bar at `bar_index`, or `None`.
    #[must_use]
    pub fn execution_bar(&self, timeframe_secs: u64, bar_index: usize) -> Option<&BarRow> {
        let bars = self.bars.get(&timeframe_secs)?;
        bars.binary_search_by_key(&bar_index, |row| row.bar)
            .ok()
            .map(|index| &bars[index])
    }

    /// Builds the one-bar snapshot the strategy engine may read.
    ///
    /// The execution view is first. Each context view is the latest bar whose
    /// close is at or before the execution bar's close, so a higher timeframe
    /// that is still forming is never read.
    #[must_use]
    pub fn input(
        &self,
        execution_timeframe_secs: u64,
        execution_bar_index: usize,
        window_bars: usize,
        context_timeframes: &[u64],
    ) -> Option<EventStrategyInput> {
        let bars = self.bars.get(&execution_timeframe_secs)?;
        let index = bars
            .binary_search_by_key(&execution_bar_index, |row| row.bar)
            .ok()?;
        let row = &bars[index];
        let previous = index.checked_sub(1).and_then(|previous| bars.get(previous));
        let execution_contiguous = previous.is_some_and(|previous| {
            row.timestamp - previous.timestamp == execution_timeframe_secs as i64
        });
        let execution_close = row.timestamp + execution_timeframe_secs as i64;

        let mut timeframes = Vec::with_capacity(1 + context_timeframes.len());
        timeframes.push(view(
            row,
            window_bars,
            self.events_for(execution_timeframe_secs, row.bar, window_bars),
        ));
        for &context in context_timeframes {
            if context == execution_timeframe_secs {
                continue;
            }
            let Some(context_bars) = self.bars.get(&context) else {
                continue;
            };
            let position = context_bars.partition_point(|candidate| {
                candidate.timestamp + context as i64 <= execution_close
            });
            if position == 0 {
                continue;
            }
            let context_row = &context_bars[position - 1];
            let context_window =
                context_window_bars(window_bars, execution_timeframe_secs, context);
            timeframes.push(view(
                context_row,
                context_window,
                self.events_for(context, context_row.bar, context_window),
            ));
        }

        Some(EventStrategyInput {
            execution_timeframe_secs,
            execution_bar_index,
            execution_open_time: row.timestamp,
            execution_contiguous,
            timeframes,
        })
    }

    /// The published events for one timeframe.
    #[must_use]
    pub fn events(&self, timeframe_secs: u64) -> &[EventView] {
        self.events
            .get(&timeframe_secs)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    fn events_for(&self, timeframe_secs: u64, bar: usize, window_bars: usize) -> Vec<EventView> {
        let start = bar.saturating_sub(window_bars.saturating_sub(1));
        self.events(timeframe_secs)
            .iter()
            .filter(|event| event.available_bar_index <= bar && event.available_bar_index >= start)
            .copied()
            .collect()
    }
}

fn view(row: &BarRow, _window_bars: usize, events: Vec<EventView>) -> EventTimeframeView {
    EventTimeframeView {
        timeframe_secs: row.timeframe_secs,
        bar_index: row.bar,
        open_time: row.timestamp,
        states: row.states,
        progression_stage: row.progression_stage,
        progression_leg_direction: row.progression_leg_direction,
        events,
        context: row.context,
    }
}

fn parse_provenance(path: &Path) -> Result<StageProvenance, StrategyBundleError> {
    crate::artifact::provenance::load_model(path).map_err(strategy_bundle_error)
}

fn strategy_bundle_error(error: ProvenanceError) -> StrategyBundleError {
    match error {
        ProvenanceError::Read { path, source } => StrategyBundleError::Read { path, source },
        ProvenanceError::Invalid { path, message } => {
            StrategyBundleError::Invalid { path, message }
        }
    }
}

fn parse_readings(path: &Path) -> Result<BTreeMap<u64, Vec<BarRow>>, StrategyBundleError> {
    let table = Table::read(path)?;
    let timeframe = table.column("timeframe_secs")?;
    let bar = table.column("bar")?;
    let timestamp = table.column("timestamp")?;
    let ema_stack = table.column("ema_stack")?;
    let rsi_zone = table.column("rsi_zone")?;
    let macd_state = table.column("macd_state")?;
    let macd_histogram_state = table.column("macd_histogram_state")?;
    let adx_regime = table.column("adx_regime")?;
    let adx_slope = table.column("adx_slope")?;
    let di_state = table.column("di_state")?;
    let rvol_regime = table.optional_column("rvol_regime");
    let vol_regime = table.optional_column("vol_regime");
    let structure_state = table.optional_column("structure_state");
    let progression_stage = table.column("progression_stage_code")?;
    let progression_direction = table.column("progression_leg_direction")?;
    let atr = table.column("atr")?;
    let atr_ratio = table.column("atr_ratio")?;
    let rvol_ratio = table.column("rvol_ratio")?;
    let context_code = table.column("context_code")?;

    let mut bars: BTreeMap<u64, Vec<BarRow>> = BTreeMap::new();
    for (index, row) in table.rows.iter().enumerate() {
        let timeframe_secs = table.u64(row, timeframe, index)?;
        let bar_index = table.usize(row, bar, index)?;
        let open_time = table.i64(row, timestamp, index)?;
        let states = ModelStates {
            ema_stack: table.optional_i8(row, ema_stack, index)?,
            rsi_zone: table.optional_i8(row, rsi_zone, index)?,
            macd_state: table.optional_i8(row, macd_state, index)?,
            macd_histogram_state: table.optional_i8(row, macd_histogram_state, index)?,
            adx_regime: table.optional_i8(row, adx_regime, index)?,
            adx_slope: table.optional_i8(row, adx_slope, index)?,
            di_state: table.optional_i8(row, di_state, index)?,
            rvol_regime: match rvol_regime {
                Some(column) => table.optional_i8(row, column, index)?,
                None => None,
            },
            vol_regime: match vol_regime {
                Some(column) => table.optional_i8(row, column, index)?,
                None => None,
            },
            structure_state: match structure_state {
                Some(column) => table.optional_i8(row, column, index)?,
                None => None,
            },
        };
        let context = ContextBar {
            atr_ratio: table.optional_f64(row, atr_ratio, index)?,
            rvol_ratio: table.optional_f64(row, rvol_ratio, index)?,
            context_code: table.optional_u8(row, context_code, index)?,
        };
        let context = if context.atr_ratio.is_none()
            && context.rvol_ratio.is_none()
            && context.context_code.is_none()
        {
            None
        } else {
            Some(context)
        };
        bars.entry(timeframe_secs).or_default().push(BarRow {
            timeframe_secs,
            bar: bar_index,
            timestamp: open_time,
            states,
            progression_stage: table.optional_i8(row, progression_stage, index)?,
            progression_leg_direction: table.optional_i8(row, progression_direction, index)?,
            context,
            atr: table.optional_f64(row, atr, index)?,
        });
    }

    for rows in bars.values_mut() {
        rows.sort_by_key(|row| row.bar);
        for pair in rows.windows(2) {
            if pair[0].bar == pair[1].bar {
                return Err(StrategyBundleError::Invalid {
                    path: path.to_path_buf(),
                    message: format!(
                        "timeframe {}s has two bars at index {}",
                        pair[0].timeframe_secs, pair[0].bar
                    ),
                });
            }
        }
    }
    Ok(bars)
}

fn parse_events(path: &Path) -> Result<BTreeMap<u64, Vec<EventView>>, StrategyBundleError> {
    let table = Table::read(path)?;
    let timeframe = table.column("timeframe_secs")?;
    let event_code = table.column("event_code")?;
    let event_type = table.column("event_type_code")?;
    let subtype = table.column("event_subtype_code")?;
    let source = table.column("source_code")?;
    let occurred = table.column("occurred_bar_index")?;
    let available = table.column("available_bar_index")?;
    let direction = table.column("direction_code")?;
    let reference_code = table.column("reference_code")?;
    let primary = table.column("primary_value")?;
    let reference = table.column("reference_value")?;

    let mut events: BTreeMap<u64, Vec<EventView>> = BTreeMap::new();
    for (index, row) in table.rows.iter().enumerate() {
        let timeframe_secs = table.u64(row, timeframe, index)?;
        let event_code = table.u16(row, event_code, index)?;
        let view = EventView {
            event_code,
            event_type: table.u8(row, event_type, index)?,
            subtype: table.u8(row, subtype, index)?,
            source: table.u16(row, source, index)?,
            direction: table.i8(row, direction, index)?,
            occurred_bar_index: table.usize(row, occurred, index)?,
            available_bar_index: table.usize(row, available, index)?,
            reference_code: table.optional_u32(row, reference_code, index)?,
            primary_value: table.optional_f64(row, primary, index)?,
            reference_value: table.optional_f64(row, reference, index)?,
        };
        events.entry(timeframe_secs).or_default().push(view);
    }
    for rows in events.values_mut() {
        rows.sort_by_key(|event| {
            (
                event.available_bar_index,
                event.event_code,
                event.occurred_bar_index,
                event.event_type,
                event.subtype,
                event.source,
            )
        });
    }
    Ok(events)
}

fn read_text(path: &Path) -> Result<String, StrategyBundleError> {
    fs::read_to_string(path).map_err(|source| StrategyBundleError::Read {
        path: path.to_path_buf(),
        source,
    })
}

struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    fn read(path: &Path) -> Result<Self, StrategyBundleError> {
        let text = read_text(path)?;
        let mut lines = text.lines();
        let header = lines.next().ok_or_else(|| StrategyBundleError::Invalid {
            path: path.to_path_buf(),
            message: "the file is empty".to_owned(),
        })?;
        let header = split_row(path, header)?;
        let mut rows = Vec::new();
        for (number, line) in lines.enumerate() {
            if line.is_empty() {
                continue;
            }
            let row = split_row(path, line)?;
            if row.len() != header.len() {
                return Err(StrategyBundleError::Invalid {
                    path: path.to_path_buf(),
                    message: format!(
                        "row {} has {} fields; the header has {}",
                        number + 2,
                        row.len(),
                        header.len()
                    ),
                });
            }
            rows.push(row);
        }
        Ok(Self { header, rows })
    }

    fn column(&self, name: &str) -> Result<usize, StrategyBundleError> {
        self.header
            .iter()
            .position(|column| column == name)
            .ok_or_else(|| StrategyBundleError::Invalid {
                path: PathBuf::from("the model bundle"),
                message: format!("the required column '{name}' is missing"),
            })
    }

    /// A column that older bundles may not carry. A missing column reads as a
    /// null state, so a strategy that does not use it can still replay an old
    /// bundle.
    fn optional_column(&self, name: &str) -> Option<usize> {
        self.header.iter().position(|column| column == name)
    }

    fn field<'a>(&self, row: &'a [String], column: usize, _number: usize) -> &'a str {
        row.get(column)
            .map(String::as_str)
            .unwrap_or_default()
            .trim()
    }

    fn u64(
        &self,
        row: &[String],
        column: usize,
        number: usize,
    ) -> Result<u64, StrategyBundleError> {
        parse_value(
            self.field(row, column, number),
            number,
            "an unsigned integer",
        )
    }

    fn usize(
        &self,
        row: &[String],
        column: usize,
        number: usize,
    ) -> Result<usize, StrategyBundleError> {
        let value = self.u64(row, column, number)?;
        usize::try_from(value).map_err(|_| StrategyBundleError::Invalid {
            path: PathBuf::from("the model bundle"),
            message: format!("row {number} has a bar index that does not fit this platform"),
        })
    }

    fn i64(
        &self,
        row: &[String],
        column: usize,
        number: usize,
    ) -> Result<i64, StrategyBundleError> {
        parse_value(self.field(row, column, number), number, "a signed integer")
    }

    fn u8(&self, row: &[String], column: usize, number: usize) -> Result<u8, StrategyBundleError> {
        let value = self.u64(row, column, number)?;
        u8::try_from(value).map_err(|_| StrategyBundleError::Invalid {
            path: PathBuf::from("the model bundle"),
            message: format!("row {number} has a value outside u8"),
        })
    }

    fn u16(
        &self,
        row: &[String],
        column: usize,
        number: usize,
    ) -> Result<u16, StrategyBundleError> {
        let value = self.u64(row, column, number)?;
        u16::try_from(value).map_err(|_| StrategyBundleError::Invalid {
            path: PathBuf::from("the model bundle"),
            message: format!("row {number} has a value outside u16"),
        })
    }

    fn i8(&self, row: &[String], column: usize, number: usize) -> Result<i8, StrategyBundleError> {
        let value = self.i64(row, column, number)?;
        i8::try_from(value).map_err(|_| StrategyBundleError::Invalid {
            path: PathBuf::from("the model bundle"),
            message: format!("row {number} has a value outside i8"),
        })
    }

    fn optional_i8(
        &self,
        row: &[String],
        column: usize,
        number: usize,
    ) -> Result<Option<i8>, StrategyBundleError> {
        let value = self.field(row, column, number);
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(parse_value(value, number, "a signed integer")?))
        }
    }

    fn optional_u8(
        &self,
        row: &[String],
        column: usize,
        number: usize,
    ) -> Result<Option<u8>, StrategyBundleError> {
        let value = self.field(row, column, number);
        if value.is_empty() {
            Ok(None)
        } else {
            let parsed: u64 = parse_value(value, number, "an unsigned integer")?;
            u8::try_from(parsed)
                .map(Some)
                .map_err(|_| StrategyBundleError::Invalid {
                    path: PathBuf::from("the model bundle"),
                    message: format!("row {number} has a value outside u8"),
                })
        }
    }

    fn optional_u32(
        &self,
        row: &[String],
        column: usize,
        number: usize,
    ) -> Result<Option<u32>, StrategyBundleError> {
        let value = self.field(row, column, number);
        if value.is_empty() {
            Ok(None)
        } else {
            let parsed: u64 = parse_value(value, number, "an unsigned integer")?;
            u32::try_from(parsed)
                .map(Some)
                .map_err(|_| StrategyBundleError::Invalid {
                    path: PathBuf::from("the model bundle"),
                    message: format!("row {number} has a value outside u32"),
                })
        }
    }

    fn optional_f64(
        &self,
        row: &[String],
        column: usize,
        number: usize,
    ) -> Result<Option<f64>, StrategyBundleError> {
        let value = self.field(row, column, number);
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(parse_value(value, number, "a finite number")?))
        }
    }
}

fn split_row(path: &Path, line: &str) -> Result<Vec<String>, StrategyBundleError> {
    if line.contains('"') {
        return Err(StrategyBundleError::Invalid {
            path: path.to_path_buf(),
            message: "quoted fields are not supported in a model artifact".to_owned(),
        });
    }
    Ok(line.split(',').map(str::to_owned).collect())
}

fn parse_value<T: std::str::FromStr>(
    value: &str,
    number: usize,
    expected: &str,
) -> Result<T, StrategyBundleError> {
    value.parse().map_err(|_| StrategyBundleError::Invalid {
        path: PathBuf::from("the model bundle"),
        message: format!("row {number} has {value:?}; expected {expected}"),
    })
}

#[cfg(test)]
mod tests {
    use super::{ModelBundle, Table};

    #[test]
    fn a_missing_bundle_is_reported() {
        let error = ModelBundle::load(std::path::Path::new("target/no-such-model-bundle"))
            .expect_err("the bundle does not exist");
        assert!(error.to_string().contains("cannot read"));
    }

    #[test]
    fn an_old_bundle_without_the_rvol_regime_column_is_readable() {
        let table = Table {
            header: vec!["bar".to_owned()],
            rows: Vec::new(),
        };
        assert_eq!(table.optional_column("bar"), Some(0));
        assert_eq!(table.optional_column("rvol_regime"), None);
    }
}
