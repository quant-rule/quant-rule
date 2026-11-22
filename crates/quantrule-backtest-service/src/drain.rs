//! Wait for in-flight worker tasks, then abort at the drain deadline.
//!
//! A signal stops new claims at the consumer adapter. This helper gives the
//! current run or stage time to finish, and drops the task at the deadline. An
//! aborted attempt leaves the run `running` with its lease and no digest; lease
//! expiry and the outbox are the recovery path, never a shutdown failure.

use std::time::Duration;

use tokio::task::JoinSet;

/// What a drain wait observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainOutcome {
    /// Every consumer task ended before the deadline.
    Finished,
    /// The deadline expired and the remaining tasks were aborted.
    DeadlineExceeded,
}

/// Wait for every task in `consumers` up to `timeout`.
///
/// A task that panics is treated as finished: the caller's join result decides
/// whether that panic matters. At the deadline every remaining task is aborted.
pub async fn drain_consumers<T: 'static>(
    consumers: &mut JoinSet<T>,
    timeout: Duration,
) -> DrainOutcome {
    let deadline = tokio::time::sleep(timeout);
    tokio::pin!(deadline);
    loop {
        tokio::select! {
            biased;
            () = &mut deadline => {
                consumers.abort_all();
                while consumers.join_next().await.is_some() {}
                return DrainOutcome::DeadlineExceeded;
            }
            next = consumers.join_next() => {
                if next.is_none() {
                    return DrainOutcome::Finished;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn an_in_flight_task_finishes_before_the_deadline() {
        let mut consumers = JoinSet::new();
        consumers.spawn(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            1
        });
        assert_eq!(
            drain_consumers(&mut consumers, Duration::from_secs(1)).await,
            DrainOutcome::Finished
        );
        assert_eq!(consumers.len(), 0);
    }

    #[tokio::test]
    async fn the_deadline_aborts_a_task_that_never_finishes() {
        let mut consumers = JoinSet::new();
        consumers.spawn(async {
            std::future::pending::<()>().await;
        });
        assert_eq!(
            drain_consumers(&mut consumers, Duration::from_millis(10)).await,
            DrainOutcome::DeadlineExceeded
        );
        assert_eq!(consumers.len(), 0, "the aborted task is still in the set");
    }
}
