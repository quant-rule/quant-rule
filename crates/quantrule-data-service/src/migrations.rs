//! The catalog migration list.
//!
//! The runner lives in `quantrule-runtime`; this list belongs to the data
//! service because it owns the `catalog` schema. A SQL file is inert until it is
//! listed here, so a new migration is not applied by copying a file into the
//! directory. Versions are local to the `catalog` ledger.

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
    ];
    migrations.sort_by_key(|migration| migration.version);
    migrations
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_runtime::migrations::{Applied, checksum, plan};

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
            vec![2, 3, 4, 5]
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
        assert_eq!(versions, vec![1, 2, 3, 4, 5]);
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
}
