//! The `simulation` migration ledger.
//!
//! Simulation owns one schema and one ledger, `simulation.schema_migrations`.
//! A SQL file is inert until it is listed here, and this list names no other
//! service's schema. The Postgres version keeps the historical number the plan
//! assigned so the file name and the record agree.

use quantrule_runtime::migrations::Migration;

/// The simulation schema's Postgres migrations, in version order.
#[must_use]
pub fn available() -> Vec<Migration> {
    let mut migrations = vec![
        Migration {
            version: 14,
            name: "simulation",
            sql: include_str!("../migrations/0014_simulation.sql"),
        },
        Migration {
            version: 15,
            name: "simulation-retention",
            sql: include_str!("../migrations/0015_simulation_retention.sql"),
        },
        Migration {
            version: 16,
            name: "simulation-concurrency",
            sql: include_str!("../migrations/0016_simulation_concurrency.sql"),
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

    fn an_empty_database_gets_the_migration() {
        let available = available();
        assert_eq!(plan(&[], &available).expect("plan").len(), 3);
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
            for forbidden in ["catalog.", "research.", "gateway.", "stage."] {
                assert!(
                    !migration.sql.contains(forbidden),
                    "{} names {forbidden}",
                    migration.name
                );
            }
        }
        assert!(
            available()[0]
                .sql
                .contains("CREATE SCHEMA IF NOT EXISTS simulation")
        );
    }

    #[test]

    fn the_profile_column_is_present_on_the_run_and_the_mark() {
        let sql = available()[0].sql;
        assert!(sql.contains("profile_id     text NOT NULL"));
        assert!(sql.contains("profile_id       text NOT NULL"));
        assert!(sql.contains("UNIQUE (sim_id, origin, chart, bar, action, side)"));
    }

    #[test]
    fn the_decision_is_terminal_in_the_schema() {
        let sql = available()[2].sql;
        assert!(sql.contains("simulation_runs_decision_complete"));
        assert!(sql.contains("simulation.runs_decision_is_terminal"));
        assert!(sql.contains("BEFORE UPDATE ON simulation.runs"));
        assert!(sql.contains("the simulation decision is terminal"));
    }
}
