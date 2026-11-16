//! Shared in-memory services and request helpers for the evidence tests.
//!
//! Each integration target compiles this module separately, and a target need
//! not use every helper, so unused helpers are allowed here.

#![allow(dead_code)]

use quantrule_api::ProfileId;

/// The in-memory object store from `quantrule_runtime`.
// Each integration target compiles this module separately, and not every
// target exercises the object store.
#[allow(unused_imports)]
pub use quantrule_runtime::adapters::memory::MemoryObjects;

/// A profile id, failing loudly on a bad fixture.
pub fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("profile")
}

pub async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 65_536)
        .await
        .expect("body")
        .to_vec()
}
