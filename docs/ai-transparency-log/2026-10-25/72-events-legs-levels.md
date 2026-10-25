# AI Transparency Log: Persist Events, Legs, and Level Records (ST8)

- Work item: [#72](https://github.com/quant-rule/quant-rule/issues/72) — feat: persist events, legs, and level records
- Plan: [`docs/execution-plans/2026-10-25/72-events-legs-levels.md`](../../execution-plans/2026-10-25/72-events-legs-levels.md)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Date: 2026-10-25 (UTC)

## Entry: event, leg, and level records

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Extend `store ingest` so a dataset also carries the
  model's event records and progression leg records, and confirm that #69's
  confirmed-level records still round-trip. The writer copies the analytics
  engine's flat rows; it does not detect an event, compute a leg, assign a
  level, or alter a metric. The approved scope grew once, to version 2 of the
  event indexes, because the version 1 key could not represent every event the
  model publishes.
- **Implementation:**
  - Added `EventRow`, `LegRow`, `event_rows`, and `leg_rows` to
    `crates/quantrule-analytics/src/layers/records.rs`; `event_rows` now breaks
    ties on `reference_value` so coincident Signal threshold crossings have a
    total order.
  - Added `events` and `legs` to `LayerSet` and `TimeframeSeries`, and
    collected them during `build_layers`.
  - Added `IngestRows.events` and `IngestRows.legs`, `EVENT_INSERT`,
    `LEG_INSERT`, parameter bindings, and pre-transaction validation for shape,
    contract ranges, availability, and duplicate keys.
  - Encoded a missing event reference as `0`, which is outside the valid
    reference domain (`1..=3`, `>= 1001`).
  - Added schema version 2. `event_key` is now partial and excludes Signal
    threshold crossings; `event_threshold_key` covers the same columns plus
    `reference_value` for `(event_type_code = 3, event_subtype_code = 1)`.
    `Store::init` migrates a version 1 database in one transaction.
  - The writer's duplicate key normalizes `reference_value` bits for threshold
    crossings and rejects a threshold event without a finite value.
- **Verification and evidence:**
  - `cargo test -p quantrule-analytics layers::tests`: 22 passed, 0 failed,
    1 ignored.
  - `cargo test -p quantrule-shell --bin quantrule store::ingest`: 34 passed,
    0 failed.
  - `cargo test -p quantrule-shell --test store_aggregates`: 1 passed,
    0 failed. The database holds 624 events, 217 legs, and 94 levels; the
    availability and duplicate-identity queries return zero; a second ingest
    prints `unchanged demo bars=333 timeframes=4` and leaves the counts
    unchanged.
  - `cargo test --workspace`: 818 passed, 0 failed, 3 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - README, the ADR, and the plan were updated with the event rows, the leg
    rows, schema version 2, and the observed evidence.
- **Learning:**
  - The new analytics uniqueness test produced the blocker before any commit:
    the 300-bar synthetic fixture emits three RSI threshold crossings at bar
    111 that share every version 1 key column and differ only in
    `reference_value` (30, 50, 70). The checked-in `state_m1.csv` does not
    trigger the collision, so the real-binary audit alone would not have found
    it. The reproducer is retained as a regression test.
  - Option 1 was recommended and approved: keep the original key for every
    non-threshold event, add a partial threshold key, and migrate the index.
    Dropping the unique index was rejected because it loses the duplicate
    invariant. Adding `reference_value` to one key for every event was rejected
    because non-threshold rows carry no value and a stray value would create a
    second identity. Reserving a `reference_code` per threshold was rejected
    because it changes the model contract and #75's reference mapping.
  - The writer's in-memory duplicate check now mirrors the two partial indexes.
    If the two drift, the writer would either reject valid model output or let
    SQLite reject a row after the transaction opens. The writer test covers
    both the three-threshold case and a duplicate threshold.
  - `reference_value` had to become a total-order tie-break, not only an
    identity component. Without it, the three threshold rows compared equal in
    the old sort key and their stored order depended on input order.
  - The CLI test expected `schema=1`; version 2 changed the observable output to
    `schema=2`, so the expectation was updated rather than the migration being
    hidden.
  - Clippy flagged two pre-existing `map_or(true, ...)` calls in the touched
    analytics test. They were replaced with `is_none_or`; no lint was
    suppressed.
  - Role passes were sequential in one session, not independent agents. No
    separate code-review or security-review agent was run; the human owns
    review and merge.

## Final verification

- Analytics layer tests: 22 passed, 1 ignored.
- Writer unit tests: 34 passed.
- Binary record audit: 1 passed.
- `cargo test --workspace`: 818 passed, 0 failed, 3 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Pull request: [#105](https://github.com/quant-rule/quant-rule/pull/105).
- Status: implementation complete and locally verified; pull request open for
  human review.
