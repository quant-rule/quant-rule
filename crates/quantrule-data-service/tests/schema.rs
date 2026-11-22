//! The data service owns one schema and links no other service.

use std::path::{Path, PathBuf};

fn source(root: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(root).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            source(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

#[test]
fn the_catalog_sql_is_the_only_schema_named_by_the_repository() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(manifest.join("src/postgres.rs")).expect("postgres");
    assert!(text.contains("catalog."), "the repository names no catalog");
    for schema in ["runs.", "research.", "simulation.", "gateway."] {
        assert!(
            !text.contains(schema),
            "the catalog repository names {schema}"
        );
    }
}

#[test]
fn the_service_does_not_link_another_service_or_an_engine_it_must_not_run() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let text = std::fs::read_to_string(manifest.join("Cargo.toml")).expect("manifest");
    for forbidden in [
        "quantrule-server",
        "quantrule-model",
        "quantrule-backtest",
        "quantrule-gateway",
        "quantrule-evidence-service",
        "quantrule-simulation-service",
    ] {
        assert!(!text.contains(forbidden), "the service links {forbidden}");
    }
    assert!(text.contains("quantrule-data"));
    assert!(text.contains("quantrule-app"));
}

#[test]
fn the_service_never_reads_the_gateway_profile_table() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    source(&manifest.join("src"), &mut files);
    assert!(!files.is_empty(), "no sources found");
    for file in files {
        let relative = file
            .strip_prefix(&manifest)
            .expect("relative")
            .to_string_lossy()
            .replace('\\', "/");
        if relative == "src/migrations.rs" {
            // The migration test asserts the absence of the other schemas.
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("source");
        assert!(
            !text.contains("gateway.profiles") && !text.contains("catalog.profiles"),
            "{relative} names a profile table"
        );
    }
}

#[test]
fn the_data_migrations_create_the_catalog_schema_only() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut sql = Vec::new();
    for entry in std::fs::read_dir(manifest.join("migrations")).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|extension| extension == "sql") {
            sql.push(path);
        }
    }
    assert_eq!(sql.len(), 9, "the catalog list has nine migrations");
    for file in sql {
        let text = std::fs::read_to_string(&file).expect("migration");
        let name = file.file_name().expect("name").to_string_lossy();
        assert!(text.contains("catalog."), "{name} names no catalog table");
        for schema in ["runs.", "research.", "simulation.", "gateway."] {
            assert!(!text.contains(schema), "{name} names {schema}");
        }
    }
}
