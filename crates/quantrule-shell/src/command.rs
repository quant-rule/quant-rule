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
mod layers;
mod model;
mod parse;
mod report;
mod report_run;
mod research;
mod store;
mod store_query;
mod store_verify;
mod strategy;
mod types;

#[cfg(test)]
mod tests;

pub(crate) use self::backtest::{parse_backtest_arguments, run_backtest_command};
pub(crate) use self::dashboard::{parse_dashboard_arguments, run_market_dashboard};
pub(crate) use self::dispatch::{execute, parse_argv};
pub(crate) use self::parse::parse;
pub(crate) use self::report::{parse_market_report_arguments, run_market_report};
pub(crate) use self::report_run::{parse_report_arguments, run_report};
pub(crate) use self::strategy::{parse_strategy_run_arguments, run_strategy_run};
pub(crate) use self::types::{Axis, Command, ParseError};

use self::chart::{parse_market_chart_arguments, run_market_chart};
use self::data::{run_candles, run_candles_tolerant, run_timeframes};
use self::fetch::{parse_fetch_arguments, run_fetch};
use self::help::run_help;
use self::model::{
    parse_axis_arguments, parse_model_arguments, parse_model_series_arguments, run_axis,
    run_events, run_legs, run_market_state, run_model_series,
};
use self::research::{parse_research_arguments, run_research_export, run_research_summary};
use self::store::{parse_store_arguments, run_store_ingest, run_store_init, run_store_remove};
use self::store_query::{run_db, run_store_list, run_store_show};
use self::store_verify::run_store_verify;
