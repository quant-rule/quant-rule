//! The interactive command-line parser.
//!
//! The parser turns one typed line into a [`Command`]. It owns tokenization
//! and the interactive-only conveniences: a path may be the trimmed remainder
//! of the line, and a model command may carry a trailing `--window` option. It
//! does not open a source or compute a model value.

use std::path::PathBuf;

use super::chart::parse_market_chart_arguments;
use super::dashboard::parse_dashboard_arguments;
use super::fetch::parse_fetch_arguments;
use super::model::parse_model_series_arguments;
use super::report::parse_market_report_arguments;
use super::research::parse_research_arguments;
use super::store::parse_store_arguments;
use super::store_query;
use super::types::{Axis, Command, ModelArgs, ParseError, WindowMode};

/// Parses one command line.
///
/// A blank line returns `Ok(None)`. A `candles` or `candles-tolerant` path is
/// the trimmed remainder of the line, so a path may contain spaces.
///
/// # Errors
///
/// Returns a [`ParseError`] for a missing path, unexpected arguments, or an
/// unknown command.
pub(crate) fn parse(line: &str) -> Result<Option<Command>, ParseError> {
    let line = line.trim();
    if line.is_empty() {
        return Ok(None);
    }

    let (name, rest) = match line.split_once(char::is_whitespace) {
        Some((name, rest)) => (name, rest.trim()),
        None => (line, ""),
    };

    match name {
        "candles" => path_command(rest, "candles", Command::Candles),
        "candles-tolerant" => path_command(rest, "candles-tolerant", Command::CandlesTolerant),
        "timeframes" => path_command(rest, "timeframes", Command::Timeframes),
        "market-state" => parse_model_command(rest, "market-state", Command::MarketState),
        "market-dashboard" => {
            let arguments = tokenize(rest)?;
            parse_dashboard_arguments(&arguments).map(|args| Some(Command::MarketDashboard(args)))
        }
        "events" => parse_model_command(rest, "events", Command::Events),
        "legs" => parse_model_command(rest, "legs", Command::Legs),
        "model-series" => {
            let arguments = tokenize(rest)?;
            parse_model_series_arguments(&arguments).map(|args| Some(Command::ModelSeries(args)))
        }
        "axis" => parse_axis(rest),
        "market-chart" => {
            let arguments = tokenize(rest)?;
            parse_market_chart_arguments(&arguments).map(|args| Some(Command::MarketChart(args)))
        }
        "market-report" => {
            let arguments = tokenize(rest)?;
            parse_market_report_arguments(&arguments).map(|args| Some(Command::MarketReport(args)))
        }
        "research" => {
            let arguments = tokenize(rest)?;
            parse_research_arguments(&arguments)
        }
        "store" => {
            let arguments = tokenize(rest)?;
            parse_store_arguments(&arguments)
        }
        "db" => {
            let arguments = tokenize(rest)?;
            store_query::parse_db_arguments(&arguments)
        }
        "fetch" => {
            let arguments = tokenize(rest)?;
            parse_fetch_arguments(&arguments).map(|args| Some(Command::Fetch(args)))
        }
        "help" => {
            if rest.is_empty() {
                Ok(Some(Command::Help))
            } else {
                Err(ParseError::UnexpectedArguments("help"))
            }
        }
        "quit" => {
            if rest.is_empty() {
                Ok(Some(Command::Quit))
            } else {
                Err(ParseError::UnexpectedArguments("quit"))
            }
        }
        _ => Err(ParseError::Unknown(name.to_owned())),
    }
}

fn tokenize(input: &str) -> Result<Vec<String>, ParseError> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut in_token = false;

    for character in input.chars() {
        match quote {
            Some(delimiter) if character == delimiter => {
                quote = None;
                in_token = true;
            }
            Some(_) => {
                current.push(character);
                in_token = true;
            }
            None if character == '"' || character == '\'' => {
                quote = Some(character);
                in_token = true;
            }
            None if character.is_whitespace() => {
                if in_token {
                    tokens.push(std::mem::take(&mut current));
                    in_token = false;
                }
            }
            None => {
                current.push(character);
                in_token = true;
            }
        }
    }

    if quote.is_some() {
        return Err(ParseError::Fetch(String::from(
            "unterminated quoted argument",
        )));
    }
    if in_token {
        tokens.push(current);
    }

    Ok(tokens)
}

fn parse_model_command(
    rest: &str,
    name: &'static str,
    build: fn(ModelArgs) -> Command,
) -> Result<Option<Command>, ParseError> {
    let (body, window) = split_window_option(rest, name)?;
    let path = body.trim();
    if path.is_empty() {
        return Err(ParseError::MissingPath(name));
    }
    Ok(Some(build(ModelArgs::new(PathBuf::from(path), window))))
}

fn parse_axis(rest: &str) -> Result<Option<Command>, ParseError> {
    let (body, window) = split_window_option(rest, "axis")?;
    let (path, axis) = body
        .rsplit_once(char::is_whitespace)
        .ok_or(ParseError::AxisUsage)?;
    let path = path.trim();
    let axis = axis.trim();
    if path.is_empty() || axis.is_empty() {
        return Err(ParseError::AxisUsage);
    }
    let axis = Axis::from_code(axis).ok_or_else(|| ParseError::UnknownAxis(axis.to_owned()))?;
    Ok(Some(Command::Axis {
        args: ModelArgs::new(PathBuf::from(path), window),
        axis,
    }))
}

/// Splits an optional trailing `--window <full|latest>` from a model command.
///
/// The path is kept as the trimmed remainder, so an interactive path may still
/// contain spaces. The option is only recognized at the end of the command.
fn split_window_option<'a>(
    rest: &'a str,
    command: &'static str,
) -> Result<(&'a str, WindowMode), ParseError> {
    let usage = format!("usage: {command} <csv-path> [--window full|latest]");
    let rest = rest.trim();
    let (body, value) = if let Some(position) = rest.rfind(" --window=") {
        (
            rest[..position].trim_end(),
            rest[position + " --window=".len()..].trim(),
        )
    } else if let Some(position) = rest.rfind(" --window") {
        let value = rest[position + " --window".len()..].trim();
        (rest[..position].trim_end(), value)
    } else if let Some(value) = rest.strip_prefix("--window=") {
        ("", value.trim())
    } else if let Some(value) = rest.strip_prefix("--window") {
        ("", value.trim())
    } else {
        (rest, "full")
    };

    if body.contains("--window") {
        return Err(ParseError::Window(format!(
            "the window option must appear once at the end\n{usage}"
        )));
    }
    if value.is_empty() {
        return Err(ParseError::Window(format!(
            "--window requires full or latest\n{usage}"
        )));
    }
    if value.split_whitespace().count() != 1 {
        return Err(ParseError::Window(format!(
            "--window requires exactly one value, full or latest\n{usage}"
        )));
    }
    let window = WindowMode::from_code(value).ok_or_else(|| {
        ParseError::Window(format!(
            "--window expects full or latest, found '{value}'\n{usage}"
        ))
    })?;
    Ok((body, window))
}

fn path_command(
    rest: &str,
    name: &'static str,
    build: fn(PathBuf) -> Command,
) -> Result<Option<Command>, ParseError> {
    if rest.is_empty() {
        Err(ParseError::MissingPath(name))
    } else {
        Ok(Some(build(PathBuf::from(rest))))
    }
}
