//! Tests for the local research memory.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde_json::json;

use super::{
    EXPORT_SCHEMA, MetricValue, NewClaim, NewConfig, NewDataset, NewNote, NewRun, NewWorkspace,
    ResearchMemory, backup, export_bytes, export_json, export_to_file, import_json, restore,
};

const AT: i64 = 1_763_000_000;

struct Fixture {
    workspace: i64,
    dataset: i64,
    config: i64,
    run: i64,
}

fn test_root(name: &str) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/memory-tests")
        .join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).expect("the test directory is created");
    root
}

fn foreign_database(path: &Path) {
    let connection = Connection::open(path).expect("the foreign database opens");
    connection
        .execute_batch(
            "CREATE TABLE unrelated (x INTEGER NOT NULL);
             INSERT INTO unrelated (x) VALUES (7);",
        )
        .expect("the foreign schema is written");
}

#[test]
fn a_foreign_database_is_refused_and_left_unchanged() {
    let path = test_root("foreign").join("foreign.db");
    foreign_database(&path);

    assert!(ResearchMemory::open(&path).is_err());

    let connection = Connection::open(&path).expect("the foreign database reopens");
    let memory_tables: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
              WHERE type = 'table' AND name IN ('meta', 'workspaces')",
            [],
            |row| row.get(0),
        )
        .expect("the table count is read");
    let unrelated: i64 = connection
        .query_row("SELECT x FROM unrelated", [], |row| row.get(0))
        .expect("the foreign row is read");
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("the version is read");
    assert_eq!(memory_tables, 0);
    assert_eq!(unrelated, 7);
    assert_eq!(version, 0);
}

#[test]
fn restore_refuses_a_foreign_database_without_creating_the_target() {
    let root = test_root("foreign-restore");
    let backup = root.join("foreign.db");
    let target = root.join("restored.db");
    foreign_database(&backup);

    assert!(restore(&backup, &target).is_err());
    assert!(!target.exists());
}

fn populated() -> (ResearchMemory, Fixture) {
    let memory = ResearchMemory::open_in_memory().expect("the memory opens");
    let workspace = memory
        .upsert_workspace(NewWorkspace {
            root: "/workspace/one",
            name: "One",
            created_at: AT,
        })
        .expect("the workspace is written");
    let dataset = memory
        .upsert_dataset(NewDataset {
            workspace_id: workspace,
            name: "BTCUSD",
            path: "/workspace/one/data/BTCUSD_M1.csv",
            data_version: "btc-v1",
            instrument: "BTCUSD",
            candles: 1_440,
            first_open_time: Some(AT),
            last_open_time: Some(AT + 86_340),
            created_at: AT,
        })
        .expect("the dataset is written");
    let config = memory
        .upsert_config(NewConfig {
            workspace_id: workspace,
            name: "btcusd-2024",
            path: "/workspace/one/config/examples/btcusd-2024/run.json",
            config_hash: "0123456789abcdef",
            schema_version: "quantrule.run/v3",
            created_at: AT,
        })
        .expect("the config is written");
    let run = memory
        .insert_run(NewRun {
            workspace_id: workspace,
            config_id: Some(config),
            run_key: "run-1",
            data_version: "btc-v1",
            config_hash: "0123456789abcdef",
            status: "complete",
            started_at: AT,
            finished_at: Some(AT + 60),
            summary_json: "{\"name\":\"btcusd-2024\"}",
        })
        .expect("the run is written");
    memory
        .upsert_metric(run, "profit_factor", &MetricValue::Real(1.5))
        .expect("the real metric is written");
    memory
        .upsert_metric(run, "trades", &MetricValue::Integer(42))
        .expect("the integer metric is written");
    memory
        .upsert_metric(run, "regime", &MetricValue::Text(String::from("trend")))
        .expect("the text metric is written");
    memory
        .upsert_metric(run, "sharpe", &MetricValue::Null)
        .expect("the null metric is written");
    memory
        .insert_claim(NewClaim {
            run_id: run,
            claim_type: "Behaviour Claim",
            statement: "the run is deterministic",
            status: "accepted",
            created_at: AT,
        })
        .expect("the claim is written");
    memory
        .insert_note(NewNote {
            workspace_id: workspace,
            run_id: Some(run),
            body: "first note",
            created_at: AT,
            updated_at: AT,
        })
        .expect("the note is written");
    (
        memory,
        Fixture {
            workspace,
            dataset,
            config,
            run,
        },
    )
}

#[test]
fn a_fresh_memory_has_the_v1_schema() {
    let memory = ResearchMemory::open_in_memory().expect("the memory opens");
    assert_eq!(
        memory.schema().expect("the schema is readable"),
        super::SCHEMA
    );
    assert_eq!(memory.version().expect("the version is readable"), 1);
    assert!(memory.is_empty().expect("the memory is empty"));
    memory.verify_integrity().expect("the memory is intact");
}

#[test]
fn a_workspace_upsert_is_idempotent() {
    let memory = ResearchMemory::open_in_memory().expect("the memory opens");
    let first = memory
        .upsert_workspace(NewWorkspace {
            root: "/workspace/one",
            name: "One",
            created_at: AT,
        })
        .expect("the first write succeeds");
    let second = memory
        .upsert_workspace(NewWorkspace {
            root: "/workspace/one",
            name: "Renamed",
            created_at: AT + 10,
        })
        .expect("the second write succeeds");
    assert_eq!(first, second);
    let record = memory.workspace(first).expect("the query succeeds");
    let record = record.expect("the workspace exists");
    assert_eq!(record.name, "Renamed");
    assert_eq!(record.created_at, AT);
}

#[test]
fn every_record_round_trips_in_id_order() {
    let (memory, fixture) = populated();
    assert_eq!(memory.workspaces().expect("workspaces are read").len(), 1);
    let datasets = memory
        .datasets(fixture.workspace)
        .expect("datasets are read");
    assert_eq!(datasets.len(), 1);
    assert_eq!(datasets[0].id, fixture.dataset);
    assert_eq!(datasets[0].candles, 1_440);
    let configs = memory.configs(fixture.workspace).expect("configs are read");
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].id, fixture.config);
    let runs = memory.runs(fixture.workspace).expect("runs are read");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].id, fixture.run);
    assert_eq!(runs[0].config_id, Some(fixture.config));

    let metrics = memory.metrics(fixture.run).expect("metrics are read");
    let names: Vec<&str> = metrics.iter().map(|metric| metric.name.as_str()).collect();
    assert_eq!(names, ["profit_factor", "trades", "regime", "sharpe"]);
    assert_eq!(metrics[0].value, MetricValue::Real(1.5));
    assert_eq!(metrics[1].value, MetricValue::Integer(42));
    assert_eq!(metrics[2].value, MetricValue::Text(String::from("trend")));
    assert_eq!(metrics[3].value, MetricValue::Null);

    let claims = memory.claims(fixture.run).expect("claims are read");
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].claim_type, "Behaviour Claim");
    let notes = memory.notes(fixture.workspace).expect("notes are read");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].body, "first note");
}

#[test]
fn a_null_metric_is_not_zero() {
    let memory = ResearchMemory::open_in_memory().expect("the memory opens");
    let workspace = memory
        .upsert_workspace(NewWorkspace {
            root: "/workspace/one",
            name: "One",
            created_at: AT,
        })
        .expect("the workspace is written");
    let run = memory
        .insert_run(NewRun {
            workspace_id: workspace,
            config_id: None,
            run_key: "run-1",
            data_version: "v1",
            config_hash: "hash",
            status: "complete",
            started_at: AT,
            finished_at: None,
            summary_json: "{}",
        })
        .expect("the run is written");
    memory
        .upsert_metric(run, "sharpe", &MetricValue::Null)
        .expect("the null metric is written");
    memory
        .upsert_metric(run, "drawdown", &MetricValue::Real(0.0))
        .expect("the zero metric is written");
    let metrics = memory.metrics(run).expect("the metrics are read");
    assert_eq!(metrics[0].value, MetricValue::Null);
    assert_eq!(metrics[1].value, MetricValue::Real(0.0));
}

#[test]
fn an_integer_metric_keeps_full_precision() {
    let memory = ResearchMemory::open_in_memory().expect("the memory opens");
    let workspace = memory
        .upsert_workspace(NewWorkspace {
            root: "/workspace/one",
            name: "One",
            created_at: AT,
        })
        .expect("the workspace is written");
    let run = memory
        .insert_run(NewRun {
            workspace_id: workspace,
            config_id: None,
            run_key: "run-1",
            data_version: "v1",
            config_hash: "hash",
            status: "complete",
            started_at: AT,
            finished_at: None,
            summary_json: "{}",
        })
        .expect("the run is written");
    memory
        .upsert_metric(run, "large", &MetricValue::Integer(i64::MAX))
        .expect("the metric is written");
    assert_eq!(
        memory.metrics(run).expect("the metrics are read")[0].value,
        MetricValue::Integer(i64::MAX)
    );
}

#[test]
fn orphan_records_are_rejected() {
    let memory = ResearchMemory::open_in_memory().expect("the memory opens");
    assert!(
        memory
            .upsert_dataset(NewDataset {
                workspace_id: 999,
                name: "BTCUSD",
                path: "x",
                data_version: "v1",
                instrument: "BTCUSD",
                candles: 1,
                first_open_time: None,
                last_open_time: None,
                created_at: AT,
            })
            .is_err()
    );
    assert!(
        memory
            .insert_claim(NewClaim {
                run_id: 999,
                claim_type: "Behaviour Claim",
                statement: "x",
                status: "accepted",
                created_at: AT,
            })
            .is_err()
    );
}

#[test]
fn a_run_config_must_belong_to_the_run_workspace() {
    let memory = ResearchMemory::open_in_memory().expect("the memory opens");
    let one = memory
        .upsert_workspace(NewWorkspace {
            root: "/one",
            name: "One",
            created_at: AT,
        })
        .expect("the first workspace is written");
    let two = memory
        .upsert_workspace(NewWorkspace {
            root: "/two",
            name: "Two",
            created_at: AT,
        })
        .expect("the second workspace is written");
    let config = memory
        .upsert_config(NewConfig {
            workspace_id: two,
            name: "two",
            path: "/two/run.json",
            config_hash: "hash",
            schema_version: "quantrule.run/v3",
            created_at: AT,
        })
        .expect("the config is written");
    assert!(
        memory
            .insert_run(NewRun {
                workspace_id: one,
                config_id: Some(config),
                run_key: "run-1",
                data_version: "v1",
                config_hash: "hash",
                status: "complete",
                started_at: AT,
                finished_at: None,
                summary_json: "{}",
            })
            .is_err()
    );
}

#[test]
fn notes_are_scoped_to_their_run() {
    let (memory, fixture) = populated();
    let other = memory
        .insert_run(NewRun {
            workspace_id: fixture.workspace,
            config_id: None,
            run_key: "run-2",
            data_version: "btc-v1",
            config_hash: "0123456789abcdef",
            status: "complete",
            started_at: AT,
            finished_at: None,
            summary_json: "{}",
        })
        .expect("the second run is written");
    memory
        .insert_note(NewNote {
            workspace_id: fixture.workspace,
            run_id: Some(other),
            body: "second note",
            created_at: AT,
            updated_at: AT,
        })
        .expect("the second note is written");

    let first = memory.notes_for_run(fixture.run).expect("the first notes");
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].body, "first note");
    let second = memory.notes_for_run(other).expect("the second notes");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].body, "second note");
}

#[test]
fn deleting_a_run_cascades_and_clears_note_links() {
    let (memory, fixture) = populated();
    assert!(memory.delete_run(fixture.run).expect("the run is deleted"));
    assert!(
        memory
            .metrics(fixture.run)
            .expect("metrics are read")
            .is_empty()
    );
    assert!(
        memory
            .claims(fixture.run)
            .expect("claims are read")
            .is_empty()
    );
    let notes = memory.notes(fixture.workspace).expect("notes are read");
    assert_eq!(notes.len(), 1);
    assert_eq!(notes[0].run_id, None);
}

#[test]
fn deleting_a_workspace_cascades() {
    let (memory, fixture) = populated();
    assert!(
        memory
            .delete_workspace(fixture.workspace)
            .expect("the workspace is deleted")
    );
    assert!(memory.workspaces().expect("workspaces are read").is_empty());
    assert!(
        memory
            .datasets(fixture.workspace)
            .expect("datasets are read")
            .is_empty()
    );
    assert!(
        memory
            .notes(fixture.workspace)
            .expect("notes are read")
            .is_empty()
    );
}

#[test]
fn ids_are_not_reused_after_delete() {
    let memory = ResearchMemory::open_in_memory().expect("the memory opens");
    let first = memory
        .upsert_workspace(NewWorkspace {
            root: "/one",
            name: "One",
            created_at: AT,
        })
        .expect("the first workspace is written");
    memory
        .delete_workspace(first)
        .expect("the first workspace is deleted");
    let second = memory
        .upsert_workspace(NewWorkspace {
            root: "/two",
            name: "Two",
            created_at: AT,
        })
        .expect("the second workspace is written");
    assert!(second > first);
}

#[test]
fn update_note_rejects_updated_at_before_created_at() {
    let memory = ResearchMemory::open_in_memory().expect("the memory opens");
    let workspace = memory
        .upsert_workspace(NewWorkspace {
            root: "/one",
            name: "One",
            created_at: AT,
        })
        .expect("the workspace is written");
    let note = memory
        .insert_note(NewNote {
            workspace_id: workspace,
            run_id: None,
            body: "first",
            created_at: AT,
            updated_at: AT,
        })
        .expect("the note is written");
    assert!(
        memory
            .update_note(note, "second", AT - 1)
            .expect_err("the update is rejected")
            .to_string()
            .contains("updated_at")
    );
    let stored = memory.notes(workspace).expect("the notes are read");
    assert_eq!(stored[0].body, "first");
    assert_eq!(stored[0].updated_at, AT);
    assert!(
        memory
            .update_note(note, "second", AT + 1)
            .expect("the update is accepted")
    );
    assert_eq!(
        memory.notes(workspace).expect("the notes are read")[0].body,
        "second"
    );
}

#[test]
fn an_export_is_byte_deterministic() {
    let (memory, _) = populated();
    let first = export_bytes(&memory).expect("the first export is written");
    let second = export_bytes(&memory).expect("the second export is written");
    assert_eq!(first, second);
    let value = export_json(&memory).expect("the export is built");
    assert_eq!(value["schema"], EXPORT_SCHEMA);
    assert_eq!(value["workspaces"].as_array().expect("workspaces").len(), 1);
    assert_eq!(value["metrics"].as_array().expect("metrics").len(), 4);
}

#[test]
fn export_to_file_replaces_the_target_on_success_and_leaves_no_temp() {
    let (memory, _) = populated();
    let root = test_root("export-atomic");
    let target = root.join("export.json");
    fs::write(&target, b"previous").expect("the previous file is written");

    export_to_file(&memory, &target).expect("the export is written");

    let text = fs::read_to_string(&target).expect("the export is readable");
    assert!(text.contains(EXPORT_SCHEMA));
    let leftovers = fs::read_dir(&root)
        .expect("the directory is readable")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".export.json.")
        })
        .count();
    assert_eq!(leftovers, 0, "the temporary file is removed");
}

#[cfg(unix)]
#[test]
fn a_failed_export_leaves_the_previous_file_unchanged() {
    use std::os::unix::fs::PermissionsExt;

    let (memory, _) = populated();
    let root = test_root("export-failure");
    let target = root.join("export.json");
    fs::write(&target, b"previous").expect("the previous file is written");
    let mut permissions = fs::metadata(&root)
        .expect("the directory metadata is read")
        .permissions();
    permissions.set_mode(0o555);
    fs::set_permissions(&root, permissions).expect("the directory is made read-only");

    let result = export_to_file(&memory, &target);

    let mut permissions = fs::metadata(&root)
        .expect("the directory metadata is re-read")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&root, permissions).expect("the directory is made writable again");
    assert!(
        result.is_err(),
        "the atomic export fails without a writable directory"
    );
    assert_eq!(
        fs::read(&target).expect("the previous file is readable"),
        b"previous",
        "the previous export is unchanged"
    );
}

#[test]
fn export_and_import_round_trip() {
    let (memory, _) = populated();
    let exported = export_json(&memory).expect("the export is built");
    let mut target = ResearchMemory::open_in_memory().expect("the target opens");
    import_json(&mut target, &exported).expect("the export imports");
    target.verify_integrity().expect("the target is intact");
    assert_eq!(
        export_bytes(&memory).expect("the source export"),
        export_bytes(&target).expect("the target export")
    );
    let next = target
        .upsert_workspace(NewWorkspace {
            root: "/workspace/two",
            name: "Two",
            created_at: AT,
        })
        .expect("a new record is written after import");
    assert!(next > 1, "the import must advance the ID sequence");
}

#[test]
fn import_refuses_a_non_empty_target() {
    let (source, _) = populated();
    let exported = export_json(&source).expect("the export is built");
    let mut target = ResearchMemory::open_in_memory().expect("the target opens");
    target
        .upsert_workspace(NewWorkspace {
            root: "/existing",
            name: "Existing",
            created_at: AT,
        })
        .expect("the existing workspace is written");
    let before = export_bytes(&target).expect("the target export");
    assert!(import_json(&mut target, &exported).is_err());
    assert_eq!(
        export_bytes(&target).expect("the target export"),
        before,
        "a refused import must not change the target"
    );
}

#[test]
fn import_rejects_a_bad_document_without_partial_writes() {
    let mut target = ResearchMemory::open_in_memory().expect("the target opens");
    let bad = json!({
        "schema": EXPORT_SCHEMA,
        "workspaces": [{"id": 1, "root": "/one", "name": "One"}],
        "datasets": [],
        "configs": [],
        "runs": [],
        "metrics": [],
        "claims": [],
        "notes": [],
    });
    assert!(import_json(&mut target, &bad).is_err());
    assert!(target.is_empty().expect("the target is empty"));
}

fn import_is_rejected(document: &serde_json::Value) {
    let mut target = ResearchMemory::open_in_memory().expect("the target opens");
    assert!(import_json(&mut target, document).is_err());
    assert!(target.is_empty().expect("the target is empty"));
}

#[test]
fn import_rejects_each_store_invariant_violation() {
    let (memory, _) = populated();
    let exported = export_json(&memory).expect("the export is built");

    let mut document = exported.clone();
    document["metrics"][1]["text_value"] = json!("not-an-integer");
    import_is_rejected(&document);

    let mut document = exported.clone();
    document["metrics"][3]["value"] = json!(1.25);
    import_is_rejected(&document);

    let mut document = exported.clone();
    document["metrics"][0]["id"] = json!(0);
    import_is_rejected(&document);

    let mut document = exported.clone();
    document["runs"][0]["finished_at"] = json!(AT - 1);
    import_is_rejected(&document);

    let mut document = exported.clone();
    document["datasets"][0]["last_open_time"] = json!(AT - 1);
    import_is_rejected(&document);

    let mut document = exported.clone();
    document["notes"][0]["updated_at"] = json!(AT - 1);
    import_is_rejected(&document);
}

#[test]
fn a_file_memory_survives_reopen() {
    let root = test_root("reopen");
    let path = root.join("memory.db");
    {
        let memory = ResearchMemory::open(&path).expect("the memory opens");
        memory
            .upsert_workspace(NewWorkspace {
                root: "/one",
                name: "One",
                created_at: AT,
            })
            .expect("the workspace is written");
    }
    let memory = ResearchMemory::open(&path).expect("the memory reopens");
    assert_eq!(memory.workspaces().expect("workspaces are read").len(), 1);
    assert_eq!(memory.version().expect("the version is readable"), 1);
}

#[test]
fn an_unsupported_schema_is_refused() {
    let root = test_root("schema");
    let path = root.join("memory.db");
    {
        let memory = ResearchMemory::open(&path).expect("the memory opens");
        memory
            .connection()
            .execute(
                "UPDATE meta SET value = 'quantrule.research-memory/v2' WHERE key = 'schema'",
                [],
            )
            .expect("the schema is tampered");
    }
    assert!(ResearchMemory::open(&path).is_err());
}

#[test]
fn backup_and_restore_round_trip() {
    let root = test_root("backup");
    let source_path = root.join("source.db");
    let backup_path = root.join("backup.db");
    let restored_path = root.join("restored.db");
    let (memory, _) = {
        let memory = ResearchMemory::open(&source_path).expect("the source opens");
        let workspace = memory
            .upsert_workspace(NewWorkspace {
                root: "/one",
                name: "One",
                created_at: AT,
            })
            .expect("the workspace is written");
        memory
            .upsert_dataset(NewDataset {
                workspace_id: workspace,
                name: "BTCUSD",
                path: "x",
                data_version: "v1",
                instrument: "BTCUSD",
                candles: 1,
                first_open_time: Some(AT),
                last_open_time: Some(AT),
                created_at: AT,
            })
            .expect("the dataset is written");
        (memory, ())
    };
    backup(&memory, &backup_path).expect("the backup is written");
    let restored = restore(&backup_path, &restored_path).expect("the backup restores");
    assert_eq!(
        export_bytes(&memory).expect("the source export"),
        export_bytes(&restored).expect("the restored export")
    );
    restored
        .verify_integrity()
        .expect("the restored memory is intact");
}

#[test]
fn backup_and_restore_targets_must_not_exist() {
    let root = test_root("existing");
    let path = root.join("memory.db");
    let memory = ResearchMemory::open(&path).expect("the memory opens");
    assert!(backup(&memory, &path).is_err());
    assert!(restore(&path, &path).is_err());
}

#[test]
fn a_run_summary_must_be_json() {
    let memory = ResearchMemory::open_in_memory().expect("the memory opens");
    let workspace = memory
        .upsert_workspace(NewWorkspace {
            root: "/one",
            name: "One",
            created_at: AT,
        })
        .expect("the workspace is written");
    assert!(
        memory
            .insert_run(NewRun {
                workspace_id: workspace,
                config_id: None,
                run_key: "run-1",
                data_version: "v1",
                config_hash: "hash",
                status: "complete",
                started_at: AT,
                finished_at: None,
                summary_json: "not json",
            })
            .is_err()
    );
}
