//! The evidence migration ledger.
//!
//! Evidence owns three schemas: `runs`, `research`, and the diagnostic `stage`
//! schema. The plan gives the first two one ledger, so this list is the only
//! place a run, hypothesis, or stage migration is applied. A SQL file is inert
//! until it is listed here; the retry-budget migration (`0013_run_attempts.sql`)
//! was parked with #646 and is registered here because the run store reads the
//! column it adds. Versions are local to the ledger and keep their historical
//! numbers.
//!

use quantrule_runtime::migrations::Migration;

/// The migrations this service carries, in version order.
#[must_use]
pub fn available() -> Vec<Migration> {
    let mut migrations = vec![
        Migration {
            version: 2,
            name: "runs",
            sql: include_str!("../migrations/0002_runs.sql"),
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
        Migration {
            version: 13,
            name: "run-attempts",
            sql: include_str!("../migrations/0013_run_attempts.sql"),
        },
        Migration {
            version: 14,
            name: "stage",
            sql: include_str!("../migrations/0014_stage.sql"),
        },
        Migration {
            version: 15,
            name: "run-model-artifact",
            sql: include_str!("../migrations/0015_run_model_artifact.sql"),
        },
        Migration {
            version: 16,
            name: "research-ledger",
            sql: include_str!("../migrations/0016_research_ledger.sql"),
        },
        Migration {
            version: 17,
            name: "retention",
            sql: include_str!("../migrations/0017_retention.sql"),
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

    fn an_empty_database_gets_every_migration() {
        let available = available();
        let pending = plan(&[], &available).expect("plan");
        assert_eq!(pending.len(), available.len());
    }

    #[test]

    fn a_fully_migrated_database_gets_none() {
        let available = available();
        let applied: Vec<Applied> = available.iter().map(applied).collect();
        assert!(plan(&applied, &available).expect("plan").is_empty());
    }

    #[test]

    fn a_partial_database_gets_only_the_rest() {
        let available = available();
        let applied = vec![applied(&available[0])];
        let pending = plan(&applied, &available).expect("plan");
        assert_eq!(
            pending.iter().map(|m| m.version).collect::<Vec<_>>(),
            vec![4, 5, 8, 9, 13, 14, 15, 16, 17]
        );
    }

    #[test]

    fn a_changed_body_is_refused() {
        let available = available();
        let mut applied = vec![applied(&available[0])];
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
        assert_eq!(versions, vec![2, 4, 5, 8, 9, 13, 14, 15, 16, 17]);
    }

    #[test]

    fn the_run_attempts_migration_is_registered() {
        let available = available();
        let migration = available
            .iter()
            .find(|migration| migration.version == 13)
            .expect("migration 13");
        assert_eq!(migration.name, "run-attempts");
        assert!(
            migration.sql.contains("ADD COLUMN attempts"),
            "migration 13 must add the attempts column the run store reads"
        );
    }

    #[test]

    fn the_stage_migration_is_registered() {
        let available = available();
        let migration = available
            .iter()
            .find(|migration| migration.version == 14)
            .expect("migration 14");
        assert_eq!(migration.name, "stage");
        for table in ["stage.runs", "stage.submissions", "stage.artifacts"] {
            assert!(
                migration.sql.contains(table),
                "migration 14 must create {table}"
            );
        }
        assert!(
            migration.sql.contains("CREATE SCHEMA IF NOT EXISTS stage"),
            "migration 14 must create the stage schema"
        );
    }

    #[test]

    fn the_list_owns_only_the_evidence_schemas() {
        for migration in available() {
            for forbidden in ["catalog.", "simulation.", "gateway."] {
                assert!(
                    !migration.sql.contains(forbidden),
                    "{} names {forbidden}",
                    migration.name
                );
            }
        }
    }
}
