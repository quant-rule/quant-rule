//! The QuantRule gateway.
//!
//! The gateway is the only surface a client sees. It owns profiles in the
//! `gateway` schema, the static client, the proxy table, and health. The data,
//! model, evidence, and simulation surfaces are forwarded to their services;
//! the gateway has no dataset, config, inspection, model, run, research, or
//! simulation handler and no domain store.
//!
//! The crate links `quantrule-api` and `quantrule-runtime` only. Its integration
//! tests use `quantrule-service-api`. It does not link `quantrule-app` or an engine, so
//! the gateway cannot compute a reading, a model, a fill, or a run result even
//! by accident. P8 retired `quantrule-server`; the worker lives in
//! `quantrule-backtest-service`.

#![forbid(unsafe_code)]

pub mod adapters;
mod auth;
pub mod migrations;
pub mod profile;
pub mod server;
mod signal_proxy;
pub mod static_files;
pub mod token;
