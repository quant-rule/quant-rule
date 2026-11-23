//! Worker drain behaviour over the in-memory lifecycle.
//!
//! A drain never marks an in-flight run failed. If the deadline aborts the
//! attempt, the run keeps its lease and the digest stays absent; lease expiry
//! is the recovery path. A reclaimed run keeps the first canonical digest, and
//! a stale worker cannot replace the new term.

use std::sync::Arc;
use std::time::Duration;

use quantrule_api::{Digest, RunState};
use quantrule_runtime::client::EvidenceClient;
use quantrule_service_api::RunClaimOutcome;

use super::support::*;

#[tokio::test]
async fn an_in_flight_run_finishes_before_the_worker_deadline() {
    let fixture = fixture().await;
    let services = Arc::new(fixture.services);
    let run_id = fixture.run_id.clone();
    let mut consumers = tokio::task::JoinSet::new();
    consumers.spawn(async move { process(run_id, None, &services).await });
    assert_eq!(
        quantrule_backtest_service::drain::drain_consumers(
            &mut consumers,
            Duration::from_secs(120),
        )
        .await,
        quantrule_backtest_service::drain::DrainOutcome::Finished
    );
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert!(record.digest.is_some(), "the in-flight run did not finish");
    assert_eq!(record.error, None);
}

#[tokio::test]
async fn a_reclaimed_run_keeps_the_first_digest() {
    let fixture = fixture().await;
    assert_eq!(
        process(fixture.run_id.clone(), None, &fixture.services)
            .await
            .expect("first worker"),
        ProcessOutcome::Computed
    );
    let first = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    let digest = first.digest.clone().expect("the first digest");

    // The worker is killed before evidence finalises. Its lease expires and a
    // second delivery arrives; the recorded digest is authoritative.
    fixture.runs.expire_lease(&fixture.run_id);
    assert_eq!(
        process(fixture.run_id.clone(), None, &fixture.services)
            .await
            .expect("second worker"),
        ProcessOutcome::Skipped
    );
    let second = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(second.digest, Some(digest));
    assert_eq!(second.artifacts, first.artifacts);
    assert_eq!(fixture.model.calls(), 1, "the model was recomputed");
    assert_eq!(fixture.queue.payloads("run.computed").len(), 2);
}

#[tokio::test]
async fn a_stale_worker_cannot_replace_the_new_term() {
    let mut fixture = fixture().await;
    let first = fixture
        .runs
        .claim(fixture.run_id.clone(), "worker-a".to_owned(), None)
        .await
        .expect("first claim");
    let RunClaimOutcome::Claimed(first) = first else {
        panic!("the first worker did not claim the run");
    };
    fixture.runs.expire_lease(&fixture.run_id);
    let second = fixture
        .runs
        .claim(fixture.run_id.clone(), "worker-b".to_owned(), None)
        .await
        .expect("second claim");
    let RunClaimOutcome::Claimed(second) = second else {
        panic!("the second worker did not claim the run");
    };
    assert!(second.lease_term > first.lease_term);

    let stale = fixture
        .services
        .evidence
        .computed(
            fixture.run_id.clone(),
            first.lease_term,
            Digest::sha256(b"stale"),
            Vec::new(),
            Vec::new(),
        )
        .await
        .expect("stale computed");
    assert_eq!(stale.status, RunState::Running);
    assert_eq!(stale.digest, None);
    assert_eq!(stale.lease_term, second.lease_term);

    // The live holder continues its own term and publishes the real digest.
    fixture.services.holder = "worker-b".to_owned();
    assert_eq!(
        process(
            fixture.run_id.clone(),
            Some(second.lease_term),
            &fixture.services,
        )
        .await
        .expect("live worker"),
        ProcessOutcome::Computed
    );
    let final_record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert!(final_record.digest.is_some());
    assert_eq!(final_record.status, RunState::Running);
}

#[tokio::test]
async fn an_aborted_attempt_leaves_the_run_running_with_its_lease() {
    let fixture = fixture().await;
    let claimed = fixture
        .runs
        .claim(fixture.run_id.clone(), "worker-a".to_owned(), None)
        .await
        .expect("claim");
    assert!(matches!(claimed, RunClaimOutcome::Claimed(_)));

    // The drain deadline dropped the attempt. Nothing wrote computed or failed.
    let record = fixture
        .runs
        .get_any(fixture.run_id.clone())
        .await
        .expect("get")
        .expect("record");
    assert_eq!(record.status, RunState::Running);
    assert_eq!(record.digest, None);
    assert_eq!(record.error, None);
    assert_eq!(record.lease_holder.as_deref(), Some("worker-a"));
    assert!(record.lease_expires_at.is_some());
}
