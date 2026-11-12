//! Logging.
//!
//! Structured logs go to standard error. The level is a configuration value so a
//! container can be turned up without a rebuild.

use tracing_subscriber::EnvFilter;

use crate::runtime::error::RuntimeError;

/// Install the global subscriber.
///
/// # Errors
///
/// Returns [`RuntimeError::Serve`] when the filter is invalid or a subscriber is
/// already installed.
pub fn init(level: &str) -> Result<(), RuntimeError> {
    init_from(level, |filter| {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(true)
            .with_writer(std::io::stderr)
            .try_init()
    })
}

fn init_from<F>(level: &str, install: F) -> Result<(), RuntimeError>
where
    F: FnOnce(EnvFilter) -> Result<(), Box<dyn std::error::Error + Send + Sync>>,
{
    let filter = EnvFilter::try_new(level)
        .map_err(|error| RuntimeError::Serve(format!("invalid log filter: {error}")))?;
    install(filter).map_err(|error| RuntimeError::Serve(format!("logging: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_invalid_filter_is_an_error() {
        let error = init_from("app=notalevel", |_| Ok(())).expect_err("reject");
        assert!(error.to_string().contains("invalid log filter"));
    }

    #[test]
    fn a_valid_filter_is_passed_through() {
        init_from("info", |_| Ok(())).expect("accepted");
    }

    #[test]
    fn a_failing_install_is_reported() {
        let error = init_from("info", |_| Err("already set".into())).expect_err("reject");
        assert!(error.to_string().contains("already set"));
    }
}
