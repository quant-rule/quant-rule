//! The shell command parser.
//!
//! Interactive input and OS argv resolve to the same [`Invocation`]. The parser
//! owns tokenization, the global flags, and the per-command grammar; it does not
//! open a file, reach the network, or compute a value.

use std::collections::{BTreeMap, BTreeSet};

use quantrule_api::{
    Digest, MarkAction, MarkOrigin, MarkQuality, RunState, Side, SimulationStatus,
};

use super::help::is_command;
use super::types::{Command, GlobalOptions, HelpTopic, Invocation, ParseError};

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
        "health" => no_arguments(args, "usage: health").map(|()| Command::Health),
        "dataset" => parse_dataset(args),
        "config" => parse_config(args),
        "run" => parse_run(args),
        "inspect" => parse_inspect(args),
        "verify" => single(args, "verify <run_id>").map(|run_id| Command::Verify { run_id }),
        "profile" => parse_profile(args),
        "simulation" => parse_simulation(args),
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

fn parse_dataset(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: dataset list|show <data_version>|upload <csv-path>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => no_arguments(rest, "usage: dataset list").map(|()| Command::DatasetList),
        "show" => {
            let usage = "usage: dataset show <data_version>";
            single(rest, usage).map(|data_version| Command::DatasetShow { data_version })
        }
        "upload" => {
            let usage = "usage: dataset upload <csv-path> [--name <name>] [--tolerant]";
            let options = parse_options(rest, usage, &["name"], &["tolerant"])?;
            options.expect_positionals(1, usage)?;
            Ok(Command::DatasetUpload {
                path: options.positional(0, "the CSV path", usage)?.into(),
                name: options.value("name").map(str::to_owned),
                tolerant: options.flag("tolerant"),
            })
        }
        other => Err(ParseError::Usage(format!(
            "unknown dataset command '{other}'\n{FAMILY}"
        ))),
    }
}

fn parse_config(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: config list|show <config_id>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => no_arguments(rest, "usage: config list").map(|()| Command::ConfigList),
        "show" => {
            let usage = "usage: config show <config_id>";
            single(rest, usage).map(|config_id| Command::ConfigShow { config_id })
        }
        other => Err(ParseError::Usage(format!(
            "unknown config command '{other}'\n{FAMILY}"
        ))),
    }
}

fn parse_run(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: run submit <data_version> <config_id>|status <run_id>|list|cancel <run_id>|fetch <run_id> <directory>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "submit" => {
            let usage = "usage: run submit <data_version> <config_id>";
            pair(rest, usage).map(|(data_version, config_id)| Command::RunSubmit {
                data_version,
                config_id,
            })
        }
        "status" => {
            let usage = "usage: run status <run_id>";
            single(rest, usage).map(|run_id| Command::RunStatus { run_id })
        }
        "list" => {
            let usage = "usage: run list [--status <state>] [--limit <n>]";
            let options = parse_options(rest, usage, &["status", "limit"], &[])?;
            options.expect_positionals(0, usage)?;
            let status = options
                .value("status")
                .map(|value| parse_run_state(value, usage))
                .transpose()?;
            let limit = options
                .value("limit")
                .map(|value| parse_limit(value, usage))
                .transpose()?;
            Ok(Command::RunList { status, limit })
        }
        "fetch" => {
            let usage = "usage: run fetch <run_id> <directory>";
            pair(rest, usage).map(|(run_id, directory)| Command::RunFetch {
                run_id,
                directory: directory.into(),
            })
        }
        "cancel" => {
            let usage = "usage: run cancel <run_id>";
            single(rest, usage).map(|run_id| Command::RunCancel { run_id })
        }
        other => Err(ParseError::Usage(format!(
            "unknown run command '{other}'\n{FAMILY}"
        ))),
    }
}

fn parse_inspect(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: inspect candles <data_version>|timeframes <data_version>|events <data_version> --timeframe <tf>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "candles" => {
            let usage = "usage: inspect candles <data_version>";
            single(rest, usage).map(|data_version| Command::InspectCandles { data_version })
        }
        "timeframes" => {
            let usage = "usage: inspect timeframes <data_version>";
            single(rest, usage).map(|data_version| Command::InspectTimeframes { data_version })
        }
        "events" => {
            let usage = "usage: inspect events <data_version> --timeframe <tf>";
            let options = parse_options(rest, usage, &["timeframe"], &[])?;
            options.expect_positionals(1, usage)?;
            Ok(Command::InspectEvents {
                data_version: options.positional(0, "the data version", usage)?.to_owned(),
                timeframe: options.required("timeframe", usage)?.to_owned(),
            })
        }
        other => Err(ParseError::Usage(format!(
            "unknown inspect command '{other}'\n{FAMILY}"
        ))),
    }
}

fn parse_profile(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: profile list|create <name> [--id <profile_id>]|use <profile_id>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => no_arguments(rest, "usage: profile list").map(|()| Command::ProfileList),
        "create" => {
            let usage = "usage: profile create <name> [--id <profile_id>]";
            let options = parse_options(rest, usage, &["id"], &[])?;
            options.expect_positionals(1, usage)?;
            let name = options.positional(0, "the profile name", usage)?.to_owned();
            if name.trim().is_empty() {
                return Err(ParseError::Usage(format!(
                    "the profile name cannot be blank\n{usage}"
                )));
            }
            if let Some(id) = options.value("id") {
                quantrule_api::ProfileId::parse(id)
                    .map_err(|error| ParseError::Usage(format!("--id {error}")))?;
            }
            Ok(Command::ProfileCreate {
                name,
                id: options.value("id").map(str::to_owned),
            })
        }
        "use" => {
            let usage = "usage: profile use <profile_id>";
            let profile_id = single(rest, usage)?;
            quantrule_api::ProfileId::parse(&profile_id)
                .map_err(|error| ParseError::Usage(format!("profile use {error}")))?;
            Ok(Command::ProfileUse { profile_id })
        }
        other => Err(ParseError::Usage(format!(
            "unknown profile command '{other}'\n{FAMILY}"
        ))),
    }
}

fn parse_simulation(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: simulation list|create|windows <sim_id>|mark <sim_id>|marks <sim_id>|oracle <sim_id>|check <sim_id>|decide <sim_id>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => {
            let usage = "usage: simulation list [--status <status>] [--limit <n>]";
            let options = parse_options(rest, usage, &["status", "limit"], &[])?;
            options.expect_positionals(0, usage)?;
            let status = options
                .value("status")
                .map(|value| parse_simulation_status(value, usage))
                .transpose()?;
            let limit = options
                .value("limit")
                .map(|value| parse_limit(value, usage))
                .transpose()?;
            Ok(Command::SimulationList { status, limit })
        }
        "create" => {
            let usage = "usage: simulation create --premise <text> --bundle <digest> --seed <n> --criteria <text>";
            let options =
                parse_options(rest, usage, &["premise", "bundle", "seed", "criteria"], &[])?;
            options.expect_positionals(0, usage)?;
            let bundle = Digest::parse(options.required("bundle", usage)?)
                .map_err(|error| ParseError::Usage(format!("--bundle {error}")))?;
            Ok(Command::SimulationCreate {
                premise: options.required("premise", usage)?.to_owned(),
                bundle,
                seed: parse_u64(options.required("seed", usage)?, "--seed", usage)?,
                criteria: options.required("criteria", usage)?.to_owned(),
            })
        }
        "windows" => {
            let usage = "usage: simulation windows <sim_id>";
            single(rest, usage).map(|sim_id| Command::SimulationWindows { sim_id })
        }
        "mark" => {
            let usage = "usage: simulation mark <sim_id> --chart <chart> --bar <n> --action entry|exit --quality good|bad|neutral --side long|short [--note <text>]";
            let options = parse_options(
                rest,
                usage,
                &["chart", "bar", "action", "quality", "side", "note"],
                &[],
            )?;
            options.expect_positionals(1, usage)?;
            Ok(Command::SimulationMark {
                sim_id: options
                    .positional(0, "the simulation id", usage)?
                    .to_owned(),
                chart: options.required("chart", usage)?.to_owned(),
                bar: parse_u64(options.required("bar", usage)?, "--bar", usage)?,
                action: parse_mark_action(options.required("action", usage)?, usage)?,
                quality: parse_mark_quality(options.required("quality", usage)?, usage)?,
                side: parse_side(options.required("side", usage)?, usage)?,
                note: options.value("note").unwrap_or("").to_owned(),
            })
        }
        "marks" => {
            let usage = "usage: simulation marks <sim_id> [--origin human|oracle|all]";
            let options = parse_options(rest, usage, &["origin"], &[])?;
            options.expect_positionals(1, usage)?;
            let origin = match options.value("origin") {
                None | Some("all") => None,
                Some(value) => Some(parse_mark_origin(value, usage)?),
            };
            Ok(Command::SimulationMarks {
                sim_id: options
                    .positional(0, "the simulation id", usage)?
                    .to_owned(),
                origin,
            })
        }
        "oracle" => {
            let usage = "usage: simulation oracle <sim_id>";
            single(rest, usage).map(|sim_id| Command::SimulationOracle { sim_id })
        }
        "check" => {
            let usage = "usage: simulation check <sim_id>";
            single(rest, usage).map(|sim_id| Command::SimulationCheck { sim_id })
        }
        "decide" => {
            let usage = "usage: simulation decide <sim_id> <test|refine|stop> [--note <text>]";
            let options = parse_options(rest, usage, &["note"], &[])?;
            options.expect_positionals(2, usage)?;
            let decision = options
                .positional(1, "a decision (test, refine, or stop)", usage)?
                .to_owned();
            if !matches!(decision.as_str(), "test" | "refine" | "stop") {
                return Err(ParseError::Usage(format!(
                    "the decision must be test, refine, or stop\n{usage}"
                )));
            }
            Ok(Command::SimulationDecide {
                sim_id: options
                    .positional(0, "the simulation id", usage)?
                    .to_owned(),
                decision,
                note: options.value("note").unwrap_or("").to_owned(),
            })
        }
        other => Err(ParseError::Usage(format!(
            "unknown simulation command '{other}'\n{FAMILY}"
        ))),
    }
}

fn parse_help(args: &[String]) -> Result<HelpTopic, ParseError> {
    const USAGE: &str = "usage: help [<command>|all|glossary|workflows]";
    if args.is_empty() {
        return Ok(HelpTopic::Short);
    }
    if args.len() > 2 {
        return Err(ParseError::Usage(USAGE.to_owned()));
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
        return Err(ParseError::Usage(USAGE.to_owned()));
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

fn parse_run_state(value: &str, usage: &str) -> Result<RunState, ParseError> {
    RunState::parse(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--status expects queued, running, succeeded, failed, or cancelled\n{usage}"
        ))
    })
}

fn parse_simulation_status(value: &str, usage: &str) -> Result<SimulationStatus, ParseError> {
    match value {
        "draft" => Ok(SimulationStatus::Draft),
        "marking" => Ok(SimulationStatus::Marking),
        "reviewed" => Ok(SimulationStatus::Reviewed),
        "checked" => Ok(SimulationStatus::Checked),
        "decided" => Ok(SimulationStatus::Decided),
        _ => Err(ParseError::Usage(format!(
            "--status expects draft, marking, reviewed, checked, or decided\n{usage}"
        ))),
    }
}

fn parse_mark_action(value: &str, usage: &str) -> Result<MarkAction, ParseError> {
    match value {
        "entry" => Ok(MarkAction::Entry),
        "exit" => Ok(MarkAction::Exit),
        _ => Err(ParseError::Usage(format!(
            "--action expects entry or exit\n{usage}"
        ))),
    }
}

fn parse_mark_quality(value: &str, usage: &str) -> Result<MarkQuality, ParseError> {
    match value {
        "good" => Ok(MarkQuality::Good),
        "bad" => Ok(MarkQuality::Bad),
        "neutral" => Ok(MarkQuality::Neutral),
        _ => Err(ParseError::Usage(format!(
            "--quality expects good, bad, or neutral\n{usage}"
        ))),
    }
}

fn parse_side(value: &str, usage: &str) -> Result<Side, ParseError> {
    match value {
        "long" => Ok(Side::Long),
        "short" => Ok(Side::Short),
        _ => Err(ParseError::Usage(format!(
            "--side expects long or short\n{usage}"
        ))),
    }
}

fn parse_mark_origin(value: &str, usage: &str) -> Result<MarkOrigin, ParseError> {
    match value {
        "human" => Ok(MarkOrigin::Human),
        "oracle" => Ok(MarkOrigin::Oracle),
        _ => Err(ParseError::Usage(format!(
            "--origin expects human or oracle\n{usage}"
        ))),
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
mod tests {
    use super::{parse_argv, parse_line};
    use crate::command::types::{Command, HelpTopic};

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn every_family_parses_its_leaf() {
        let cases = [
            (vec!["health"], "health"),
            (vec!["dataset", "list"], "list"),
            (vec!["dataset", "show", "sha256:a"], "show"),
            (vec!["dataset", "upload", "c.csv"], "upload"),
            (vec!["config", "list"], "list"),
            (vec!["config", "show", "cfg"], "show"),
            (vec!["run", "submit", "d", "c"], "submit"),
            (vec!["run", "status", "r"], "status"),
            (vec!["run", "list"], "list"),
            (vec!["run", "fetch", "r", "out"], "fetch"),
            (vec!["run", "cancel", "r"], "cancel"),
            (vec!["inspect", "candles", "d"], "candles"),
            (vec!["inspect", "timeframes", "d"], "timeframes"),
            (
                vec!["inspect", "events", "d", "--timeframe", "5m"],
                "events",
            ),
            (vec!["verify", "r"], "verify"),
            (vec!["profile", "list"], "list"),
            (vec!["profile", "create", "Research"], "create"),
            (vec!["profile", "use", "research"], "use"),
            (vec!["simulation", "list"], "list"),
            (vec!["simulation", "windows", "s"], "windows"),
            (
                vec![
                    "simulation",
                    "mark",
                    "s",
                    "--chart",
                    "w1",
                    "--bar",
                    "4",
                    "--action",
                    "entry",
                    "--quality",
                    "good",
                    "--side",
                    "long",
                ],
                "mark",
            ),
            (vec!["simulation", "marks", "s"], "marks"),
            (vec!["simulation", "oracle", "s"], "oracle"),
            (vec!["simulation", "check", "s"], "check"),
            (vec!["simulation", "decide", "s", "test"], "decide"),
        ];
        for (tokens, _label) in cases {
            let parsed = parse_argv(&argv(&tokens)).expect("the command parses");
            assert!(parsed.is_some(), "{tokens:?}");
        }
    }

    #[test]
    fn a_global_profile_is_removed_from_the_command() {
        let parsed = parse_line("--profile research run list")
            .expect("the line parses")
            .expect("a command");
        assert_eq!(parsed.globals.profile.as_deref(), Some("research"));
        assert!(matches!(parsed.command, Command::RunList { .. }));
    }

    #[test]
    fn a_global_json_flag_is_removed_from_the_command() {
        let parsed = parse_line("health --json")
            .expect("the line parses")
            .expect("a command");
        assert!(parsed.globals.json);
        assert!(matches!(parsed.command, Command::Health));
    }

    #[test]
    fn help_topics_are_one_or_two_words() {
        assert_eq!(
            parse_line("help all").expect("parses").expect("command"),
            crate::command::types::Invocation {
                globals: Default::default(),
                command: Command::Help(HelpTopic::All),
            }
        );
        assert!(matches!(
            parse_line("help run status")
                .expect("parses")
                .expect("command")
                .command,
            Command::Help(HelpTopic::Command(_))
        ));
    }

    #[test]
    fn a_simulation_create_carries_every_required_option() {
        let bundle = format!("sha256:{}", "a".repeat(64));
        let parsed = parse_line(&format!(
            "simulation create --premise p --bundle {bundle} --seed 7 --criteria c"
        ))
        .expect("the line parses")
        .expect("a command");
        assert!(matches!(
            parsed.command,
            Command::SimulationCreate { seed: 7, .. }
        ));
    }

    #[test]
    fn an_unknown_option_is_rejected() {
        assert!(parse_line("run list --path /tmp").is_err());
    }

    #[test]
    fn a_duplicate_profile_is_rejected() {
        assert!(parse_line("--profile a --profile b health").is_err());
    }
}
