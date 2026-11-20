//! The shell command parser.
//!
//! Interactive input and OS argv resolve to the same [`Invocation`]. The parser
//! owns tokenization, the global flags, and the per-command grammar; it does not
//! open a file, reach the network, or compute a value.

use std::collections::{BTreeMap, BTreeSet};

use super::help::{is_command, usage as registered_usage};
use super::types::{Command, GlobalOptions, HelpTopic, Invocation, ParseError};

mod history;
mod parse_research;
mod parse_stage;
use self::history::parse_history;
use self::parse_research::{parse_baseline, parse_invalidation, parse_research};
use self::parse_stage::parse_stage;
use crate::token::BearerToken;

/// The `usage: ...` line for a registered command.
///
/// The line comes from the help registry, which is the checked command
/// contract; the parser keeps no second copy. A name the registry does not list
/// is a parser defect, and the fallback still names the command rather than
/// panicking the client.
fn usage(name: &str) -> String {
    format!("usage: {}", registered_usage(name).unwrap_or(name))
}

/// Parse an already-split argv, after the binary name.
///
/// # Errors
///
/// Returns [`ParseError`] when a global flag, command, argument, or option is
/// invalid.
pub(crate) fn parse_argv(argv: &[String]) -> Result<Option<Invocation>, ParseError> {
    let (globals, rest) = extract_globals(argv)?;
    if rest.is_empty() {
        return Ok(None);
    }
    let command = parse_command(&rest)?;
    Ok(Some(Invocation { globals, command }))
}

/// Parse one interactive line.
///
/// # Errors
///
/// Returns the same failures as [`parse_argv`].
pub(crate) fn parse_line(line: &str) -> Result<Option<Invocation>, ParseError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    parse_argv(&tokenize(trimmed)?)
}

/// Split a line into tokens, honouring single and double quotes.
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

/// Remove the global flags from anywhere in the invocation.
fn extract_globals(argv: &[String]) -> Result<(GlobalOptions, Vec<String>), ParseError> {
    let mut globals = GlobalOptions::default();
    let mut rest = Vec::with_capacity(argv.len());
    let mut index = 0;

    while index < argv.len() {
        let token = argv[index].as_str();
        if token == "--json" {
            globals.json = true;
            index += 1;
            continue;
        }
        if let Some(value) = token.strip_prefix("--profile=") {
            set_profile(&mut globals, value)?;
            index += 1;
            continue;
        }
        if token == "--profile" {
            let value = argv
                .get(index + 1)
                .ok_or_else(|| ParseError::Usage(String::from("--profile requires a value")))?;
            set_profile(&mut globals, value)?;
            index += 2;
            continue;
        }
        if let Some(value) = token.strip_prefix("--token=") {
            set_token(&mut globals, value)?;
            index += 1;
            continue;
        }
        if token == "--token" {
            let value = argv
                .get(index + 1)
                .ok_or_else(|| ParseError::Usage(String::from("--token requires a value")))?;
            set_token(&mut globals, value)?;
            index += 2;
            continue;
        }
        rest.push(token.to_owned());
        index += 1;
    }

    Ok((globals, rest))
}

fn set_profile(globals: &mut GlobalOptions, value: &str) -> Result<(), ParseError> {
    if globals.profile.is_some() {
        return Err(ParseError::Usage(String::from(
            "--profile was supplied twice",
        )));
    }
    quantrule_api::ProfileId::parse(value)
        .map_err(|error| ParseError::Usage(format!("--profile {error}")))?;
    globals.profile = Some(value.to_owned());
    Ok(())
}

fn set_token(globals: &mut GlobalOptions, value: &str) -> Result<(), ParseError> {
    if globals.token.is_some() {
        return Err(ParseError::Usage(String::from(
            "--token was supplied twice",
        )));
    }
    let token = BearerToken::parse(value)
        .map_err(|reason| ParseError::Usage(format!("--token {reason}")))?;
    globals.token = Some(token);
    Ok(())
}

/// The options a subcommand accepts.
struct Options<'a> {
    positionals: Vec<&'a str>,
    values: BTreeMap<&'a str, &'a str>,
    flags: BTreeSet<&'a str>,
}

impl<'a> Options<'a> {
    fn value(&self, name: &str) -> Option<&'a str> {
        self.values.get(name).copied()
    }

    fn required(&self, name: &str, usage: &str) -> Result<&'a str, ParseError> {
        self.value(name)
            .ok_or_else(|| ParseError::Usage(format!("--{name} is required\n{usage}")))
    }

    fn flag(&self, name: &str) -> bool {
        self.flags.contains(name)
    }

    fn positional(&self, index: usize, name: &str, usage: &str) -> Result<&'a str, ParseError> {
        self.positionals
            .get(index)
            .copied()
            .ok_or_else(|| ParseError::Usage(format!("{name} is required\n{usage}")))
    }

    fn expect_positionals(&self, count: usize, usage: &str) -> Result<(), ParseError> {
        if self.positionals.len() > count {
            return Err(ParseError::Usage(format!(
                "unexpected argument '{}'\n{usage}",
                self.positionals[count]
            )));
        }
        Ok(())
    }
}

fn parse_options<'a>(
    args: &'a [String],
    usage: &str,
    value_names: &[&str],
    flag_names: &[&str],
) -> Result<Options<'a>, ParseError> {
    let mut options = Options {
        positionals: Vec::new(),
        values: BTreeMap::new(),
        flags: BTreeSet::new(),
    };
    let mut index = 0;

    while index < args.len() {
        let token = args[index].as_str();
        if let Some(body) = token.strip_prefix("--") {
            let (name, inline) = match body.split_once('=') {
                Some((name, value)) => (name, Some(value)),
                None => (body, None),
            };
            if flag_names.contains(&name) {
                if inline.is_some() {
                    return Err(ParseError::Usage(format!(
                        "--{name} takes no value\n{usage}"
                    )));
                }
                options.flags.insert(name);
                index += 1;
                continue;
            }
            if value_names.contains(&name) {
                let value = match inline {
                    Some(value) => {
                        index += 1;
                        value
                    }
                    None => {
                        let value = args.get(index + 1).ok_or_else(|| {
                            ParseError::Usage(format!("--{name} requires a value\n{usage}"))
                        })?;
                        index += 2;
                        value.as_str()
                    }
                };
                if options.values.insert(name, value).is_some() {
                    return Err(ParseError::Usage(format!(
                        "--{name} was supplied twice\n{usage}"
                    )));
                }
                continue;
            }
            return Err(ParseError::Usage(format!(
                "unknown option '--{name}'\n{usage}"
            )));
        }
        options.positionals.push(token);
        index += 1;
    }

    Ok(options)
}

/// Parse an option value from the argv.
fn parse_command(rest: &[String]) -> Result<Command, ParseError> {
    let (name, args) = rest
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from("no command; type help")))?;
    match name.as_str() {
        "health" => {
            let line = usage("health");
            no_arguments(args, line.as_str()).map(|()| Command::Health)
        }
        "dataset" => parse_dataset(args),
        "config" => parse_config(args),
        "run" => parse_run(args),
        "stage" => parse_stage(args),
        "research" => parse_research(args),
        "baseline" => parse_baseline(args),
        "invalidation" => parse_invalidation(args),
        "inspect" => parse_inspect(args),
        "verify" => single(args, registered_usage("verify").unwrap_or("verify"))
            .map(|run_id| Command::Verify { run_id }),
        "profile" => parse_profile(args),
        "simulation" => parse_simulation(args),
        "history" => parse_history(args),
        "help" => parse_help(args).map(Command::Help),
        "quit" => no_arguments(args, "usage: quit").map(|()| Command::Quit),
        other => Err(ParseError::Unknown(other.to_owned())),
    }
}

fn no_arguments(args: &[String], usage: &str) -> Result<(), ParseError> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(ParseError::Usage(format!(
            "unexpected argument '{}'\n{usage}",
            args[0]
        )))
    }
}

fn single(args: &[String], usage: &str) -> Result<String, ParseError> {
    let options = parse_options(args, usage, &[], &[])?;
    options.expect_positionals(1, usage)?;
    Ok(options.positional(0, "an argument", usage)?.to_owned())
}

fn pair(args: &[String], usage: &str) -> Result<(String, String), ParseError> {
    let options = parse_options(args, usage, &[], &[])?;
    options.expect_positionals(2, usage)?;
    Ok((
        options
            .positional(0, "the first argument", usage)?
            .to_owned(),
        options
            .positional(1, "the second argument", usage)?
            .to_owned(),
    ))
}

mod config;
mod dataset;
mod inspect;
mod profile;
mod run;
mod simulation;

use self::config::parse_config;
use self::dataset::parse_dataset;
use self::inspect::parse_inspect;
use self::profile::parse_profile;
use self::run::parse_run;
use self::simulation::parse_simulation;
fn parse_help(args: &[String]) -> Result<HelpTopic, ParseError> {
    let line = usage("help");
    let usage = line.as_str();
    if args.is_empty() {
        return Ok(HelpTopic::Short);
    }
    if args.len() > 2 {
        return Err(ParseError::Usage(usage.to_owned()));
    }
    if args.len() == 1 {
        match args[0].as_str() {
            "all" => return Ok(HelpTopic::All),
            "glossary" => return Ok(HelpTopic::Glossary),
            "workflows" => return Ok(HelpTopic::Workflows),
            _ => {}
        }
    }
    if matches!(args[0].as_str(), "all" | "glossary" | "workflows") {
        return Err(ParseError::Usage(usage.to_owned()));
    }
    let name = args.join(" ");
    if is_command(&name) {
        Ok(HelpTopic::Command(name))
    } else {
        Err(ParseError::Help(format!(
            "unknown help topic '{name}'; use help all"
        )))
    }
}

fn parse_u64(value: &str, name: &str, usage: &str) -> Result<u64, ParseError> {
    value
        .parse::<u64>()
        .map_err(|_| ParseError::Usage(format!("{name} expects a non-negative integer\n{usage}")))
}

fn parse_limit(value: &str, usage: &str) -> Result<usize, ParseError> {
    let limit = value
        .parse::<usize>()
        .map_err(|_| ParseError::Usage(format!("--limit expects an integer\n{usage}")))?;
    if !(1..=200).contains(&limit) {
        return Err(ParseError::Usage(format!(
            "--limit must be between 1 and 200\n{usage}"
        )));
    }
    Ok(limit)
}

#[cfg(test)]
mod tests;
