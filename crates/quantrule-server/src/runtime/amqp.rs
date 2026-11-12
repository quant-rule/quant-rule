//! The message broker.
//!
//! The control plane publishes `run.requested` and consumes `run.computed`. The
//! connection is long-lived; the check reports the state the library already
//! tracks.

pub use lapin::Connection;
use lapin::ConnectionProperties;

use crate::runtime::error::RuntimeError;

/// Connect to the broker.
///
/// # Errors
///
/// Returns [`RuntimeError::Queue`] when the broker refuses the connection.
pub async fn connect(url: &str) -> Result<Connection, RuntimeError> {
    Connection::connect(url, ConnectionProperties::default())
        .await
        .map_err(RuntimeError::from)
}

/// Report the connection state.
///
/// # Errors
///
/// Returns the library state when the connection is not open.
pub fn check(connection: &Connection) -> Result<(), String> {
    if connection.status().connected() {
        Ok(())
    } else {
        Err(format!("connection {:?}", connection.status()))
    }
}
