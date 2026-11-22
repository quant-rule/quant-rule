//! Scale invariance: one worker and two workers on the same run.
//!
//! The run is not sharded: a second worker must either claim the run once or
//! observe it already computed. The canonical digest is the same either way,
//! and the model runs once.

use super::support::*;

#[tokio::test]
async fn one_worker_and_two_workers_produce_the_same_digest() {
    let sequential = fixture().await;
    assert_eq!(
        process(sequential.run_id.clone(), None, &sequential.services)
            .await
            .expect("sequential"),
        ProcessOutcome::Computed
    );
    let expected = sequential
        .runs
        .get_any(sequential.run_id.clone())
        .await
        .expect("get")
        .expect("record")
        .digest
        .expect("the sequential digest");

    let concurrent = fixture().await;
    let (left, right) = tokio::join!(
        process(concurrent.run_id.clone(), None, &concurrent.services,),
        process(concurrent.run_id.clone(), None, &concurrent.services,),
    );
    let outcomes = [left.expect("left"), right.expect("right")];
    assert!(outcomes.contains(&ProcessOutcome::Computed));
    assert!(
        outcomes
            .iter()
            .all(|outcome| matches!(outcome, ProcessOutcome::Computed | ProcessOutcome::Skipped))
    );
    let actual = concurrent
        .runs
        .get_any(concurrent.run_id.clone())
        .await
        .expect("get")
        .expect("record")
        .digest
        .expect("the concurrent digest");
    assert_eq!(actual, expected);
    assert_eq!(concurrent.model.calls(), 1);
}
