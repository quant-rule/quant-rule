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
//! The SQLite list is the baseline for the embedded desktop profile, not a
//! translation of the Postgres history: a desktop database starts at v1.1 and
//! has no earlier rows to migrate. It writes the same schemas under the
//! `runs__`, `research__`, and `stage__` table prefixes and starts at version 1
//! in the same `runs` ledger. The stage tables are an additive migration so an
//! existing desktop database keeps its rows.

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
    ];
    migrations.sort_by_key(|migration| migration.version);
    migrations
}

/// The SQLite baseline for the embedded desktop profile, in version order.
///
/// The ledger starts at 1 and lives in the shared `schema_migrations` table
/// under the `runs` ledger name. Version 2 is additive: it adds the diagnostic
/// stage tables without touching the run or research rows.
#[must_use]
pub fn sqlite() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            name: "runs-research",
            sql: include_str!("../migrations/sqlite/0001_runs_research.sql"),
        },
        Migration {
            version: 2,
            name: "stage",
            sql: include_str!("../migrations/sqlite/0002_stage.sql"),
        },
        Migration {
            version: 3,
            name: "run-model-artifact",
            sql: include_str!("../migrations/sqlite/0003_run_model_artifact.sql"),
        },
    ]
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
            vec![4, 5, 8, 9, 13, 14, 15]
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
        assert_eq!(versions, vec![2, 4, 5, 8, 9, 13, 14, 15]);
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
    fn the_model_artifact_migration_is_registered() {
        let migration = available()
            .into_iter()
            .find(|migration| migration.version == 15)
            .expect("migration 15");
        assert_eq!(migration.name, "run-model-artifact");
        assert!(migration.sql.contains("ADD COLUMN model_artifact_id"));
        let sqlite = sqlite()
            .into_iter()
            .find(|migration| migration.version == 3)
            .expect("sqlite migration 3");
        assert_eq!(sqlite.name, "run-model-artifact");
        assert!(sqlite.sql.contains("ADD COLUMN model_artifact_id"));
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

    /// The executable statements, without the explanatory comments. A comment
    /// may name the Postgres spelling it is explaining.
    fn statements(sql: &str) -> String {
        sql.lines()
            .filter(|line| !line.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_sqlite_baseline_starts_at_one_and_is_complete() {
        let baseline = sqlite();
        assert_eq!(baseline.len(), 3);
        assert_eq!(baseline[0].version, 1);
        assert_eq!(baseline[1].version, 2);
        assert_eq!(baseline[2].version, 3);
        assert_eq!(plan(&[], &baseline).expect("plan").len(), 3);
    }

    #[test]
    fn the_sqlite_stage_migration_is_additive() {
        let stage = &sqlite()[1];
        assert_eq!(stage.name, "stage");
        assert!(
            !stage.sql.contains("runs__runs"),
            "the stage migration must not rewrite the run table"
        );
        assert!(
            !stage.sql.contains("research__hypotheses"),
            "the stage migration must not rewrite the hypothesis table"
        );
        for table in ["stage__runs", "stage__submissions", "stage__artifacts"] {
            assert!(stage.sql.contains(table), "{table} is missing");
        }
    }

    #[test]
    fn the_sqlite_baseline_uses_the_double_prefixes_and_sqlite_types() {
        let sql = sqlite()
            .iter()
            .map(|migration| statements(migration.sql))
            .collect::<Vec<_>>()
            .join("\n");
        for table in [
            "runs__runs",
            "runs__stages",
            "runs__artifacts",
            "runs__submissions",
            "research__hypotheses",
            "stage__runs",
            "stage__submissions",
            "stage__artifacts",
        ] {
            assert!(sql.contains(table), "{table} is missing");
        }
        for forbidden in [
            "CREATE SCHEMA",
            "jsonb",
            "timestamptz",
            "boolean",
            "plpgsql",
            "to_char",
            "now()",
        ] {
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
            for forbidden in ["gateway__", "catalog__", "simulation__", "queue__"] {
                assert!(
                    !sql.contains(forbidden),
                    "{} names {forbidden}",
                    migration.name
                );
            }
        }
    }

    #[test]
    fn the_sqlite_baseline_refuses_to_rewrite_a_sealed_hypothesis() {
        let sql = statements(sqlite()[0].sql);
        assert!(
            sql.contains("BEFORE UPDATE ON research__hypotheses"),
            "the sealed-row update trigger is missing"
        );
        assert!(
            sql.contains("BEFORE DELETE ON research__hypotheses"),
            "the sealed-row delete trigger is missing"
        );
        assert_eq!(
            sql.matches("RAISE(ABORT, 'a sealed hypothesis is immutable')")
                .count(),
            2,
            "both sealed-row triggers must abort"
        );
    }
}
