//! Health checks.
//!
//! A service is healthy when its check passes, not when its container is
//! running. The registry is injected so a test can supply a fake dependency.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use quantrule_api::{API_VERSION, HealthReport, ServiceHealth, ServiceState};

use crate::shutdown::Shutdown;

/// A dependency check.
pub type CheckFuture = Pin<Box<dyn Future<Output = Result<(), String>> + Send>>;

/// A named, repeatable probe.
pub type Check = Arc<dyn Fn() -> CheckFuture + Send + Sync>;

/// The set of probes behind `/health` and `/ready`.
#[derive(Clone, Default)]
pub struct HealthRegistry {
    service: String,
    checks: Vec<(String, Check)>,
    metrics: Option<crate::metrics::Metrics>,
    shutdown: Option<Shutdown>,
}

impl std::fmt::Debug for HealthRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HealthRegistry")
            .field("service", &self.service)
            .field(
                "checks",
                &self.checks.iter().map(|(name, _)| name).collect::<Vec<_>>(),
            )
            .field("metrics", &self.metrics.is_some())
            .field("drain", &self.shutdown.is_some())
            .finish()
    }
}

impl HealthRegistry {
    /// An empty registry, which reports healthy with no dependencies.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty registry that names the service it belongs to.
    ///
    /// The name travels in the report so a readiness probe can reject a
    /// response from a different process that happened to answer the name.
    #[must_use]
    pub fn named(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
            checks: Vec::new(),
            metrics: None,
            shutdown: None,
        }
    }

    /// Attach a metrics registry so each probe result updates
    /// `quantrule_dependency_up`.
    #[must_use]
    pub fn with_metrics(mut self, metrics: crate::metrics::Metrics) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Attach the drain channel so `/ready` refuses new routing immediately.
    #[must_use]
    pub fn with_shutdown(mut self, shutdown: Shutdown) -> Self {
        self.shutdown = Some(shutdown);
        self
    }

    /// Whether a drain signal has arrived.
    #[must_use]
    pub fn is_draining(&self) -> bool {
        self.shutdown.as_ref().is_some_and(Shutdown::is_requested)
    }

    /// The report to answer `/ready` with while draining.
    ///
    /// No dependency probe runs: a draining process must answer immediately and
    /// must not start new upstream work.
    #[must_use]
    pub fn draining_report(&self, image_digest: Option<quantrule_api::Digest>) -> HealthReport {
        HealthReport {
            v: API_VERSION.to_owned(),
            service: self.service.clone(),
            state: ServiceState::Down,
            services: vec![ServiceHealth {
                name: "drain".to_owned(),
                state: ServiceState::Down,
                detail: Some("draining".to_owned()),
            }],
            queue_depth: 0,
            workers: 0,
            image_digest,
        }
    }

    /// Build the `/ready` answer.
    ///
    /// A draining process is unavailable without probing anything; otherwise
    /// the aggregate dependency report decides. `/health` stays the local
    /// liveness probe and does not use this method.
    pub async fn readiness(
        &self,
        image_digest: Option<quantrule_api::Digest>,
    ) -> (bool, HealthReport) {
        if self.is_draining() {
            return (false, self.draining_report(image_digest));
        }
        let report = self.report(image_digest).await;
        (report.state.is_available(), report)
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
            let state = if result.is_ok() {
                ServiceState::Healthy
            } else {
                ServiceState::Down
            };
            if let Some(metrics) = &self.metrics {
                metrics.set_dependency(name.clone(), state.is_available());
            }
            services.push(match result {
                Ok(()) => ServiceHealth {
                    name: name.clone(),
                    state,
                    detail: None,
                },
                Err(detail) => ServiceHealth {
                    name: name.clone(),
                    state,
                    detail: Some(detail),
                },
            });
        }
        let state = HealthReport::aggregate(services.clone());
        HealthReport {
            v: API_VERSION.to_owned(),
            service: self.service.clone(),
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

    #[tokio::test]
    async fn a_named_registry_reports_its_service() {
        let report = HealthRegistry::named("data").report(None).await;
        assert_eq!(report.service, "data");
    }

    #[tokio::test]
    async fn a_draining_registry_is_unready_without_running_probes() {
        let (trigger, shutdown) = crate::shutdown::Shutdown::channel();
        let mut registry = HealthRegistry::named("data").with_shutdown(shutdown);
        registry.register("postgres", || async { Ok(()) });
        let (ready, report) = registry.readiness(None).await;
        assert!(ready);
        trigger.trigger();
        assert!(registry.is_draining());
        let (ready, report_while_draining) = registry.readiness(None).await;
        assert!(!ready);
        assert_eq!(report_while_draining.state, ServiceState::Down);
        assert_eq!(
            report_while_draining.services[0].detail.as_deref(),
            Some("draining")
        );
        // The liveness report never gains the drain component.
        assert_eq!(report.services.len(), 1);
    }

    #[tokio::test]
    async fn a_down_dependency_makes_readiness_false() {
        let mut registry = HealthRegistry::named("data");
        registry.register("postgres", || async { Err("down".to_owned()) });
        let (ready, report) = registry.readiness(None).await;
        assert!(!ready);
        assert_eq!(report.state, ServiceState::Down);
    }

    #[tokio::test]
    async fn a_report_updates_the_dependency_metrics() {
        let metrics = crate::metrics::Metrics::new("data", "1.5.2", None);
        let mut registry = HealthRegistry::named("data").with_metrics(metrics.clone());
        registry.register("postgres", || async { Ok(()) });
        registry.register("queue", || async { Err("down".to_owned()) });
        registry.report(None).await;
        let text = metrics.encode();
        assert!(
            text.contains("quantrule_dependency_up{name=\"postgres\"} 1"),
            "{text}"
        );
        assert!(
            text.contains("quantrule_dependency_up{name=\"queue\"} 0"),
            "{text}"
        );
    }
}
