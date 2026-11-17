//! Shared helpers and conformance bodies for the research-ledger tests.

use quantrule_api::{
    BaselineRecord, Digest, HoldoutAccess, IMPROVEMENT_RUN_SCHEMA, ImprovementDecision,
    ImprovementManifest, ImprovementPartition, ImprovementRunState, ImprovementTier,
    InvalidationRecord, ProfileId, RunId,
};
use quantrule_evidence_service::research::ImprovementStore;

pub fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("a test profile id")
}

pub fn manifest(improvement_id: &str) -> ImprovementManifest {
    let mut green_criteria = serde_json::Map::new();
    green_criteria.insert("profit_factor".to_owned(), serde_json::json!(1.1));
    ImprovementManifest {
        schema: IMPROVEMENT_RUN_SCHEMA.to_owned(),
        improvement_id: improvement_id.to_owned(),
        tier: ImprovementTier::Model,
        hypothesis: "the compression expansion holds on the development partition".to_owned(),
        created_utc: "2026-11-17T00:00:00Z".to_owned(),
        changed: ImprovementTier::Model,
        frozen: vec![ImprovementTier::Strategy, ImprovementTier::Portfolio],
        partition: ImprovementPartition::ModelDev,
        data_version: Digest::sha256(b"the test dataset"),
        config_id: "cfg_test".to_owned(),
        criteria_id: None,
        green_criteria,
        holdout_access: HoldoutAccess::None,
        baseline: None,
        code_commit: "c236539f".to_owned(),
        seed: 7,
        command: Some("run submit".to_owned()),
        analytics: Some(true),
    }
}

pub fn digest(seed: &str) -> Digest {
    Digest::sha256(seed.as_bytes())
}

pub fn baseline(owner: &str) -> BaselineRecord {
    BaselineRecord {
        baseline_id: format!("BASELINE-{owner}"),
        tier: ImprovementTier::Model,
        run_id: Some(format!("MODEL-{owner}")),
        improvement_id: None,
        accepted_by: "Human".to_owned(),
        accepted_on: "2026-11-17".to_owned(),
        scope: "the test baseline scope".to_owned(),
        supersedes: None,
        created_at: "2026-11-17T00:00:00Z".to_owned(),
    }
}

pub fn invalidation(owner: &str) -> InvalidationRecord {
    InvalidationRecord {
        invalidation_id: format!("INVALIDATION-{owner}"),
        date: "2026-11-17".to_owned(),
        commit: "worktree".to_owned(),
        removed: format!("MODEL-{owner}"),
        reason: "the test invalidation reason".to_owned(),
        superseded_by: None,
        created_at: "2026-11-17T00:00:00Z".to_owned(),
    }
}

pub async fn ledger_conformance(store: &dyn ImprovementStore, owner: &ProfileId) {
    let other = profile(&format!("{}-other", owner.as_str()));
    let first = "MODEL-2026-11-17-01";
    let second = "MODEL-2026-11-17-02";

    let created = store
        .create(
            owner.clone(),
            manifest(first),
            "2026-11-17T00:00:00Z".to_owned(),
        )
        .await
        .expect("the pre-registration");
    assert!(created.created());
    assert_eq!(created.record().state, ImprovementRunState::Created);
    store
        .create(
            owner.clone(),
            manifest(second),
            "2026-11-17T00:00:01Z".to_owned(),
        )
        .await
        .expect("the second pre-registration");

    let repeated = store
        .create(
            owner.clone(),
            manifest(first),
            "2026-11-17T00:00:02Z".to_owned(),
        )
        .await
        .expect("the idempotent create");
    assert!(!repeated.created());
    assert_eq!(repeated.record().created_at, "2026-11-17T00:00:00Z");

    let mut changed = manifest(first);
    changed.hypothesis = "a different hypothesis under the same id".to_owned();
    assert!(
        store
            .create(owner.clone(), changed, "2026-11-17T00:00:03Z".to_owned())
            .await
            .is_err(),
        "an existing improvement id must not be rewritten"
    );

    assert_eq!(
        store
            .get(other.clone(), first.to_owned())
            .await
            .expect("the other profile read"),
        None
    );
    assert!(
        store
            .list(other.clone(), 10)
            .await
            .expect("the other profile list")
            .is_empty()
    );

    assert!(
        store
            .decide(
                owner.clone(),
                first.to_owned(),
                ImprovementDecision::Accepted,
                "not yet submitted".to_owned(),
                "2026-11-17T00:10:00Z".to_owned(),
            )
            .await
            .is_err(),
        "a decision needs a submitted pipeline run"
    );

    let run = RunId(digest("the first pipeline run"));
    let submitted = store
        .link_run(
            owner.clone(),
            first.to_owned(),
            run.clone(),
            "2026-11-17T00:05:00Z".to_owned(),
        )
        .await
        .expect("the pipeline link");
    assert_eq!(submitted.state, ImprovementRunState::Submitted);
    assert_eq!(submitted.pipeline_run_id, Some(run));
    assert_eq!(submitted.canonical_digest, None);

    let duplicate_link = store
        .link_run(
            owner.clone(),
            first.to_owned(),
            RunId(digest("another run")),
            "2026-11-17T00:06:00Z".to_owned(),
        )
        .await
        .expect_err("a pipeline link is recorded once");
    assert!(
        duplicate_link.to_string().contains("already"),
        "{duplicate_link}"
    );

    let missing_digest = store
        .decide(
            owner.clone(),
            first.to_owned(),
            ImprovementDecision::Accepted,
            "the criteria passed".to_owned(),
            "2026-11-17T00:10:00Z".to_owned(),
        )
        .await
        .expect_err("an accepted decision needs the digest");
    assert!(
        missing_digest.to_string().contains("canonical digest"),
        "{missing_digest}"
    );

    let canonical = digest("the first canonical result");
    let with_digest = store
        .link_digest(owner.clone(), first.to_owned(), canonical.clone())
        .await
        .expect("the canonical digest");
    assert_eq!(with_digest.state, ImprovementRunState::Submitted);
    assert_eq!(with_digest.canonical_digest, Some(canonical));

    let duplicate_digest = store
        .link_digest(owner.clone(), first.to_owned(), digest("another result"))
        .await
        .expect_err("a canonical digest is recorded once");
    assert!(
        duplicate_digest.to_string().contains("already"),
        "{duplicate_digest}"
    );

    let decided = store
        .decide(
            owner.clone(),
            first.to_owned(),
            ImprovementDecision::Accepted,
            "the pre-registered criteria passed".to_owned(),
            "2026-11-17T00:10:00Z".to_owned(),
        )
        .await
        .expect("the accepted decision");
    assert_eq!(decided.state, ImprovementRunState::Decided);
    assert_eq!(decided.decision, Some(ImprovementDecision::Accepted));
    assert_eq!(
        decided.decision_note.as_deref(),
        Some("the pre-registered criteria passed")
    );

    assert!(
        store
            .decide(
                owner.clone(),
                first.to_owned(),
                ImprovementDecision::Rejected,
                "a second decision".to_owned(),
                "2026-11-17T00:11:00Z".to_owned(),
            )
            .await
            .is_err(),
        "a decision is recorded once"
    );

    let rejected = store
        .link_run(
            owner.clone(),
            second.to_owned(),
            RunId(digest("the failed pipeline run")),
            "2026-11-17T00:06:00Z".to_owned(),
        )
        .await
        .expect("the second pipeline link");
    assert_eq!(rejected.canonical_digest, None);
    let rejected = store
        .decide(
            owner.clone(),
            second.to_owned(),
            ImprovementDecision::Rejected,
            "the run failed before a digest existed".to_owned(),
            "2026-11-17T00:11:00Z".to_owned(),
        )
        .await
        .expect("a rejected decision without a digest");
    assert_eq!(rejected.state, ImprovementRunState::Decided);
    assert_eq!(rejected.canonical_digest, None);

    let late_digest = store
        .link_digest(owner.clone(), second.to_owned(), digest("the late digest"))
        .await
        .expect("the late canonical digest");
    assert!(late_digest.canonical_digest.is_some());

    let listed = store.list(owner.clone(), 10).await.expect("the list");
    assert_eq!(listed.len(), 2);
    assert!(
        listed
            .iter()
            .all(|row| row.state == ImprovementRunState::Decided)
    );
}

pub async fn registry_conformance(store: &dyn ImprovementStore, owner: &ProfileId) {
    let other = profile(&format!("{}-other", owner.as_str()));
    let empty = profile(&format!("{}-empty", owner.as_str()));
    let baseline_record = baseline(owner.as_str());
    let invalidation_record = invalidation(owner.as_str());

    let stored = store
        .add_baseline(owner.clone(), baseline_record.clone())
        .await
        .expect("the baseline append");
    assert_eq!(stored, baseline_record);
    assert!(
        store
            .add_baseline(owner.clone(), baseline_record.clone())
            .await
            .is_err(),
        "a duplicate baseline id is refused"
    );
    assert_eq!(
        store
            .add_baseline(other.clone(), baseline_record.clone())
            .await
            .expect("the same id in another profile"),
        baseline_record
    );

    let stored = store
        .add_invalidation(owner.clone(), invalidation_record.clone())
        .await
        .expect("the invalidation append");
    assert_eq!(stored, invalidation_record);
    assert!(
        store
            .add_invalidation(owner.clone(), invalidation_record.clone())
            .await
            .is_err(),
        "a duplicate invalidation id is refused"
    );
    assert_eq!(
        store
            .add_invalidation(other.clone(), invalidation_record.clone())
            .await
            .expect("the same id in another profile"),
        invalidation_record
    );

    assert_eq!(
        store.baselines(owner.clone()).await.expect("the baselines"),
        vec![baseline_record.clone()]
    );
    assert_eq!(
        store
            .baselines(other.clone())
            .await
            .expect("the other baselines"),
        vec![baseline_record.clone()]
    );
    assert!(
        store
            .baselines(empty.clone())
            .await
            .expect("the empty baselines")
            .is_empty()
    );
    assert_eq!(
        store
            .invalidations(owner.clone())
            .await
            .expect("the invalidations"),
        vec![invalidation_record.clone()]
    );
    assert_eq!(
        store
            .invalidations(other.clone())
            .await
            .expect("the other invalidations"),
        vec![invalidation_record]
    );
    assert!(
        store
            .invalidations(empty)
            .await
            .expect("the empty invalidations")
            .is_empty()
    );

    let mut missing_identity = baseline_record.clone();
    missing_identity.baseline_id = "BASELINE-MISSING".to_owned();
    missing_identity.run_id = None;
    missing_identity.improvement_id = None;
    assert!(
        store
            .add_baseline(owner.clone(), missing_identity)
            .await
            .is_err(),
        "a baseline needs a run or improvement reference"
    );

    let mut empty_owner = baseline_record.clone();
    empty_owner.baseline_id = "BASELINE-EMPTY-OWNER".to_owned();
    empty_owner.accepted_by = " ".to_owned();
    assert!(
        store
            .add_baseline(owner.clone(), empty_owner)
            .await
            .is_err(),
        "a baseline needs a non-empty accepted_by"
    );

    let mut empty_scope = baseline_record;
    empty_scope.baseline_id = "BASELINE-EMPTY-SCOPE".to_owned();
    empty_scope.scope = String::new();
    assert!(
        store
            .add_baseline(owner.clone(), empty_scope)
            .await
            .is_err(),
        "a baseline needs a non-empty scope"
    );

    for (field, value) in [
        ("date", String::new()),
        ("commit", String::new()),
        ("removed", String::new()),
        ("reason", String::new()),
    ] {
        let mut record = invalidation(owner.as_str());
        record.invalidation_id = format!("INVALIDATION-EMPTY-{field}");
        match field {
            "date" => record.date = value,
            "commit" => record.commit = value,
            "removed" => record.removed = value,
            _ => record.reason = value,
        }
        assert!(
            store.add_invalidation(owner.clone(), record).await.is_err(),
            "an invalidation needs a non-empty {field}"
        );
    }
}
