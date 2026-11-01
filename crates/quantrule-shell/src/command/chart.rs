//! Market-chart command parsing and execution.
//!
//! The command builds the analytical layers, projects them into a chart
//! series, and writes one PNG. The layers are built before the chart is
//! rendered, so a failure leaves no file behind.

use std::io::Write;
use std::path::PathBuf;

use quantrule_analytics::chart::ChartSeries;
use quantrule_data::Timeframe;

use super::layers::{LayerBuild, load_layers_with_window_and_ladder};
use super::types::{MarketChartArgs, ParseError, RunError, WindowMode};
use crate::chart;
use crate::export;
use crate::fetch::DEFAULT_CHART_CANDLES;

/// Parses the arguments of a `market-chart` command.
///
/// # Errors
///
/// Returns [`ParseError::MarketChart`] when the path, PNG target, timeframe, or
/// candle count is missing or invalid.
pub(crate) fn parse_market_chart_arguments(
    arguments: &[String],
) -> Result<MarketChartArgs, ParseError> {
    const USAGE: &str = "usage: market-chart <csv-path> --png <path> [--timeframe <1m|5m|15m|1h|4h>] [--candles <n>] [--window full|latest]";
    let mut path = None;
    let mut png = None;
    let mut timeframe = None;
    let mut candles = DEFAULT_CHART_CANDLES;
    let mut candles_seen = false;
    let mut window = WindowMode::Full;
    let mut window_seen = false;
    let mut index = 0;

    while index < arguments.len() {
        match arguments[index].as_str() {
            "--png" => {
                let value = market_chart_value(arguments, &mut index, "--png", USAGE)?;
                if png.is_some() {
                    return Err(ParseError::MarketChart(format!(
                        "--png was supplied twice\n{USAGE}"
                    )));
                }
                png = Some(PathBuf::from(value));
            }
            "--timeframe" => {
                let value = market_chart_value(arguments, &mut index, "--timeframe", USAGE)?;
                if timeframe.is_some() {
                    return Err(ParseError::MarketChart(format!(
                        "--timeframe was supplied twice\n{USAGE}"
                    )));
                }
                timeframe = Some(Timeframe::from_code(value).ok_or_else(|| {
                    ParseError::MarketChart(format!(
                        "--timeframe expects 1m, 5m, 15m, 1h, or 4h, found '{value}'\n{USAGE}"
                    ))
                })?);
            }
            "--candles" => {
                if candles_seen {
                    return Err(ParseError::MarketChart(format!(
                        "--candles was supplied twice\n{USAGE}"
                    )));
                }
                let value = market_chart_value(arguments, &mut index, "--candles", USAGE)?;
                let count = value.parse::<usize>().map_err(|_| {
                    ParseError::MarketChart(format!(
                        "--candles expects a positive integer, found '{value}'\n{USAGE}"
                    ))
                })?;
                if count == 0 || count > DEFAULT_CHART_CANDLES {
                    return Err(ParseError::MarketChart(format!(
                        "--candles must be between 1 and {DEFAULT_CHART_CANDLES}\n{USAGE}"
                    )));
                }
                candles = count;
                candles_seen = true;
            }
            "--window" => {
                if window_seen {
                    return Err(ParseError::MarketChart(format!(
                        "--window was supplied twice\n{USAGE}"
                    )));
                }
                let value = market_chart_value(arguments, &mut index, "--window", USAGE)?;
                window = WindowMode::from_code(value).ok_or_else(|| {
                    ParseError::MarketChart(format!(
                        "--window expects full or latest, found '{value}'\n{USAGE}"
                    ))
                })?;
                window_seen = true;
            }
            option if option.starts_with("--") => {
                return Err(ParseError::MarketChart(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                if path.is_none() {
                    path = Some(PathBuf::from(value));
                } else {
                    return Err(ParseError::MarketChart(format!(
                        "unexpected argument '{value}'\n{USAGE}"
                    )));
                }
            }
        }
        index += 1;
    }

    let path = path.ok_or_else(|| ParseError::MarketChart(format!("missing CSV path\n{USAGE}")))?;
    let png = png.ok_or_else(|| ParseError::MarketChart(format!("missing --png\n{USAGE}")))?;
    if export::outputs_conflict(&path, &png) {
        return Err(ParseError::MarketChart(format!(
            "--png must not resolve to the same file or temporary path as the CSV\n{USAGE}"
        )));
    }

    Ok(MarketChartArgs {
        path,
        png,
        timeframe: timeframe.unwrap_or(Timeframe::M1),
        candles,
        window,
    })
}

fn market_chart_value<'a>(
    arguments: &'a [String],
    index: &mut usize,
    option: &str,
    usage: &str,
) -> Result<&'a str, ParseError> {
    *index += 1;
    let value = arguments
        .get(*index)
        .ok_or_else(|| ParseError::MarketChart(format!("{option} needs a value\n{usage}")))?;
    if value.is_empty() {
        return Err(ParseError::MarketChart(format!(
            "{option} needs a non-empty value\n{usage}"
        )));
    }
    Ok(value)
}

/// Renders the last candles and the four axis panels as a PNG.
///
/// The layers are built before the chart is rendered, so a source or model
/// failure leaves no PNG. The chart is written atomically after encoding.
///
/// # Errors
///
/// Returns a [`RunError`] when the source, the layer build, the chart, the PNG
/// write, or the summary writer fails.
pub(crate) fn run_market_chart(
    args: &MarketChartArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let LayerBuild { layers, .. } =
        load_layers_with_window_and_ladder(&args.path, args.window, &Timeframe::ALL)?;
    let series =
        ChartSeries::from_layers(&layers, u64::from(args.timeframe.seconds()), args.candles);
    let bytes = chart::render_market_png(&series, args.timeframe).map_err(RunError::Chart)?;
    export::write_atomic(&args.png, &bytes).map_err(RunError::ChartWrite)?;
    writeln!(
        writer,
        "png={} timeframe={} candles={} bytes={}",
        args.png.display(),
        args.timeframe.code(),
        series.len(),
        bytes.len()
    )
    .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}
