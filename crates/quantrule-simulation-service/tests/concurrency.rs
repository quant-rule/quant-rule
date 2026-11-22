//! The simulation decision and mark concurrency contract.
//!
//! The memory cases run in the fast gate. The Postgres cases are ignored and
//! `tools/postgres_composed.sh` runs them against a disposable database. Both
//! suites assert the same three rules: one decision wins, concurrent marks keep
//! the full set, and a duplicate decision is a stable conflict.

mod common;

use std::collections::BTreeMap;
use std::sync::Arc;

use common::MemoryStore;
use quantrule_api::{
    Digest, MarkAction, MarkOrigin, MarkQuality, ProfileId, Side, SimulationStatus,
};
use quantrule_runtime::adapters::postgres::{PgDatabase, apply_migrations_in};
use quantrule_simulation::{CheckConfig, Control, OracleSpec, WindowSpec};
use quantrule_simulation_service::migrations;
use quantrule_simulation_service::postgres::PgSimulationStore;
use quantrule_simulation_service::records::{
    DecisionKind, DecisionRecord, MarkRecord, NewSimulation,
};
use quantrule_simulation_service::store::{SimulationStore, StoreError};

fn profile() -> ProfileId {
    ProfileId::parse("concurrency").expect("the test profile")
}

fn new_run(sim_id: &str) -> NewSimulation {
    NewSimulation {
        sim_id: sim_id.to_owned(),
        premise: "two replicas must not race one simulation".to_owned(),
        bundle: Digest::sha256(b"concurrency-bundle"),
        inputs: "unspecified".to_owned(),
        inputs_digest: Digest::sha256(b"concurrency-inputs"),
        seed: 0,
        criteria: "min_events=30".to_owned(),
        window_spec: WindowSpec {
            event_codes: vec![149],
            window_bars: 400,
            candidate_zone: (0, 200),
            max_windows: 3,
            min_windows: 1,
            seed: 0,
            overlap_bars: 0,
            from_ts: None,
            to_ts: None,
        },
        oracle_spec: OracleSpec {
            horizon_bars: 60,
            target_atr: 2.0,
            stop_atr: 1.0,
            bad_stop_atr: 1.5,
            bad_target_atr: 1.0,
            entry_offset_max: 2,
        },
        check_config: CheckConfig {
            run_id: sim_id.to_owned(),
            event_code: 149,
            from: 0,
            to: i64::MAX,
            from_text: "2018-01-01".to_owned(),
            to_text: "2022-12-31".to_owned(),
            horizon_bars: 20,
            confirmation_bars: 0,
            cost_bps: 6.0,
            control: Control::Event,
            sides: BTreeMap::new(),
        },
        created_at: "2026-11-22T00:00:00Z".to_owned(),
    }
}

fn decision(kind: DecisionKind, note: &str, second: u8) -> DecisionRecord {
    DecisionRecord {
        decision: kind,
        note: note.to_owned(),
        decided_at_utc: format!("2026-11-22T00:00:{second:02}Z"),
    }
}

fn mark(sim_id: &str, bar: u64, quality: MarkQuality, note: &str) -> MarkRecord {
    MarkRecord {
        mark_id: quantrule_simulation_service::research::mark_id(
            sim_id,
            MarkOrigin::Human,
            0,
            bar,
            MarkAction::Entry,
            Side::Long,
        ),
        chart: 0,
        bar,
        marked_time_utc: "2018-01-01T00:00:00Z".to_owned(),
        action: MarkAction::Entry,
        quality,
        side: Side::Long,
        origin: MarkOrigin::Human,
        note: note.to_owned(),
        created_at: "2026-11-22T00:00:00Z".to_owned(),
    }
}

async fn create<S: SimulationStore>(store: &S, sim_id: &str) {
    store
        .create(profile(), new_run(sim_id))
        .await
        .expect("the test run");
}

async fn race_decisions<S>(
    store: Arc<S>,
    sim_id: &str,
    count: usize,
) -> Vec<Result<DecisionRecord, StoreError>>
where
    S: SimulationStore + 'static,
{
    let barrier = Arc::new(tokio::sync::Barrier::new(count));
    let kinds = [DecisionKind::Test, DecisionKind::Refine, DecisionKind::Stop];
    let mut tasks = Vec::with_capacity(count);
    for index in 0..count {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        let sim_id = sim_id.to_owned();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let kind = kinds[index % kinds.len()];
            store
                .decide(
                    profile(),
                    sim_id,
                    decision(kind, &format!("decision {index}"), index as u8),
                )
                .await
        }));
    }
    let mut results = Vec::with_capacity(count);
    for task in tasks {
        results.push(task.await.expect("the decision task"));
    }
    results
}

async fn race_marks<S>(store: Arc<S>, sim_id: &str, count: u64)
where
    S: SimulationStore + 'static,
{
    let barrier = Arc::new(tokio::sync::Barrier::new(count as usize));
    let mut tasks = Vec::with_capacity(count as usize);
    for bar in 0..count {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        let sim_id = sim_id.to_owned();
        tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let quality = if bar % 2 == 0 {
                MarkQuality::Good
            } else {
                MarkQuality::Bad
            };
            let record = mark(&sim_id, bar, quality, &format!("mark {bar}"));
            store.put_mark(profile(), sim_id, record).await
        }));
    }
    for task in tasks {
        task.await
            .expect("the mark task")
            .expect("a concurrent mark");
    }
}

fn winners(results: &[Result<DecisionRecord, StoreError>]) -> Vec<&DecisionRecord> {
    results
        .iter()
        .filter_map(|result| result.as_ref().ok())
        .collect()
}

async fn race_marks_with_decision<S>(
    store: Arc<S>,
    sim_id: &str,
    count: u64,
) -> (
    Result<DecisionRecord, StoreError>,
    Vec<Result<MarkRecord, StoreError>>,
)
where
    S: SimulationStore + 'static,
{
    let barrier = Arc::new(tokio::sync::Barrier::new(count as usize + 1));
    let mut mark_tasks = Vec::with_capacity(count as usize);
    for bar in 0..count {
        let store = Arc::clone(&store);
        let barrier = Arc::clone(&barrier);
        let sim_id = sim_id.to_owned();
        mark_tasks.push(tokio::spawn(async move {
            barrier.wait().await;
            let record = mark(&sim_id, bar, MarkQuality::Good, &format!("mark {bar}"));
            store.put_mark(profile(), sim_id, record).await
        }));
    }
    let store_for_decision = Arc::clone(&store);
    let barrier_for_decision = Arc::clone(&barrier);
    let sim_id_for_decision = sim_id.to_owned();
    let decision_task = tokio::spawn(async move {
        barrier_for_decision.wait().await;
        store_for_decision
            .decide(
                profile(),
                sim_id_for_decision,
                decision(DecisionKind::Stop, "stop during marks", 9),
            )
            .await
    });

    let decision = decision_task.await.expect("the decision task");
    let mut marks = Vec::with_capacity(count as usize);
    for task in mark_tasks {
        marks.push(task.await.expect("the mark task"));
    }
    (decision, marks)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_decisions_have_one_winner_in_memory() {
    let store = Arc::new(MemoryStore::default());
    let sim_id = "sim_memory_decisions";
    create(store.as_ref(), sim_id).await;

    let results = race_decisions(Arc::clone(&store), sim_id, 8).await;
    let winners = winners(&results);
    assert_eq!(winners.len(), 1, "exactly one decision must win");
    assert!(
        results
            .iter()
            .filter(|result| result.is_err())
            .all(|result| matches!(result, Err(StoreError::Locked))),
        "every losing decision must be the stable locked conflict"
    );

    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.status, SimulationStatus::Decided);
    assert_eq!(stored.decision.as_ref(), Some(winners[0]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_marks_keep_the_full_set_in_memory() {
    let store = Arc::new(MemoryStore::default());
    let sim_id = "sim_memory_marks";
    create(store.as_ref(), sim_id).await;

    race_marks(Arc::clone(&store), sim_id, 32).await;

    let marks = store
        .list_marks(profile(), sim_id.to_owned(), None)
        .await
        .expect("the mark list");
    assert_eq!(marks.len(), 32);
    assert_eq!(
        marks.iter().map(|mark| mark.bar).collect::<Vec<_>>(),
        (0..32).collect::<Vec<_>>(),
        "the mark order must stay deterministic"
    );
    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.status, SimulationStatus::Marking);
    assert_eq!(stored.human_marks, 32);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_decision_and_marks_leave_one_consistent_outcome_in_memory() {
    let store = Arc::new(MemoryStore::default());
    let sim_id = "sim_memory_mixed";
    create(store.as_ref(), sim_id).await;

    let (decision, marks) = race_marks_with_decision(Arc::clone(&store), sim_id, 16).await;
    assert!(decision.is_ok(), "the only decision must win");
    assert!(
        marks
            .iter()
            .filter(|result| result.is_err())
            .all(|result| matches!(result, Err(StoreError::Locked))),
        "a mark that loses to the decision must be refused"
    );
    let successful = marks.iter().filter(|result| result.is_ok()).count() as u64;
    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.status, SimulationStatus::Decided);
    assert_eq!(stored.human_marks, successful);
    let listed = store
        .list_marks(profile(), sim_id.to_owned(), None)
        .await
        .expect("the mark list");
    assert_eq!(listed.len() as u64, successful);
}

#[tokio::test]
async fn a_duplicate_decision_conflicts_in_memory() {
    let store = MemoryStore::default();
    let sim_id = "sim_memory_duplicate_decision";
    create(&store, sim_id).await;

    let first = store
        .decide(
            profile(),
            sim_id.to_owned(),
            decision(DecisionKind::Stop, "no edge", 1),
        )
        .await
        .expect("the first decision");
    let second = store
        .decide(profile(), sim_id.to_owned(), first.clone())
        .await;
    assert!(
        matches!(second, Err(StoreError::Locked)),
        "a duplicate decision must be refused, not treated as a success"
    );

    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.decision, Some(first));
}

#[tokio::test]
async fn a_duplicate_mark_is_idempotent_in_memory() {
    let store = MemoryStore::default();
    let sim_id = "sim_memory_duplicate_mark";
    create(&store, sim_id).await;

    let original = mark(sim_id, 8, MarkQuality::Good, "first note");
    let first = store
        .put_mark(profile(), sim_id.to_owned(), original.clone())
        .await
        .expect("the first mark");
    let second = store
        .put_mark(profile(), sim_id.to_owned(), original)
        .await
        .expect("the duplicate mark");
    assert_eq!(second.mark_id, first.mark_id);
    assert_eq!(second.created_at, first.created_at);

    let marks = store
        .list_marks(profile(), sim_id.to_owned(), None)
        .await
        .expect("the mark list");
    assert_eq!(marks.len(), 1);
}

// The Postgres cases are ignored in the fast gate. `tools/postgres_composed.sh`
// gives this target its own disposable database and runs them with one thread.

async fn postgres_store() -> (sqlx::PgPool, PgSimulationStore) {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name a disposable Postgres database; \
         this test is ignored in the fast gate",
    );
    let database = PgDatabase::connect(&url).await.expect("the Postgres pool");
    apply_migrations_in(database.pool(), "simulation", &migrations::available())
        .await
        .expect("the simulation migrations");
    (
        database.pool().clone(),
        PgSimulationStore::new(database.pool().clone()),
    )
}

async fn reset_postgres(pool: &sqlx::PgPool, sim_id: &str) {
    sqlx::query("DELETE FROM simulation.runs WHERE sim_id = $1")
        .bind(sim_id)
        .execute(pool)
        .await
        .expect("the run cleanup");
    sqlx::query("DELETE FROM simulation.retention_events WHERE record_id = $1")
        .bind(sim_id)
        .execute(pool)
        .await
        .expect("the retention-event cleanup");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn concurrent_decisions_have_one_winner_in_postgres() {
    let (pool, store) = postgres_store().await;
    let sim_id = "sim_pg_decisions";
    reset_postgres(&pool, sim_id).await;
    create(&store, sim_id).await;

    let results = race_decisions(Arc::new(store.clone()), sim_id, 8).await;
    let winners = winners(&results);
    assert_eq!(winners.len(), 1, "exactly one decision must win");
    assert!(
        results
            .iter()
            .filter(|result| result.is_err())
            .all(|result| matches!(result, Err(StoreError::Locked))),
        "every losing decision must be the stable locked conflict"
    );

    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.status, SimulationStatus::Decided);
    assert_eq!(stored.decision.as_ref(), Some(winners[0]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn concurrent_marks_keep_the_full_set_in_postgres() {
    let (pool, store) = postgres_store().await;
    let sim_id = "sim_pg_marks";
    reset_postgres(&pool, sim_id).await;
    create(&store, sim_id).await;

    race_marks(Arc::new(store.clone()), sim_id, 32).await;

    let marks = store
        .list_marks(profile(), sim_id.to_owned(), None)
        .await
        .expect("the mark list");
    assert_eq!(marks.len(), 32);
    assert_eq!(
        marks.iter().map(|mark| mark.bar).collect::<Vec<_>>(),
        (0..32).collect::<Vec<_>>(),
        "the mark order must stay deterministic"
    );
    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.status, SimulationStatus::Marking);
    assert_eq!(stored.human_marks, 32);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn a_decision_and_marks_leave_one_consistent_outcome_in_postgres() {
    let (pool, store) = postgres_store().await;
    let sim_id = "sim_pg_mixed";
    reset_postgres(&pool, sim_id).await;
    create(&store, sim_id).await;

    let (decision, marks) = race_marks_with_decision(Arc::new(store.clone()), sim_id, 16).await;
    assert!(decision.is_ok(), "the only decision must win");
    assert!(
        marks
            .iter()
            .filter(|result| result.is_err())
            .all(|result| matches!(result, Err(StoreError::Locked))),
        "a mark that loses to the decision must be refused"
    );
    let successful = marks.iter().filter(|result| result.is_ok()).count() as u64;
    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.status, SimulationStatus::Decided);
    assert_eq!(stored.human_marks, successful);
    let listed = store
        .list_marks(profile(), sim_id.to_owned(), None)
        .await
        .expect("the mark list");
    assert_eq!(listed.len() as u64, successful);
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn a_duplicate_decision_conflicts_in_postgres() {
    let (pool, store) = postgres_store().await;
    let sim_id = "sim_pg_duplicate_decision";
    reset_postgres(&pool, sim_id).await;
    create(&store, sim_id).await;

    let first = store
        .decide(
            profile(),
            sim_id.to_owned(),
            decision(DecisionKind::Test, "take it to strategy research", 1),
        )
        .await
        .expect("the first decision");
    let second = store
        .decide(profile(), sim_id.to_owned(), first.clone())
        .await;
    assert!(
        matches!(second, Err(StoreError::Locked)),
        "a duplicate decision must be refused, not treated as a success"
    );

    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.decision, Some(first));
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn the_decision_trigger_refuses_an_overwrite_in_postgres() {
    let (pool, store) = postgres_store().await;
    let sim_id = "sim_pg_terminal_trigger";
    reset_postgres(&pool, sim_id).await;
    create(&store, sim_id).await;

    let first = store
        .decide(
            profile(),
            sim_id.to_owned(),
            decision(DecisionKind::Stop, "no edge", 1),
        )
        .await
        .expect("the first decision");
    let overwrite = sqlx::query(
        "UPDATE simulation.runs
            SET decision = 'test', decision_note = 'overwrite',
                decided_at = '2026-11-22T00:00:02Z'::timestamptz
          WHERE profile_id = $1 AND sim_id = $2",
    )
    .bind(profile().as_str())
    .bind(sim_id)
    .execute(&pool)
    .await;
    assert!(overwrite.is_err(), "the schema must refuse an overwrite");

    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.decision, Some(first));
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn a_failed_mark_transaction_leaves_no_partial_state() {
    let (pool, store) = postgres_store().await;
    let sim_id = "sim_pg_failed_mark";
    reset_postgres(&pool, sim_id).await;
    create(&store, sim_id).await;

    // The trigger fails the run status update that follows the mark insert, so
    // the repository's transaction must roll both writes back.
    sqlx::query("DROP TRIGGER IF EXISTS aa_simulation_inject_marking_failure ON simulation.runs")
        .execute(&pool)
        .await
        .expect("the stale trigger cleanup");
    sqlx::query("DROP FUNCTION IF EXISTS simulation.inject_marking_failure()")
        .execute(&pool)
        .await
        .expect("the stale function cleanup");
    sqlx::query(
        "CREATE OR REPLACE FUNCTION simulation.inject_marking_failure()
         RETURNS trigger AS $$
         BEGIN
             RAISE EXCEPTION 'injected marking failure';
         END;
         $$ LANGUAGE plpgsql",
    )
    .execute(&pool)
    .await
    .expect("the failure function");
    sqlx::query(
        "CREATE TRIGGER aa_simulation_inject_marking_failure
         BEFORE UPDATE ON simulation.runs
         FOR EACH ROW
         WHEN (NEW.sim_id = 'sim_pg_failed_mark' AND NEW.status = 'marking'
               AND OLD.status = 'draft')
         EXECUTE FUNCTION simulation.inject_marking_failure()",
    )
    .execute(&pool)
    .await
    .expect("the failure trigger");

    let refused = store
        .put_mark(
            profile(),
            sim_id.to_owned(),
            mark(sim_id, 8, MarkQuality::Good, "must roll back"),
        )
        .await;
    assert!(
        matches!(refused, Err(StoreError::Backend(_))),
        "the injected failure must reach the caller"
    );

    let marks = store
        .list_marks(profile(), sim_id.to_owned(), None)
        .await
        .expect("the mark list");
    assert!(marks.is_empty(), "the mark insert must roll back");
    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.status, SimulationStatus::Draft);
    assert_eq!(stored.human_marks, 0);

    sqlx::query("DROP TRIGGER aa_simulation_inject_marking_failure ON simulation.runs")
        .execute(&pool)
        .await
        .expect("the trigger cleanup");
    sqlx::query("DROP FUNCTION simulation.inject_marking_failure()")
        .execute(&pool)
        .await
        .expect("the function cleanup");
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn a_reader_never_observes_an_uncommitted_mark_or_decision() {
    let (pool, store) = postgres_store().await;
    let sim_id = "sim_pg_read_isolation";
    reset_postgres(&pool, sim_id).await;
    create(&store, sim_id).await;

    let mut transaction = pool.begin().await.expect("the read-isolation transaction");
    sqlx::query(
        "INSERT INTO simulation.marks
             (mark_id, sim_id, profile_id, chart, bar, marked_time_utc, action,
              quality, side, origin, note, created_at)
         VALUES ('m_uncommitted', $1, $2, 0, 8, $3::timestamptz, 'entry',
                 'good', 'long', 'human', 'uncommitted', $3::timestamptz)",
    )
    .bind(sim_id)
    .bind(profile().as_str())
    .bind("2018-01-01T00:00:00Z")
    .execute(&mut *transaction)
    .await
    .expect("the uncommitted mark");
    sqlx::query(
        "UPDATE simulation.runs
            SET status = 'decided', decision = 'stop', decision_note = 'uncommitted',
                decided_at = $2::timestamptz
          WHERE profile_id = $1 AND sim_id = $3",
    )
    .bind(profile().as_str())
    .bind("2026-11-22T00:00:00Z")
    .bind(sim_id)
    .execute(&mut *transaction)
    .await
    .expect("the uncommitted decision");

    // A different pooled connection must see the committed state only.
    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read")
        .expect("the run");
    assert_eq!(stored.status, SimulationStatus::Draft);
    assert_eq!(stored.decision, None);
    let marks = store
        .list_marks(profile(), sim_id.to_owned(), None)
        .await
        .expect("the mark list");
    assert!(marks.is_empty());

    transaction.rollback().await.expect("the rollback");
    let stored = store
        .get(profile(), sim_id.to_owned())
        .await
        .expect("the run read after rollback")
        .expect("the run");
    assert_eq!(stored.status, SimulationStatus::Draft);
    assert_eq!(stored.decision, None);
}
