//! The `simulation` family parser.

use quantrule_api::{Digest, MarkAction, MarkOrigin, MarkQuality, Side, SimulationStatus};

use super::{parse_limit, parse_options, parse_u64, single, usage};
use crate::command::types::{Command, ParseError};

pub(super) fn parse_simulation(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: simulation list|create|windows <sim_id>|mark <sim_id>|marks <sim_id>|oracle <sim_id>|check <sim_id>|decide <sim_id>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => {
            let line = usage("simulation list");
            let usage = line.as_str();
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
            let line = usage("simulation create");
            let usage = line.as_str();
            let options = parse_options(
                rest,
                usage,
                &["premise", "bundle", "inputs", "seed", "criteria"],
                &[],
            )?;
            options.expect_positionals(0, usage)?;
            let bundle = Digest::parse(options.required("bundle", usage)?)
                .map_err(|error| ParseError::Usage(format!("--bundle {error}")))?;
            Ok(Command::SimulationCreate {
                premise: options.required("premise", usage)?.to_owned(),
                bundle,
                inputs: options.value("inputs").unwrap_or("").to_owned(),
                seed: parse_u64(options.required("seed", usage)?, "--seed", usage)?,
                criteria: options.required("criteria", usage)?.to_owned(),
            })
        }
        "windows" => {
            let line = usage("simulation windows");
            let usage = line.as_str();
            single(rest, usage).map(|sim_id| Command::SimulationWindows { sim_id })
        }
        "mark" => {
            let line = usage("simulation mark");
            let usage = line.as_str();
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
            let line = usage("simulation marks");
            let usage = line.as_str();
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
            let line = usage("simulation oracle");
            let usage = line.as_str();
            single(rest, usage).map(|sim_id| Command::SimulationOracle { sim_id })
        }
        "check" => {
            let line = usage("simulation check");
            let usage = line.as_str();
            single(rest, usage).map(|sim_id| Command::SimulationCheck { sim_id })
        }
        "decide" => {
            let line = usage("simulation decide");
            let usage = line.as_str();
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
