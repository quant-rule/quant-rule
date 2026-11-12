//! Desktop command tests.

    use std::path::Path;
    use std::sync::{Mutex, MutexGuard};

    use quantrule_app::memory::{MetricRecord, MetricValue};

    use super::scope::{SCOPE, Scope};
    use super::views::metric_view;
    use super::{
        check_run, dataset_summary, memory_claim_add, memory_claim_delete, memory_claims,
        memory_export, memory_metric_delete, memory_metric_set, memory_metrics, memory_note_add,
        memory_note_delete, memory_note_update, memory_notes_for_run, memory_runs, memory_status,
        memory_workspaces, open_workspace, record_run, run_model,
    };

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn test_guard() -> MutexGuard<'static, ()> {
        let guard = TEST_LOCK.lock().expect("the test lock is available");
        *SCOPE.lock().expect("the scope is available") = Scope::default();
        guard
    }

    fn repository_root() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../..")
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn the_repository_workspace_lists_its_runs() {
        let _guard = test_guard();
        let view = open_workspace(repository_root()).expect("the repository opens");
        assert!(!view.name.is_empty());
        assert!(view
            .runs
            .iter()
            .any(|path| path.ends_with("config/examples/btcusd-2024/run.json")));
    }

    fn fixture_run() -> String {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../crates/quantrule-shell/tests/fixtures/config-run/run.json")
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn out_of_scope_paths_are_rejected() {
        let _guard = test_guard();
        open_workspace(repository_root()).expect("the repository opens");
        let outside = std::env::temp_dir().join("quantrule-outside.csv");
        assert!(
            dataset_summary(outside.to_string_lossy().into_owned()).is_err(),
            "a dataset outside the workspace is rejected"
        );

        let directory =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target/desktop-tests/scope");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the test directory is created");
        let db = directory.join("research.db");
        let db = db.to_string_lossy().into_owned();
        memory_status(db.clone()).expect("the memory opens");
        assert!(
            memory_export(
                db,
                std::env::temp_dir()
                    .join("quantrule-outside-export.json")
                    .to_string_lossy()
                    .into_owned(),
            )
            .is_err(),
            "an export outside the workspace and memory directory is rejected"
        );
    }

    #[test]
    fn the_run_commands_match_the_service() {
        let _guard = test_guard();
        open_workspace(repository_root()).expect("the repository opens");
        let summary = check_run(fixture_run()).expect("the fixture validates");
        assert_eq!(summary.name, "fixture");
        assert_eq!(summary.instrument, "EURUSD");
        assert_eq!(summary.data_version, "test:v1");
        assert_eq!(summary.data_timeframes, ["1m", "5m", "15m", "1h"]);
        assert_eq!(summary.model_timeframes, ["1m", "5m", "15m", "1h"]);
        assert_eq!(summary.execution_fill_timeframe, "5m");
        let model = run_model(fixture_run()).expect("the fixture runs");
        assert!(!model.timeframes.is_empty());
        assert!(model.events > 0);
    }

    #[test]
    fn the_memory_commands_record_and_export() {
        let _guard = test_guard();
        open_workspace(repository_root()).expect("the repository opens");
        let directory =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../target/desktop-tests/memory");
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).expect("the test directory is created");
        let db = directory.join("research.db");
        let db = db.to_string_lossy().into_owned();
        let status = memory_status(db.clone()).expect("the memory opens");
        assert_eq!(status.schema, "quantrule.research-memory/v1");
        let recorded = record_run(
            db.clone(),
            fixture_run(),
            "run-1".to_owned(),
            "complete".to_owned(),
            1_763_000_000,
            Some(1_763_000_060),
            None,
        )
        .expect("the run is recorded");
        assert!(recorded.run_id > 0);
        let workspaces = memory_workspaces(db.clone()).expect("the workspaces are read");
        assert_eq!(workspaces.len(), 1);
        let runs = memory_runs(db.clone(), workspaces[0].id).expect("the runs are read");
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].run_key, "run-1");
        assert!(
            memory_notes_for_run(db.clone(), runs[0].id)
                .expect("the run notes are read")
                .is_empty()
        );
        let run_id = runs[0].id;
        assert!(memory_metric_set(
            db.clone(),
            run_id,
            "sharpe".to_owned(),
            "real".to_owned(),
            Some(1.5),
            None
        )
        .expect("the metric is written")
            > 0);
        assert_eq!(
            memory_metrics(db.clone(), run_id)
                .expect("the metrics are read")
                .len(),
            1
        );
        let claim = memory_claim_add(
            db.clone(),
            run_id,
            "Behaviour Claim".to_owned(),
            "the run is deterministic".to_owned(),
            "accepted".to_owned(),
            1,
        )
        .expect("the claim is written");
        assert_eq!(
            memory_claims(db.clone(), run_id)
                .expect("the claims are read")
                .len(),
            1
        );
        let note = memory_note_add(
            db.clone(),
            workspaces[0].id,
            Some(run_id),
            "first review".to_owned(),
            1,
            1,
        )
        .expect("the note is written");
        assert_eq!(
            memory_notes_for_run(db.clone(), run_id)
                .expect("the run notes are read")
                .len(),
            1
        );
        assert!(memory_note_update(db.clone(), note, "updated".to_owned(), 2)
            .expect("the note is updated"));
        assert!(memory_note_delete(db.clone(), note).expect("the note is deleted"));
        assert!(memory_claim_delete(db.clone(), claim).expect("the claim is deleted"));
        assert!(memory_metric_delete(db.clone(), run_id, "sharpe".to_owned())
            .expect("the metric is deleted"));
        let out = directory.join("export.json");
        memory_export(db, out.to_string_lossy().into_owned()).expect("the export is written");
        assert!(out.is_file());
    }

    #[test]
    fn metric_views_preserve_null_and_integer() {
        let null = metric_view(MetricRecord {
            id: 1,
            run_id: 1,
            name: "sharpe".to_owned(),
            value: MetricValue::Null,
        });
        assert_eq!(null.kind, "null");
        assert!(null.value.is_null());

        let integer = metric_view(MetricRecord {
            id: 2,
            run_id: 1,
            name: "trades".to_owned(),
            value: MetricValue::Integer(42),
        });
        assert_eq!(integer.kind, "integer");
        assert_eq!(integer.value, serde_json::json!(42));
    }
