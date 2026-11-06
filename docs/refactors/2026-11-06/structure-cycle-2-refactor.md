# Structure Re-Scan — Second Refactor Campaign

- Date: 2026-11-06 (UTC)
- Baseline commit: `7e6ab31`
- Scope: current Rust workspace, tests, documentation, tools, and repository layout
- Workflow: [`docs/workflows/refactor-workflow.md`](../../workflows/refactor-workflow.md)
- Format: [`docs/refactors/README.md`](../README.md)
- Predecessor: [`current-structure-refactor.md`](current-structure-refactor.md)

## Baseline

| Item | Observed |
|---|---|
| Commit | `7e6ab31` (`git rev-parse HEAD`) |
| Toolchain | `rustc 1.99.1`, `cargo 1.99.1`, `Python 3.14.8` |
| Test inventory | 1,309 passed, 0 failed, 8 ignored (`cargo test --workspace --no-fail-fast`) |
| Golden fixtures | 105 files; combined SHA256 `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1` |
| Representative bundle | `report model` over `crates/quantrule-shell/tests/fixtures/config-run/run.json`; bundle SHA256 `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512` |
| Structural report | 196 `src` files / 63,743 lines; 234 files / 75,472 lines including integration tests |
| Largest file | `crates/quantrule-backtest/src/portfolio/ledger_tests.rs`, 973 lines (test-only) |
| Files over 800 lines | 7; two have substantial production code (`spec/v2.rs` 813, `events/signals.rs` 685), the rest are test-heavy or test-only |
| Files over 600 lines | 34 |
| Files over 1,000 lines | 0 |
| `pub` / `pub(crate)` items | 1,653 / 791 |
| `unwrap`/`expect` lines in non-test source | 315, heuristic; the count includes inline test modules and doc examples |
| Gates | fmt, clippy, test, doc, file size, structure, registry, validation, verification matrix, verification evidence, emitters, command surface, links, terms, legacy, oracle, progression, historical improvement and research tooling all pass |

Reproduction:

```bash
git rev-parse HEAD
rustc --version && cargo --version && python3 --version
cargo test --workspace --no-fail-fast
find crates -path '*/fixtures/*' -type f | sort | xargs sha256sum | sha256sum
python3 tools/check_structure.py --report
```

The representative bundle is reproducible with:

```bash
rm -rf target/refactor-baseline
target/debug/quantrule report model \
  --config crates/quantrule-shell/tests/fixtures/config-run/run.json \
  --out target/refactor-baseline
(cd target/refactor-baseline/model && LC_ALL=C sha256sum * | LC_ALL=C sort -k2 | sha256sum)
```

The first assessment recorded a bundle hash without its computation command. That
hash is not reproducible from the repository; this re-scan defines the method
above and uses `59124f...` as the cycle-2 baseline. The earlier value is left in
place as history, not rewritten.

## Findings

| ID | Impact | Area | Finding | Evidence | Principle | Options | Recommended | Verification | Status | Links |
|---|---|---|---|---|---|---|---|---|---|---|
| REF-2026-11-06-02 | High | shell | Stage modules import the command request vocabulary | 6 stage modules, 7 imports: `analytics/run.rs:13`, `backtest/run.rs:21`, `execution/run.rs:23`, `portfolio/run.rs:22`, `qualification/run.rs:15`, `strategy/run.rs:24`; `command/types.rs` is 399 lines and holds 11 argument structs at lines 78-214 | boundary, coupling | A move all argument structs to `crate::request`; B move only the seven crossing types; C leave | A, this cycle | import guard, full gate, fixture and bundle hashes unchanged | In progress | [#525](https://github.com/quant-rule/quant-rule/issues/525) |
| REF-2026-11-06-03 | Medium | shell | Date windows are owned by the `source` adapter and imported by non-source modules | `source/date.rs` defines `DateRange`, `DateBounds`, and `DateRangeError`; imported by `config/data.rs:13-14`, `csv.rs:22`, `backtest_bundle.rs:20`, and `pipeline.rs:19`; re-exported by `source/mod.rs:19` | boundary, coupling | A move `source/date.rs` to `crate::date`; B move only the bounds and error to `config`; C leave | A, separate cycle | import guard and config/CSV tests | Triaged | first assessment |
| REF-2026-11-06-04 | Info | shell | The public-surface and panic counts are heuristics, not findings | `check_structure.py:298-309` counts every `pub` line and every `unwrap`/`expect` line, including inline test modules; the shell has no executable `unwrap`/`expect` outside test modules, and the model/backtest hits are doc examples or test modules | surface, panic hygiene | A compiler-based count and ratchet; B leave as an observation | B, no cycle | a clippy `unwrap_used`/`expect_used` pass would be a separate tooling decision | Triaged | first assessment |
| REF-2026-11-06-05 | High | shell | The backtest stage calls the command-layer model-bundle writer | `backtest/run.rs:140` calls `crate::command::model_bundle_from`; defined at `command/report_run/bundle.rs:29`; re-exported by `command/report_run/mod.rs:23` and `command.rs:34` | boundary, atomicity | A move bundle assembly to `crate::model_bundle` with a neutral error; B move only `model_bundle_from` to `render_report`; C leave | A, separate cycle | stage-to-command import guard, full gate, bundle hashes, error-string tests | Triaged | — |
| REF-2026-11-06-06 | Info | tests | Test suites are the files closest to the 1,000-line hard gate | `ledger_tests.rs` 973, `progression/tests.rs` 929, `per_row_base_events.rs` 867, `timeframe_set/tests.rs` 769, `extended/tests.rs` 699, `portfolio/tests.rs` 694 | readability | A split `ledger_tests.rs` by behavior when next touched; B leave | B, no cycle; split on the next ledger change if it would cross the gate | `wc -l` and the file-size gate | Triaged | — |

The Planner re-verified every carried finding against `7e6ab31` and agreed the
route shown. REF-02 is `In progress` under #525; REF-03 and REF-05 remain
`Triaged` for a later campaign; REF-04 and REF-06 are observations with no
cycle.

REF-2026-11-06-01 remains `Verified` from the first cycle; it is not re-opened.
Carried findings keep their IDs. The first assessment remains the historical
record for cycle 1; this document is the status record for the second campaign.

### REF-2026-11-06-02 — Stage modules import the command request vocabulary

- Expected invariant: the command layer depends on the stage modules. A stage
  module accepts its input from a neutral module and never imports
  `crate::command`.
- Observed: six stage modules import seven argument types from
  `crate::command::types`. `command/types.rs` mixes the parsed-command enum,
  eleven argument structs, a `WindowMode` re-export, and three error types.
- Structural cause: argument structs were defined in the command vocabulary
  while the run functions were extracted into stage modules. The command
  envelope and the stage input share one module.
- Impact: a command-layer change can force a stage-module rebuild; a stage module
  cannot be read or tested without the command layer; the module graph has a
  cycle even though the crate compiles.
- Affected contracts/docs: shell module documentation only. No command output,
  schema, or error string changes.
- Behavior risk: low. The move is compiler-checked and the shell tests exercise
  every command path.
- Dependencies: none.
- Effort: small.
- Metric before: 6 stage modules / 7 imports of `crate::command::types`.
- Metric target: 0.

#### Options

A. Create `crates/quantrule-shell/src/request.rs`, move all eleven parsed
argument structs and `ModelArgs::new` there, leave `Command`, `HelpTopic`,
`ParseError`, `ReportError`, and `RunError` in `command/types.rs`, and remove the
`WindowMode` re-export.
B. Move only the seven types the stage modules import, leaving the four
command-only argument structs in `command/types.rs`.
C. Leave it; revisit when a command change touches the same types.

#### Recommendation

A. The request types are one concept: the parsed input to a command or stage
run. A single neutral module makes the vocabulary predictable and leaves
`command/types.rs` with the command envelope and failures. Moving all eleven is
the same move as moving seven, with a smaller conceptual exception list.

#### Verification

- `grep -rn "crate::command::types"` over the six stage directories returns
  nothing.
- `python3 tools/check_structure.py` gains a `stage-request-coupling` rule with a
  positive and a negative unit test.
- Full gate passes; cargo test count remains 1,309 passed and 8 ignored.
- Fixture hash `da7389...` and bundle hash `59124f...` are unchanged.
- No fixture, expected value, command, schema, or error string changes.

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

### REF-2026-11-06-04 — Public-surface and panic counts are heuristics

- Expected invariant: the public surface is intentional and library code does
  not panic on invalid input.
- Observed: `check_structure.py` counts every `pub` line and every
  `unwrap`/`expect` line in `src`, including inline test modules and doc
  examples. The count is a scan trigger, not a finding.
- Structural cause: no compiler-based surface measurement and no panic ratchet.
- Impact: unknown until measured. A refactor based on the heuristic would move
  code without evidence.
- Recommendation: leave it as an observation. A clippy `unwrap_used` /
  `expect_used` pass on library targets, or a reviewed ratchet, is a separate
  tooling decision.

#### Verification

`python3 tools/check_structure.py --report`; `check_structure.py:298-309` shows
the counting method.

### REF-2026-11-06-05 — The backtest stage calls the command-layer writer

- Expected invariant: a stage module does not call the command adapter. Shared
  model-bundle assembly lives in a neutral module.
- Observed: `backtest/run.rs:140` calls `crate::command::model_bundle_from` to
  write the model stage of a backtest. The writer is defined in
  `command/report_run/bundle.rs` and re-exported through the command layer.
- Structural cause: `report model` owned the model-bundle writer; the backtest
  runner reused it through `command` rather than through a shared artifact
  module.
- Impact: the backtest stage depends on the command adapter for a writer; a
  command-layer change can break the backtest; the writer cannot be tested as a
  stage artifact without the command.
- Behavior risk: medium. The move needs a neutral error type so the existing
  error strings, stdout, stderr, and exit codes stay identical.
- Metric before: one stage module calls `crate::command` (one function).
- Metric target: 0.

#### Options

A. Move `bundle.rs`, `provenance.rs`, and `model_artifacts.rs` to
`src/model_bundle/`, add `ModelBundleError` for `EmptyPeriod`, `BundleError`,
and output failures, and update both callers.
B. Move only `model_bundle_from` into `render_report` with a `BundleError`-based
return, leaving the spec path in `command`.
C. Leave it.

#### Recommendation

A, in a separate cycle after REF-02. The whole writer is one artifact contract;
moving one entry point would leave the spec path behind and create a second
half-boundary.

#### Verification

- A stage-to-command guard covers the writer as well as the request types.
- The full gate passes; the `report model` bundle hash and the backtest bundle
  are byte-identical.
- Shell tests assert the empty-period and bundle error strings before and after.

### REF-2026-11-06-06 — Test suites near the hard size gate

- Expected invariant: a source file stays under 1,000 lines where practical.
- Observed: the six largest test suites are between 694 and 973 lines;
  `ledger_tests.rs` is 27 lines below the hard gate. The production files are
  under 1,000.
- Structural cause: the test suites grew by behavior, not by mixed production
  responsibility.
- Impact: a large test addition would cross the gate. The suites are cohesive
  today, so a split would be behavior-neutral but is not urgent.
- Recommendation: no cycle. Split `ledger_tests.rs` by behavior in the change
  that would otherwise cross 1,000 lines.

#### Verification

`wc -l` on the listed files and `python3 tools/check_file_size.py`.

## Observations

- The only production file over 800 lines is
  `crates/quantrule-model/src/spec/v2.rs` at 813. It holds one versioned spec
  schema and its evaluator; no mixed responsibility was found. The 600-line
  trigger is recorded, not escalated.
- `command/types.rs` also re-exports `WindowMode` from `pipeline`. REF-02 removes
  that re-export so the command module exposes only its own vocabulary.
- The structural report does not catch the stage-to-command direction. REF-02
  adds a narrow guard for the request vocabulary; REF-05 extends it to the
  writer.

## Out of scope

- Stale documentation content, a correctness defect, or a command behavior
  change is not a refactor.
- A model, strategy, execution, portfolio, or analytics behavior change belongs
  to its owning workflow.
- A performance change requires a benchmark and a separate plan.
- REF-03 and REF-05 are recorded but not folded into the REF-02 cycle.

## Campaign

| Item | Value |
|---|---|
| Baseline commit | `7e6ab31` |
| Finding list | REF-2026-11-06-02 |
| Named metric | stage modules importing `crate::command::types`: 6 -> 0 |
| Cycle cap | 1 |
| Stop rule | stop after cycle-2 verification and re-scan; REF-03 and REF-05 require a new campaign and an approved plan |

## Selected cycle

REF-2026-11-06-02: move the parsed request types to `crate::request`. The cycle
is proposed in
[`docs/execution-plans/2026-11-06/shell-request-boundary.md`](../../execution-plans/2026-11-06/shell-request-boundary.md).

## Loop decision

Cycle 2 is implemented and locally verified at commit `f6e8c22`. The finding
stays `In progress` until the pull request merges; the Refactor Verifier then
checks equivalence on the merged tree and moves it to `Verified`. The loop
decision follows that pass and re-scans the new HEAD.

## Limitations

- One agent performed the Structure Auditor, Refactor Planner, Refactor
  Implementer, and local verifier passes. The passes are sequential, not
  independent; the post-merge verifier role remains to be exercised.
- The structural report is a heuristic. Cohesion and coupling findings are
  supported by code references and review, not by a metric alone.
- The first assessment's bundle hash method is unrecoverable; this assessment
  defines a reproducible method and does not claim the old value.
- The re-scan covers the structural report, the shell module imports, the test
  sizes, and the documented boundaries. It is not a line-by-line review of every
  engine module; findings outside the campaign may still be discovered later.
