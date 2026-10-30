//! The `strategy explain` command.
//!
//! The command renders the strategy's structured config as the human-readable
//! rule a trader would state. It never runs the model or invents a condition: it
//! prints the thesis, timeframe, entry thresholds, exit rule, and risk envelope
//! exactly as the validated config defines them.

use std::io::Write;
use std::path::Path;

use crate::config::RunConfig;
use crate::labels;

use super::types::{ParseError, RunError, StrategyExplainArgs};

/// Parses the already-split arguments after `strategy explain`.
///
/// # Errors
///
/// Returns a usage error unless the shape is `explain --config <run.json>`.
pub(crate) fn parse_strategy_explain_arguments(
    arguments: &[String],
) -> Result<StrategyExplainArgs, ParseError> {
    const USAGE: &str = "usage: strategy explain --config <run.json>";
    if arguments.first().map(String::as_str) != Some("explain") {
        return Err(ParseError::StrategyExplain(USAGE.to_owned()));
    }

    let mut config = None;
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--config" => {
                if config.is_some() {
                    return Err(ParseError::StrategyExplain(format!(
                        "--config was supplied twice\n{USAGE}"
                    )));
                }
                index += 1;
                let value = arguments
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .ok_or_else(|| {
                        ParseError::StrategyExplain(format!(
                            "--config requires a non-empty path\n{USAGE}"
                        ))
                    })?;
                config = Some(std::path::PathBuf::from(value));
            }
            option if option.starts_with("--") => {
                return Err(ParseError::StrategyExplain(format!(
                    "unknown option '{option}'\n{USAGE}"
                )));
            }
            value => {
                return Err(ParseError::StrategyExplain(format!(
                    "unexpected argument '{value}'\n{USAGE}"
                )));
            }
        }
        index += 1;
    }

    let config = config
        .ok_or_else(|| ParseError::StrategyExplain(format!("--config is required\n{USAGE}")))?;
    Ok(StrategyExplainArgs { config })
}

/// Loads the run and prints the strategy in trader words.
///
/// # Errors
///
/// Returns [`RunError::Config`] when the config is invalid, and
/// [`RunError::Output`] when the writer fails.
pub(crate) fn run_strategy_explain(
    args: &StrategyExplainArgs,
    writer: &mut impl Write,
) -> Result<(), RunError> {
    let config = RunConfig::load(Path::new(&args.config)).map_err(RunError::Config)?;
    let strategy = &config.strategy;
    let engine = &strategy.engine;
    let execution = labels::timeframe_label(engine.timeframe_secs());
    let context = engine
        .ladder()
        .context_timeframes()
        .iter()
        .map(|seconds| labels::timeframe_label(*seconds))
        .collect::<Vec<_>>()
        .join(",");
    let entry = engine.entry();
    let execution_entry = entry.execution();
    let context_entry = entry.context();
    let short_structure_max = if execution_entry.structure_min() == 0.0 {
        0.0
    } else {
        -execution_entry.structure_min()
    };

    writeln!(writer, "strategy ladder-context").map_err(RunError::Output)?;
    writeln!(writer, "execution {execution}").map_err(RunError::Output)?;
    writeln!(writer, "context {context}").map_err(RunError::Output)?;
    writeln!(writer, "thesis {}", strategy.thesis).map_err(RunError::Output)?;
    writeln!(
        writer,
        "entry-long direction > {}, momentum > {}, structure >= {}, shift <= {} -> >= {}",
        execution_entry.direction_min(),
        execution_entry.momentum_min(),
        execution_entry.structure_min(),
        execution_entry.structure_shift_from_max(),
        execution_entry.structure_shift_to_min(),
    )
    .map_err(RunError::Output)?;
    writeln!(
        writer,
        "entry-short direction < -{}, momentum < -{}, structure <= {}, shift >= {} -> <= {}",
        execution_entry.direction_min(),
        execution_entry.momentum_min(),
        short_structure_max,
        -execution_entry.structure_shift_from_max(),
        -execution_entry.structure_shift_to_min(),
    )
    .map_err(RunError::Output)?;
    writeln!(
        writer,
        "context-long direction share >= {}, momentum bearish share <= {}, conflicts <= {}, progression bullish, late/terminal <= {}",
        context_entry.direction_share_min(),
        context_entry.momentum_bearish_share_max(),
        context_entry.structure_conflict_count_max(),
        context_entry.progression_late_or_terminal_count_max(),
    )
    .map_err(RunError::Output)?;
    writeln!(
        writer,
        "context-short direction share >= {}, momentum bullish share <= {}, conflicts <= {}, progression bearish, late/terminal <= {}",
        context_entry.direction_share_min(),
        context_entry.momentum_bearish_share_max(),
        context_entry.structure_conflict_count_max(),
        context_entry.progression_late_or_terminal_count_max(),
    )
    .map_err(RunError::Output)?;
    let mut exits = Vec::new();
    if engine.exit().on_context_lost() {
        exits.push("context_lost");
    }
    if engine.exit().on_structure_lost() {
        exits.push("structure_lost");
    }
    writeln!(writer, "exit {}", exits.join(",")).map_err(RunError::Output)?;
    writeln!(
        writer,
        "risk stop={} ATR target={} ATR reward_to_risk={}",
        strategy.stop_atr_multiple(),
        strategy.target_atr_multiple(),
        strategy.reward_to_risk()
    )
    .map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}

#[cfg(test)]
mod tests {
    use super::parse_strategy_explain_arguments;

    fn arguments(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|token| (*token).to_owned()).collect()
    }

    #[test]
    fn parses_the_explain_shape() {
        assert!(
            parse_strategy_explain_arguments(&arguments(&["explain", "--config", "run.json"]))
                .is_ok()
        );
    }

    #[test]
    fn rejects_other_shapes() {
        assert!(parse_strategy_explain_arguments(&arguments(&["explain"])).is_err());
        assert!(
            parse_strategy_explain_arguments(&arguments(&["run", "--config", "run.json"])).is_err()
        );
    }
}
