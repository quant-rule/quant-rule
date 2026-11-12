//! The QuantRule application service.
//!
//! The application service is the shared use-case boundary for every client.
//! The command shell and the desktop UI call the same workspace, dataset, run
//! configuration, model-run, export, pipeline-stage, and memory operations
//! here. The shell keeps command parsing, terminal output, explicit source
//! fetch, and rendering; the desktop keeps its window, IPC, and rendering.
//!
//! The engine crates remain pure. This crate owns file access, path resolution,
//! orchestration, and the local research memory, and returns explicit errors
//! that name the failing input. The shell and the desktop UI are thin adapters:
//! they call the same use cases and neither duplicates path resolution,
//! configuration parsing, or memory rules.
//!
//! # Examples
//!
//! ```
//! use std::path::Path;
//! use quantrule_app::service::AppService;
//!
//! let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
//! let service = AppService::open(&root).expect("the repository workspace opens");
//! let summary = service
//!     .check_run(&root.join("config/examples/btcusd-2024/run.json"))
//!     .expect("the example run is valid");
//! assert_eq!(summary.name, "btcusd-2024");
//! ```

#![forbid(unsafe_code)]
#![recursion_limit = "512"]

pub mod artifact;
pub mod config;
pub mod csv;
pub mod dataset;
pub mod date;
pub mod error;
pub mod export;
pub mod fingerprint;
pub mod memory;
pub mod pipeline;
pub mod service;
pub mod stages;
pub mod workspace;
