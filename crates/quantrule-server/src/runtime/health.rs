//! Health checks.
//!
//! A service is healthy when its check passes, not when its container is
//! running. The registry is injected so a test can supply a fake dependency.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use quantrule_api::{API_VERSION, HealthReport, ServiceHealth, ServiceState};

/// A dependency check.
pub type CheckFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;

/// A named, repeatable probe.
pub type Check = Arc<dyn Fn() -> CheckFuture + Send + Sync>;

/// The set of probes behind `/health`.
#[derive(Clone, Default)]
pub struct HealthRegistry {
    checks: Vec<(String, Check)>,
}

impl std::fmt::Debug for HealthRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HealthRegistry")
            .field(
                "checks",
                &self.checks.iter().map(|(name, _)| name).collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl HealthRegistry {
    /// An empty registry, which reports healthy with no dependencies.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register one probe.
    pub fn register<F, Fut>(&mut self, name: impl Into<String>, check: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<(), String>> + Send + 'static,
    {
        self.checks
            .push((name.into(), Arc::new(move || Box::pin(check()))));
    }

    /// The registered names, in registration order.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.checks.iter().map(|(name, _)| name.as_str()).collect()
    }

    /// Run every probe and build the report.
    pub async fn report(&self, image_digest: Option<quantrule_api::Digest>) -> HealthReport {
        let mut services = Vec::with_capacity(self.checks.len());
        for (name, check) in &self.checks {
            let result = check().await;
            services.push(match result {
                Ok(()) => ServiceHealth {
                    name: name.clone(),
                    state: ServiceState::Healthy,
                    detail: None,
                },
                Err(detail) => ServiceHealth {
                    name: name.clone(),
                    state: ServiceState::Down,
                    detail: Some(detail),
                },
            });
        }
        let state = HealthReport::aggregate(services.clone());
        HealthReport {
            v: API_VERSION.to_owned(),
            state,
            services,
            queue_depth: 0,
            workers: 0,
            image_digest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_empty_registry_is_healthy() {
        let report = HealthRegistry::new().report(None).await;
        assert_eq!(report.state, ServiceState::Healthy);
        assert!(report.services.is_empty());
    }

    #[test]
    fn registration_keeps_order() {
        let mut registry = HealthRegistry::new();
        registry.register("postgres", || async { Ok(()) });
        registry.register("s3", || async { Ok(()) });
        assert_eq!(registry.names(), vec!["postgres", "s3"]);
    }

    #[tokio::test]
    async fn a_failing_check_makes_the_report_down() {
        let mut registry = HealthRegistry::new();
        registry.register("postgres", || async { Ok(()) });
        registry.register("s3", || async { Err("connection refused".to_owned()) });
        let report = registry.report(None).await;
        assert_eq!(report.state, ServiceState::Down);
        assert_eq!(
            report.services[1].detail.as_deref(),
            Some("connection refused")
        );
    }

    #[tokio::test]
    async fn the_report_carries_the_contract_version() {
        let report = HealthRegistry::new().report(None).await;
        assert_eq!(report.v, API_VERSION);
    }
}
