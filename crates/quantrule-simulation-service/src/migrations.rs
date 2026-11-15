//! The `simulation` migration ledger.
//!
//! Simulation owns one schema and one ledger, `simulation.schema_migrations`.
//! A SQL file is inert until it is listed here, and this list names no other
//! service's schema. The Postgres version keeps the historical number the plan
//! assigned so the file name and the record agree; the SQLite baseline starts
//! at 1 because a desktop database has no earlier rows to migrate.
//!
//! The Postgres and SQLite histories are separate on purpose. Both write the
//! same logical relations, `simulation.runs` and `simulation.marks`, and the
//! SQLite spelling is `simulation__runs` and `simulation__marks`.

use quantrule_runtime::migrations::Migration;

/// The simulation schema's Postgres migrations, in version order.
#[must_use]
pub fn available() -> Vec<Migration> {
    vec![Migration {
        version: 14,
        name: "simulation",
        sql: include_str!("../migrations/0014_simulation.sql"),
    }]
}

/// The simulation schema's SQLite baseline, in version order.
///
/// The SQLite ledger starts at 1 and lives in the shared `schema_migrations`
/// table under the `simulation` ledger name.
#[must_use]
pub fn sqlite() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        name: "simulation",
        sql: include_str!("../migrations/sqlite/0001_simulation.sql"),
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

    #[test]
    fn the_sqlite_baseline_starts_at_one_and_is_complete() {
        let available = sqlite();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0].version, 1);
        assert_eq!(plan(&[], &available).expect("plan").len(), 1);
        let applied: Vec<Applied> = available.iter().map(applied).collect();
        assert!(plan(&applied, &available).expect("plan").is_empty());
    }

    #[test]
    fn the_sqlite_baseline_uses_the_double_prefix_and_the_sqlite_types() {
        let sql = statements(sqlite()[0].sql);
        assert!(
            sql.contains("simulation__runs"),
            "the run table prefix is missing"
        );
        assert!(
            sql.contains("simulation__marks"),
            "the mark table prefix is missing"
        );
        assert!(
            !sql.contains("simulation.runs"),
            "a dotted Postgres name is not a SQLite name"
        );
        assert!(
            !sql.contains("simulation_marks"),
            "the single-underscore prefix is not a SQLite name"
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
    fn the_sqlite_baseline_carries_the_mark_and_decision_rules() {
        let sql = statements(sqlite()[0].sql);
        assert!(sql.contains("UNIQUE (sim_id, origin, chart, bar, action, side)"));
        assert!(sql.contains("ON DELETE CASCADE"));
        assert!(sql.contains("status IN ('draft', 'marking', 'reviewed', 'checked', 'decided')"));
        assert!(sql.contains("decision IN ('test', 'refine', 'stop')"));
        assert!(
            sql.contains("CHECK ((status = 'decided') = (decision IS NOT NULL))"),
            "a decision must exist exactly when the run is decided"
        );
        for trigger in [
            "simulation__marks_closed_after_decision_insert",
            "simulation__marks_closed_after_decision_update",
            "simulation__marks_closed_after_decision_delete",
            "simulation__runs_are_final",
        ] {
            assert!(sql.contains(trigger), "the rule {trigger} is missing");
        }
    }

    #[test]
    fn the_sqlite_baseline_never_names_another_service_prefix() {
        for migration in sqlite() {
            let sql = statements(migration.sql);
            for forbidden in ["catalog__", "runs__", "research__", "gateway__", "queue__"] {
                assert!(
                    !sql.contains(forbidden),
                    "{} names {forbidden}",
                    migration.name
                );
            }
        }
    }
}
