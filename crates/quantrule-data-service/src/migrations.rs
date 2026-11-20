//! The catalog migration lists.
//!
//! The runner lives in `quantrule-runtime`; these lists belong to the data
//! service because it owns the `catalog` schema. A SQL file is inert until it is
//! listed here, so a new migration is not applied by copying a file into the
//! directory. Versions are local to the `catalog` ledger.
//!
//! The Postgres and SQLite histories are separate on purpose. The Postgres list
//! replays the released history (`0001`-`0005`); the SQLite baseline is the
//! final logical shape at version 1, because a desktop database has no earlier
//! rows to migrate. Both write the same `catalog.datasets`,
//! `catalog.configs`, and `catalog.config_bundles` relations.

use quantrule_runtime::migrations::Migration;

/// The migrations this service carries, in version order.
#[must_use]
pub fn available() -> Vec<Migration> {
    let mut migrations = vec![
        Migration {
            version: 1,
            name: "catalog",
            sql: include_str!("../migrations/0001_catalog.sql"),
        },
        Migration {
            version: 2,
            name: "catalog-profile",
            sql: include_str!("../migrations/0002_catalog_profile.sql"),
        },
        Migration {
            version: 3,
            name: "dataset-quality",
            sql: include_str!("../migrations/0003_dataset_quality.sql"),
        },
        Migration {
            version: 4,
            name: "config-files",
            sql: include_str!("../migrations/0004_config_files.sql"),
        },
        Migration {
            version: 5,
            name: "config-profile",
            sql: include_str!("../migrations/0005_config_profile.sql"),
        },
        Migration {
            version: 6,
            name: "dataset-references",
            sql: include_str!("../migrations/0006_dataset_references.sql"),
        },
    ];
    migrations.sort_by_key(|migration| migration.version);
    migrations
}

/// The catalog's SQLite baseline, in version order.
///
/// The SQLite ledger starts at 1 and lives in the shared `schema_migrations`
/// table under the `catalog` ledger name.
#[must_use]
pub fn sqlite() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            name: "catalog",
            sql: include_str!("../migrations/sqlite/0001_catalog.sql"),
        },
        Migration {
            version: 2,
            name: "dataset-references",
            sql: include_str!("../migrations/sqlite/0002_dataset_references.sql"),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_runtime::migrations::{Applied, checksum, plan};

    /// The executable statements, without the explanatory comments. A comment
    /// may name the Postgres spelling it is explaining.
    fn statements(sql: &str) -> String {
        sql.lines()
            .filter(|line| !line.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn applied(version: i64, migration: &Migration) -> Applied {
        Applied {
            version,
            name: migration.name.to_owned(),
            checksum: checksum(migration.sql),
        }
    }

    #[test]
    fn an_empty_database_gets_every_migration() {
        let available = available();
        let pending = plan(&[], &available).expect("plan");
        assert_eq!(pending.len(), available.len());
    }

    #[test]
    fn a_fully_migrated_database_gets_none() {
        let available = available();
        let applied: Vec<Applied> = available
            .iter()
            .map(|migration| applied(migration.version, migration))
            .collect();
        assert!(plan(&applied, &available).expect("plan").is_empty());
    }

    #[test]
    fn a_partial_database_gets_only_the_rest() {
        let available = available();
        let applied = vec![applied(1, &available[0])];
        let pending = plan(&applied, &available).expect("plan");
        assert_eq!(
            pending.iter().map(|m| m.version).collect::<Vec<_>>(),
            vec![2, 3, 4, 5, 6]
        );
    }

    #[test]
    fn a_changed_body_is_refused() {
        let available = available();
        let mut applied = vec![applied(1, &available[0])];
        applied[0].checksum = "0000000000000000".to_owned();
        assert!(plan(&applied, &available).is_err());
    }

    #[test]
    fn versions_are_unique_and_ordered() {
        let available = available();
        let versions: Vec<i64> = available
            .iter()
            .map(|migration| migration.version)
            .collect();
        assert_eq!(versions, vec![1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn the_list_owns_only_the_catalog_schema() {
        let available = available();
        for migration in &available {
            assert!(
                !migration.sql.contains("runs."),
                "{} names runs",
                migration.name
            );
            assert!(
                !migration.sql.contains("research."),
                "{} names research",
                migration.name
            );
            assert!(
                !migration.sql.contains("simulation."),
                "{} names simulation",
                migration.name
            );
            assert!(
                !migration.sql.contains("gateway."),
                "{} names gateway",
                migration.name
            );
        }
    }

    #[test]
    fn the_sqlite_baseline_starts_at_one_and_is_complete() {
        let available = sqlite();
        assert_eq!(available.len(), 2);
        assert_eq!(available[0].version, 1);
        assert_eq!(available[1].version, 2);
        assert_eq!(plan(&[], &available).expect("plan").len(), 2);
    }

    #[test]
    fn the_sqlite_baseline_uses_the_double_prefix_and_the_sqlite_types() {
        let sql = statements(sqlite()[0].sql);
        for table in [
            "catalog__datasets",
            "catalog__configs",
            "catalog__config_bundles",
        ] {
            assert!(sql.contains(table), "the baseline omits {table}");
        }
        assert!(
            !sql.contains("catalog_datasets") && !sql.contains("catalog_configs"),
            "the single-underscore prefix is not a SQLite name"
        );
        assert!(
            !sql.contains("catalog.datasets") && !sql.contains("catalog.configs"),
            "a dotted Postgres name is not a SQLite name"
        );
        for forbidden in ["CREATE SCHEMA", "jsonb", "timestamptz", "boolean"] {
            assert!(
                !sql.to_ascii_lowercase()
                    .contains(&forbidden.to_ascii_lowercase()),
                "the SQLite baseline names the Postgres type or statement {forbidden}"
            );
        }
    }

    #[test]
    fn the_sqlite_baseline_never_names_another_service_prefix() {
        for migration in sqlite() {
            let sql = statements(migration.sql);
            for forbidden in [
                "gateway__",
                "runs__",
                "research__",
                "simulation__",
                "queue__",
            ] {
                assert!(
                    !sql.contains(forbidden),
                    "{} names {forbidden}",
                    migration.name
                );
            }
        }
    }
}
