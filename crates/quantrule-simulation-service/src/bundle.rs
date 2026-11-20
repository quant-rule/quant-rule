//! The recorded model bundle the research plane reads.
//!
//! A simulation stage never reruns the model. The service fetches one recorded
//! bundle from the content-addressed store, verifies the digest the researcher
//! named, and hands the domain crate the bars and events it already knows. The
//! bundle carries no strategy, execution, or portfolio value.
//!
//! The object key is `simulation/bundles/<sha256>.json`. The digest in the key
//! is the SHA-256 of the stored bytes, so a changed body cannot be read under
//! the name the run recorded.

use std::collections::BTreeMap;
use std::sync::Arc;

use quantrule_api::Digest;
use quantrule_runtime::adapters::ObjectStore;
use quantrule_simulation::{Bar, Bundle, Catalog, Event, utc_from_seconds};
use serde::{Deserialize, Serialize};

/// The bundle schema the service accepts.
pub const SIMULATION_BUNDLE_SCHEMA: &str = quantrule_api::simulation::BUNDLE_SCHEMA;

/// Why a recorded bundle cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleError {
    /// The object is absent.
    Missing,
    /// The object store could not be reached.
    Unavailable,
    /// The stored bytes do not hash to the requested digest.
    DigestMismatch,
    /// The body is not a bundle.
    Malformed(String),
    /// The bundle carries no bars.
    Empty,
}

impl std::fmt::Display for BundleError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => formatter.write_str("the recorded bundle is absent"),
            Self::Unavailable => formatter.write_str("the object store is unavailable"),
            Self::DigestMismatch => {
                formatter.write_str("the stored bundle does not match its digest")
            }
            Self::Malformed(message) => write!(formatter, "the bundle is malformed: {message}"),
            Self::Empty => formatter.write_str("the bundle has no bars"),
        }
    }
}

impl std::error::Error for BundleError {}

/// One confirmed level record, as the chart draws it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Level {
    /// The reference code.
    pub reference_code: u64,
    /// The scale code.
    pub scale_code: i32,
    /// The kind code: -1 low, 0 mid, 1 high.
    pub kind_code: i32,
    /// The price.
    pub price: f64,
    /// The bar on which the level occurred.
    pub occurred_bar: u64,
    /// The first bar on which the level became knowable.
    pub available_bar: u64,
    /// The distance to the level, in ATR, when the model published one.
    pub distance_atr: Option<f64>,
}

impl Default for Level {
    fn default() -> Self {
        Self {
            reference_code: 0,
            scale_code: 0,
            kind_code: 0,
            price: f64::NAN,
            occurred_bar: 0,
            available_bar: 0,
            distance_atr: None,
        }
    }
}

/// The recorded bundle envelope, field for field over the model report.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimulationBundle {
    /// The schema token.
    pub schema: String,
    /// The bar timeframe, in seconds.
    pub timeframe_secs: u64,
    /// The bars, ordered by the loader.
    pub bars: Vec<Bar>,
    /// The published events.
    #[serde(default)]
    pub events: Vec<Event>,
    /// The confirmed levels.
    #[serde(default)]
    pub levels: Vec<Level>,
    /// The event-code to registry-name map.
    #[serde(default)]
    pub catalog: Catalog,
    /// The recorded provenance.
    #[serde(default)]
    pub provenance: BTreeMap<String, String>,
}

/// A verified bundle ready for the domain stages.
#[derive(Debug, Clone)]
pub struct LoadedBundle {
    /// The domain view, sorted and indexed.
    pub bundle: Bundle,
    /// The levels, ordered by availability.
    pub levels: Vec<Level>,
    /// The event catalog.
    pub catalog: Catalog,
    /// The recorded data version, when present.
    pub data_version: String,
}

impl LoadedBundle {
    /// The position of a bundle bar identity in the ordered bars.
    #[must_use]
    pub fn position_of(&self, bar: u64) -> Option<usize> {
        self.bundle.index_of(bar)
    }

    /// The first bar timestamp.
    #[must_use]
    pub fn first_timestamp(&self) -> i64 {
        self.bundle.bars.first().map_or(0, |bar| bar.timestamp)
    }

    /// The last bar timestamp.
    #[must_use]
    pub fn last_timestamp(&self) -> i64 {
        self.bundle.bars.last().map_or(0, |bar| bar.timestamp)
    }
}

/// The content-addressed key of one recorded bundle.
pub use quantrule_api::simulation::bundle_key;

/// Read and verify one recorded bundle.
///
/// # Errors
///
/// Returns [`BundleError::Missing`] when the object store fails or the key is
/// absent, [`BundleError::DigestMismatch`] when the bytes do not hash to the
/// requested digest, and [`BundleError::Malformed`] when the body is not a
/// valid bundle.
pub async fn load(
    objects: &Arc<dyn ObjectStore>,
    digest: &Digest,
) -> Result<LoadedBundle, BundleError> {
    let key = bundle_key(digest);
    let present = objects
        .exists(&key)
        .await
        .map_err(|_| BundleError::Unavailable)?;
    if !present {
        return Err(BundleError::Missing);
    }
    let bytes = objects
        .get(&key)
        .await
        .map_err(|_| BundleError::Unavailable)?;
    if &Digest::sha256(&bytes) != digest {
        return Err(BundleError::DigestMismatch);
    }
    let decoded: SimulationBundle = serde_json::from_slice(&bytes)
        .map_err(|error| BundleError::Malformed(error.to_string()))?;
    if decoded.schema != SIMULATION_BUNDLE_SCHEMA {
        return Err(BundleError::Malformed(format!(
            "the schema is {}, not {SIMULATION_BUNDLE_SCHEMA}",
            decoded.schema
        )));
    }
    if decoded.bars.is_empty() {
        return Err(BundleError::Empty);
    }
    if decoded.timeframe_secs == 0 {
        return Err(BundleError::Malformed(
            "the timeframe must be a positive number of seconds".to_owned(),
        ));
    }

    let mut bars = decoded.bars;
    bars.sort_by_key(|bar| (bar.timestamp, bar.bar));
    let mut seen = BTreeMap::new();
    for bar in &bars {
        if seen.insert(bar.bar, ()).is_some() {
            return Err(BundleError::Malformed(format!(
                "bar {} appears twice",
                bar.bar
            )));
        }
    }
    if bars
        .windows(2)
        .any(|pair| pair[0].timestamp == pair[1].timestamp && pair[0].bar == pair[1].bar)
    {
        return Err(BundleError::Malformed(
            "the bars are not totally ordered".to_owned(),
        ));
    }

    let mut levels = decoded.levels;
    levels.retain(|level| seen.contains_key(&level.available_bar));
    levels.sort_by(|left, right| {
        (left.available_bar, left.price.to_bits(), left.occurred_bar).cmp(&(
            right.available_bar,
            right.price.to_bits(),
            right.occurred_bar,
        ))
    });
    for level in &levels {
        if !level.price.is_finite() {
            return Err(BundleError::Malformed(
                "a level carries a non-finite price".to_owned(),
            ));
        }
    }

    let data_version = decoded
        .provenance
        .get("data_version")
        .cloned()
        .unwrap_or_else(|| "unknown".to_owned());
    let events = decoded.events;
    let bundle =
        Bundle::new(decoded.timeframe_secs, bars, events).with_data_version(data_version.clone());
    Ok(LoadedBundle {
        bundle,
        levels,
        catalog: decoded.catalog,
        data_version,
    })
}

/// Format a bar timestamp the way the mark schema stores it.
#[must_use]
pub fn bar_time(timestamp: i64) -> String {
    utc_from_seconds(timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_runtime::adapters::{Backend, BoxFuture, StoreError};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MapStore {
        inner: Mutex<BTreeMap<String, Vec<u8>>>,
    }

    struct DownStore;

    impl ObjectStore for DownStore {
        fn backend(&self) -> Backend {
            Backend::S3
        }

        fn put(&self, _key: &str, _bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
            Box::pin(async { Err(StoreError("down".to_owned())) })
        }

        fn get(&self, _key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
            Box::pin(async { Err(StoreError("down".to_owned())) })
        }

        fn exists(&self, _key: &str) -> BoxFuture<Result<bool, StoreError>> {
            Box::pin(async { Err(StoreError("down".to_owned())) })
        }

        fn delete(&self, _key: &str) -> BoxFuture<Result<(), StoreError>> {
            Box::pin(async { Err(StoreError("down".to_owned())) })
        }
    }

    impl MapStore {
        fn put_sync(&self, key: &str, bytes: Vec<u8>) {
            self.inner
                .lock()
                .expect("object lock")
                .insert(key.to_owned(), bytes);
        }
    }

    impl ObjectStore for MapStore {
        fn backend(&self) -> Backend {
            Backend::Filesystem
        }

        fn put(&self, key: &str, bytes: Vec<u8>) -> BoxFuture<Result<(), StoreError>> {
            self.put_sync(key, bytes);
            Box::pin(async { Ok(()) })
        }

        fn get(&self, key: &str) -> BoxFuture<Result<Vec<u8>, StoreError>> {
            let found = self
                .inner
                .lock()
                .expect("object lock")
                .get(key)
                .cloned()
                .ok_or_else(|| StoreError(format!("{key} is absent")));
            Box::pin(async move { found })
        }

        fn exists(&self, key: &str) -> BoxFuture<Result<bool, StoreError>> {
            let found = self.inner.lock().expect("object lock").contains_key(key);
            Box::pin(async move { Ok(found) })
        }

        fn delete(&self, key: &str) -> BoxFuture<Result<(), StoreError>> {
            self.inner.lock().expect("object lock").remove(key);
            Box::pin(async { Ok(()) })
        }
    }

    fn raw() -> Vec<u8> {
        permissive_json()
    }

    fn permissive_json() -> Vec<u8> {
        let bars = (0..30)
            .map(|index| {
                serde_json::json!({
                    "bar": index,
                    "timestamp": 1_514_764_800_i64 + i64::from(index) * 900,
                    "open": 100.0,
                    "high": 101.0,
                    "low": 99.0,
                    "close": 100.0,
                    "volume": 1.0,
                    "atr": 1.0,
                })
            })
            .collect::<Vec<_>>();
        serde_json::to_vec(&serde_json::json!({
            "schema": SIMULATION_BUNDLE_SCHEMA,
            "timeframe_secs": 900,
            "bars": bars,
            "events": [],
            "levels": [],
            "catalog": {},
            "provenance": {"data_version": "fixture"}
        }))
        .expect("encode")
    }

    fn store(bytes: &[u8]) -> (Arc<MapStore>, Digest) {
        let store = Arc::new(MapStore::default());
        let digest = Digest::sha256(bytes);
        store.put_sync(&bundle_key(&digest), bytes.to_vec());
        (store, digest)
    }

    #[tokio::test]
    async fn a_bundle_orders_its_bars_and_reports_the_data_version() {
        let mut value: serde_json::Value = serde_json::from_slice(&raw()).expect("json");
        value["bars"].as_array_mut().expect("bars").reverse();
        let bytes = serde_json::to_vec(&value).expect("encode");
        let (store, digest) = store(&bytes);
        let loaded = load(&(store as Arc<dyn ObjectStore>), &digest)
            .await
            .expect("load");
        assert_eq!(loaded.bundle.bars.len(), 30);
        assert_eq!(loaded.bundle.bars[0].bar, 0);
        assert_eq!(loaded.data_version, "fixture");
        assert_eq!(loaded.bundle.data_version(), "fixture");
        assert_eq!(
            bundle_key(&digest),
            format!("simulation/bundles/{}.json", digest.as_hex())
        );
    }

    #[tokio::test]
    async fn a_duplicate_bar_is_refused() {
        let mut value: serde_json::Value = serde_json::from_slice(&raw()).expect("json");
        value["bars"][1]["bar"] = serde_json::json!(0);
        let bytes = serde_json::to_vec(&value).expect("encode");
        let (store, digest) = store(&bytes);
        assert!(matches!(
            load(&(store as Arc<dyn ObjectStore>), &digest).await,
            Err(BundleError::Malformed(_))
        ));
    }

    #[tokio::test]
    async fn a_changed_body_does_not_match_its_digest() {
        let bytes = raw();
        let (store, digest) = store(&bytes);
        store.put_sync(&bundle_key(&digest), b"changed".to_vec());
        assert!(matches!(
            load(&(store as Arc<dyn ObjectStore>), &digest).await,
            Err(BundleError::DigestMismatch)
        ));
    }

    #[tokio::test]
    async fn a_down_object_store_is_not_reported_as_a_missing_bundle() {
        let store: Arc<dyn ObjectStore> = Arc::new(DownStore);
        let digest = Digest::sha256(b"anything");
        assert!(matches!(
            load(&store, &digest).await,
            Err(BundleError::Unavailable)
        ));
    }

    #[tokio::test]
    async fn a_missing_bundle_is_reported() {
        let store = Arc::new(MapStore::default());
        let digest = Digest::sha256(b"absent");
        assert!(matches!(
            load(&(store as Arc<dyn ObjectStore>), &digest).await,
            Err(BundleError::Missing)
        ));
    }

    #[test]
    fn a_missing_level_price_is_not_zero() {
        let level = Level::default();
        assert!(level.price.is_nan());
    }
}
