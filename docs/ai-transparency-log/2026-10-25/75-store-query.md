# AI Transparency Log: Store Query and Inspection Commands

- Work item: [#75](https://github.com/quant-rule/quant-rule/issues/75) — feat: add store query and inspection commands
- Plan: [`docs/execution-plans/2026-10-25/75-store-query.md`](../../execution-plans/2026-10-25/75-store-query.md)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Date: 2026-10-25 (UTC)

## Entry: read-only store inspection

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Make the stored dataset inspectable through a
  read-only shell surface. The commands read stored values and render them; they
  do not recompute a reading, metric, resolution, aggregate, event, or leg, and
  they never write, migrate, or create a database.
- **Implementation:**
  - Added `Store::open_read_only`, which rejects a missing file and a newer
    schema, and never runs a migration.
  - Added `store/query.rs` with a closed `Table` enum, deterministic primary-key
    ordering, typed `Cell`/`TableRows`, dataset summaries, candle series, metric
    projections, and the event reference sentinel conversion.
  - Added `store list`, `store show`, and the `db series/bars/metrics/layer3`
    views. `store list` and `store show` print CSV; `db series` prints NDJSON;
    the wide `db` views print CSV.
  - Added `command/store_query.rs` for parsing and rendering so `command.rs`
    contributes only dispatch, help, and the argument types.
  - Added `--timeframe`, `--bar`, `--limit`, and `--db` filters. `--limit` is
    `1..=1000` with a default of 100.
  - Added parser tests, store query tests, and binary exact-output/read-only
    tests. README and the help fixture were updated.
- **Verification and evidence:**
  - `cargo test -p quantrule-shell store::query`: 9 passed.
  - `cargo test -p quantrule-shell command::store_query`: 5 passed.
  - `cargo test -p quantrule-shell --test store_commands`: 7 passed.
  - `cargo test --workspace`: 861 passed, 0 failed, 3 ignored.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` are clean.
  - The binary test over `state_m1.csv` reads all seven tables, exact headers
    (74/7/285/8/8/12/19), 61 metric columns, and the 285-column Layer 3 view.
- **Learning:**
  - The event table stores a missing reference as `0` because the unique index
    needs a non-null value. The query layer converts that sentinel back to an
    empty CSV field; exposing the raw `0` would leak an implementation detail and
    put a value outside the contract's reference domain in front of the user.
  - The roadmap called for a combined surface. `store show` keeps every table
    reachable and the `db` views provide the per-bar projections the issue
    names; both use the same query layer, so they cannot disagree about ordering
    or null rendering.
  - A read command must not call `Store::open`: that would create an empty
    database and migrate it. The read-only path was tested against a missing
    file specifically to prove no file appears.
  - CSV uses an empty field for `NULL`, matching the research export. A valid
    empty text field is therefore indistinguishable from null in this output;
    the dictionary/contract already defines that encoding.
  - `store list` uses correlated counts over the indexed dataset id. This is
    bounded by the number of datasets; the 100k query timing remains #74's
    evidence, not a claim here.
  - Role passes were sequential in one session, not independent agents. The
    human owns review and merge.

## Final verification

- Store query tests: 9 passed.
- Query parser tests: 5 passed.
- Binary store-command tests: 7 passed.
- `cargo test --workspace`: 861 passed, 0 failed, 3 ignored.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Pull request: pending human authorization.
- Status: implementation complete and locally verified; commit, push, and pull
  request await human authorization.
