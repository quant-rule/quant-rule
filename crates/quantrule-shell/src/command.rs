//! Command parsing and execution for the shell.
//!
//! The shell parses the command text and calls the source, model, and rendering
//! modules. It does not validate candles or assign series indices; those stay in
//! the data engine. Each command family owns its argument grammar and its
//! execution path behind this vocabulary.

mod data;
mod dispatch;
mod fetch;
mod help;
mod parse;
mod policy;
mod report_run;
mod types;

#[cfg(test)]
mod tests;

pub(crate) use self::dispatch::{execute, parse_argv};
pub(crate) use self::parse::{parse, parse_help_arguments, parse_model_arguments};
pub(crate) use self::policy::{parse_policy_arguments, run_policy_check};
pub(crate) use self::report_run::{parse_report_arguments, run_report};
pub(crate) use self::types::{Command, ParseError};

use self::data::{run_candles, run_candles_tolerant, run_timeframes};
use self::fetch::{parse_fetch_arguments, run_fetch};
use self::help::run_help;
use self::types::RunError;

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
    args: &types::ModelArgs,
    writer: &mut impl std::io::Write,
) -> Result<(), RunError> {
    let set = crate::pipeline::load_set(&args.path, args.window).map_err(RunError::Source)?;
    let run = crate::pipeline::model_run_from_set(
        &set,
        &quantrule_model::input::MarketStateConfig::default(),
    );
    crate::render_flat::write_event_batch(&run, writer).map_err(RunError::Output)?;
    writer.flush().map_err(RunError::Output)
}
