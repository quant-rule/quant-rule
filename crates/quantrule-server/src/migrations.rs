//! The gateway's compiled-in migration lists.
//!
//! The runner lives in `quantrule-runtime`; a list belongs to the service that
//! owns the schema. P3 moves the `catalog` list to the data service. The gateway
//! keeps its own profile table in the `gateway` schema, and the run, research,
//! and simulation lists stay here until P5 and P7 move them to their services.
//! Each list is recorded under its own ledger, so a version number may repeat
//! across services.

use quantrule_runtime::migrations::Migration;

/// The gateway schema's migrations, in version order.
#[must_use]
pub fn gateway() -> Vec<Migration> {
    vec![Migration {
        version: 7,
        name: "profiles",
        sql: include_str!("../migrations/0007_profiles.sql"),
    }]
}

/// The run, research, and simulation migrations still applied by the gateway.
///
/// P5 moves the run and research lists to evidence; P7 moves simulation. The
/// retry-budget migration (`0013_run_attempts.sql`) stays unregistered here
/// because D2 is owned by P5.
#[must_use]
pub fn service() -> Vec<Migration> {
    let mut migrations = vec![
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
            version: 8,
            name: "hypothesis-profile",
            sql: include_str!("../migrations/0008_hypothesis_profile.sql"),
        },
        Migration {
            version: 9,
            name: "run-submissions",
            sql: include_str!("../migrations/0009_run_submissions.sql"),
        },
    ];
    migrations.sort_by_key(|migration| migration.version);
    migrations
}

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_runtime::migrations::{Applied, checksum, plan};

    fn applied(migration: &Migration) -> Applied {
        Applied {
            version: migration.version,
            name: migration.name.to_owned(),
            checksum: checksum(migration.sql),
        }
    }

    #[test]
    fn an_empty_database_gets_every_gateway_migration() {
        let available = gateway();
        let pending = plan(&[], &available).expect("plan");
        assert_eq!(pending.len(), available.len());
    }

    #[test]
    fn an_empty_database_gets_every_service_migration() {
        let available = service();
        let pending = plan(&[], &available).expect("plan");
        assert_eq!(pending.len(), available.len());
    }

    #[test]
    fn a_fully_migrated_database_gets_none() {
        for available in [gateway(), service()] {
            let applied: Vec<Applied> = available.iter().map(applied).collect();
            assert!(plan(&applied, &available).expect("plan").is_empty());
        }
    }

    #[test]
    fn a_changed_body_is_refused() {
        let available = service();
        let mut applied = vec![applied(&available[0])];
        applied[0].checksum = "0000000000000000".to_owned();
        assert!(plan(&applied, &available).is_err());
    }

    #[test]
    fn versions_are_unique_and_ordered() {
        for available in [gateway(), service()] {
            let versions: Vec<i64> = available
                .iter()
                .map(|migration| migration.version)
                .collect();
            let mut sorted = versions.clone();
            sorted.sort_unstable();
            assert_eq!(versions, sorted);
            assert!(versions.windows(2).all(|pair| pair[0] != pair[1]));
        }
        assert_eq!(
            service().iter().map(|m| m.version).collect::<Vec<_>>(),
            vec![2, 3, 4, 5, 8, 9]
        );
    }

    #[test]
    fn the_gateway_list_never_names_the_catalog_schema() {
        for migration in gateway() {
            assert!(!migration.sql.contains("catalog."));
        }
    }

    #[test]
    fn the_service_list_never_names_the_catalog_schema() {
        for migration in service() {
            assert!(!migration.sql.contains("catalog."));
        }
    }
}
