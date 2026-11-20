//! Printed-output tests for the HTTP shell surface.
//!
//! Every test runs the real `quantrule` binary against the in-process fixture
//! and asserts on stdout, stderr, exit status, and the request the fixture saw.
//! No test reads `data/`, a live network, or the wall clock.
//!
//! The cases live under `tests/cases/`, grouped by command family; this file is
//! the single integration entry point, so Cargo test discovery is unchanged.

mod cases;
mod support;
