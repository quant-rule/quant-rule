//! Shared in-memory services and request helpers for the model-service tests.
//!
//! Each integration target compiles this module separately, and a target need
//! not use every helper, so unused helpers are allowed here.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use quantrule_api::Digest;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_runtime::client::MemoryDataClient;
use quantrule_runtime::{Config, HealthRegistry};
use quantrule_service_api::{ConfigBundle, DatasetObject};

use quantrule_model_service::{ModelServices, ModelState};

/// The in-memory object store from `quantrule_runtime`.
// Each integration target compiles this module separately, and not every
// target exercises the object store.
#[allow(unused_imports)]
pub use quantrule_runtime::adapters::memory::MemoryObjects;

/// A seeded dataset and config catalog.
pub struct Seed {
    /// Where the dataset and model objects live.
    pub objects: MemoryObjects,
    /// The data service's typed reads.
    pub data: MemoryDataClient,
    /// The dataset version.
    pub data_version: Digest,
}

/// Seed the 250-minute fixture dataset, one valid config, and one invalid config.
pub async fn seed() -> Seed {
    seed_from(&csv(250), 250).await
}

/// Seed a canonical dataset and the shared configs.
pub async fn seed_from(dataset: &[u8], candles: u64) -> Seed {
    let objects = MemoryObjects::default();
    let data = MemoryDataClient::default();
    let profile = quantrule_api::ProfileId::parse("default").expect("profile");

    let version = Digest::sha256(dataset);
    let key = format!("datasets/{}", version.as_hex());
    objects
        .put(&key, dataset.to_vec())
        .await
        .expect("dataset object");
    data.add_dataset(
        &profile,
        DatasetObject {
            data_version: version.clone(),
            object_key: key,
            sha256: version.clone(),
            candles,
            first_bar: 1_704_067_200,
            last_bar: 1_704_067_200 + (candles.saturating_sub(1)) * 60,
            bytes: dataset.len() as u64,
        },
    );
    data.add_config(
        &profile,
        ConfigBundle {
            config_id: "cfg_valid".to_owned(),
            references: Vec::new(),
            files: valid_files(),
            valid: true,
            message: None,
            config_hash: Some(Digest::sha256(b"valid")),
        },
    );
    data.add_config(
        &profile,
        ConfigBundle {
            config_id: "cfg_invalid".to_owned(),
            references: Vec::new(),
            files: BTreeMap::from([("run.json".to_owned(), "{}".to_owned())]),
            valid: false,
            message: Some("the bundle has no data.json".to_owned()),
            config_hash: None,
        },
    );
    Seed {
        objects,
        data,
        data_version: version,
    }
}

/// Build a model-service state over the seed.
pub fn state(seed: &Seed) -> Arc<ModelState> {
    Arc::new(ModelState::with_services(
        &Config::default(),
        HealthRegistry::new(),
        ModelServices {
            data: Some(Arc::new(seed.data.clone())),
            objects: Some(Arc::new(seed.objects.clone())),
        },
    ))
}

/// The valid run-config bundle.
pub fn valid_files() -> BTreeMap<String, String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/config/valid");
    [
        "run.json",
        "data.json",
        "model.json",
        "execution.json",
        "portfolio.json",
    ]
    .into_iter()
    .map(|name| {
        (
            name.to_owned(),
            std::fs::read_to_string(root.join(name)).expect("fixture"),
        )
    })
    .collect()
}

/// The 250 one-minute fixture candles, canonicalized the way ingest does.
pub fn csv(minutes: usize) -> Vec<u8> {
    let mut text = String::new();
    for offset in 0..minutes {
        let hour = offset / 60;
        let minute = offset % 60;
        let open = 100.0 + offset as f64 * 0.01;
        text.push_str(&format!(
            "2024-01-01 {hour:02}:{minute:02}:00+00:00,{open},{},{},{},10\n",
            open + 0.5,
            open - 0.5,
            open + 0.25
        ));
    }
    canonical(&text)
}

/// Canonicalize a date-and-price CSV for a test, the way ingest does.
pub fn canonical(csv: &str) -> Vec<u8> {
    use quantrule_data::{Candle, RawCandle};

    let mut out = Vec::new();
    let mut index = 0_u64;
    for line in csv.lines() {
        let fields: Vec<&str> = line.split(',').map(str::trim).collect();
        if fields.len() < 5 || !fields[0].starts_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let raw = RawCandle {
            date: fields[0],
            open: fields[1],
            high: fields[2],
            low: fields[3],
            close: fields[4],
            volume: fields.get(5).copied(),
        };
        let candle = Candle::parse(index, raw).expect("test candle");
        out.extend_from_slice(candle.canonical_line().as_bytes());
        out.push(b'\n');
        index += 1;
    }
    out
}

/// Read a response body.
pub async fn body_bytes(response: axum::response::Response) -> Vec<u8> {
    axum::body::to_bytes(response.into_body(), 1_048_576)
        .await
        .expect("body")
        .to_vec()
}
