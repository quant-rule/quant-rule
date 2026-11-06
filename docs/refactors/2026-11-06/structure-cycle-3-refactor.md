# Structure Re-Scan — Third Refactor Campaign

- Date: 2026-11-06 (UTC)
- Baseline commit: `172b65a`
- Scope: current Rust workspace, tests, documentation, tools, and repository layout
- Workflow: [`docs/workflows/refactor-workflow.md`](../../workflows/refactor-workflow.md)
- Format: [`docs/refactors/README.md`](../README.md)
- Predecessor: [`structure-cycle-2-refactor.md`](structure-cycle-2-refactor.md)

## Baseline

| Item | Observed |
|---|---|
| Commit | `172b65a` (`git rev-parse HEAD`) |
| Toolchain | `rustc 1.99.1`, `cargo 1.99.1`, `Python 3.14.8` |
| Test inventory | 1,309 passed, 0 failed, 8 ignored (`cargo test --workspace --no-fail-fast`) |
| Golden fixtures | 105 files; combined SHA256 `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1` |
| Report bundle | `report model` over `crates/quantrule-shell/tests/fixtures/config-run/run.json`; bundle SHA256 `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512` |
| Backtest bundle | `backtest run` over the same config and `strategy/event-composite.json`; bundle SHA256 `4173b5055814048597ee5e946da9b117e34dadf5f867de750aaa2f2cf15f085e` |
| Structural report | 197 `src` files / 63,774 lines; 235 files / 75,503 lines including integration tests |
| Largest file | `crates/quantrule-backtest/src/portfolio/ledger_tests.rs`, 973 lines (test-only) |
| Files over 800 lines | 7; two have substantial production code (`spec/v2.rs` 813, `events/signals.rs` 685), the rest are test-heavy or test-only |
| Files over 600 lines | 34 |
| Files over 1,000 lines | 0 |
| `pub` / `pub(crate)` items | 1,653 / 790 |
| `unwrap`/`expect` lines in non-test source | 315, heuristic; the count includes inline test modules and doc examples |
| Gates | the post-merge pass at `1dd13ca` and the docs closure at `172b65a` pass; the current gate is green |

Reproduction:

```bash
git rev-parse HEAD
rustc --version && cargo --version && python3 --version
cargo test --workspace --no-fail-fast
find crates -path '*/fixtures/*' -type f | sort | xargs sha256sum | sha256sum
python3 tools/check_structure.py --report
```

The report bundle is reproducible with:

```bash
rm -rf target/refactor-baseline
target/debug/quantrule report model \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/refactor-baseline
(cd target/refactor-baseline/model && LC_ALL=C sha256sum * | LC_ALL=C sort -k2 | sha256sum)
```

The backtest bundle is reproducible with:

```bash
rm -rf target/refactor-backtest
target/debug/quantrule backtest run \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --strategy crates/quantrule-shell/tests/fixtures/strategy/event-composite.json \
  --out target/refactor-backtest
(cd target/refactor-backtest/backtest && find . -type f | sort | xargs sha256sum | sha256sum)
```

Both bundle hashes are unchanged from the post-merge pass. The backtest bundle contains no clock value or path-dependent bytes; two output directories produced identical bytes.

## Findings

| ID | Impact | Area | Finding | Evidence | Principle | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| REF-2026-11-06-05 | High | shell | The backtest stage calls the command-layer model-bundle writer | `backtest/run.rs:140` calls `crate::command::model_bundle_from`; defined at `command/report_run/bundle.rs:29`; re-exported by `command/report_run/mod.rs:24` and `command.rs:34`; the writer covers `bundle.rs` 275, `provenance.rs` 261, and `model_artifacts.rs` 601 | boundary, atomicity | A move bundle assembly to `crate::model_bundle` with a neutral error; B move only `model_bundle_from` to `render_report`; C leave | A, this cycle | stage-to-command guard, full gate, report and backtest hashes, error-string tests | Verified | [#528](https://github.com/quant-rule/quant-rule/issues/528), [#529](https://github.com/quant-rule/quant-rule/pull/529) |
| REF-2026-11-06-03 | Medium | shell | Date windows are owned by the `source` adapter and imported by non-source modules | `source/date.rs` defines `DateRange`, `DateBounds`, and `DateRangeError`; imported by `config/data.rs:13-14`, `csv.rs:22`, `backtest_bundle.rs:20`, and `pipeline.rs:19`; re-exported by `source/mod.rs:19` | boundary, coupling | A move `source/date.rs` to `crate::date`; B move only the bounds and error to `config`; C leave | A, separate cycle | import guard and config/CSV tests | Triaged | cycle-2 assessment |
| REF-2026-11-06-04 | Info | shell | The public-surface and panic counts are heuristics, not findings | `check_structure.py` counts every `pub` line and every `unwrap`/`expect` line, including inline test modules; the shell has no executable `unwrap`/`expect` outside test modules, and the model/backtest hits are doc examples or test modules | surface, panic hygiene | A compiler-based count and ratchet; B leave as an observation | B, no cycle | a clippy `unwrap_used`/`expect_used` pass would be a separate tooling decision | Triaged | cycle-2 assessment |
| REF-2026-11-06-06 | Info | tests | Test suites are the files closest to the 1,000-line hard gate | `ledger_tests.rs` 973, `progression/tests.rs` 929, `per_row_base_events.rs` 867, `timeframe_set/tests.rs` 769, `extended/tests.rs` 699, `portfolio/tests.rs` 694 | readability | A split `ledger_tests.rs` by behavior when next touched; B leave | B, no cycle; split on the next ledger change if it would cross the gate | `wc -l` and the file-size gate | Triaged | cycle-2 assessment |

REF-2026-11-06-02 is `Verified` and is not re-opened. The Planner re-verified
REF-05 and REF-03 against `172b65a` and agreed the routes shown. REF-05 is
`Verified` under #528 and PR #529; REF-03 remains `Triaged` for a later campaign.

### REF-2026-11-06-05 — The backtest stage calls the command-layer writer

- Expected invariant: a stage module does not call the command adapter. Shared
  model-bundle assembly lives in a neutral module.
- Observed: `backtest/run.rs:140` calls `crate::command::model_bundle_from` to
  write the model stage of a backtest. The writer is defined in
  `command/report_run/bundle.rs`, which also owns provenance assembly and the
  CSV artifact writers.
- Structural cause: `report model` owned the model-bundle writer; the backtest
  runner reused it through `command` rather than through a shared artifact
  module.
- Impact: the backtest stage depends on the command adapter for a writer; a
  command-layer change can break the backtest; the writer cannot be tested as a
  stage artifact without the command.
- Affected contracts/docs: shell module documentation and the model-bundle
  artifact contract. No command output, schema, or error string changes.
- Behavior risk: medium. The move needs a neutral error type so the existing
  `EmptyPeriod`, `BundleError`, and output failure strings stay identical.
- Dependencies: none.
- Effort: medium.
- Metric before: one stage module references `crate::command` (one function).
- Metric target: 0.

#### Options

A. Move `bundle.rs`, `provenance.rs`, and `model_artifacts.rs` to
`src/model_bundle/`, add `ModelBundleError` for `EmptyPeriod`, `BundleError`,
and output failures, and update both callers.
B. Move only `model_bundle_from` into `render_report` with a `BundleError`-based
return, leaving the spec path in `command`.
C. Leave it.

#### Recommendation

A. The whole writer is one artifact contract; moving one entry point would leave
the spec path behind and create a second half-boundary. The error type is the
only behavior-sensitive part, and the existing tests plus new error-message tests
make it verifiable.

#### Verification

- A stage-to-command guard covers the writer as well as the request types.
- The full gate passes; the `report model` bundle hash and the backtest bundle
  hash are byte-identical.
- Shell tests assert the empty-period, bundle, and output error strings before
  and after.

### REF-2026-11-06-03 — Date windows are owned by the source adapter

- Expected invariant: a shared value type lives in the module that owns its
  meaning. Shell adapters do not import another adapter's internals.
- Observed: `source/date.rs` owns `DateRange`, `DateBounds`, and
  `DateRangeError`; configuration, CSV ingestion, the bundle reader, and the
  pipeline import them through `source`.
- Structural cause: date parsing was first needed for the fetch range and was
  placed under `source`; later consumers reused the path rather than the type.
- Impact: a source-adapter change can reach configuration and CSV parsing; the
  date window is not a source protocol detail.
- Behavior risk: low. The move is file-level and import-only.
- Metric before: four non-source modules import `crate::source::date`.
- Metric target: 0; `crate::date` owns the types and `source` imports them.

#### Options

A. Move `source/date.rs` to `src/date.rs`, update every importer, remove the
`source` re-export, and move its tests with it.
B. Move only `DateBounds` and `DateRangeError` to `config`, leaving `DateRange`
in `source`.
C. Leave it.

#### Recommendation

A, in a separate cycle. The three types are one date-window contract; splitting
them would preserve the coupling under a different name.

#### Verification

- An import guard proves no module outside `source` imports
  `crate::source::date`.
- Config, CSV, bundle-reader, and full shell tests pass.
- Fixture and bundle hashes are unchanged.

### REF-2026-11-06-04 and REF-2026-11-06-06

These remain observations. The public-surface and panic counts are heuristic, and
the largest test suites are cohesive and below the 1,000-line gate. No cycle is
proposed.

## Observations

- The only stage-to-command reference left is `backtest/run.rs:140`. Cycle 3
  removes it and broadens the structural guard to reject any future stage or
  shared-artifact import of `crate::command`.
- `command/types.rs` still owns the command envelope and failure types after
  cycle 2. The bundle writer's failure variants (`EmptyPeriod`, `Bundle`) are
  the only parts that need to move to a neutral error.
- The backtest bundle is deterministic across two output directories; the hash
  is usable as an equivalence target for this cycle.

## Out of scope

- REF-03 is recorded but not folded into the REF-05 cycle.
- REF-04 and REF-06 are observations, not cycles.
- Stale documentation content, a correctness defect, or a command behavior
  change is not a refactor.
- A performance change requires a benchmark and a separate plan.

## Campaign

| Item | Value |
|---|---|
| Baseline commit | `172b65a` |
| Finding list | REF-2026-11-06-05 |
| Named metric | stage modules referencing `crate::command`: 1 -> 0 |
| Cycle cap | 1 |
| Stop rule | stop after cycle-3 verification and re-scan; REF-03 requires a new campaign and an approved plan |

## Selected cycle

REF-2026-11-06-05: move the model-bundle writer to `crate::model_bundle` and
extract the neutral error. The cycle is proposed in
[`docs/execution-plans/2026-11-06/model-bundle-boundary.md`](../../execution-plans/2026-11-06/model-bundle-boundary.md).

## Loop decision

REF-2026-11-06-05 is `Verified` at merge commit `5e9c4df` (PR #529). The
post-merge clean-tree pass re-ran the full gate: 1,312 passed, 0 failed, 8
ignored; the fixture hash `da7389...`, report bundle hash `59124f...`, and
backtest bundle hash `4173b505...` are unchanged; both command-coupling guards
pass. The pass was performed by the same agent and is disclosed as sequential,
not independent.

The campaign stops after one cycle. REF-03 remains `Triaged` and requires a new
campaign and an approved plan; the next campaign decision belongs to the human.

## Limitations

- One agent performed the Structure Auditor, Refactor Planner, Refactor
  Implementer, and verifier passes, including the post-merge pass. The passes are
  sequential, not independent. The finding is `Verified` with that limitation.
- The structural report is a heuristic. Cohesion and coupling findings are
  supported by code references and review, not by a metric alone.
- The backtest bundle hash is a new baseline. It is deterministic across two
  output directories, but it is not a substitute for the command-level shell
  tests.
- The re-scan covers the structural report, the shell imports, the test sizes,
  and the documented boundaries. It is not a line-by-line review of every engine
  module; findings outside the campaign may still be discovered later.
