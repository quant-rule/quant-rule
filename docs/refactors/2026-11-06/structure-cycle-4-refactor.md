# Structure Re-Scan — Fourth Refactor Campaign

- Date: 2026-11-06 (UTC)
- Baseline commit: `9ac6565`
- Scope: current Rust workspace, tests, documentation, tools, and repository layout
- Workflow: [`docs/workflows/refactor-workflow.md`](../../workflows/refactor-workflow.md)
- Format: [`docs/refactors/README.md`](../README.md)
- Predecessor: [`structure-cycle-3-refactor.md`](structure-cycle-3-refactor.md)

## Baseline

| Item | Observed |
|---|---|
| Commit | `9ac6565` (`git rev-parse HEAD`) |
| Toolchain | `rustc 1.99.1`, `cargo 1.99.1`, `Python 3.14.8` |
| Test inventory | 1,312 passed, 0 failed, 8 ignored (`cargo test --workspace --no-fail-fast`) |
| Golden fixtures | 105 files; combined SHA256 `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1` |
| Report bundle | `report model` over `crates/quantrule-shell/tests/fixtures/config-run/run.json`; SHA256 `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512` |
| Backtest bundle | `backtest run` over the same config and `strategy/event-composite.json`; SHA256 `4173b5055814048597ee5e946da9b117e34dadf5f867de750aaa2f2cf15f085e` |
| Structural report | 199 `src` files / 63,873 lines; 237 files / 75,602 lines including integration tests |
| Largest file | `crates/quantrule-backtest/src/portfolio/ledger_tests.rs`, 973 lines (test-only) |
| Files over 800 lines | 7; two have substantial production code (`spec/v2.rs` 813, `events/signals.rs` 685), the rest are test-heavy or test-only |
| Files over 600 lines | 34 |
| Files over 1,000 lines | 0 |
| `pub` / `pub(crate)` items | 1,653 / 798 |
| `unwrap`/`expect` lines in non-test source | 315, heuristic; the count includes inline test modules and doc examples |
| Gates | fmt, clippy, test, doc, file size, structure, registry, validation, verification matrix, verification evidence, emitters, command surface, links, terms, legacy, oracle, progression, improvement, research, program, coverage, and diff check pass |

Reproduction:

```bash
git rev-parse HEAD
rustc --version && cargo --version && python3 --version
cargo test --workspace --no-fail-fast
find crates -path '*/fixtures/*' -type f | sort | xargs sha256sum | sha256sum
python3 tools/check_structure.py --report
```

The report and backtest bundle hashes use the methods defined in the cycle-3
assessment; both are unchanged.

## Findings

| ID | Impact | Area | Finding | Evidence | Principle | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| REF-2026-11-06-03 | Medium | shell | Date windows are owned by the `source` adapter and imported by non-source modules | `source/date.rs` is 322 lines and defines `DateRange`, `DateBounds`, and `DateRangeError`; 10 date-type references go through `crate::source`: `config/data.rs:13-14`, `csv.rs:22`, `backtest_bundle.rs:20`, `pipeline.rs:19`, `request.rs:14`, `fetch.rs:17`, `command/fetch.rs:17`, `command/tests.rs:9`, `source/dukascopy.rs:475`, `source/mod.rs:19` | boundary, coupling | A move `source/date.rs` to `crate::date`; B move only the bounds and error to `config`; C leave | A, this cycle | import guard and config/CSV/fetch tests | Planned | [#531](https://github.com/quant-rule/quant-rule/issues/531) |
| REF-2026-11-06-04 | Info | shell | The public-surface and panic counts are heuristics, not findings | `check_structure.py` counts every `pub` line and every `unwrap`/`expect` line, including inline test modules; the shell has no executable `unwrap`/`expect` outside test modules | surface, panic hygiene | A compiler-based count and ratchet; B leave | B, no cycle | a clippy `unwrap_used`/`expect_used` pass would be a separate tooling decision | Triaged | cycle-2 assessment |
| REF-2026-11-06-06 | Info | tests | Test suites are the files closest to the 1,000-line hard gate | `ledger_tests.rs` 973, `progression/tests.rs` 929, `per_row_base_events.rs` 867, `timeframe_set/tests.rs` 769, `extended/tests.rs` 699, `portfolio/tests.rs` 694 | readability | A split `ledger_tests.rs` by behavior when next touched; B leave | B, no cycle; split on the next ledger change if it would cross the gate | `wc -l` and the file-size gate | Triaged | cycle-2 assessment |

REF-2026-11-06-02 and REF-2026-11-06-05 are `Verified` and are not re-opened.
The Planner re-verified REF-03 against `9ac6565` and agreed the route shown.
REF-03 is `Planned` under #531.

### REF-2026-11-06-03 — Date windows are owned by the source adapter

- Expected invariant: a shared value type lives in the module that owns its
  meaning. Shell adapters do not import another adapter's internals.
- Observed: `source/date.rs` owns `DateRange`, `DateBounds`, and
  `DateRangeError`. Configuration, CSV ingestion, the bundle reader, the
  pipeline, the request vocabulary, fetch orchestration, the fetch command, and
  the Dukascopy tests all reach the types through `source`.
- Structural cause: date parsing was first needed for the fetch range and was
  placed under `source`; later consumers reused the path rather than the type.
- Impact: a source-adapter change can reach configuration and CSV parsing; the
  date window is not a source protocol detail.
- Affected contracts/docs: shell module documentation only. No command output,
  schema, or error string changes.
- Behavior risk: low. The move is file-level and import-only.
- Dependencies: none.
- Effort: small.
- Metric before: 10 date-type references through `crate::source`.
- Metric target: 0; `crate::date` owns the types and `source` imports them.

#### Options

A. Move `source/date.rs` to `src/date.rs`, update every importer, remove the
`source` re-export, and move its tests with it.
B. Move only `DateBounds` and `DateRangeError` to `config`, leaving `DateRange`
in `source`.
C. Leave it.

#### Recommendation

A. The three types are one date-window contract; splitting them would preserve
the coupling under a different name.

#### Verification

- An import guard proves no module imports the old `crate::source::date` path.
- Config, CSV, fetch, Dukascopy, and full shell tests pass.
- Fixture, report, and backtest hashes are unchanged.

## Observations

- The cycle-3 module move left one structural Medium finding; after REF-03, no
  High or Medium finding remains.
- `source/date.rs` has no I/O, clock, or source-protocol dependency; it only
  parses UTC dates with `quantrule_data`.
- `source/dukascopy.rs` uses `DateRange` in its request type and its tests; both
  imports move with the type.

## Out of scope

- REF-04 and REF-06 are observations, not cycles.
- Any formula, event, strategy, execution, portfolio, analytics, command, or
  output change.
- Any performance change requires a benchmark and a separate plan.

## Campaign

| Item | Value |
|---|---|
| Baseline commit | `9ac6565` |
| Finding list | REF-2026-11-06-03 |
| Named metric | date-type references through `crate::source`: 10 -> 0 |
| Cycle cap | 1 |
| Stop rule | stop after cycle-4 verification and re-scan; no High or Medium finding remains |

## Selected cycle

REF-2026-11-06-03: move the date module to `crate::date`. The cycle is proposed
in
[`docs/execution-plans/2026-11-06/date-module-boundary.md`](../../execution-plans/2026-11-06/date-module-boundary.md).

## Loop decision

Pending cycle-4 verification. The loop decision is recorded after the verifier
checks the merged refactor and re-scans the new HEAD.

## Limitations

- One agent performed the Structure Auditor and Refactor Planner passes. The
  passes are sequential, not independent; the verifier role is exercised after
  the cycle.
- The structural report is a heuristic. Cohesion and coupling findings are
  supported by code references and review, not by a metric alone.
- The re-scan covers the structural report, the shell imports, the test sizes,
  and the documented boundaries. It is not a line-by-line review of every engine
  module.
