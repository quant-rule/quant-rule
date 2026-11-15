//! The gateway's compiled-in migration lists.
//!
//! The runner lives in `quantrule-runtime`; a list belongs to the service that
//! owns the schema. P3 moved the `catalog` list to the data service and P5 moved
//! the run and research lists to `quantrule-evidence-service`. The gateway keeps
//! only its own `gateway.profiles` table. The simulation schema and its `0014`
//! migration belong to `quantrule-simulation-service`; this list never applies
//! them.
//!
//! The Postgres and SQLite histories are separate on purpose. The Postgres list
//! replays the released history (`0007`); the SQLite baseline is the final
//! logical shape at version 1, because a desktop database has no earlier rows to
//! migrate. Both write the same `gateway.profiles` relation.

use quantrule_runtime::migrations::Migration;

/// The gateway schema's Postgres migrations, in version order.
#[must_use]
pub fn gateway() -> Vec<Migration> {
    vec![Migration {
        version: 7,
        name: "profiles",
        sql: include_str!("../migrations/0007_profiles.sql"),
    }]
}

/// The gateway's SQLite baseline, in version order.
///
/// The SQLite ledger starts at 1 and lives in the shared `schema_migrations`
/// table under the `gateway` ledger name.
#[must_use]
pub fn sqlite() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        name: "profiles",
        sql: include_str!("../migrations/sqlite/0001_profiles.sql"),
    }]
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

    /// The executable statements, without the explanatory comments. A comment
    /// may name the Postgres spelling it is explaining.
    fn statements(sql: &str) -> String {
        sql.lines()
            .filter(|line| !line.trim_start().starts_with("--"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn an_empty_database_gets_every_gateway_migration() {
        let available = gateway();
        let pending = plan(&[], &available).expect("plan");
        assert_eq!(pending.len(), available.len());
    }

    #[test]
    fn a_fully_migrated_database_gets_none() {
        let available = gateway();
        let applied: Vec<Applied> = available.iter().map(applied).collect();
        assert!(plan(&applied, &available).expect("plan").is_empty());
    }

    #[test]
    fn a_changed_body_is_refused() {
        let available = gateway();
        let mut applied = vec![applied(&available[0])];
        applied[0].checksum = "0000000000000000".to_owned();
        assert!(plan(&applied, &available).is_err());
    }

    #[test]
    fn versions_are_unique_and_ordered() {
        let available = gateway();
        let versions: Vec<i64> = available
            .iter()
            .map(|migration| migration.version)
            .collect();
        assert_eq!(versions, vec![7]);
    }

    #[test]
    fn the_gateway_list_never_names_another_service_schema() {
        for migration in gateway() {
            for forbidden in ["catalog.", "runs.", "research.", "simulation."] {
                assert!(
                    !migration.sql.contains(forbidden),
                    "{} names {forbidden}",
                    migration.name
                );
            }
        }
    }

    #[test]
    fn the_run_attempts_migration_is_not_registered_here() {
        // Evidence owns migration 13; the gateway must not apply a run schema.
        assert!(gateway().iter().all(|migration| migration.version != 13));
    }

    #[test]
    fn the_sqlite_baseline_starts_at_one_and_is_complete() {
        let available = sqlite();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].version, 1);
        assert!(plan(&[], &available).expect("plan").len() == 1);
    }

    #[test]
    fn the_sqlite_baseline_uses_the_double_prefix_and_the_sqlite_types() {
        let sql = statements(sqlite()[0].sql);
        assert!(
            sql.contains("gateway__profiles"),
            "the double prefix is missing"
        );
        assert!(
            !sql.contains("gateway_profiles"),
            "the single-underscore prefix is not a SQLite name"
        );
        assert!(
            !sql.contains("gateway.profiles"),
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
                "catalog__",
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
