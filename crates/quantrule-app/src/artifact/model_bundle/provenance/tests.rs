//! Regression tests for the model bundle's strategy-only reuse invariant.
//!
//! The model artifact is upstream of strategy. A change to the strategy file or
//! its name must change the run config identity but not the model bundle files
//! or the content address computed over them.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_api::Digest;
use serde_json::Value;

use super::config_files;
use crate::artifact::model_bundle::model_bundle_from;
use crate::config::RunConfig;
use crate::pipeline;

static NEXT: AtomicU64 = AtomicU64::new(1);

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "quantrule-model-reuse-{}-{label}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).expect("the fixture directory is created");
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../quantrule-data-service/tests/fixtures/config/valid-v4");
        for name in [
            "run.json",
            "data.json",
            "model.json",
            "execution.json",
            "portfolio.json",
            "strategy.json",
        ] {
            fs::copy(source.join(name), root.join(name)).expect("the fixture file is copied");
        }

        // The model reads only the candles; the catalogued source path is not
        // available in a unit test.
        let data_path = root.join("data.json");
        let mut data: Value = serde_json::from_str(
            &fs::read_to_string(&data_path).expect("the data fixture is readable"),
        )
        .expect("the data fixture is JSON");
        data["source"]["path"] = serde_json::json!("candles.csv");
        fs::write(
            data_path,
            serde_json::to_vec_pretty(&data).expect("the data fixture is JSON"),
        )
        .expect("the data fixture is written");

        let mut csv = String::from("Date,Open,High,Low,Close,Volume\n");
        for offset in 0..300_u32 {
            let hour = offset / 60;
            let minute = offset % 60;
            let open = 100.0 + f64::from(offset) * 0.01;
            csv.push_str(&format!(
                "2024-01-01 {hour:02}:{minute:02}:00+00:00,{open},{},{},{},10\n",
                open + 0.5,
                open - 0.5,
                open + 0.25
            ));
        }
        fs::write(root.join("candles.csv"), csv).expect("the candle fixture is written");
        Self { root }
    }

    fn load(&self) -> RunConfig {
        RunConfig::load(&self.root.join("run.json")).expect("the fixture run is valid")
    }

    fn rename_strategy(&self, name: &str) {
        fs::rename(self.root.join("strategy.json"), self.root.join(name))
            .expect("the strategy file is renamed");
        let run_path = self.root.join("run.json");
        let mut run: Value = serde_json::from_str(
            &fs::read_to_string(&run_path).expect("the run fixture is readable"),
        )
        .expect("the run fixture is JSON");
        run["strategy"] = serde_json::json!(name);
        fs::write(
            &run_path,
            serde_json::to_vec_pretty(&run).expect("the run fixture is JSON"),
        )
        .expect("the run fixture is written");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn artifact_digests(root: &Path) -> BTreeMap<String, Digest> {
    let mut digests = BTreeMap::new();
    for entry in fs::read_dir(root).expect("the bundle is readable") {
        let entry = entry.expect("the bundle entry is readable");
        let path = entry.path();
        if path.is_file() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = fs::read(&path).expect("the artifact is readable");
            digests.insert(name, Digest::sha256(&bytes));
        }
    }
    digests
}

fn set_digest(entries: &BTreeMap<String, Digest>) -> Digest {
    let entries: Vec<(String, Digest)> = entries
        .iter()
        .map(|(name, digest)| (name.clone(), digest.clone()))
        .collect();
    Digest::canonical(&entries)
}

#[test]
fn a_v4_model_bundle_lists_no_strategy_file() {
    let fixture = Fixture::new("files");
    let config = fixture.load();
    assert_eq!(
        config_files(&config),
        vec![
            "run.json".to_owned(),
            "data.json".to_owned(),
            "model.json".to_owned(),
            "execution.json".to_owned(),
            "portfolio.json".to_owned(),
        ]
    );
}

#[test]
fn a_strategy_only_rename_keeps_the_model_bundle_identity() {
    let fixture = Fixture::new("rename");
    let original = fixture.load();
    let original_files = config_files(&original);
    let original_model = pipeline::run_model(&original).expect("the original model runs");
    let original_bundle = model_bundle_from(
        &original,
        &fixture.root.join("out-original"),
        &original_model,
    )
    .expect("the original bundle is written");

    fixture.rename_strategy("strategy-renamed.json");
    let renamed = fixture.load();
    assert_eq!(renamed.model_input_hash, original.model_input_hash);
    assert_ne!(renamed.config_hash, original.config_hash);
    assert_eq!(config_files(&renamed), original_files);

    let renamed_model = pipeline::run_model(&renamed).expect("the renamed model runs");
    let renamed_bundle =
        model_bundle_from(&renamed, &fixture.root.join("out-renamed"), &renamed_model)
            .expect("the renamed bundle is written");

    let first = artifact_digests(&original_bundle.path);
    let second = artifact_digests(&renamed_bundle.path);
    assert_eq!(first, second);
    assert_eq!(first.get("provenance.json"), second.get("provenance.json"));
    assert_eq!(set_digest(&first), set_digest(&second));
}

#[test]
fn a_v4_strategy_content_change_keeps_the_model_bundle_identity() {
    let fixture = Fixture::new("content");
    let original = fixture.load();
    let original_model = pipeline::run_model(&original).expect("the original model runs");
    let original_bundle = model_bundle_from(
        &original,
        &fixture.root.join("out-original"),
        &original_model,
    )
    .expect("the original bundle is written");

    let strategy_path = fixture.root.join("strategy.json");
    let mut strategy: Value = serde_json::from_str(
        &fs::read_to_string(&strategy_path).expect("the strategy is readable"),
    )
    .expect("the strategy is JSON");
    strategy["risk"]["reward_to_risk"] = serde_json::json!(1.25);
    fs::write(
        &strategy_path,
        serde_json::to_vec_pretty(&strategy).expect("the changed strategy is JSON"),
    )
    .expect("the changed strategy is written");
    let changed = fixture.load();
    assert_eq!(changed.model_input_hash, original.model_input_hash);
    assert_ne!(changed.config_hash, original.config_hash);

    let changed_model = pipeline::run_model(&changed).expect("the changed model runs");
    let changed_bundle =
        model_bundle_from(&changed, &fixture.root.join("out-changed"), &changed_model)
            .expect("the changed bundle is written");

    let first = artifact_digests(&original_bundle.path);
    let second = artifact_digests(&changed_bundle.path);
    assert_eq!(first, second);
    assert_eq!(first.get("provenance.json"), second.get("provenance.json"));
    assert_eq!(set_digest(&first), set_digest(&second));
}
