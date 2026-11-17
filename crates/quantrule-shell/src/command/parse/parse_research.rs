//! The `research`, `baseline`, and `invalidation` family parsers.

use quantrule_api::{
    ImprovementDecision, ImprovementPartition, ImprovementRunState, ImprovementTier,
};
use serde_json::Value;

use super::{parse_limit, parse_options, single, usage};
use crate::command::types::{Command, ParseError};

pub(super) fn parse_research(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: research create|submit <improvement_id>|list|show <improvement_id>|compare <improvement_id>|decide <improvement_id> --decision <decision>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "create" => {
            let line = usage("research create");
            let usage = line.as_str();
            let options = parse_options(
                rest,
                usage,
                &[
                    "tier",
                    "hypothesis",
                    "partition",
                    "data-version",
                    "config",
                    "criteria",
                    "green-criteria",
                    "baseline",
                    "code-commit",
                    "seed",
                    "analytics",
                ],
                &[],
            )?;
            options.expect_positionals(0, usage)?;
            let tier = parse_tier(options.required("tier", usage)?, usage)?;
            let partition = parse_partition(options.required("partition", usage)?, usage)?;
            let green_criteria =
                parse_green_criteria(options.required("green-criteria", usage)?, usage)?;
            let seed = options
                .value("seed")
                .map(|value| parse_seed(value, usage))
                .transpose()?
                .unwrap_or(0);
            let analytics = options
                .value("analytics")
                .map(|value| parse_bool(value, usage))
                .transpose()?;
            Ok(Command::ResearchCreate {
                tier,
                hypothesis: options.required("hypothesis", usage)?.to_owned(),
                partition,
                data_version: options.required("data-version", usage)?.to_owned(),
                config_id: options.required("config", usage)?.to_owned(),
                criteria_id: options.value("criteria").map(str::to_owned),
                green_criteria,
                baseline: options.value("baseline").map(str::to_owned),
                code_commit: options.required("code-commit", usage)?.to_owned(),
                seed,
                analytics,
            })
        }
        "submit" => {
            let line = usage("research submit");
            let usage = line.as_str();
            single(rest, usage).map(|improvement_id| Command::ResearchSubmit { improvement_id })
        }
        "list" => {
            let line = usage("research list");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["tier", "state", "limit"], &[])?;
            options.expect_positionals(0, usage)?;
            let tier = options
                .value("tier")
                .map(|value| parse_tier(value, usage))
                .transpose()?;
            let state = options
                .value("state")
                .map(|value| parse_state(value, usage))
                .transpose()?;
            let limit = options
                .value("limit")
                .map(|value| parse_limit(value, usage))
                .transpose()?;
            Ok(Command::ResearchList { tier, state, limit })
        }
        "show" => {
            let line = usage("research show");
            let usage = line.as_str();
            single(rest, usage).map(|improvement_id| Command::ResearchShow { improvement_id })
        }
        "compare" => {
            let line = usage("research compare");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["baseline"], &[])?;
            options.expect_positionals(1, usage)?;
            Ok(Command::ResearchCompare {
                improvement_id: options
                    .positional(0, "the improvement id", usage)?
                    .to_owned(),
                baseline: options.value("baseline").map(str::to_owned),
            })
        }
        "decide" => {
            let line = usage("research decide");
            let usage = line.as_str();
            let options = parse_options(rest, usage, &["decision", "note"], &[])?;
            options.expect_positionals(1, usage)?;
            Ok(Command::ResearchDecide {
                improvement_id: options
                    .positional(0, "the improvement id", usage)?
                    .to_owned(),
                decision: parse_decision(options.required("decision", usage)?, usage)?,
                note: options.required("note", usage)?.to_owned(),
            })
        }
        other => Err(ParseError::Usage(format!(
            "unknown research command '{other}'\n{FAMILY}"
        ))),
    }
}

pub(super) fn parse_baseline(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: baseline list|show <model|strategy|portfolio>";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => {
            let line = usage("baseline list");
            super::no_arguments(rest, line.as_str()).map(|()| Command::BaselineList)
        }
        "show" => {
            let line = usage("baseline show");
            let usage = line.as_str();
            let tier = parse_tier(&single(rest, usage)?, usage)?;
            Ok(Command::BaselineShow { tier })
        }
        other => Err(ParseError::Usage(format!(
            "unknown baseline command '{other}'\n{FAMILY}"
        ))),
    }
}

pub(super) fn parse_invalidation(args: &[String]) -> Result<Command, ParseError> {
    const FAMILY: &str = "usage: invalidation list";
    let (name, rest) = args
        .split_first()
        .ok_or_else(|| ParseError::Usage(String::from(FAMILY)))?;
    match name.as_str() {
        "list" => {
            let line = usage("invalidation list");
            super::no_arguments(rest, line.as_str()).map(|()| Command::InvalidationList)
        }
        other => Err(ParseError::Usage(format!(
            "unknown invalidation command '{other}'\n{FAMILY}"
        ))),
    }
}

fn parse_tier(value: &str, usage: &str) -> Result<ImprovementTier, ParseError> {
    ImprovementTier::parse(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--tier expects model, strategy, or portfolio\n{usage}"
        ))
    })
}

fn parse_partition(value: &str, usage: &str) -> Result<ImprovementPartition, ParseError> {
    ImprovementPartition::parse(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--partition expects model_dev, model_validation, strategy_dev, strategy_validation, portfolio_dev, or portfolio_holdout\n{usage}"
        ))
    })
}

fn parse_state(value: &str, usage: &str) -> Result<ImprovementRunState, ParseError> {
    ImprovementRunState::parse(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--state expects created, submitted, or decided\n{usage}"
        ))
    })
}

fn parse_decision(value: &str, usage: &str) -> Result<ImprovementDecision, ParseError> {
    ImprovementDecision::parse(value).ok_or_else(|| {
        ParseError::Usage(format!(
            "--decision expects accepted, rejected, no-edge, invalidated, or new-hypothesis\n{usage}"
        ))
    })
}

fn parse_green_criteria(value: &str, usage: &str) -> Result<Value, ParseError> {
    let value: Value = serde_json::from_str(value).map_err(|_| {
        ParseError::Usage(format!("--green-criteria expects a JSON object\n{usage}"))
    })?;
    if !value.is_object() {
        return Err(ParseError::Usage(format!(
            "--green-criteria expects a JSON object\n{usage}"
        )));
    }
    Ok(value)
}

fn parse_seed(value: &str, usage: &str) -> Result<u64, ParseError> {
    value
        .parse::<u64>()
        .map_err(|_| ParseError::Usage(format!("--seed expects a non-negative integer\n{usage}")))
}

fn parse_bool(value: &str, usage: &str) -> Result<bool, ParseError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(ParseError::Usage(format!(
            "--analytics expects true or false\n{usage}"
        ))),
    }
}
