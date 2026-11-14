//! Command parsing and execution for the shell.
//!
//! The shell is a gateway client. It parses one command, makes one HTTP call,
//! and renders the contract type the gateway returns. It opens no engine, reads
//! no local database, and computes no measurement.

mod dispatch;
pub(crate) mod help;
mod parse;
pub(crate) mod types;

#[cfg(test)]
mod tests;

pub(crate) use self::dispatch::execute;
pub(crate) use self::parse::{parse_argv, parse_line};
pub(crate) use self::types::{Command, GlobalOptions};
