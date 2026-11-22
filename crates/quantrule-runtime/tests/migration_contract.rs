//! The migration contract guard.
//!
//! Each database-owning service carries one compiled-in ledger. The checked-in
//! `deploy/postgres/migration-ledger.json` pins every version and every SQL
//! checksum, so a ledger that is reordered, a body that is edited after it was
//! released, or a service that loses its one-shot migration path fails the fast
//! gate without a live database.
//!
//! Compose ownership and the owner/app URL split are covered by
//! `tests/database_topology.rs`. This file covers the ledger and the
//! `--migrate-only` entry point, which is the part the four jobs rely on.

use std::path::{Path, PathBuf};

use serde::Deserialize;

const MANIFEST: &str = "deploy/postgres/migration-ledger.json";

/// One database, its owning service, and its ledger schema.
const DATABASES: [(&str, &str, &str); 4] = [
    ("qr_gateway", "gateway", "gateway"),
    ("qr_catalog", "data", "catalog"),
    ("qr_evidence", "evidence", "runs"),
    ("qr_simulation", "simulation", "simulation"),
];

/// The services that own no database and must not carry a migration job.
const STATELESS_SERVICES: [(&str, &str); 3] = [
    ("model", "crates/quantrule-model-service/src/main.rs"),
    ("backtest", "crates/quantrule-backtest-service/src/main.rs"),
    ("strategy", "crates/quantrule-strategy-service/src/main.rs"),
];

#[derive(Debug, Deserialize)]
struct Manifest {
    schema: String,
    ledgers: Vec<Ledger>,
}

#[derive(Debug, Deserialize)]
struct Ledger {
    database: String,
    service: String,
    ledger: String,
    entry: String,
    module: String,
    migrations: Vec<PinnedMigration>,
}

#[derive(Debug, Deserialize)]
struct PinnedMigration {
    version: i64,
    name: String,
    file: String,
    checksum: String,
}

struct SourceMigration {
    version: i64,
    name: String,
    file: PathBuf,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the runtime manifest has two ancestors")
        .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

fn manifest() -> Manifest {
    let text = read(MANIFEST);
    let manifest: Manifest =
        serde_json::from_str(&text).unwrap_or_else(|error| panic!("parse {MANIFEST}: {error}"));
    assert_eq!(
        manifest.schema, "quantrule.migration-ledger/v1",
        "the manifest schema must stay pinned"
    );
    manifest
}

/// Parse the `Migration { version, name, sql: include_str!() }` literals in one
/// service module. The module format is fixed by rustfmt and reviewed here, so
/// a ledger that stops matching the pinned manifest fails loudly.
fn source_migrations(source: &str, module: &str) -> Vec<SourceMigration> {
    let module_path = workspace_root().join(module);
    let directory = module_path
        .parent()
        .expect("the migration module has a parent directory");
    let mut migrations = Vec::new();
    let mut rest = source;
    while let Some(start) = rest.find("Migration {") {
        let block = &rest[start..];
        let end = block
            .find('}')
            .expect("a Migration literal ends with a brace");
        let literal = &block[..end];
        let version = number(literal, "version:");
        let name = quoted(literal, "name:");
        let include = quoted(literal, "sql:");
        let file = directory.join(include);
        migrations.push(SourceMigration {
            version,
            name: name.to_owned(),
            file: canonical(&file),
        });
        rest = &block[end + 1..];
    }
    migrations
}

fn number(literal: &str, key: &str) -> i64 {
    let (_, after) = literal
        .split_once(key)
        .unwrap_or_else(|| panic!("the literal has no {key}"));
    let digits: String = after
        .trim_start()
        .chars()
        .take_while(|character| character.is_ascii_digit() || *character == '-')
        .collect();
    digits
        .parse()
        .unwrap_or_else(|error| panic!("parse {key} {digits:?}: {error}"))
}

fn quoted<'a>(literal: &'a str, key: &str) -> &'a str {
    let (_, after) = literal
        .split_once(key)
        .unwrap_or_else(|| panic!("the literal has no {key}"));
    let start = after
        .find('"')
        .unwrap_or_else(|| panic!("the {key} value is not quoted"))
        + 1;
    let end = after[start..]
        .find('"')
        .unwrap_or_else(|| panic!("the {key} value has no closing quote"))
        + start;
    &after[start..end]
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path)
        .unwrap_or_else(|error| panic!("resolve {}: {error}", path.display()))
}

fn assert_strict_order(versions: impl Iterator<Item = i64>, what: &str) {
    let mut previous: Option<i64> = None;
    for version in versions {
        if let Some(prior) = previous {
            assert!(
                version > prior,
                "{what} is not strictly increasing: {version} follows {prior}"
            );
        }
        previous = Some(version);
    }
}

#[test]
fn every_database_has_one_pinned_ledger() {
    let manifest = manifest();
    assert_eq!(
        manifest.ledgers.len(),
        DATABASES.len(),
        "the manifest must pin exactly one ledger per database"
    );

    let mut expected: Vec<(&str, &str, &str)> = DATABASES.to_vec();
    expected.sort_unstable();
    let mut observed: Vec<(&str, &str, &str)> = manifest
        .ledgers
        .iter()
        .map(|ledger| {
            (
                ledger.database.as_str(),
                ledger.service.as_str(),
                ledger.ledger.as_str(),
            )
        })
        .collect();
    observed.sort_unstable();
    assert_eq!(observed, expected, "a database, service, or ledger moved");

    for ledger in &manifest.ledgers {
        assert!(
            workspace_root().join(&ledger.entry).is_file(),
            "{} has no entry point at {}",
            ledger.service,
            ledger.entry
        );
        assert!(
            workspace_root().join(&ledger.module).is_file(),
            "{} has no migration module at {}",
            ledger.service,
            ledger.module
        );
        assert!(
            !ledger.migrations.is_empty(),
            "{} has an empty ledger",
            ledger.service
        );
    }
}

#[test]
fn every_ledger_is_strictly_ordered_and_matches_its_source() {
    for ledger in manifest().ledgers {
        assert_strict_order(
            ledger.migrations.iter().map(|migration| migration.version),
            &format!("{} manifest", ledger.service),
        );

        let source = read(&ledger.module);
        let parsed = source_migrations(&source, &ledger.module);
        assert_strict_order(
            parsed.iter().map(|migration| migration.version),
            &format!("{} source", ledger.service),
        );
        assert_eq!(
            parsed.len(),
            ledger.migrations.len(),
            "{} migration count differs from the pinned manifest",
            ledger.service
        );

        for (source, pinned) in parsed.iter().zip(&ledger.migrations) {
            assert_eq!(
                source.version, pinned.version,
                "{} version differs from the pinned manifest",
                ledger.service
            );
            assert_eq!(
                source.name, pinned.name,
                "{} migration {} name differs",
                ledger.service, pinned.version
            );
            assert_eq!(
                source.file,
                canonical(&workspace_root().join(&pinned.file)),
                "{} migration {} file differs",
                ledger.service,
                pinned.version
            );
        }
    }
}

#[test]
fn every_pinned_checksum_matches_the_sql() {
    for ledger in manifest().ledgers {
        for migration in ledger.migrations {
            let sql = read(&migration.file);
            assert!(
                !sql.trim().is_empty(),
                "{} migration {} is empty",
                ledger.service,
                migration.version
            );
            assert_eq!(
                quantrule_runtime::migrations::checksum(&sql),
                migration.checksum,
                "{} migration {} ({}) changed after it was pinned",
                ledger.service,
                migration.version,
                migration.file
            );
        }
    }
}

#[test]
fn every_database_owning_service_has_a_migration_only_job() {
    for ledger in manifest().ledgers {
        let entry = read(&ledger.entry);
        for required in [
            "migrate_only_requested()",
            "migrate_owner_database(",
            "open_service_database(",
        ] {
            assert!(
                entry.contains(required),
                "{} entry point does not call {required}",
                ledger.service
            );
        }
        assert!(
            entry.contains(&format!("\"{}\"", ledger.ledger)),
            "{} entry point does not name its {} ledger",
            ledger.service,
            ledger.ledger
        );
        assert!(
            !entry.contains("apply_migrations("),
            "{} entry point must reach the runner only through the migration job",
            ledger.service
        );
    }
}

#[test]
fn stateless_services_have_no_migration_job() {
    for (service, entry) in STATELESS_SERVICES {
        let text = read(entry);
        for forbidden in ["migrate_only_requested", "migrate_owner_database"] {
            assert!(
                !text.contains(forbidden),
                "{service} owns no database and must not carry {forbidden}"
            );
        }
    }
}
