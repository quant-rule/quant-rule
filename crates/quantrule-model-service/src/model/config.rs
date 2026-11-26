//! Materialize a catalogued config bundle into the model inputs.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use quantrule_app::config::DataConfig;
use quantrule_app::config::RunConfig;
use quantrule_app::pack::MemoryReferences;
use quantrule_data::Timeframe;
use quantrule_model::input::MarketStateConfig;

use crate::model::ModelError;

static NEXT: AtomicU64 = AtomicU64::new(1);

/// The parts of a run config the model needs.
#[derive(Debug, Clone)]
pub struct ModelConfig {
    /// The declared data ladder, session, and bounds.
    pub data: DataConfig,
    /// The model engine configuration.
    pub model: MarketStateConfig,
    /// The model ladder, or the full data ladder.
    pub ladder: Vec<Timeframe>,
}

impl ModelConfig {
    /// Take the model inputs from a validated run config.
    #[must_use]
    pub fn from_run(config: &RunConfig) -> Self {
        let ladder = config
            .model_ladder
            .clone()
            .unwrap_or_else(|| config.data.available_timeframes.clone());
        Self {
            data: config.data.clone(),
            model: config.model,
            ladder,
        }
    }
}

/// Write a config bundle to a private directory and load the run config.
///
/// A v5 bundle's pack and signal references are supplied by the caller, which
/// resolves them once from the data service. The model stage consumes only the
/// data and model files, but the shared loader validates the whole document.
/// The directory is removed before this returns. The returned config carries
/// parsed values and file names, never a live path into the staging directory.
///
/// # Errors
///
/// Returns [`ModelError::Config`] when a file cannot be written, the run config
/// does not validate, or the bundle is missing `run.json`.
pub fn load_run_config(
    files: &BTreeMap<String, String>,
    references: &MemoryReferences,
    staging_root: &Path,
) -> Result<RunConfig, ModelError> {
    if !files.contains_key("run.json") {
        return Err(ModelError::Config(
            "the config bundle has no run.json".to_owned(),
        ));
    }
    let root = staging_root.join(format!(
        "quantrule-model-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _staging = Staging(root.clone());
    std::fs::create_dir_all(&root)
        .map_err(|error| ModelError::Config(format!("create the staging directory: {error}")))?;
    for (relative, body) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| ModelError::Config(format!("create {relative}: {error}")))?;
        }
        std::fs::write(&path, body)
            .map_err(|error| ModelError::Config(format!("write {relative}: {error}")))?;
    }
    RunConfig::load_with(&root.join("run.json"), references)
        .map_err(|error| ModelError::Config(error.to_string()))
}

struct Staging(PathBuf);

impl Drop for Staging {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_api::{Digest, ReferenceKind};
    use quantrule_app::pack::canonical_json;
    use serde_json::{Value, json};

    fn pack(kind: &str, payload: Value) -> Vec<u8> {
        canonical_json(&json!({
            "schema": "quantrule.pack/v1",
            "kind": kind,
            "version": "1.0.0",
            "name": format!("test-{kind}"),
            "owner": "operator",
            "payload": payload
        }))
        .into_bytes()
    }

    fn strategy() -> Value {
        json!({
            "mode": "fold",
            "engine": "event_composite",
            "execution_timeframe": "5m",
            "context_timeframes": ["15m"],
            "universe": ["BTCUSD"],
            "event_window_bars": 1,
            "time_stop_bars": 0,
            "giveback_atr": null,
            "giveback_arm_atr": 0.0,
            "require_contiguous": true,
            "risk": {"stop_atr_multiple": 2.0, "reward_to_risk": 1.5},
            "entry": [
                {"kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 7,
                 "source": 5, "direction": 0, "contrarian": true}
            ],
            "exit": []
        })
    }

    fn execution() -> Value {
        json!({
            "fill_timeframe": "5m",
            "order_type": "market_next_open",
            "tie_break": "stop_first",
            "fee_rate": 0.0,
            "slippage_bps": 2.0,
            "commission": {"rate": 0.001, "base": "equity", "per": "operation"},
            "funding": {"rate": 0.0001, "interval_hours": 8},
            "zero_volume": "reject",
            "end_of_data": "leave_open"
        })
    }

    fn portfolio() -> Value {
        json!({
            "sizing": {"mode": "equity_fraction", "fraction": 0.05, "quantity_step": 0.0001},
            "risk": {"per_trade_risk": 0.01, "max_exposure": 1.0, "max_open_positions": 1},
            "drawdown": {"limit": 0.15, "action": "halt"},
            "rebalance": "none",
            "constraints": []
        })
    }

    fn files() -> (BTreeMap<String, String>, MemoryReferences) {
        let mut references = MemoryReferences::new();
        let strategy = references.insert(ReferenceKind::StrategyPack, pack("strategy", strategy()));
        let execution = references.insert(
            ReferenceKind::ExecutionModel,
            pack("execution", execution()),
        );
        let portfolio = references.insert(
            ReferenceKind::PortfolioPolicy,
            pack("portfolio", portfolio()),
        );
        let run = json!({
            "schema": "quantrule.run/v5",
            "name": "model-v5",
            "model_id": "M1.0",
            "data": "data.json",
            "model": "model.json",
            "strategy_pack": strategy.to_value(),
            "portfolio_policy": portfolio.to_value(),
            "execution_model": execution.to_value(),
            "account": {"initial_cash": 10000.0, "currency": "USD"},
            "output_root": "out"
        });
        let files = BTreeMap::from([
            ("run.json".to_owned(), run.to_string()),
            (
                "data.json".to_owned(),
                include_str!("../../../quantrule-app/tests/fixtures/run-v5/data.json").to_owned(),
            ),
            (
                "model.json".to_owned(),
                include_str!("../../../quantrule-app/tests/fixtures/run-v5/model.json").to_owned(),
            ),
        ]);
        (files, references)
    }

    #[test]
    fn a_v5_bundle_loads_the_model_inputs_without_a_strategy_file() {
        let (files, references) = files();
        let root = std::env::temp_dir().join(format!(
            "quantrule-model-config-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let config = load_run_config(&files, &references, &root).expect("the v5 config");
        assert_eq!(config.schema_version, "quantrule.run/v5");
        assert!(config.strategy.is_some());
        assert_eq!(config.data.instrument, "BTCUSD");
        let model = ModelConfig::from_run(&config);
        assert_eq!(model.ladder, config.data.available_timeframes);
        assert_eq!(model.data.data_version, config.data.data_version);
    }

    #[test]
    fn a_v5_bundle_without_its_pack_bytes_is_refused() {
        let (files, _) = files();
        let empty = MemoryReferences::new();
        let root = std::env::temp_dir().join(format!(
            "quantrule-model-config-missing-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let error = load_run_config(&files, &empty, &root).expect_err("the pack is missing");
        assert!(matches!(error, ModelError::Config(_)), "{error:?}");
    }

    #[test]
    fn the_reference_identity_is_the_stored_bytes() {
        let (_, references) = files();
        let strategy = references
            .get(
                ReferenceKind::StrategyPack,
                &quantrule_app::pack::ContentReference {
                    id: Digest::sha256(&pack("strategy", strategy())),
                    hash: Digest::sha256(&pack("strategy", strategy())),
                },
            )
            .expect("the strategy bytes");
        assert!(!strategy.is_empty());
    }
}
