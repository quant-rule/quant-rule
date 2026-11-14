//! Shared in-memory services and request helpers for the gateway tests.
//!
//! Each integration target compiles this module separately, and a target need
//! not use every helper, so unused helpers are allowed here.

#![allow(dead_code)]

use axum::body::Body;
use axum::http::Request;
use quantrule_api::{PROFILE_HEADER, ProfileId};
use quantrule_gateway::profile::{MemoryProfiles, ProfileRecord, ProfileStore};
use quantrule_gateway::server::{AppState, Services};
use quantrule_runtime::client::MemoryDataClient;
use quantrule_runtime::{Config, HealthRegistry};
use std::sync::Arc;

pub fn state() -> Arc<AppState> {
    Arc::new(AppState::new(&Config::default(), HealthRegistry::new()))
}

pub async fn state_with_data(data: MemoryDataClient) -> Arc<AppState> {
    Arc::new(AppState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        Services {
            profiles: Some(memory_profiles(&["default", "alpha", "beta", "gamma"]).await),
            data: Some(Arc::new(data)),
            ..Services::default()
        },
    ))
}

pub async fn memory_profiles(ids: &[&str]) -> Arc<MemoryProfiles> {
    let store = Arc::new(MemoryProfiles::default());
    for id in ids {
        store
            .create(ProfileRecord {
                profile_id: ProfileId::parse(id).expect("profile"),
                name: (*id).to_owned(),
                created_at: "2026-11-13T00:00:00Z".to_owned(),
            })
            .await
            .expect("create profile");
    }
    store
}

pub async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 65_536)
        .await
        .expect("body")
        .to_vec()
}

/// A request builder the tests share for the profile header.
pub fn with_profile(request: Request<Body>, profile: &str) -> Request<Body> {
    let (mut parts, body) = request.into_parts();
    parts
        .headers
        .insert(PROFILE_HEADER, profile.parse().expect("header"));
    Request::from_parts(parts, body)
}
