# Execution Plan: Close the Analytical Evidence Gaps

- Issue: none; branch slug used
- Branch: `test/analytical-evidence-gaps`
- Worktree: `analytical-evidence-gaps` (opencode worktree)
- Pull request: [#114](https://github.com/quant-rule/quant-rule/pull/114)
- Status: Verified; PR #114 is open
- Date: 2026-10-25 (UTC)
- Related: [history policy ADR](../../adr/2026-10-25/history-retention-and-display-policy.md), [remaining hardening](harden-analytics-remaining.md)

## 1. Intent

Make the remaining hardening claims true at their exact boundaries. The code paths exist; the current tests use shorter fixtures or unit-level objects. This work adds the missing evidence without changing command behavior, model formulas, the schema, or dependencies.

## 2. Current state

- The model commands use the full supported series by default; `--window latest` uses the 1,000-bar tail.
- The cap is 100,000 M1 bars; fetch warns above it.
- `EventBatch` is full; `EventHistory` is bounded to 1,024 records per timeframe.
- The longest shell fixture is 1,001 M1 bars, so H1 never reaches EMA 200 and `model-series` never crosses the 1,000-bar tail boundary.
- The retention unit test uses synthetic records, not a real model run.
- `docs/execution-plans/2026-10-25/d2-command-modules.md` said PR #113 was open.

## 3. Scope

Included:
- a 1,500-bar fixture with an early event, plus default versus `--window latest` assertions;
- a 1,200-bar `model-series --bars 1000` boundary test;
- an H1 fixture long enough for EMA 200;
- a 100,001-row cap test for `research export` and `store ingest`;
- a 354,442-row fetch-warning assertion at the shell boundary;
- a real >1,024-event model run and store/CLI retention test;
- a `bar` insert-order test against the checked-in DDL and research column order;
- documentation status corrections.

Excluded:
- model formula changes;
- schema changes;
- new dependencies;
- strategy, execution, portfolio, or backtest code;
- changes to #10, #11, or #12.

## 4. Acceptance criteria

- A 1,500-bar run reports M1 bar 1499 by default and includes the early M1 event; `--window latest` reports bar 999 and omits that event.
- A 1,200-bar run prints 1,000 `model-series` bars, first bar 200, last bar 1199, `window_start=200`, `window_len=1200`.
- A 12,000-bar M1 run produces at least 200 H1 bars and a non-null H1 EMA 200.
- A 100,001-row CSV is rejected by both `research export` and `store ingest`, with no dataset directory and no database file.
- The fetch cap warning is asserted for the 354,442-row real-world count.
- A real run with more than 1,024 M1 events has a 1,024-record history, a larger full batch, and the newest records retained; `events` and the store retain the full batch.
- The `bar` insert order equals the DDL order and the research column order.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc` pass.

## 5. Work breakdown

1. Added generated fixture helpers and the full-history model tests in `crates/quantrule-shell/tests/cli.rs`.
2. Added the event-retention model test in `crates/quantrule-model/tests/event_retention.rs`.
3. Added the real event store/CLI test in `crates/quantrule-shell/tests/cli.rs`.
4. Added the 100,001-row cap workflow test in `crates/quantrule-shell/tests/cap_workflow.rs`; it runs by default and rejects before the model or database is opened.
5. Added the `bar` insert/select/DDL/research order guard in `crates/quantrule-shell/src/store/ingest/tests.rs`.
6. Corrected the D2 plan and the current hardening plans to merged status.
7. Added a point-in-time note for historical AI logs in `docs/README.md`.

## 6. Verification

| Check | Result |
|---|---|
| Full-history model | `cargo test -p quantrule-shell --test cli`: 71 passed, including the 1,500-bar early event, 1,200-bar tail, and H1 EMA 200 cases |
| Event retention model | `cargo test -p quantrule-model --test event_retention`: 1 passed; batch >1,024, history exactly 1,024, newest slice retained, five-bar counts equal |
| Event retention store/CLI | M1 `events` count >1,024 equals the stored M1 event count |
| Cap workflow | `cargo test -p quantrule-shell --test cap_workflow`: 1 passed; both commands rejected 100,001 rows with no export directory or database |
| Fetch warning | helper assertion covers 100,001 and the real-world 354,442 count |
| Column order | insert order equals DDL, select, research order, and the 74-column fixture |
| Full gate | **890 passed, 4 ignored**; `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo doc --no-deps` with `-D warnings` clean |
| 100k model | 2.530 s |
| 100k layer/export | build 4.466 s, render 4.509 s |
| 100k store | run 1: ingest 11.173 s, verify 26.826 s, export 8.314 s; run 2: ingest 8.592 s, verify 32.110 s, export 8.201 s; DB 229,732,352 bytes |
| Status audit | D2 and current hardening plans read merged; historical AI log status lines are documented as point-in-time |

## 7. Risks and recovery

- The 12,000-bar H1 fixture is generated deterministically under `target/`; it adds about a second to the CLI suite. The 100,001-row cap test rejects before the model, so it runs by default in under a second.
- The >1,024-event synthetic series is deterministic but its exact event count is not asserted; only the retention relationship is.
- The 100k verify timing varies materially between runs on this machine; the production code is unchanged, so the range is recorded rather than treated as a regression.
- If the early event does not survive the extension, the fixture design is wrong; stop and reassess rather than weakening the assertion.

## 8. Stop conditions

- A model value changes unexpectedly.
- A test requires a schema change, a new dependency, or trading code.
- The 100,001-row path creates a partial artifact.
- The event history drops a record from the full batch.

## 9. Review

- AI transparency: [`docs/ai-transparency-log/2026-10-25/analytical-evidence-gaps.md`](../../ai-transparency-log/2026-10-25/analytical-evidence-gaps.md).
- The human owns review and merge authorization for PR #114.
