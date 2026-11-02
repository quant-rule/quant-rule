//! The `policy check` command.
//!
//! The command loads the run configuration. The loader validates the policy and
//! every cross-config rule, so this command is the human-visible proof that the
//! policy envelope, strategy, execution, portfolio, and universe agree.

use std::io::Write;
use std::path::Path;

use crate::config::RunConfig;
use crate::labels;

use super::types::{PolicyCheckArgs, RunError};

/// Parses the already-split arguments after `policy`.
///
/// # Errors
///
/// Returns a usage error unless the shape is `check --config <run.json>`.
pub(crate) fn parse_policy_arguments(
    arguments: &[String],
) -> Result<super::types::Command, super::types::ParseError> {
    const USAGE: &str = "usage: policy check --config <run.json>";
    if arguments.first().map(String::as_str) != Some("check") {
        return Err(super::types::ParseError::Policy(USAGE.to_owned()));
    }

    let mut config = None;
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => {
                if config.is_some() {
                    return Err(super::types::ParseError::Policy(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        super::types::ParseError::Policy(format!(
                            "--config requires a non-empty path\n{USAGE}"
                        ))
                    })?;
                config = Some(std::path::PathBuf::from(value));
            }
            option if option.starts_with("--") => {
                return Err(super::types::ParseError::Policy(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                return Err(super::types::ParseError::Policy(format!(
                    "unexpected argument '{value}'\n{USAGE}"
                )));
            }
        }
        index += 1;
    }

    let config = config.ok_or_else(|| {
        super::types::ParseError::Policy(format!("--config is required\n{USAGE}"))
    })?;
    Ok(super::types::Command::PolicyCheck(PolicyCheckArgs {
        config,
    }))
}

/// Loads and validates the policy and prints a deterministic summary.
///
/// # Errors
///
/// Returns [`RunError::Config`] when any config file is invalid, and
/// [`RunError::Output`] when the writer fails.
pub(crate) fn run_policy_check(
    args: &PolicyCheckArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let config = RunConfig::load(Path::new(&args.config)).map_err(RunError::Config)?;
    let timeframes = config
        .policy
        .universe
        .timeframes
        .iter()
        .map(|timeframe| timeframe.code())
        .collect::<Vec<_>>()
        .join(",");
    let tickets = config.policy.universe.tickets.join(",");
    let (strategy_name, risk) = match config.strategy.engine.mode() {
        quantrule_backtest::strategy::StrategyMode::ThreeTimeframe(three) => (
            "three-timeframe",
            format!(
                "stop_level_buffer={} target_next_level min_reward_to_risk={} agreement_bars={}",
                three.level().stop_buffer_fraction(),
                three.level().min_reward_to_risk(),
                three.min_agreement_bars()
            ),
        ),
        quantrule_backtest::strategy::StrategyMode::Breakout(breakout) => (
            "breakout",
            format!(
                "stop_level_buffer={} target_next_level min_reward_to_risk={}",
                breakout.stop_buffer_fraction(),
                breakout.min_reward_to_risk()
            ),
        ),
        quantrule_backtest::strategy::StrategyMode::LevelFade(fade) => (
            "level-fade",
            format!(
                "stop_level_buffer={} target_next_level min_reward_to_risk={}",
                fade.stop_buffer_fraction(),
                fade.min_reward_to_risk()
            ),
        ),
        quantrule_backtest::strategy::StrategyMode::LevelRetest(level) => (
            "level-retest",
            format!(
                "stop_level_buffer={} target_next_level min_reward_to_risk={}",
                level.stop_buffer_fraction(),
                level.min_reward_to_risk()
            ),
        ),
        quantrule_backtest::strategy::StrategyMode::LadderContext => {
            let risk = config.strategy.engine.risk().unwrap_or_default();
            (
                "ladder-context",
                format!(
                    "stop_atr={} reward_to_risk={}",
                    risk.stop_atr_multiple(),
                    risk.reward_to_risk()
                ),
            )
        }
    };
    writeln!(
        writer,
        "policy ok run={} account={} {} target={} {}d max_drawdown={} risk={} leverage={} positions={} tickets={} timeframes={} strategy={} execution={} ladder={} {}",
        config.name,
        config.policy.account.currency,
        config.policy.account.initial_cash,
        config.policy.objective.performance_target,
        config.policy.objective.evaluation_days,
        config.policy.objective.max_drawdown,
        config.policy.risk.max_position_fraction,
        config.policy.risk.max_leverage,
        config.policy.risk.max_open_positions,
        tickets,
        timeframes,
        strategy_name,
        labels::timeframe_label(config.strategy.engine.timeframe_secs()),
        ladder_label(&config),
        risk,
    )
    .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

fn ladder_label(config: &RunConfig) -> String {
    let mut parts =
        Vec::with_capacity(config.strategy.engine.ladder().context_timeframes().len() + 1);
    parts.push(labels::timeframe_label(
        config.strategy.engine.timeframe_secs(),
    ));
    for seconds in config.strategy.engine.ladder().context_timeframes() {
        parts.push(labels::timeframe_label(*seconds));
    }
    parts.join(",")
}

#[cfg(test)]
mod tests {
    use super::parse_policy_arguments;

    fn arguments(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|token| (*token).to_owned()).collect()
    }

    #[test]
    fn parses_the_check_shape() {
        assert!(parse_policy_arguments(&arguments(&["check", "--config", "run.json"])).is_ok());
    }

    #[test]
    fn rejects_other_shapes() {
        assert!(parse_policy_arguments(&arguments(&["check"])).is_err());
        assert!(parse_policy_arguments(&arguments(&["other", "--config", "run.json"])).is_err());
        assert!(
            parse_policy_arguments(&arguments(&[
                "check", "--config", "a.json", "--config", "b.json"
            ]))
            .is_err()
        );
    }
}
