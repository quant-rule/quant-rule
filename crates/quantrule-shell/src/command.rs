//! Command parsing and execution for the shell.
//!
//! The shell parses the command text and calls the source and rendering
//! modules. It does not validate candles or assign series indices; those stay
//! in the data engine.

use std::fmt;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::csv::{self, SourceError};
use crate::render;

/// The help text shown by the `help` command.
pub(crate) const HELP: &str = r#"QuantRule shell commands:
  candles <csv-path>            Read a CSV and print the retained candle window as NDJSON.
  candles-tolerant <csv-path>   Read a CSV, skip bad rows, and report quality on stderr.
  help                          Show this help.
  quit                          Exit the shell.

CSV header: Date,Open,High,Low,Close,Volume
Timestamp: YYYY-MM-DD HH:MM:SS+00:00
The window retains the last 1000 candles.
NDJSON: one candle per line, oldest first:
{"series_index":0,"window_index":0,"datetime":"2025-01-01T00:00:00Z","open":1,"high":2,"low":1,"close":2,"volume":null}
Quality report (stderr, candles-tolerant):
quality rows_read=2 accepted=1 rejected=1 duplicate=0 out_of_order=0 invalid=1 evicted=0 retained=1
skipped line=3 reason=invalid: high is below low
Non-interactive: quantrule candles <csv-path>
Non-interactive: quantrule candles-tolerant <csv-path>
"#;

/// A parsed shell command.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    /// Read a CSV source strictly and print the retained window.
    Candles(PathBuf),
    /// Read a CSV source tolerantly and print the window plus a quality report.
    CandlesTolerant(PathBuf),
    /// Print the command reference.
    Help,
    /// Exit the shell.
    Quit,
}

/// A command line could not be parsed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ParseError {
    /// A command that takes a path was entered without one.
    MissingPath(&'static str),
    /// A command that takes no arguments received some.
    UnexpectedArguments(&'static str),
    /// The command name is not recognized.
    Unknown(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPath(command) => {
                write!(formatter, "missing CSV path; usage: {command} <csv-path>")
            }
            Self::UnexpectedArguments(command) => {
                write!(formatter, "{command} takes no arguments")
            }
            Self::Unknown(command) => {
                write!(formatter, "unknown command '{command}'; type 'help'")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// A command was parsed but could not be completed.
#[derive(Debug)]
pub(crate) enum RunError {
    /// The source could not be loaded.
    Source(SourceError),
    /// The output could not be written.
    Output(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(source) => source.fmt(formatter),
            Self::Output(source) => write!(formatter, "cannot write output: {source}"),
        }
    }
}

impl std::error::Error for RunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(source) => Some(source),
            Self::Output(source) => Some(source),
        }
    }
}

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

/// Loads a CSV source strictly and writes its retained window as NDJSON.
///
/// # Errors
///
/// Returns a [`RunError`] when the source fails or the writer fails.
pub(crate) fn run_candles(path: &Path, writer: &mut impl Write) -> Result<(), RunError> {
    let window = csv::load_window(path).map_err(RunError::Source)?;
    render::write_ndjson(&window, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

/// Loads a CSV source tolerantly, writes the retained window as NDJSON, and
/// writes the quality report to a second writer.
///
/// The NDJSON is flushed before the report is written. A source-level failure
/// leaves stdout untouched because the window is built before rendering.
///
/// # Errors
///
/// Returns a [`RunError`] when the source fails or either writer fails.
pub(crate) fn run_candles_tolerant(
    path: &Path,
    output: &mut impl Write,
    report: &mut impl Write,
) -> Result<(), RunError> {
    let loaded = csv::load_window_tolerant(path).map_err(RunError::Source)?;
    render::write_ndjson(&loaded.window, output).map_err(RunError::Output)?;
    output.flush().map_err(RunError::Output)?;
    loaded.report.write(report).map_err(RunError::Output)?;
    report.flush().map_err(RunError::Output)
}

/// Writes the command reference.
///
/// # Errors
///
/// Returns a [`RunError`] when the writer fails.
pub(crate) fn run_help(writer: &mut impl Write) -> Result<(), RunError> {
    writer
        .write_all(HELP.as_bytes())
        .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

#[cfg(test)]
mod tests {
    use super::{Command, ParseError, parse};
    use std::path::PathBuf;

    #[test]
    fn blank_lines_are_ignored() {
        assert_eq!(parse(""), Ok(None));
        assert_eq!(parse("   \t "), Ok(None));
    }

    #[test]
    fn parses_candles_with_a_path() {
        assert_eq!(
            parse("candles data/example.csv"),
            Ok(Some(Command::Candles(PathBuf::from("data/example.csv"))))
        );
    }

    #[test]
    fn parses_tolerant_candles_with_a_path() {
        assert_eq!(
            parse("candles-tolerant data/example.csv"),
            Ok(Some(Command::CandlesTolerant(PathBuf::from(
                "data/example.csv"
            ))))
        );
    }

    #[test]
    fn preserves_spaces_in_a_candles_path() {
        assert_eq!(
            parse("candles my file.csv"),
            Ok(Some(Command::Candles(PathBuf::from("my file.csv"))))
        );
    }

    #[test]
    fn preserves_spaces_in_a_tolerant_candles_path() {
        assert_eq!(
            parse("candles-tolerant my file.csv"),
            Ok(Some(Command::CandlesTolerant(PathBuf::from("my file.csv"))))
        );
    }

    #[test]
    fn rejects_candles_without_a_path() {
        assert_eq!(parse("candles"), Err(ParseError::MissingPath("candles")));
        assert_eq!(parse("candles   "), Err(ParseError::MissingPath("candles")));
    }

    #[test]
    fn rejects_tolerant_candles_without_a_path() {
        assert_eq!(
            parse("candles-tolerant"),
            Err(ParseError::MissingPath("candles-tolerant"))
        );
        assert_eq!(
            parse("candles-tolerant   "),
            Err(ParseError::MissingPath("candles-tolerant"))
        );
    }

    #[test]
    fn parses_help_and_quit() {
        assert_eq!(parse("help"), Ok(Some(Command::Help)));
        assert_eq!(parse("quit"), Ok(Some(Command::Quit)));
    }

    #[test]
    fn rejects_arguments_to_help_and_quit() {
        assert_eq!(
            parse("help now"),
            Err(ParseError::UnexpectedArguments("help"))
        );
        assert_eq!(
            parse("quit now"),
            Err(ParseError::UnexpectedArguments("quit"))
        );
    }

    #[test]
    fn rejects_an_unknown_command() {
        assert_eq!(
            parse("bogus"),
            Err(ParseError::Unknown(String::from("bogus")))
        );
    }
}
