//! Model command parsing and execution.
//!
//! These commands load an M1 CSV, build the M1/M5/M15/H1 ladder, run the
//! model, and render the result. The module owns the model commands' argument
//! grammar and their source-to-render path; it does not implement the model.

use std::io::Write;
use std::path::{Path, PathBuf};

use quantrule_data::{Candle, DEFAULT_WINDOW, Timeframe};
use quantrule_model::run::ModelRun;
use quantrule_model::state::{MarketState, MarketStateConfig, TimeframeInput};

use super::types::{Axis, ModelArgs, ModelSeriesArgs, ParseError, RunError, WindowMode};
use crate::csv;
use crate::render_state;

/// Parses the already-split arguments of a model analysis command.
///
/// Used by the non-interactive entry point, where the operating system has
/// already separated the arguments.
///
/// # Errors
///
/// Returns a parse error for a missing path, a repeated window, an unknown
/// option, or an invalid window value.
pub(crate) fn parse_model_arguments(
    arguments: &[String],
    name: &'static str,
) -> Result<ModelArgs, ParseError> {
    let usage = format!("usage: {name} <csv-path> [--window full|latest]");
    let mut path = None;
    let mut window = WindowMode::Full;
    let mut window_seen = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--window" => {
                if window_seen {
                    return Err(ParseError::Window(format!(
                        "--window was supplied twice\n{usage}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Window(format!("--window requires full or latest\n{usage}"))
                })?;
                window = WindowMode::from_code(value).ok_or_else(|| {
                    ParseError::Window(format!(
                        "--window expects full or latest, found '{value}'\n{usage}"
                    ))
                })?;
                window_seen = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Window(format!(
                    "unknown option '{option}'\n{usage}"
                )));
            }
            value => {
                if path.is_some() {
                    return Err(ParseError::Window(format!(
                        "unexpected argument '{value}'\n{usage}"
                    )));
                }
                path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let path = path.ok_or(ParseError::MissingPath(name))?;
    Ok(ModelArgs::new(path, window))
}

/// Parses the already-split arguments of `model-series`.
///
/// # Errors
///
/// Returns a parse error for a missing path, a repeated or unknown option, an
/// invalid timeframe, or a bar count outside `1..=1000`.
pub(crate) fn parse_model_series_arguments(
    arguments: &[String],
) -> Result<ModelSeriesArgs, ParseError> {
    const USAGE: &str = "usage: model-series <csv-path> [--timeframe <1m|5m|15m|1h>] [--bars <1..1000>] [--window full|latest]";
    let mut path = None;
    let mut window = WindowMode::Full;
    let mut window_seen = false;
    let mut timeframe = None;
    let mut bars = DEFAULT_WINDOW;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--window" => {
                if window_seen {
                    return Err(ParseError::Window(format!(
                        "--window was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Window(format!("--window requires full or latest\n{USAGE}"))
                })?;
                window = WindowMode::from_code(value).ok_or_else(|| {
                    ParseError::Window(format!(
                        "--window expects full or latest, found '{value}'\n{USAGE}"
                    ))
                })?;
                window_seen = true;
            }
            "--timeframe" => {
                if timeframe.is_some() {
                    return Err(ParseError::Window(format!(
                        "--timeframe was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Window(format!("--timeframe requires a value\n{USAGE}"))
                })?;
                timeframe = Some(Timeframe::from_code(value).ok_or_else(|| {
                    ParseError::Window(format!(
                        "--timeframe expects 1m, 5m, 15m, or 1h, found '{value}'\n{USAGE}"
                    ))
                })?);
            }
            "--bars" => {
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Window(format!("--bars requires a value\n{USAGE}"))
                })?;
                let count = value.parse::<usize>().map_err(|_| {
                    ParseError::Window(format!(
                        "--bars expects a positive integer, found '{value}'\n{USAGE}"
                    ))
                })?;
                if count == 0 || count > DEFAULT_WINDOW {
                    return Err(ParseError::Window(format!(
                        "--bars must be between 1 and {DEFAULT_WINDOW}\n{USAGE}"
                    )));
                }
                bars = count;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Window(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if path.is_some() {
                    return Err(ParseError::Window(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
                path = Some(PathBuf::from(value));
            }
        }
        index += 1;
    }

    let path = path.ok_or(ParseError::MissingPath("model-series"))?;
    Ok(ModelSeriesArgs {
        path,
        window,
        timeframe,
        bars,
    })
}

/// Parses the already-split arguments of the `axis` command.
///
/// # Errors
///
/// Returns a parse error for a missing path or axis, an unknown axis, a
/// repeated window, an unknown option, or an invalid window value.
pub(crate) fn parse_axis_arguments(arguments: &[String]) -> Result<(ModelArgs, Axis), ParseError> {
    const USAGE: &str =
        "usage: axis <csv-path> <direction|momentum|structure|progression> [--window full|latest]";
    let mut path = None;
    let mut axis = None;
    let mut window = WindowMode::Full;
    let mut window_seen = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--window" => {
                if window_seen {
                    return Err(ParseError::Window(format!(
                        "--window was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments.get(index).ok_or_else(|| {
                    ParseError::Window(format!("--window requires full or latest\n{USAGE}"))
                })?;
                window = WindowMode::from_code(value).ok_or_else(|| {
                    ParseError::Window(format!(
                        "--window expects full or latest, found '{value}'\n{USAGE}"
                    ))
                })?;
                window_seen = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::Window(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if path.is_none() {
                    path = Some(PathBuf::from(value));
                } else if axis.is_none() {
                    axis = Some(
                        Axis::from_code(value)
                            .ok_or_else(|| ParseError::UnknownAxis(value.to_owned()))?,
                    );
                } else {
                    return Err(ParseError::Window(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
            }
        }
        index += 1;
    }

    let path = path.ok_or_else(|| ParseError::Window(USAGE.to_string()))?;
    let axis = axis.ok_or_else(|| ParseError::Window(USAGE.to_string()))?;
    Ok((ModelArgs::new(path, window), axis))
}

/// Prints the assembled market state for the M1/M5/M15/H1 ladder.
///
/// # Errors
///
/// Returns a [`RunError`] when the source or writer fails.
pub(crate) fn run_market_state(args: &ModelArgs, writer: &mut impl Write) -> Result<(), RunError> {
    let state = load_market_state(&args.path, args.window)?;
    render_state::write_market_state(&state, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Prints one axis per timeframe for the ladder.
///
/// # Errors
///
/// Returns a [`RunError`] when the source or writer fails.
pub(crate) fn run_axis(
    args: &ModelArgs,
    axis: Axis,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let state = load_market_state(&args.path, args.window)?;
    render_state::write_axis(&state, axis, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Prints every event record in availability order.
///
/// # Errors
///
/// Returns a [`RunError`] when the source or writer fails.
pub(crate) fn run_events(args: &ModelArgs, writer: &mut impl Write) -> Result<(), RunError> {
    let run = load_model_run(&args.path, args.window)?;
    render_state::write_event_batch(&run, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Prints the minor and major leg per timeframe.
///
/// # Errors
///
/// Returns a [`RunError`] when the source or writer fails.
pub(crate) fn run_legs(args: &ModelArgs, writer: &mut impl Write) -> Result<(), RunError> {
    let state = load_market_state(&args.path, args.window)?;
    render_state::write_legs(&state, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Prints the last `bars` complete model bars per timeframe as NDJSON.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, the model, or the writer fails.
pub(crate) fn run_model_series(
    args: &ModelSeriesArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let run = load_model_run(&args.path, args.window)?;
    for timeframe in run.timeframes() {
        if let Some(selected) = args.timeframe
            && timeframe.timeframe_secs() != u64::from(selected.seconds())
        {
            continue;
        }
        let window_len = timeframe.len();
        let window_start = window_len.saturating_sub(args.bars);
        for bar in timeframe.tail(args.bars) {
            crate::render_series::write_bar(&bar, window_start, window_len, writer)
                .map_err(RunError::Output)?;
        }
    }
    writer.flush().map_err(RunError::Output)
}

fn load_model_run(path: &Path, window: WindowMode) -> Result<ModelRun, RunError> {
    let set = match window {
        WindowMode::Full => csv::load_full_timeframes(path),
        WindowMode::Latest => csv::load_timeframes(path),
    }
    .map_err(RunError::Source)?;
    let m1: Vec<Candle> = set.series(Timeframe::M1).iter().copied().collect();
    let m5: Vec<Candle> = set.series(Timeframe::M5).iter().copied().collect();
    let m15: Vec<Candle> = set.series(Timeframe::M15).iter().copied().collect();
    let h1: Vec<Candle> = set.series(Timeframe::H1).iter().copied().collect();

    Ok(ModelRun::compute(
        &[
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::M1.seconds()),
                candles: &m1,
            },
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::M5.seconds()),
                candles: &m5,
            },
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::M15.seconds()),
                candles: &m15,
            },
            TimeframeInput {
                timeframe_secs: u64::from(Timeframe::H1.seconds()),
                candles: &h1,
            },
        ],
        &MarketStateConfig::default(),
    ))
}

fn load_market_state(path: &Path, window: WindowMode) -> Result<MarketState, RunError> {
    let run = load_model_run(path, window)?;
    Ok(MarketState::from_run(&run))
}
