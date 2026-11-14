//! The worker source has no in-process coupled fast path.

use super::support::*;

#[test]
fn the_worker_has_no_in_process_backtest_fast_path() {
    let source = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/backtest/worker.rs"),
    )
    .expect("the worker source");
    assert!(
        !source.contains("AppService::run_backtest("),
        "the worker still calls the coupled AppService::run_backtest"
    );
    assert!(
        !source.contains("BacktestRequest"),
        "the worker still names the coupled request type"
    );
    assert!(source.contains("AppService::run_backtest_from_model"));
    assert!(source.contains(".model"));
    assert!(source.contains(".bundle("));
    assert!(
        !source.contains("pipeline::run_model"),
        "the worker still computes a model"
    );

    let staged = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../quantrule-app/src/stages/backtest_from_model.rs"),
    )
    .expect("the staged backtest source");
    assert!(
        !staged.contains("pipeline::run_model"),
        "the staged backtest still computes a model"
    );
    assert!(staged.contains("copy_bundle"));

    let bin = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"))
        .expect("the worker binary source");
    assert!(bin.contains("Service::Model"));
    assert!(bin.contains("QR_MODEL_URL") || bin.contains("service_config_error(Service::Model"));
}
