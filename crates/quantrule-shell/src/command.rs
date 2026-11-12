//! Command parsing and execution for the shell.
//!
//! The shell parses the command text, calls the source and application-service
//! use cases, and renders the result. It does not validate candles, assign
//! series indices, or compute a model value. Each command family owns its
//! argument grammar and its execution path behind this vocabulary.

mod analytics;
mod backtest;
mod data;
mod dispatch;
mod execution;
mod fetch;
mod help;
mod memory;
mod parse;
mod portfolio;
mod qualify;
mod report_run;
mod run_check;
mod strategy;
pub(crate) mod types;

#[cfg(test)]
mod tests;

pub(crate) use self::analytics::{parse_analytics_arguments, run_analytics};
pub(crate) use self::backtest::{parse_backtest_arguments, run_backtest};
pub(crate) use self::dispatch::{execute, parse_argv};
pub(crate) use self::execution::{parse_execution_arguments, run_execution};
pub(crate) use self::memory::{parse_memory_arguments, run_memory};
pub(crate) use self::parse::{parse, parse_help_arguments, parse_model_arguments};
pub(crate) use self::portfolio::{parse_portfolio_arguments, run_portfolio};
pub(crate) use self::qualify::{parse_qualify_arguments, run_qualify};
pub(crate) use self::report_run::{parse_report_arguments, run_report};
pub(crate) use self::run_check::{parse_run_check_arguments, run_run_check};
pub(crate) use self::strategy::{parse_strategy_arguments, run_explain, run_run};
pub(crate) use self::types::{Command, ParseError};

use self::data::{run_candles, run_candles_tolerant, run_timeframes};
use self::fetch::{parse_fetch_arguments, run_fetch};
use self::help::run_help;
use self::types::RunError;
use crate::request::ModelArgs;

/// Resolves the four timeframes the fetch command documents.
///
/// The data engine also supports H4; the fetch command contract does not, so H4
/// must be rejected here rather than in the shared code parser.
pub(crate) fn parse_native_timeframe(code: &str) -> Option<quantrule_data::Timeframe> {
    quantrule_data::Timeframe::from_code(code).filter(|timeframe| {
        matches!(
            *timeframe,
            quantrule_data::Timeframe::M1
                | quantrule_data::Timeframe::M5
                | quantrule_data::Timeframe::M15
                | quantrule_data::Timeframe::H1
        )
    })
}

/// Prints the flat event batch for one CSV.
pub(crate) fn run_events(
    args: &ModelArgs,
    writer: &mut impl std::io::Write,
) -> Result<(), RunError> {
    let set =
        quantrule_app::pipeline::load_set(&args.path, args.window).map_err(RunError::Source)?;
    let run = quantrule_app::pipeline::model_run_from_set(
        &set,
        &quantrule_model::input::MarketStateConfig::default(),
    );
    crate::render_flat::write_event_batch(&run, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}
