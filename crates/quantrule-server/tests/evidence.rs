//! The evidence finalizer over in-memory services.

mod common;

use std::sync::Arc;

use quantrule_api::{
    ArtifactRef, Digest, ProfileId, RunState, StageName, StageProgress, StageStatus,
};
use quantrule_server::evidence::{
    EvidenceServices, FinalizeOutcome, RunManifest, digest, finalize,
};
use quantrule_server::runs::{MemoryRuns, NewRun, RunRecord, RunStore, derive};
use quantrule_server::runtime::adapters::ObjectStore;

fn profile(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("profile")
}

fn new_run(suffix: &str) -> NewRun {
    let data_version = Digest::sha256(b"data");
    let config_hash = Digest::sha256(b"config");
    let image_digest = Digest::sha256(suffix.as_bytes());
    NewRun {
        run_id: derive(&data_version, &config_hash, "M1.0", &image_digest),
        config_id: "cfg_abc".to_owned(),
        data_version,
        config_hash,
        model_version: "M1.0".to_owned(),
        image_digest,
        owner: "default".to_owned(),
        submitted_at: "2026-11-13T00:00:00Z".to_owned(),
    }
}

fn computed_stages() -> Vec<StageProgress> {
    [
        StageName::Data,
        StageName::Model,
        StageName::Backtest,
        StageName::Evidence,
    ]
    .into_iter()
    .map(|name| StageProgress {
        name,
        state: if name == StageName::Evidence {
            StageStatus::Pending
        } else {
            StageStatus::Done
        },
        elapsed_ms: None,
    })
    .collect()
}

struct Fixture {
    runs: MemoryRuns,
    objects: common::MemoryObjects,
    run_id: String,
}

async fn fixture(suffix: &str) -> Fixture {
    let runs = MemoryRuns::default();
    let objects = common::MemoryObjects::default();
    let submitted = runs
        .submit(profile("default"), new_run(suffix))
        .await
        .expect("submit");
    let run_id = submitted.record().run_id.to_string();
    runs.claim(run_id.clone(), false).await.expect("claim");
    Fixture {
        runs,
        objects,
        run_id,
    }
}

impl Fixture {
    fn services(&self) -> EvidenceServices {
        EvidenceServices {
            runs: Arc::new(self.runs.clone()) as Arc<dyn RunStore>,
            objects: Arc::new(self.objects.clone()),
        }
    }

    /// Write one content-addressed blob and return its reference.
    async fn put(&self, name: &str, bytes: &[u8]) -> ArtifactRef {
        let sha256 = Digest::sha256(bytes);
        let key = RunManifest::blob_key(&sha256);
        self.objects.put(&key, bytes.to_vec()).await.expect("put");
        ArtifactRef {
            name: name.to_owned(),
            bytes: bytes.len() as u64,
            sha256,
        }
    }

    /// Publish a manifest for these references.
    async fn manifest(&self, artifacts: &[ArtifactRef]) -> RunManifest {
        let manifest = RunManifest::from_artifacts(&self.run_id, artifacts);
        self.objects
            .put(
                &RunManifest::key(&self.run_id),
                manifest.to_bytes().expect("manifest bytes"),
            )
            .await
            .expect("manifest");
        manifest
    }

    /// Record the computed set; the manifest is written separately so a test
    /// can omit or disagree with it.
    async fn computed(
        &self,
        artifacts: Vec<ArtifactRef>,
        digest: Digest,
        stages: Vec<StageProgress>,
    ) -> RunRecord {
        self.runs
            .computed(self.run_id.clone(), digest, artifacts, stages)
            .await
            .expect("computed")
    }

    /// Publish the manifest and then record the same set.
    async fn publish(
        &self,
        artifacts: &[ArtifactRef],
        record_digest: Option<Digest>,
        stages: Vec<StageProgress>,
    ) -> RunRecord {
        let manifest = self.manifest(artifacts).await;
        self.computed(
            artifacts.to_vec(),
            record_digest.unwrap_or_else(|| manifest.digest.clone()),
            stages,
        )
        .await
    }

    async fn record(&self) -> RunRecord {
        self.runs
            .get_any(self.run_id.clone())
            .await
            .expect("get")
            .expect("record")
    }
}

#[tokio::test]
async fn a_computed_run_is_verified_and_finished() {
    let fixture = fixture("ok").await;
    let results = fixture.put("results.json", b"{\"profit\":1}").await;
    let trades = fixture.put("trades.csv", b"entry,exit\n").await;
    let provenance = fixture.put("provenance.json", b"{\"image\":\"I1\"}").await;
    let artifacts = vec![results, trades, provenance.clone()];
    let canonical = digest(&artifacts);
    assert_ne!(
        canonical,
        digest(&[provenance]),
        "provenance is excluded from the digest"
    );
    fixture
        .publish(&artifacts, Some(canonical.clone()), computed_stages())
        .await;

    assert_eq!(
        finalize(fixture.run_id.clone(), &fixture.services())
            .await
            .expect("finalize"),
        FinalizeOutcome::Succeeded
    );
    let record = fixture.record().await;
    assert_eq!(record.status, RunState::Succeeded);
    assert_eq!(record.digest, Some(canonical));
    assert_eq!(record.error, None);
    let evidence_stage = record
        .stages
        .iter()
        .find(|stage| stage.name == StageName::Evidence)
        .expect("evidence stage");
    assert_eq!(evidence_stage.state, StageStatus::Done);
    assert!(
        fixture
            .objects
            .exists(&RunManifest::key(&fixture.run_id))
            .await
            .expect("manifest")
    );
    assert!(
        !fixture
            .objects
            .exists(&format!("runs/{}/results.json", fixture.run_id))
            .await
            .expect("per-name key")
    );
}

#[tokio::test]
async fn a_missing_manifest_blocks_success() {
    let fixture = fixture("no-manifest").await;
    let results = fixture.put("results.json", b"{\"profit\":1}").await;
    fixture
        .computed(
            vec![results.clone()],
            digest(std::slice::from_ref(&results)),
            computed_stages(),
        )
        .await;
    assert_eq!(
        finalize(fixture.run_id.clone(), &fixture.services())
            .await
            .expect("finalize"),
        FinalizeOutcome::Failed
    );
    let record = fixture.record().await;
    assert_eq!(record.status, RunState::Failed);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("no artifact manifest"))
    );
}

#[tokio::test]
async fn a_manifest_that_disagrees_with_the_run_blocks_success() {
    let fixture = fixture("disagree").await;
    let results = fixture.put("results.json", b"{\"profit\":1}").await;
    let other = fixture.put("other.csv", b"a,b\n").await;
    let manifest = fixture.manifest(std::slice::from_ref(&other)).await;
    fixture
        .computed(vec![results], manifest.digest.clone(), computed_stages())
        .await;
    assert_eq!(
        finalize(fixture.run_id.clone(), &fixture.services())
            .await
            .expect("finalize"),
        FinalizeOutcome::Failed
    );
    let record = fixture.record().await;
    assert_eq!(record.status, RunState::Failed);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("does not match the run record"))
    );
}

#[tokio::test]
async fn a_missing_artifact_blocks_success() {
    let fixture = fixture("missing").await;
    let missing = ArtifactRef {
        name: "results.json".to_owned(),
        bytes: 2,
        sha256: Digest::sha256(b"{}"),
    };
    let manifest = fixture.manifest(std::slice::from_ref(&missing)).await;
    fixture
        .computed(vec![missing], manifest.digest.clone(), computed_stages())
        .await;
    assert_eq!(
        finalize(fixture.run_id.clone(), &fixture.services())
            .await
            .expect("finalize"),
        FinalizeOutcome::Failed
    );
    let record = fixture.record().await;
    assert_eq!(record.status, RunState::Failed);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("missing"))
    );
    let evidence_stage = record
        .stages
        .iter()
        .find(|stage| stage.name == StageName::Evidence)
        .expect("evidence stage");
    assert_eq!(evidence_stage.state, StageStatus::Failed);
}

#[tokio::test]
async fn one_changed_artifact_blocks_success() {
    let fixture = fixture("changed").await;
    let results = fixture.put("results.json", b"{\"profit\":1}").await;
    fixture
        .publish(std::slice::from_ref(&results), None, computed_stages())
        .await;
    let key = RunManifest::blob_key(&results.sha256);
    fixture
        .objects
        .put(&key, b"{\"profit\":9}".to_vec())
        .await
        .expect("overwrite");
    assert_eq!(
        finalize(fixture.run_id.clone(), &fixture.services())
            .await
            .expect("finalize"),
        FinalizeOutcome::Failed
    );
    let record = fixture.record().await;
    assert_eq!(record.status, RunState::Failed);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("does not match"))
    );
}

#[tokio::test]
async fn a_digest_mismatch_blocks_success() {
    let fixture = fixture("mismatch").await;
    let results = fixture.put("results.json", b"{\"profit\":1}").await;
    fixture
        .publish(
            std::slice::from_ref(&results),
            Some(Digest::sha256(b"not-the-artifacts")),
            computed_stages(),
        )
        .await;
    assert_eq!(
        finalize(fixture.run_id.clone(), &fixture.services())
            .await
            .expect("finalize"),
        FinalizeOutcome::Failed
    );
    let record = fixture.record().await;
    assert_eq!(record.status, RunState::Failed);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("stored digest"))
    );
}

#[tokio::test]
async fn an_unfinished_stage_blocks_success() {
    let fixture = fixture("unfinished").await;
    let results = fixture.put("results.json", b"{\"profit\":1}").await;
    let mut stages = computed_stages();
    for stage in &mut stages {
        if stage.name == StageName::Model {
            stage.state = StageStatus::Pending;
        }
    }
    fixture
        .publish(std::slice::from_ref(&results), None, stages)
        .await;
    assert_eq!(
        finalize(fixture.run_id.clone(), &fixture.services())
            .await
            .expect("finalize"),
        FinalizeOutcome::Failed
    );
    let record = fixture.record().await;
    assert_eq!(record.status, RunState::Failed);
    assert!(
        record
            .error
            .as_deref()
            .is_some_and(|error| error.contains("model"))
    );
}

#[tokio::test]
async fn a_finished_run_is_not_finalized_again() {
    let fixture = fixture("twice").await;
    let results = fixture.put("results.json", b"{\"profit\":1}").await;
    fixture
        .publish(std::slice::from_ref(&results), None, computed_stages())
        .await;
    let services = fixture.services();
    assert_eq!(
        finalize(fixture.run_id.clone(), &services)
            .await
            .expect("first"),
        FinalizeOutcome::Succeeded
    );
    assert_eq!(
        finalize(fixture.run_id.clone(), &services)
            .await
            .expect("second"),
        FinalizeOutcome::Skipped
    );
    assert_eq!(fixture.record().await.status, RunState::Succeeded);
}

#[tokio::test]
async fn a_cancelled_run_is_not_finalized() {
    let fixture = fixture("cancelled").await;
    fixture
        .runs
        .cancel(profile("default"), fixture.run_id.clone())
        .await
        .expect("cancel");
    assert_eq!(
        finalize(fixture.run_id.clone(), &fixture.services())
            .await
            .expect("finalize"),
        FinalizeOutcome::Skipped
    );
    assert_eq!(fixture.record().await.status, RunState::Cancelled);
}

#[tokio::test]
async fn two_profiles_share_one_finalized_run() {
    let fixture = fixture("shared").await;
    let submitted = fixture
        .runs
        .submit(profile("alpha"), new_run("shared"))
        .await
        .expect("submit");
    assert_eq!(submitted.record().run_id.to_string(), fixture.run_id);
    let results = fixture.put("results.json", b"{\"profit\":1}").await;
    fixture
        .publish(std::slice::from_ref(&results), None, computed_stages())
        .await;
    assert_eq!(
        finalize(fixture.run_id.clone(), &fixture.services())
            .await
            .expect("finalize"),
        FinalizeOutcome::Succeeded
    );
    for name in ["default", "alpha"] {
        let record = fixture
            .runs
            .get(profile(name), fixture.run_id.clone())
            .await
            .expect("get")
            .expect("record");
        assert_eq!(record.status, RunState::Succeeded);
    }
    assert!(
        fixture
            .runs
            .get(profile("beta"), fixture.run_id)
            .await
            .expect("get")
            .is_none()
    );
}
