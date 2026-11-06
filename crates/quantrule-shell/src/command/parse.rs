//! The interactive command-line parser.
//!
//! The parser turns one typed line into a [`Command`]. It owns tokenization
//! and the interactive-only conveniences: a path may be the trimmed remainder
//! of the line, and the event command may carry a trailing `--window` option.
//! It does not open a source or compute a model value.

use std::path::PathBuf;

use super::fetch::parse_fetch_arguments;
use super::policy::parse_policy_arguments;
use super::report_run::parse_report_arguments;
use super::strategy::parse_strategy_arguments;
use super::types::{Command, HelpTopic, ModelArgs, ParseError, WindowMode};

/// Parses one command line.
///
/// A blank line returns `Ok(None)`. A path command takes the trimmed remainder
/// of the line, so a path may contain spaces.
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
        "events" => parse_model_command(rest, "events", Command::Events),
        "report" => {
            let arguments = tokenize(rest)?;
            parse_report_arguments(&arguments).map(|args| Some(Command::Report(args)))
        }
        "policy" => {
            let arguments = tokenize(rest)?;
            parse_policy_arguments(&arguments).map(Some)
        }
        "strategy" => {
            let arguments = tokenize(rest)?;
            parse_strategy_arguments(&arguments).map(Some)
        }
        "fetch" => {
            let arguments = tokenize(rest)?;
            parse_fetch_arguments(&arguments).map(|args| Some(Command::Fetch(args)))
        }
        "help" => {
            let arguments = tokenize(rest)?;
            parse_help_arguments(&arguments).map(|topic| Some(Command::Help(topic)))
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
        return Err(ParseError::Usage(String::from(
            "unterminated quoted argument",
        )));
    }
    if in_token {
        tokens.push(current);
    }

    Ok(tokens)
}

/// Parses an already-split `events <csv-path> [--window full|latest]`.
///
/// # Errors
///
/// Returns [`ParseError::MissingPath`] when the path is absent, and
/// [`ParseError::Window`] for a repeated or invalid window.
pub(crate) fn parse_model_arguments(
    arguments: &[String],
    name: &'static str,
) -> Result<ModelArgs, ParseError> {
    const USAGE: &str = "usage: events <csv-path> [--window full|latest]";
    let mut path = None;
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
                    ParseError::Window(format!("--window requires a value\n{USAGE}"))
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

    let path = path.ok_or(ParseError::MissingPath(name))?;
    Ok(ModelArgs::new(path, window))
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

/// Splits an optional trailing `--window <full|latest>` from the event command.
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

/// Parses the optional one- or two-word help topic.
///
/// `all` and `glossary` are reserved topics. Any other topic must name a
/// command in the help metadata.
///
/// # Errors
///
/// Returns [`ParseError::Usage`] for more than two words, and
/// [`ParseError::Help`] for a name that is not a command.
pub(crate) fn parse_help_arguments(arguments: &[String]) -> Result<HelpTopic, ParseError> {
    const USAGE: &str = "usage: help [<command>|all|glossary]";
    if arguments.len() > 2 {
        return Err(ParseError::Usage(USAGE.to_owned()));
    }
    if arguments.is_empty() {
        return Ok(HelpTopic::Short);
    }
    if arguments.len() == 1 {
        match arguments[0].as_str() {
            "all" => return Ok(HelpTopic::All),
            "glossary" => return Ok(HelpTopic::Glossary),
            _ => {}
        }
    }
    if matches!(arguments[0].as_str(), "all" | "glossary") {
        return Err(ParseError::Usage(USAGE.to_owned()));
    }
    let name = arguments.join(" ");
    if super::help::is_command(&name) {
        Ok(HelpTopic::Command(name))
    } else {
        Err(ParseError::Help(format!(
            "unknown help topic '{name}'; use help all"
        )))
    }
}
