//! A recording in-memory service client.
//!
//! It is the fake the client tests use: a gateway route or a worker hop is
//! exercised without a socket, and the recorded call is the assertion.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use crate::adapters::BoxFuture;

use super::{ClientError, Service, ServiceClient, ServiceRequest, ServiceResponse};

/// An in-memory client that records calls and replays configured responses.
#[derive(Debug, Clone, Default)]
pub struct MemoryClient {
    responses: Arc<Mutex<BTreeMap<(Service, String), ServiceResponse>>>,
    calls: Arc<Mutex<Vec<(Service, ServiceRequest)>>>,
}

impl MemoryClient {
    /// An empty client.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure the response for one service and path.
    pub fn respond(&self, service: Service, path: impl Into<String>, response: ServiceResponse) {
        self.responses
            .lock()
            .expect("the response lock")
            .insert((service, path.into()), response);
    }

    /// The calls this client has seen, in order.
    #[must_use]
    pub fn calls(&self) -> Vec<(Service, ServiceRequest)> {
        self.calls.lock().expect("the call lock").clone()
    }
}

impl ServiceClient for MemoryClient {
    fn send(
        &self,
        service: Service,
        request: ServiceRequest,
    ) -> BoxFuture<Result<ServiceResponse, ClientError>> {
        let responses = Arc::clone(&self.responses);
        let calls = Arc::clone(&self.calls);
        Box::pin(async move {
            calls
                .lock()
                .expect("the call lock")
                .push((service, request.clone()));
            responses
                .lock()
                .expect("the response lock")
                .get(&(service, request.path))
                .cloned()
                .ok_or(ClientError::Unconfigured { service })
        })
    }
}
