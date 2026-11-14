//! The `simulation` migration ledger.
//!
//! Simulation owns one schema and one ledger, `simulation.schema_migrations`.
//! A SQL file is inert until it is listed here, and this list names no other
//! service's schema. The version is local to the ledger; it keeps the
//! historical number the plan assigned so the file name and the record agree.

use quantrule_runtime::migrations::Migration;

/// The migration this service carries, in version order.
#[must_use]
pub fn available() -> Vec<Migration> {
    vec![Migration {
        version: 14,
        name: "simulation",
        sql: include_str!("../migrations/0014_simulation.sql"),
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

    #[test]
    fn an_empty_database_gets_the_migration() {
        let available = available();
        assert_eq!(plan(&[], &available).expect("plan").len(), 1);
    }

    #[test]
    fn a_fully_migrated_database_gets_none() {
        let available = available();
        let applied: Vec<Applied> = available.iter().map(applied).collect();
        assert!(plan(&applied, &available).expect("plan").is_empty());
    }

    #[test]
    fn a_changed_body_is_refused() {
        let mut applied = vec![applied(&available()[0])];
        applied[0].checksum = "0000000000000000".to_owned();
        assert!(plan(&applied, &available()).is_err());
    }

    #[test]
    fn the_ledger_owns_only_the_simulation_schema() {
        for migration in available() {
            for forbidden in ["catalog.", "runs.", "research.", "gateway."] {
                assert!(
                    !migration.sql.contains(forbidden),
                    "{} names {forbidden}",
                    migration.name
                );
            }
            assert!(
                migration
                    .sql
                    .contains("CREATE SCHEMA IF NOT EXISTS simulation"),
                "{} does not create the simulation schema",
                migration.name
            );
        }
    }

    #[test]
    fn the_profile_column_is_present_on_the_run_and_the_mark() {
        let sql = available()[0].sql;
        assert!(sql.contains("profile_id     text NOT NULL"));
        assert!(sql.contains("profile_id       text NOT NULL"));
        assert!(sql.contains("UNIQUE (sim_id, origin, chart, bar, action, side)"));
    }
}
