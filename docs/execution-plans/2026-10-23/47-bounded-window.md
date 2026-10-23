# Execution Plan: Retain the Last 1000 Candles in a Bounded Window

- Issue: [#47](https://github.com/quant-rule/quant-rule/issues/47) — feat: retain the last 1000 candles in a bounded window
- Branch: `feat/47-bounded-window`
- Worktree: `/home/zantgo/Proyectos/quant-rule--47-bounded-window`
- Status: Verified — acceptance criteria evaluated against observed evidence; Git handoff authorized on 2026-10-23
- Date: 2026-10-23 (UTC)
- Transparency log: [`docs/ai-transparency-log/2026-10-23/47-bounded-window.md`](../../ai-transparency-log/2026-10-23/47-bounded-window.md)

## Intent

Let the data engine consume source candles one at a time and retain only the most recent 1000. The window is the engine's bounded memory: it never holds the whole source file, and a candle's identity survives eviction.

Affected users: the shell command in #48, which will push parsed rows into the window, and every later data or model stage that reads the retained series.

## Current state

- `main` is at `e05b52e`, after PR #51 merged the workspace and candle entity.
- `quantrule-data` exposes `Candle`, `RawCandle`, `CandleError`, UTC timestamp parsing/formatting, and `Timeframe::M1`.
- There is no window type, no series-index assignment, and no ordering check.
- The source data has strictly increasing timestamps, but the engine must enforce that contract rather than assume it.

## Scope

Included:

- `CandleWindow` with `DEFAULT_WINDOW = 1000`.
- `push`, `first`, `last`, `get`, `iter`, `len`, and `is_empty`.
- Stable monotonic `series_index` assigned on acceptance.
- Oldest-first eviction when a 1001st candle is accepted.
- Strictly increasing timestamps; duplicate and out-of-order input are errors.
- Unit tests for empty, single, 999, 1000, 1001, and 2500 candles; all fields; determinism; the bound; failed-push index behavior; and causality.

Explicit non-goals:

- No CSV parsing, source access, shell command, or NDJSON. Those are #48.
- No `window_index`; the shell derives it from the first retained `series_index`.
- No tolerant ingestion or quality counts. Those are #49.
- No higher timeframes, gaps, indicators, or model behavior.
- No configurable capacity. The product bound is fixed at 1000 for this slice.
- No I/O, clock, randomness, or global mutable state.

## Assumptions and uncertainty

- The shell passes each source row as `RawCandle`; the window owns parsing and index assignment.
- A rejected candle must not consume a `series_index`.
- `last_timestamp` tracks the last accepted candle even after that candle is evicted, so a later out-of-order candle is still rejected.
- `get(index)` uses the position in the retained window, not the stable `series_index`.
- A `VecDeque` is sufficient; no dependency is needed.
- The fixed capacity is small enough that preallocating it is harmless.

## Decisions

| # | Decision | Choice | Rationale |
|---|---|---|---|
| W1 | Window storage | `VecDeque<Candle>` | O(1) push and oldest eviction; ordered iteration |
| W2 | Capacity | Fixed `DEFAULT_WINDOW = 1000` | The product bound is not configurable in this slice |
| W3 | Input boundary | `push(RawCandle)` | The engine owns validation and index assignment; the shell only reads source rows |
| W4 | Index assignment | Assign only after parse and ordering checks pass | A rejected row is not part of the series |
| W5 | Ordering | Strictly increasing timestamps | Duplicates and out-of-order rows cannot form a valid series |
| W6 | Eviction | Pop the oldest before appending the 1001st | The window never exceeds 1000 after a successful push |
| W7 | Position API | `get` by retained position | The shell needs a positional view; stable identity stays on the candle |

## Acceptance criteria

- A window holds at most 1000 candles after any number of pushes.
- `first`, `last`, `get`, and `iter` expose every retained candle and every OHLCV field.
- `series_index` starts at 0 and increases by one per accepted candle; it survives eviction.
- Duplicate, out-of-order, and invalid candles are rejected without consuming an index.
- Appending later candles does not change the values of candles that remain in the window.
- Repeated construction from the same inputs produces the same window.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and `cargo doc --no-deps` are clean.
- No third-party dependency is added.

## Work breakdown

| Step | Action | Role | Paths |
|---|---|---|---|
| 1 | Write this plan and log | Orchestrator | `docs/` |
| 2 | Implement the bounded window and errors | Implementation | `crates/quantrule-data/src/window.rs` |
| 3 | Export the window and update crate docs | Implementation | `crates/quantrule-data/src/lib.rs` |
| 4 | Add the boundary, ordering, determinism, and causality tests | Test Engineer | `crates/quantrule-data/src/window.rs` |
| 5 | Run the quality gates and record evidence | Test Engineer | `crates/` |
| 6 | Review the complete diff | Code Reviewer | all |
| 7 | Commit, push, open PR after authorization | Orchestrator | git state, PR |

## Verification

Commands to run from the worktree with `~/.cargo/bin` on `PATH`:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

Expected evidence: every command exits zero; the tests cover the counts, ordering, eviction, index stability, field access, determinism, and causality named above. No result is claimed until it is observed.

Observed evidence on 2026-10-23:

- `cargo fmt --all -- --check` exited 0.
- `cargo clippy --all-targets -- -D warnings` checked the three crates and exited 0 with no warnings.
- `cargo test` exited 0: 50 unit tests passed in `quantrule-data`, 0 in `quantrule-model`, 0 in `quantrule-shell`, and 2 doctests passed. The window tests cover empty, single, 999, 1000, 1001, and 2500 candles; every field; stable indices; duplicate, out-of-order, and invalid rejection; eviction-safe ordering; retained-position access; iteration order; determinism; the bound; and causality.
- `cargo doc --no-deps` generated documentation for all three crates and exited 0.
- `Cargo.lock` is unchanged and lists only the three workspace packages.
- None of these verification commands read `data/`; the tests use generated timestamps and in-code candle fields.

The acceptance criteria are met. Commit, push, and pull request remain subject to the human authorization gate.

## Risks and recovery

- **A fixed window can silently hide history.** Mitigation: the bound is explicit, tested, and documented; the shell prints only the retained window in #48.
- **A rejected candle could consume an index.** Mitigation: tests assert the next accepted candle keeps the expected index after each rejection.
- **An out-of-order candle could slip in after eviction.** Mitigation: `last_timestamp` is retained independently of the candle buffer.
- **The `VecDeque` preallocation is wasted on an empty run.** Mitigation: 1000 candles is small, and the bound is the product requirement.
- **Recovery.** The change is additive to `quantrule-data`; it can be reverted with a new commit on `main`. The worktree is preserved until the PR is accepted.

## Stop conditions

Stop and request human direction if:

- A configurable window or a different capacity is required.
- The shell needs to assign `series_index` instead of the window.
- A dependency is needed.
- Verification cannot be completed with the installed toolchain.
