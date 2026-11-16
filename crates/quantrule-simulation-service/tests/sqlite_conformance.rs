//! The shared `SimulationStore` conformance suite.
//!
//! The same body runs against the SQLite store always and against the Postgres
//! store when `QR_TEST_DATABASE_URL` names a disposable database. The fast tier
//! runs the SQLite case; the Postgres case is opt-in because it needs a live
//! server. The suite asserts the repository contract, not a SQL dialect:
//! profile isolation, the status transitions, the mark identity, oracle
//! replacement, check idempotence, and the one-shot decision.

use std::collections::BTreeMap;

use quantrule_api::{
    Digest, MarkAction, MarkOrigin, MarkQuality, ProfileId, Side, SimulationStatus,
};
use quantrule_runtime::adapters::postgres::{
    PgDatabase, apply_migrations_in as apply_postgres_migrations,
};
use quantrule_runtime::adapters::sqlite::{SqliteDatabase, apply_migrations_in};
use quantrule_simulation::check::Thresholds;
use quantrule_simulation::{
    CheckConfig, CheckRecord, CheckVerdict, Control, OracleSpec, Selection, SideFilter,
    WindowEvent, WindowRecord, WindowSpec,
};
use quantrule_simulation_service::postgres::PgSimulationStore;
use quantrule_simulation_service::records::{
    DecisionKind, DecisionRecord, MarkRecord, NewSimulation,
};
use quantrule_simulation_service::sqlite::SqliteSimulationStore;
use quantrule_simulation_service::store::{SimulationStore, StoreError};

const PROFILE: &str = "conformance-sim-a";
const OTHER: &str = "conformance-sim-b";

fn profile_id(name: &str) -> ProfileId {
    ProfileId::parse(name).expect("the profile id")
}

fn digest(seed: u8) -> Digest {
    Digest::parse(&format!("{seed:064x}")).expect("the digest")
}

fn window_spec() -> WindowSpec {
    WindowSpec {
        event_codes: vec![149],
        window_bars: 400,
        candidate_zone: (40, 360),
        max_windows: 5,
        min_windows: 2,
        seed: 0,
        overlap_bars: 0,
        from_ts: None,
        to_ts: None,
    }
}

fn oracle_spec() -> OracleSpec {
    OracleSpec {
        horizon_bars: 20,
        target_atr: 2.0,
        stop_atr: 1.0,
        bad_stop_atr: 2.0,
        bad_target_atr: 1.0,
        entry_offset_max: 1,
    }
}

fn check_config(sim_id: &str) -> CheckConfig {
    CheckConfig {
        run_id: sim_id.to_owned(),
        event_code: 149,
        from: 1_514_764_800,
        to: 1_515_000_000,
        from_text: "2020-01-01T00:00:00Z".to_owned(),
        to_text: "2020-02-01T00:00:00Z".to_owned(),
        horizon_bars: 20,
        confirmation_bars: 1,
        cost_bps: 5.0,
        control: Control::Event,
        sides: BTreeMap::from([(
            "long".to_owned(),
            SideFilter {
                direction_only: Some(true),
                ..SideFilter::default()
            },
        )]),
    }
}

fn new_simulation(sim_id: &str, created_at: &str) -> NewSimulation {
    NewSimulation {
        sim_id: sim_id.to_owned(),
        premise: "a fib level needs a close back on the entry side".to_owned(),
        bundle: digest(1),
        inputs: "EMA stack and RSI zone".to_owned(),
        inputs_digest: digest(2),
        seed: 0,
        criteria: "min_events=30; mean net > 0".to_owned(),
        window_spec: window_spec(),
        oracle_spec: oracle_spec(),
        check_config: check_config(sim_id),
        created_at: created_at.to_owned(),
    }
}

fn window(name: &str, start: usize) -> WindowRecord {
    let start_index = start;
    let end_index = start + 400;
    let candidate = start + 80;
    WindowRecord {
        window_id: name.to_owned(),
        start_index,
        end_index,
        start_bar: u64::try_from(start).expect("a bar"),
        end_bar: u64::try_from(end_index - 1).expect("a bar"),
        start_time: "2020-01-01T00:00:00Z".to_owned(),
        end_time: "2020-01-05T00:00:00Z".to_owned(),
        candidate_index: candidate,
        candidate_bar: u64::try_from(candidate).expect("a bar"),
        candidate_time: "2020-01-02T00:00:00Z".to_owned(),
        events: vec![WindowEvent {
            event_index: u64::try_from(start).expect("an event"),
            code: 149,
            name: "level.break".to_owned(),
            type_code: 1,
            subtype_code: 1,
            source_code: 1,
            occurred_bar: u64::try_from(candidate).expect("a bar"),
            available_bar: u64::try_from(candidate).expect("a bar"),
            direction: 1,
            primary: Some(100.0),
            reference: Some(99.0),
        }],
    }
}

fn selection() -> Selection {
    Selection {
        schema: "quantrule.simulation-windows/v1".to_owned(),
        timeframe_secs: 900,
        window_bars: 400,
        candidate_zone: [40, 360],
        max_windows: 5,
        min_windows: 2,
        seed: 0,
        overlap_bars: 0,
        event_codes: vec![149],
        event_names: vec!["level.break".to_owned()],
        from_time: "2020-01-01T00:00:00Z".to_owned(),
        to_time: "2020-02-01T00:00:00Z".to_owned(),
        candidate_count: 12,
        selected_count: 2,
        underpowered: false,
        windows: vec![window("w001", 0), window("w002", 400)],
    }
}

fn mark(
    mark_id: &str,
    origin: MarkOrigin,
    chart: u64,
    bar: u64,
    quality: MarkQuality,
    note: &str,
) -> MarkRecord {
    MarkRecord {
        mark_id: mark_id.to_owned(),
        chart,
        bar,
        marked_time_utc: "2026-11-15T00:00:00Z".to_owned(),
        action: MarkAction::Entry,
        quality,
        side: Side::Long,
        origin,
        note: note.to_owned(),
        created_at: "2026-11-15T00:00:00Z".to_owned(),
    }
}

fn check_record(sim_id: &str) -> CheckRecord {
    CheckRecord {
        schema: "quantrule.simulation-check/v1".to_owned(),
        run_id: sim_id.to_owned(),
        event: "level.break".to_owned(),
        event_code: 149,
        horizon_bars: 20,
        confirmation_bars: 1,
        control: "event".to_owned(),
        cost_bps: 5.0,
        from_text: "2020-01-01T00:00:00Z".to_owned(),
        to_text: "2020-02-01T00:00:00Z".to_owned(),
        data_version: "conformance".to_owned(),
        model: "conformance".to_owned(),
        registry_version: "1".to_owned(),
        thresholds: Thresholds {
            min_events: 30,
            min_positive_half_years: 3,
        },
        candidate_count: 40,
        results: Vec::new(),
        verdict: CheckVerdict::NoPattern,
        claim: "a screen, not a backtest".to_owned(),
    }
}

fn decision(kind: DecisionKind, note: &str, decided_at_utc: &str) -> DecisionRecord {
    DecisionRecord {
        decision: kind,
        note: note.to_owned(),
        decided_at_utc: decided_at_utc.to_owned(),
    }
}

async fn conformance(store: &dyn SimulationStore) {
    let profile = profile_id(PROFILE);
    let other = profile_id(OTHER);

    // A new run starts in draft, and a repeated create returns the stored row.
    let first = new_simulation("sim_conformance_1", "2026-11-15T00:00:01Z");
    let created = store
        .create(profile.clone(), first.clone())
        .await
        .expect("the first create");
    assert_eq!(created.status, SimulationStatus::Draft);
    assert_eq!(created.sim_id, "sim_conformance_1");
    assert_eq!(created.window_spec, first.window_spec);
    assert_eq!(created.oracle_spec, first.oracle_spec);
    assert_eq!(created.check_config, first.check_config);
    assert_eq!(created.selection, None);
    assert_eq!(created.human_marks, 0);
    assert_eq!(created.oracle_marks, 0);
    assert_eq!(created.marked_windows, 0);
    assert!(created.created_at.ends_with('Z'), "{}", created.created_at);

    let repeated = store
        .create(profile.clone(), first.clone())
        .await
        .expect("a repeated create");
    assert_eq!(repeated.created_at, created.created_at);
    assert_eq!(repeated.status, SimulationStatus::Draft);

    // The same id under another profile is a conflict, not a second row.
    let clash = store.create(other.clone(), first.clone()).await;
    assert!(
        matches!(clash, Err(StoreError::Conflict(_))),
        "a cross-profile id was accepted: {clash:?}"
    );
    assert!(
        store
            .get(other.clone(), first.sim_id.clone())
            .await
            .expect("the other profile get")
            .is_none()
    );

    // The list is newest first and carries the selection count.
    let second = new_simulation("sim_conformance_2", "2026-11-15T00:00:02Z");
    store
        .create(profile.clone(), second)
        .await
        .expect("the second create");
    let listed = store
        .list(profile.clone(), None, 10)
        .await
        .expect("the list");
    assert_eq!(
        listed
            .iter()
            .map(|row| row.sim_id.to_string())
            .collect::<Vec<_>>(),
        vec!["sim_conformance_2", "sim_conformance_1"]
    );
    assert_eq!(listed[1].windows, 0);
    assert_eq!(
        store
            .list(profile.clone(), Some(SimulationStatus::Draft), 10)
            .await
            .expect("the draft list")
            .len(),
        2
    );

    // Selection advances draft to marking and is stored once.
    let saved = store
        .save_windows(profile.clone(), first.sim_id.clone(), selection())
        .await
        .expect("the window save");
    assert_eq!(saved.status, SimulationStatus::Marking);
    assert_eq!(saved.windows().len(), 2);
    let listed = store
        .list(profile.clone(), None, 10)
        .await
        .expect("the list after selection");
    assert_eq!(listed[1].windows, 2);

    // A human mark is inserted once; a second write of the same identity
    // updates that row rather than adding a second one.
    let first_mark = mark(
        "mark-a",
        MarkOrigin::Human,
        0,
        30,
        MarkQuality::Good,
        "first",
    );
    let stored = store
        .put_mark(profile.clone(), first.sim_id.clone(), first_mark.clone())
        .await
        .expect("the first mark");
    assert_eq!(stored.mark_id, "mark-a");
    assert_eq!(stored.note, "first");
    let mut changed = first_mark;
    changed.mark_id = "mark-a-ignored".to_owned();
    changed.quality = MarkQuality::Bad;
    changed.note = "changed".to_owned();
    let updated = store
        .put_mark(profile.clone(), first.sim_id.clone(), changed)
        .await
        .expect("the mark update");
    assert_eq!(updated.mark_id, "mark-a", "the identity is the stored id");
    assert_eq!(updated.quality, MarkQuality::Bad);
    assert_eq!(updated.note, "changed");

    let second_mark = mark(
        "mark-b",
        MarkOrigin::Human,
        1,
        10,
        MarkQuality::Neutral,
        "second",
    );
    store
        .put_mark(profile.clone(), first.sim_id.clone(), second_mark)
        .await
        .expect("the second mark");
    let run = store
        .get(profile.clone(), first.sim_id.clone())
        .await
        .expect("the run get")
        .expect("the stored run");
    assert_eq!(run.human_marks, 2);
    assert_eq!(run.oracle_marks, 0);
    assert_eq!(run.marked_windows, 2);

    let marks = store
        .list_marks(profile.clone(), first.sim_id.clone(), None)
        .await
        .expect("the mark list");
    assert_eq!(
        marks
            .iter()
            .map(|mark| mark.mark_id.as_str())
            .collect::<Vec<_>>(),
        vec!["mark-a", "mark-b"]
    );
    assert_eq!(marks[0].marked_time_utc, "2026-11-15T00:00:00Z");
    assert_eq!(
        store
            .list_marks(
                profile.clone(),
                first.sim_id.clone(),
                Some(MarkOrigin::Oracle)
            )
            .await
            .expect("the oracle filter")
            .len(),
        0
    );

    assert!(
        store
            .delete_mark(profile.clone(), first.sim_id.clone(), "mark-b".to_owned())
            .await
            .expect("the mark delete")
    );
    assert!(
        !store
            .delete_mark(profile.clone(), first.sim_id.clone(), "mark-b".to_owned())
            .await
            .expect("the repeated delete")
    );

    // Oracle replacement is idempotent and keeps human marks.
    let oracle_marks = vec![
        mark(
            "oracle-a",
            MarkOrigin::Oracle,
            0,
            31,
            MarkQuality::Good,
            "oracle",
        ),
        mark(
            "oracle-b",
            MarkOrigin::Oracle,
            1,
            11,
            MarkQuality::Bad,
            "oracle",
        ),
    ];
    let first_oracle = store
        .replace_oracle(profile.clone(), first.sim_id.clone(), oracle_marks.clone())
        .await
        .expect("the oracle run");
    let second_oracle = store
        .replace_oracle(profile.clone(), first.sim_id.clone(), oracle_marks)
        .await
        .expect("the repeated oracle run");
    assert_eq!(first_oracle, second_oracle);
    assert_eq!(first_oracle.len(), 2);
    let run = store
        .get(profile.clone(), first.sim_id.clone())
        .await
        .expect("the run get")
        .expect("the stored run");
    assert_eq!(run.status, SimulationStatus::Reviewed);
    assert_eq!(run.oracle_marks, 2);
    assert_eq!(run.human_marks, 1);

    // The check is idempotent for the same record, and a later oracle run does
    // not move a checked run backward.
    let check = check_record(&first.sim_id);
    store
        .record_check(profile.clone(), first.sim_id.clone(), check.clone())
        .await
        .expect("the check");
    store
        .record_check(profile.clone(), first.sim_id.clone(), check.clone())
        .await
        .expect("the repeated check");
    let run = store
        .get(profile.clone(), first.sim_id.clone())
        .await
        .expect("the run get")
        .expect("the stored run");
    assert_eq!(run.status, SimulationStatus::Checked);
    assert_eq!(run.check_result, Some(check.clone()));
    store
        .replace_oracle(
            profile.clone(),
            first.sim_id.clone(),
            vec![mark(
                "oracle-c",
                MarkOrigin::Oracle,
                0,
                31,
                MarkQuality::Neutral,
                "oracle again",
            )],
        )
        .await
        .expect("the oracle after the check");
    let run = store
        .get(profile.clone(), first.sim_id.clone())
        .await
        .expect("the run get")
        .expect("the stored run");
    assert_eq!(run.status, SimulationStatus::Checked);

    // A decision lands once. The same decision is idempotent and keeps its
    // original time; a different decision is refused.
    let recorded = decision(
        DecisionKind::Refine,
        "one condition only",
        "2026-11-15T00:00:09Z",
    );
    let stored = store
        .decide(profile.clone(), first.sim_id.clone(), recorded.clone())
        .await
        .expect("the decision");
    assert_eq!(stored, recorded);
    let repeated = store
        .decide(
            profile.clone(),
            first.sim_id.clone(),
            decision(
                DecisionKind::Refine,
                "one condition only",
                "2026-11-15T00:01:00Z",
            ),
        )
        .await
        .expect("the repeated decision");
    assert_eq!(repeated.decided_at_utc, recorded.decided_at_utc);
    let changed = store
        .decide(
            profile.clone(),
            first.sim_id.clone(),
            decision(DecisionKind::Stop, "no edge", "2026-11-15T00:01:00Z"),
        )
        .await;
    assert_eq!(changed, Err(StoreError::Locked));

    // Every mark mutation is closed, while the check may still be refreshed.
    assert_eq!(
        store
            .put_mark(
                profile.clone(),
                first.sim_id.clone(),
                mark("mark-c", MarkOrigin::Human, 0, 40, MarkQuality::Good, ""),
            )
            .await,
        Err(StoreError::Locked)
    );
    assert_eq!(
        store
            .delete_mark(profile.clone(), first.sim_id.clone(), "mark-a".to_owned())
            .await,
        Err(StoreError::Locked)
    );
    assert_eq!(
        store
            .replace_oracle(profile.clone(), first.sim_id.clone(), Vec::new())
            .await,
        Err(StoreError::Locked)
    );
    assert!(matches!(
        store
            .save_windows(profile.clone(), first.sim_id.clone(), selection())
            .await,
        Err(StoreError::Locked)
    ));
    store
        .record_check(profile.clone(), first.sim_id.clone(), check.clone())
        .await
        .expect("the check after the decision");
    let run = store
        .get(profile.clone(), first.sim_id.clone())
        .await
        .expect("the run get")
        .expect("the stored run");
    assert_eq!(run.status, SimulationStatus::Decided);
    assert_eq!(run.decision, Some(recorded));
    assert_eq!(run.check_result, Some(check));
    assert_eq!(
        store
            .list(profile.clone(), Some(SimulationStatus::Decided), 10)
            .await
            .expect("the decided list")
            .len(),
        1
    );

    // The other profile is empty.
    assert!(
        store
            .list(other, None, 10)
            .await
            .expect("the other profile list")
            .is_empty()
    );
}

async fn sqlite_store() -> SqliteSimulationStore {
    let database = SqliteDatabase::open_in_memory().expect("the database opens");
    apply_migrations_in(
        &database,
        "simulation",
        &quantrule_simulation_service::migrations::sqlite(),
    )
    .await
    .expect("the simulation baseline");
    SqliteSimulationStore::new(database)
}

#[tokio::test]
async fn the_sqlite_store_conforms() {
    conformance(&sqlite_store().await).await;
}

#[tokio::test]
#[ignore = "needs QR_TEST_DATABASE_URL pointing at a disposable Postgres"]
async fn the_postgres_store_conforms() {
    let url = std::env::var("QR_TEST_DATABASE_URL").expect(
        "QR_TEST_DATABASE_URL must name a disposable Postgres database; \
         this test is ignored in the fast gate",
    );
    let database = PgDatabase::connect(&url).await.expect("the Postgres pool");
    apply_postgres_migrations(
        database.pool(),
        "simulation",
        &quantrule_simulation_service::migrations::available(),
    )
    .await
    .expect("the simulation migrations");
    for profile in [PROFILE, OTHER] {
        sqlx::query("DELETE FROM simulation.marks WHERE profile_id = $1")
            .bind(profile)
            .execute(database.pool())
            .await
            .expect("the mark cleanup");
        sqlx::query("DELETE FROM simulation.runs WHERE profile_id = $1")
            .bind(profile)
            .execute(database.pool())
            .await
            .expect("the run cleanup");
    }
    conformance(&PgSimulationStore::new(database.pool().clone())).await;
}
