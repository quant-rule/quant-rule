//! The profile count seam.
//!
//! The gateway summarises a profile with counts read from the service that
//! owns the records. The dataset count is already on [`DataClient`]; this module
//! adds the run count from evidence and the simulation count from simulation.
//! Each service exposes its records only through its list route, so a count is
//! the first page of that list: at most [`COUNT_PAGE`] records, the largest page
//! the run and simulation routes accept. A profile with more records reports the
//! page, not an invented total; an exact count needs a dedicated route the
//! services do not expose yet.
//!
//! [`DataClient`]: crate::client::DataClient

use std::collections::BTreeMap;

use quantrule_api::{PROFILE_HEADER, ProfileId, RunSummary, SimulationSummary};

use crate::adapters::BoxFuture;
use crate::client::{
    ClientError, HttpServiceClient, Service, ServiceClient, ServiceRequest, upstream_message,
};

/// The largest page a run or simulation list route returns.
pub const COUNT_PAGE: usize = 200;

/// How many run records a profile holds.
pub trait RunCounter: Send + Sync {
    /// Count one profile's runs from the evidence list route.
    fn run_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>>;
}

/// How many simulation runs a profile holds.
pub trait SimulationCounter: Send + Sync {
    /// Count one profile's simulation runs from the simulation list route.
    fn simulation_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>>;
}

/// Read one list page and report how many rows it held.
///
/// The profile is always the one the gateway validated; the service scopes the
/// list. A 404 means the profile holds no visible record, which is zero rather
/// than a failure: the gateway and the service can race a profile deletion.
async fn page_count<T>(
    client: &HttpServiceClient,
    service: Service,
    path: String,
    profile: &ProfileId,
) -> Result<u64, ClientError>
where
    T: serde::de::DeserializeOwned,
{
    let request = ServiceRequest::get(path).with_header(PROFILE_HEADER, profile.as_str());
    let response = <HttpServiceClient as ServiceClient>::send(client, service, request).await?;
    match response.status {
        200 => {
            let rows: Vec<T> = serde_json::from_slice(&response.body).map_err(|error| {
                ClientError::InvalidResponse {
                    service,
                    message: error.to_string(),
                }
            })?;
            Ok(rows.len() as u64)
        }
        404 => Ok(0),
        status => Err(ClientError::Status {
            service,
            status,
            message: upstream_message(&response.body),
        }),
    }
}

impl RunCounter for HttpServiceClient {
    fn run_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            page_count::<RunSummary>(
                &client,
                Service::Evidence,
                format!("/runs?limit={COUNT_PAGE}"),
                &profile,
            )
            .await
        })
    }
}

impl SimulationCounter for HttpServiceClient {
    fn simulation_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>> {
        let client = self.clone();
        Box::pin(async move {
            page_count::<SimulationSummary>(
                &client,
                Service::Simulation,
                format!("/simulation/runs?limit={COUNT_PAGE}"),
                &profile,
            )
            .await
        })
    }
}

/// In-memory profile counts, for gateway tests.
#[derive(Debug, Clone, Default)]
pub struct MemoryCounts {
    runs: std::sync::Arc<std::sync::Mutex<BTreeMap<String, u64>>>,
    simulations: std::sync::Arc<std::sync::Mutex<BTreeMap<String, u64>>>,
}

impl MemoryCounts {
    /// Set one profile's run count.
    pub fn set_runs(&self, profile: &ProfileId, count: u64) {
        self.runs
            .lock()
            .expect("the run count lock")
            .insert(profile.as_str().to_owned(), count);
    }

    /// Set one profile's simulation count.
    pub fn set_simulations(&self, profile: &ProfileId, count: u64) {
        self.simulations
            .lock()
            .expect("the simulation count lock")
            .insert(profile.as_str().to_owned(), count);
    }

    fn lookup(map: &std::sync::Mutex<BTreeMap<String, u64>>, profile: &ProfileId) -> u64 {
        map.lock()
            .expect("the count lock")
            .get(profile.as_str())
            .copied()
            .unwrap_or(0)
    }
}

impl RunCounter for MemoryCounts {
    fn run_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>> {
        let count = Self::lookup(&self.runs, &profile);
        Box::pin(async move { Ok(count) })
    }
}

impl SimulationCounter for MemoryCounts {
    fn simulation_count(&self, profile: ProfileId) -> BoxFuture<Result<u64, ClientError>> {
        let count = Self::lookup(&self.simulations, &profile);
        Box::pin(async move { Ok(count) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum::extract::State;
    use axum::routing::get;
    use std::sync::Arc;

    #[derive(Clone, Default)]
    struct Seen {
        paths: Arc<std::sync::Mutex<Vec<String>>>,
        profile: Arc<std::sync::Mutex<Option<String>>>,
    }

    fn profile(headers: &axum::http::HeaderMap) -> Option<String> {
        headers
            .get(PROFILE_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    async fn runs(
        State(seen): State<Seen>,
        headers: axum::http::HeaderMap,
    ) -> impl axum::response::IntoResponse {
        seen.paths
            .lock()
            .expect("the path lock")
            .push("/runs".to_owned());
        *seen.profile.lock().expect("the profile lock") = profile(&headers);
        let row = serde_json::json!({
            "run_id": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "status": "queued",
            "config_id": "cfg",
            "data_version": "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "submitted_at": "2026-11-15T00:00:00Z",
        });
        axum::Json(serde_json::json!([row, row]))
    }

    async fn simulations(
        State(seen): State<Seen>,
        headers: axum::http::HeaderMap,
    ) -> impl axum::response::IntoResponse {
        seen.paths
            .lock()
            .expect("the path lock")
            .push("/simulation/runs".to_owned());
        *seen.profile.lock().expect("the profile lock") = profile(&headers);
        axum::Json(serde_json::json!([{
            "sim_id": "sim_1",
            "premise": "test",
            "status": "draft",
            "windows": 3,
            "marks": 0,
        }]))
    }

    async fn start(seen: Seen) -> (std::net::SocketAddr, tokio::task::JoinHandle<()>) {
        let app = Router::new()
            .route("/runs", get(runs))
            .route("/simulation/runs", get(simulations))
            .with_state(seen);
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let address = listener.local_addr().expect("address");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("upstream");
        });
        (address, handle)
    }

    #[tokio::test]
    async fn a_count_reads_one_list_page_per_service() {
        let seen = Seen::default();
        let (address, handle) = start(seen.clone()).await;
        let evidence = HttpServiceClient::new(Service::Evidence, &format!("http://{address}"))
            .expect("evidence client");
        let simulation = HttpServiceClient::new(Service::Simulation, &format!("http://{address}"))
            .expect("simulation client");
        let profile = ProfileId::parse("alpha").expect("profile");

        assert_eq!(
            RunCounter::run_count(&evidence, profile.clone())
                .await
                .expect("runs"),
            2
        );
        assert_eq!(
            SimulationCounter::simulation_count(&simulation, profile)
                .await
                .expect("simulations"),
            1
        );
        assert_eq!(
            seen.paths.lock().expect("the path lock").as_slice(),
            ["/runs", "/simulation/runs"]
        );
        assert_eq!(
            seen.profile.lock().expect("the profile lock").as_deref(),
            Some("alpha")
        );
        handle.abort();
    }

    #[tokio::test]
    async fn the_memory_counts_are_per_profile() {
        let counts = MemoryCounts::default();
        let alpha = ProfileId::parse("alpha").expect("alpha");
        let beta = ProfileId::parse("beta").expect("beta");
        counts.set_runs(&alpha, 3);
        counts.set_simulations(&alpha, 2);
        assert_eq!(
            RunCounter::run_count(&counts, alpha.clone())
                .await
                .expect("runs"),
            3
        );
        assert_eq!(
            SimulationCounter::simulation_count(&counts, alpha)
                .await
                .expect("simulations"),
            2
        );
        assert_eq!(
            RunCounter::run_count(&counts, beta).await.expect("empty"),
            0
        );
    }
}
