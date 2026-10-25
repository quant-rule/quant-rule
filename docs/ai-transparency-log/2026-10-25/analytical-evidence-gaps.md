# AI Transparency Log: Close the Analytical Evidence Gaps

- Work item: `analytical-evidence-gaps`
- Plan: [`docs/execution-plans/2026-10-25/analytical-evidence-gaps.md`](../../execution-plans/2026-10-25/analytical-evidence-gaps.md)
- Date: 2026-10-25 (UTC)

## Entry: exact-boundary evidence and status corrections

- **Date and tool:** 2026-10-25 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Add the missing proof for the full-history input,
  1,000-bar display tail, H1 warm-up, 100,000-bar cap, event retention, and
  column order. No model formula, command contract, schema, dependency, or
  trading code changed.
- **Implementation:**
  - Added deterministic generated fixtures under `target/` for 1,500 M1 bars,
    1,200 M1 bars, 12,000 M1 bars, 3,000 event-heavy M1 bars, and 100,001 cap
    rows. The checked-in fixtures were not enlarged.
  - Added a 1,500-bar early-event test: default run reports M1 bar 1499 and
    includes the event at `2025-01-01T00:14:00Z`; `--window latest` reports bar
    999 and does not include it.
  - Added a 1,200-bar `model-series --bars 1000` test: exactly 1,000 lines,
    first bar 200, last bar 1199, `window_start=200`, `window_len=1200`.
  - Added a 12,000-bar H1 test: 200 H1 bars and a non-null EMA 200.
  - Added a 100,001-row cap test for `research export` and `store ingest`;
    both reject at line 100002 and leave no dataset directory or database.
  - Extended the fetch-warning helper test to the real-world 354,442 count.
  - Added a real 3,000-bar event run: M1 batch >1,024, history exactly 1,024,
    history equals the newest batch slice, five-bar counts equal, and the store
    keeps the full M1 event count.
  - Added a `bar` insert-order guard against the DDL, select order, research
    column order, and the 74-column fixture.
  - Corrected the D2 plan and the current hardening plans to merged status, and
    documented historical AI logs as point-in-time records.
- **Verification and evidence:**
  - `cargo test`: **890 passed, 4 ignored**.
  - `cargo test -p quantrule-shell --test cli`: 71 passed.
  - `cargo test -p quantrule-model --test event_retention`: 1 passed.
  - `cargo test -p quantrule-shell --test cap_workflow`: 1 passed.
  - `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
    and `cargo doc --no-deps` with `-D warnings`: clean.
  - 100k release model: 2.530 s. Layer build 4.466 s, render 4.509 s.
  - 100k store run 1: ingest 11.173 s, verify 26.826 s, export 8.314 s; run 2:
    ingest 8.592 s, verify 32.110 s, export 8.201 s; DB 229,732,352 bytes.
- **Learning:**
  - The first CLI test draft borrowed the output of a temporary `String`; the
    compiler rejected it before the test ran. Binding the text first fixed it.
  - The 100,001-row cap test rejects before the model, so it completes in under
    a second and can stay in the default suite rather than becoming another
    ignored 100k benchmark.
  - The 100k verify timing moved by about 9 seconds between two runs with no
    production change. The observed range is recorded; the benchmark is not
    treated as a regression when the diff is test-only.
  - The early-event fixture works because the checked-in 260-bar state series
    already contains an M1 event at bar 14, which the appended synthetic rows
    cannot move.
- **Remaining:** none in the approved scope. The historical AI log status lines
  are point-in-time records; current status is in Git, GitHub, and the execution
  plans.
