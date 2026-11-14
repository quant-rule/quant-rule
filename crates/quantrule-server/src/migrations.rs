//! The service's compiled-in migration list.
//!
//! The runner lives in `quantrule-runtime`; the list belongs to the service that
//! owns the schema. A SQL file is inert until it is listed here, so a new
//! migration is not applied by copying a file into the directory.

use quantrule_runtime::migrations::Migration;

/// The migrations this binary carries, in version order.
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
            name: "runs",
            sql: include_str!("../migrations/0002_runs.sql"),
        },
        Migration {
            version: 3,
            name: "simulation",
            sql: include_str!("../migrations/0003_simulation.sql"),
        },
        Migration {
            version: 4,
            name: "research",
            sql: include_str!("../migrations/0004_research.sql"),
        },
        Migration {
            version: 5,
            name: "hypothesis-body",
            sql: include_str!("../migrations/0005_hypothesis_body.sql"),
        },
        Migration {
            version: 6,
            name: "catalog-profile",
            sql: include_str!("../migrations/0006_catalog_profile.sql"),
        },
        Migration {
            version: 7,
            name: "profiles",
            sql: include_str!("../migrations/0007_profiles.sql"),
        },
        Migration {
            version: 8,
            name: "hypothesis-profile",
            sql: include_str!("../migrations/0008_hypothesis_profile.sql"),
        },
        Migration {
            version: 9,
            name: "run-submissions",
            sql: include_str!("../migrations/0009_run_submissions.sql"),
        },
        Migration {
            version: 10,
            name: "dataset-quality",
            sql: include_str!("../migrations/0010_dataset_quality.sql"),
        },
        Migration {
            version: 11,
            name: "config-files",
            sql: include_str!("../migrations/0011_config_files.sql"),
        },
        Migration {
            version: 12,
            name: "config-profile",
            sql: include_str!("../migrations/0012_config_profile.sql"),
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
            vec![2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
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
        assert_eq!(versions, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
    }
}
