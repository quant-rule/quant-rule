//! Cancellation at each of the four documented boundaries.

use super::support::*;

#[tokio::test]
async fn a_cancel_at_each_boundary_leaves_no_artifact() {
    for boundary in 1..=4 {
        let fixture = fixture().await;
        let evidence = BoundaryEvidence {
            inner: fixture.runs.clone(),
            cancel_on: boundary,
            checks: Arc::new(Mutex::new(0)),
        };
        let services = services_with(
            &fixture,
            Arc::new(evidence),
            Arc::clone(&fixture.services.data),
            Arc::clone(&fixture.services.model),
        );
        assert_eq!(
            process(fixture.run_id.clone(), false, &services)
                .await
                .expect("process"),
            ProcessOutcome::Skipped,
            "boundary {boundary}"
        );
        let record = fixture
            .runs
            .get_any(fixture.run_id.clone())
            .await
            .expect("get")
            .expect("record");
        assert_eq!(record.status, RunState::Running, "boundary {boundary}");
        assert!(record.digest.is_none(), "boundary {boundary}");
        assert!(record.artifacts.is_empty(), "boundary {boundary}");
        assert!(
            !fixture
                .objects
                .exists(&RunManifest::key(&fixture.run_id))
                .await
                .expect("manifest"),
            "boundary {boundary} published a manifest"
        );
        assert!(
            fixture
                .objects
                .keys()
                .iter()
                .all(|key| !key.starts_with("artifacts/")),
            "boundary {boundary} wrote an artifact"
        );
        assert!(
            fixture.queue.keys().is_empty(),
            "boundary {boundary} published a terminal message"
        );
        let expected_model_calls = usize::from(boundary > 2);
        assert_eq!(
            fixture.model.calls(),
            expected_model_calls,
            "boundary {boundary}"
        );
    }
}
