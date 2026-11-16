//! Sidecar supervision for the QuantRule desktop package.
//!
//! The supervisor is packaging, not a service. It resolves the resource
//! directory the shell passed, allocates loopback ports, starts the six service
//! binaries, waits for health, restarts a crashed child within a bounded
//! budget, and stops the children when it is stopped. It owns no domain state,
//! no schema, no HTTP contract, and no compute.
//!
//! A child is never the service binary directly. Each service is wrapped by
//! `qr-desktop --guard`, which holds a connection to the supervisor's control
//! socket and kills its service when that connection closes. That is how a
//! supervisor crash leaves no orphan: the operating system closes the socket,
//! the guard wakes, and the service exits. The same binary serves as the guard,
//! so the package still carries exactly one supervisor executable.
//!
//! The embedded backend contract (`QR_PROFILE=desktop`, `QR_DATA_DIR`) is owned
//! by #693/#694. The supervisor passes it today; the profile is only selected
//! by an explicit `--profile desktop`, `QR_DESKTOP_PROFILE=desktop`, or a build
//! with the `embedded-profile` feature, so a development build never pretends
//! that a Postgres stack is a desktop profile.

#![forbid(unsafe_code)]

pub mod cli;
pub mod console;
pub mod guard;
pub mod health;
pub mod package;
pub mod ports;
pub mod run;
pub mod status;
