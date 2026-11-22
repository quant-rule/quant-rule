//! The Postgres evidence leader lease.
//!
//! Acquisition, renewal, and release use the database clock. A finalisation
//! takes the same row lock before it writes a terminal state, so an acquisition
//! and a stale leader's write cannot interleave.

use quantrule_runtime::adapters::BoxFuture;
use sqlx::PgPool;

use crate::leader::{LEADER_LEASE_SECONDS, LeaderLease, LeaderLeaseError, LeaderStatus};

/// A leader lease backed by `runs.leader_lease`.
#[derive(Debug, Clone)]
pub struct PgLeaderLease {
    pool: PgPool,
    lease_seconds: u64,
}

impl PgLeaderLease {
    /// Build the lease over an existing pool with the production window.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self::with_lease_seconds(pool, LEADER_LEASE_SECONDS)
    }

    /// Build the lease with an explicit window.
    ///
    /// The production service uses [`Self::new`]. A composed test can pass zero
    /// to observe an expired lease without waiting.
    #[must_use]
    pub fn with_lease_seconds(pool: PgPool, lease_seconds: u64) -> Self {
        Self {
            pool,
            lease_seconds,
        }
    }
}

impl LeaderLease for PgLeaderLease {
    fn acquire(&self, holder: String) -> BoxFuture<Result<Option<u64>, LeaderLeaseError>> {
        let pool = self.pool.clone();
        let lease_seconds = self.lease_seconds;
        Box::pin(async move {
            let term: Option<i64> = sqlx::query_scalar(
                "UPDATE runs.leader_lease
                    SET holder = $1, term = term + 1,
                        expires_at = now() + make_interval(secs => $2),
                        updated_at = now()
                  WHERE singleton AND expires_at <= now()
                 RETURNING term",
            )
            .bind(&holder)
            .bind(lease_seconds as f64)
            .fetch_optional(&pool)
            .await
            .map_err(|error| LeaderLeaseError(error.to_string()))?;
            term.map(term_value).transpose()
        })
    }

    fn renew(&self, holder: String, term: u64) -> BoxFuture<Result<Option<u64>, LeaderLeaseError>> {
        let pool = self.pool.clone();
        let lease_seconds = self.lease_seconds;
        Box::pin(async move {
            let term = term_value_i64(term)?;
            let renewed: Option<i64> = sqlx::query_scalar(
                "UPDATE runs.leader_lease
                    SET expires_at = now() + make_interval(secs => $3),
                        updated_at = now()
                  WHERE singleton AND holder = $1 AND term = $2
                 RETURNING term",
            )
            .bind(&holder)
            .bind(term)
            .bind(lease_seconds as f64)
            .fetch_optional(&pool)
            .await
            .map_err(|error| LeaderLeaseError(error.to_string()))?;
            renewed.map(term_value).transpose()
        })
    }

    fn release(&self, holder: String, term: u64) -> BoxFuture<Result<(), LeaderLeaseError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let term = term_value_i64(term)?;
            sqlx::query(
                "UPDATE runs.leader_lease
                    SET expires_at = '-infinity', updated_at = now()
                  WHERE singleton AND holder = $1 AND term = $2",
            )
            .bind(&holder)
            .bind(term)
            .execute(&pool)
            .await
            .map_err(|error| LeaderLeaseError(error.to_string()))?;
            Ok(())
        })
    }

    fn current(&self) -> BoxFuture<Result<Option<LeaderStatus>, LeaderLeaseError>> {
        let pool = self.pool.clone();
        Box::pin(async move {
            let row: Option<(String, i64, bool)> = sqlx::query_as(
                "SELECT holder, term, expires_at > now()
                   FROM runs.leader_lease WHERE singleton",
            )
            .fetch_optional(&pool)
            .await
            .map_err(|error| LeaderLeaseError(error.to_string()))?;
            row.map(|(holder, term, live)| {
                Ok(LeaderStatus {
                    holder,
                    term: term_value(term)?,
                    live,
                })
            })
            .transpose()
        })
    }
}

fn term_value(value: i64) -> Result<u64, LeaderLeaseError> {
    u64::try_from(value)
        .map_err(|_| LeaderLeaseError("a stored leader term is negative".to_owned()))
}

fn term_value_i64(value: u64) -> Result<i64, LeaderLeaseError> {
    i64::try_from(value)
        .map_err(|_| LeaderLeaseError("a leader term does not fit in bigint".to_owned()))
}
