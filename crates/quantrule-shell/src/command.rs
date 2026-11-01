//! Command parsing and execution for the shell.
//!
//! The shell parses the command text and calls the source and rendering
//! modules. It does not validate candles or assign series indices; those stay
//! in the data engine. Each command family owns its argument grammar and its
//! execution path behind this vocabulary.

mod backtest;
mod chart;
mod dashboard;
mod data;
mod dispatch;
mod fetch;
mod help;
mod lab;
mod layers;
mod model;
mod parse;
mod policy;
mod report;
mod report_run;
mod research;
mod store;
mod store_query;
mod store_verify;
mod strategy;
mod strategy_explain;
mod types;

#[cfg(test)]
mod tests;

pub(crate) use self::backtest::{parse_backtest_arguments, run_backtest_command};
pub(crate) use self::dashboard::{parse_dashboard_arguments, run_market_dashboard};
pub(crate) use self::dispatch::{execute, parse_argv};
pub(crate) use self::parse::{parse, parse_help_arguments};
pub(crate) use self::policy::{parse_policy_arguments, run_policy_check};
pub(crate) use self::report::{parse_market_report_arguments, run_market_report};
pub(crate) use self::report_run::{parse_report_arguments, run_report};
pub(crate) use self::strategy::{parse_strategy_run_arguments, run_strategy_run};
pub(crate) use self::strategy_explain::{parse_strategy_explain_arguments, run_strategy_explain};
pub(crate) use self::types::{Command, ParseError};

use self::chart::{parse_market_chart_arguments, run_market_chart};
use self::data::{run_candles, run_candles_tolerant, run_timeframes};
use self::fetch::{parse_fetch_arguments, run_fetch};
use self::help::run_help;
use self::lab::{parse_lab_arguments, run_lab_diagnose, run_lab_list, run_lab_summary};
use self::model::{
    parse_axis_arguments, parse_model_arguments, parse_model_series_arguments, run_axis,
    run_events, run_legs, run_market_state, run_model_series,
};
use self::research::{parse_research_arguments, run_research_export, run_research_summary};
use self::store::{parse_store_arguments, run_store_ingest, run_store_init, run_store_remove};
use self::store_query::{run_db, run_store_list, run_store_show};
use self::store_verify::run_store_verify;

/// Resolves the four timeframes the store and fetch commands document.
///
/// The data engine also supports H4; the store and fetch command contracts do
/// not, so H4 must be rejected here rather than in the shared code parser.
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
