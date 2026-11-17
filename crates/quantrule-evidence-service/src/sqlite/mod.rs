//! The evidence service's SQLite profile repositories.
//!
//! The store types implement the existing [`crate::runs::RunStore`] and
//! [`crate::research::HypothesisStore`] traits; no route, use case, or contract
//! changes between the server and desktop profiles. Every call runs on the
//! runtime's blocking pool through
//! [`SqliteDatabase::call`](quantrule_runtime::adapters::sqlite::SqliteDatabase::call),
//! so the async route shape is unchanged.
//!
//! The tables are the double-prefix spelling of the Postgres relations:
//! `runs.runs` is `runs__runs`, `runs.stages` is `runs__stages`,
//! `research.hypotheses` is `research__hypotheses`, and the research-ledger
//! tables are `research__improvement_runs`, `research__baselines`, and
//! `research__invalidations`. Evidence names only its own `runs__` and
//! `research__` prefixes; the shared ledger and the other service baselines
//! live in the runtime and their owning crates.
//!
//! A state transition runs inside `BEGIN IMMEDIATE` and re-reads the run inside
//! that transaction. SQLite has no `SELECT ... FOR UPDATE`: the write lock taken
//! by `BEGIN IMMEDIATE` is what makes the read-then-decide sequence atomic, so a
//! second process delivering the same run queues behind the lock and then sees
//! the first process's committed attempt count.

pub mod research;
pub mod runs;

pub use research::SqliteHypothesisStore;
pub use runs::SqliteRunStore;

/// The run table, from the shared `Schema` mapping.
pub(crate) const RUNS_TABLE: &str = "runs__runs";
/// The per-run stage table.
pub(crate) const STAGES_TABLE: &str = "runs__stages";
/// The per-run artifact table.
pub(crate) const ARTIFACTS_TABLE: &str = "runs__artifacts";
/// The profile-to-run binding table.
pub(crate) const SUBMISSIONS_TABLE: &str = "runs__submissions";
/// The sealed hypothesis table.
pub(crate) const HYPOTHESES_TABLE: &str = "research__hypotheses";
/// The pre-registered improvement-run table.
pub(crate) const IMPROVEMENT_RUNS_TABLE: &str = "research__improvement_runs";
/// The accepted-baseline registry table.
pub(crate) const BASELINES_TABLE: &str = "research__baselines";
/// The append-only invalidation registry table.
pub(crate) const INVALIDATIONS_TABLE: &str = "research__invalidations";

#[cfg(test)]
mod tests {
    use super::*;
    use quantrule_runtime::adapters::{Dialect, Schema};

    #[test]
    fn the_tables_are_the_service_prefixes() {
        assert_eq!(
            RUNS_TABLE,
            Dialect::Sqlite
                .table(Schema::Runs, "runs")
                .expect("the run table")
        );
        assert_eq!(
            STAGES_TABLE,
            Dialect::Sqlite
                .table(Schema::Runs, "stages")
                .expect("the stage table")
        );
        assert_eq!(
            ARTIFACTS_TABLE,
            Dialect::Sqlite
                .table(Schema::Runs, "artifacts")
                .expect("the artifact table")
        );
        assert_eq!(
            SUBMISSIONS_TABLE,
            Dialect::Sqlite
                .table(Schema::Runs, "submissions")
                .expect("the submission table")
        );
        assert_eq!(
            HYPOTHESES_TABLE,
            Dialect::Sqlite
                .table(Schema::Research, "hypotheses")
                .expect("the hypothesis table")
        );
        assert_eq!(
            IMPROVEMENT_RUNS_TABLE,
            Dialect::Sqlite
                .table(Schema::Research, "improvement_runs")
                .expect("the improvement-run table")
        );
        assert_eq!(
            BASELINES_TABLE,
            Dialect::Sqlite
                .table(Schema::Research, "baselines")
                .expect("the baseline table")
        );
        assert_eq!(
            INVALIDATIONS_TABLE,
            Dialect::Sqlite
                .table(Schema::Research, "invalidations")
                .expect("the invalidation table")
        );
        assert_eq!(
            Dialect::Postgres
                .table(Schema::Runs, "runs")
                .expect("the Postgres name"),
            "runs.runs"
        );
        assert_eq!(
            Dialect::Postgres
                .table(Schema::Research, "hypotheses")
                .expect("the Postgres name"),
            "research.hypotheses"
        );
    }

    #[test]
    fn every_table_stays_inside_the_evidence_prefixes() {
        for table in [
            RUNS_TABLE,
            STAGES_TABLE,
            ARTIFACTS_TABLE,
            SUBMISSIONS_TABLE,
            HYPOTHESES_TABLE,
            IMPROVEMENT_RUNS_TABLE,
            BASELINES_TABLE,
            INVALIDATIONS_TABLE,
        ] {
            assert!(
                table.starts_with("runs__") || table.starts_with("research__"),
                "{table} is outside the evidence prefixes"
            );
        }
    }
}
