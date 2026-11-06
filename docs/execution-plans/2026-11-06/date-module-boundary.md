# Fourth Refactor Cycle — Move the Date Module Out of the Source Adapter

- Date: 2026-11-06 (UTC)
- Status: Approved
- Issue: #531 `Execute the fourth behavior-neutral refactor cycle`
- Assessment: [`REF-2026-11-06-03`](../../refactors/2026-11-06/structure-cycle-4-refactor.md)
- Branch: `refactor/date-module-boundary`
- Worktree: `../quant-rule--date-module-boundary`
- Claim type: behavior-neutral file move. No formula, event, strategy, execution,
  portfolio, analytics, command, schema, or output change.

## 1. Intent

Move the UTC date-window types out of the `source` adapter into a neutral
`crate::date` module. Configuration, CSV ingestion, the bundle reader, the
pipeline, the request vocabulary, and fetch orchestration already share these
types; none of them should import a source adapter to reach a date. The types,
their constructors, their error strings, and their tests move unchanged.

## 2. Current state

| Item | Observed |
|---|---:|
| Baseline commit | `9ac6565` |
| Test inventory | 1,312 passed, 0 failed, 8 ignored |
| Golden fixture hash | `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1` |
| Report bundle hash | `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512` |
| Backtest bundle hash | `4173b5055814048597ee5e946da9b117e34dadf5f867de750aaa2f2cf15f085e` |
| Date module | `source/date.rs`, 322 lines including tests; `DateRange`, `DateBounds`, `DateRangeError` |
| Source re-export | `source/mod.rs:13` declares `pub(crate) mod date;`; line 19 re-exports `DateBounds` and `DateRange` |
| Importers through `crate::source` | `config/data.rs:13-14`, `csv.rs:22`, `backtest_bundle.rs:20`, `pipeline.rs:19`, `request.rs:14`, `fetch.rs:17`, `command/fetch.rs:17`, `command/tests.rs:9`, `source/dukascopy.rs:475`, `source/mod.rs:19` |
| Internal source use | `source/dukascopy.rs:20` imports `super::date::DateRange` |
| Full gate | fmt, clippy, test, doc, file size, structure, registry, validation, verification matrix, verification evidence, emitters, command surface, links, terms, legacy, oracle, progression, improvement, research, program, coverage, and diff check pass |

The structural report is reproducible with
`python3 tools/check_structure.py --report`. The baseline table is in
[`structure-cycle-4-refactor.md`](../../refactors/2026-11-06/structure-cycle-4-refactor.md).

### Campaign

| Item | Value |
|---|---|
| Baseline commit | `9ac6565` |
| Finding list | REF-2026-11-06-03 |
| Named metric | date-type references through `crate::source`: 10 -> 0 |
| Cycle cap | 1 |
| Stop rule | stop after cycle-4 verification and re-scan; no High or Medium finding remains |

## 3. Scope

Included:

- Move `crates/quantrule-shell/src/source/date.rs` to
  `crates/quantrule-shell/src/date.rs` with `git mv`; keep the file history.
- Update the module doc to say the date window is shared by configuration,
  source, and bundle readers.
- Move the existing date tests with the file; no test assertion changes.
- Remove `pub(crate) mod date;` and the `DateBounds`/`DateRange` re-export from
  `source/mod.rs`.
- Add `mod date;` to `main.rs`.
- Update the importers:
  - `config/data.rs`: `DateBounds`, `DateRangeError`
  - `csv.rs`, `backtest_bundle.rs`, `pipeline.rs`: `DateBounds`
  - `request.rs`: `DateRange`
  - `fetch.rs`, `command/fetch.rs`, `command/tests.rs`: `DateRange`
  - `source/dukascopy.rs`: `DateRange` in production and tests
- Add a `date-module-coupling` structural rule that rejects the old
  `crate::source::date` path, `crate::source::DateBounds`/`DateRange`, and a
  `source/date.rs` file, with tests.
- Update the assessment, this plan, and the AI log.

Excluded:

- REF-2026-11-06-04 and REF-2026-11-06-06 (observations, no cycle).
- Any formula, reading, event, strategy, execution, portfolio, analytics,
  command, schema, fixture, or output change.
- Any change to date parsing, bounds arithmetic, error strings, or tests.
- Any rename outside the moved file and module path.
- Any new dependency, `#[allow]`, `unsafe`, `unwrap`, or `expect`.

## 4. Assumptions and uncertainty

- The three types are one contract and are used together; the whole file moves.
- `crate::date` may depend on `quantrule_data` only. It does not import
  `source`, so no cycle is introduced.
- The visibility stays `pub(crate)`; the date items are internal to the shell.
- `source/dukascopy.rs` can import `crate::date::DateRange` from its production
  and test modules.
- The guard's substring rule covers the current import style. The tests cover a
  violating file and a clean `crate::date` file.
- Moving the file does not change the command output, the report bundle, or the
  backtest bundle.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Module name | `crate::date` | One owner for the date-window contract |
| Move scope | Whole `date.rs`, tests included | No split of one contract |
| Visibility | `pub(crate)` | Internal shell surface only |
| Source module | Remove the date declaration and re-export | `source` owns source access only |
| Guard | `date-module-coupling` | Rejects the old path and a reintroduced file |
| Dependencies | None | No `Cargo.toml` or lockfile change |
| Issue | Create after approval | The board tracks the implementation |

## 6. Acceptance criteria

- `crates/quantrule-shell/src/date.rs` owns `DateRange`, `DateBounds`, and
  `DateRangeError`; `source/date.rs` no longer exists.
- `source/mod.rs` has no date module or date re-export.
- `grep -rn "crate::source::date\|crate::source::DateBounds\|crate::source::DateRange"`
  returns nothing.
- `python3 tools/check_structure.py` passes, and the new `date-module-coupling`
  tests fail on the old path and pass on `crate::date`.
- `cargo test --workspace --no-fail-fast` remains 1,312 passed, 0 failed, 8
  ignored; no test is removed, skipped, or weakened.
- The golden fixture hash remains `da7389...`.
- The report bundle hash remains `59124f...`.
- The backtest bundle hash remains `4173b505...`.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo doc`, the file-size gate, the structural gate, and every documentation
  and registry gate pass.
- The diff contains only the move, import updates, the guard, tests, and
  documentation.

## 7. Work breakdown

| Step | Work | Paths | Owner |
|---:|---|---|---|
| 1 | Move the date file with `git mv`; update its module doc; add `mod date` | `src/date.rs`, `src/main.rs` | Refactor Implementer |
| 2 | Remove the source declaration and re-export | `src/source/mod.rs` | Refactor Implementer |
| 3 | Update the production and test imports | the ten importer files | Refactor Implementer |
| 4 | Add the `date-module-coupling` rule and its tests | `tools/check_structure.py`, `tools/test_check_structure.py` | Refactor Implementer |
| 5 | Run the full gate; compare the three hashes; record the evidence | this plan, assessment | Refactor Verifier |
| 6 | Update the assessment status and links after merge; update the AI log | assessment, AI log | Refactor Implementer / Verifier |

No step may change date logic to make a path compile.

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
python3 tools/check_file_size.py
python3 tools/check_structure.py
python3 -m unittest discover -s tools -p 'test_check_structure.py'
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_verification_matrix.py
python3 tools/check_verification.py
python3 tools/check_event_emitters.py
python3 tools/check_command_surface.py
python3 tools/check_links.py
python3 tools/check_terms.py
python3 tools/check_legacy.py
python3 -m unittest discover -s tools/oracle -p 'test_*.py'
python3 tools/oracle/progression_replay.py --check
python3 -m unittest discover -s tools/improvement -p 'test_*.py'
python3 -m unittest discover -s tools/research -p 'test_*.py'
python3 tools/research/program.py --program config/research/PROGRAM-2026-11-05-03/program.json --check
python3 tools/research/coverage.py --program config/research/PROGRAM-2026-11-05-03/program.json
git diff --check
```

Equivalence evidence:

- `cargo test --workspace --no-fail-fast`: 1,312 passed, 0 failed, 8 ignored.
- Golden fixture hash: `da7389...`, unchanged.
- Report bundle hash: `59124f...`, unchanged.
- Backtest bundle hash: `4173b505...`, unchanged.
- `git diff --stat` shows only the paths named in section 7.
- The date tests move with the file; no assertion is changed.

Limitations:

- The move proves behavior preservation only for the paths the tests exercise.
  The full gate and the three hashes cover the shell surface.
- If one agent performs the implementer, reviewer, and verifier roles, the
  sequential passes are disclosed and are not an independent review.

## 9. Risks and recovery

| Risk | Mitigation | Recovery |
|---|---|---|
| An importer keeps the old path | Compiler and the `date-module-coupling` guard | Fix the import; extend the guard test |
| A test module still uses `source::date` | The guard scans every shell source file; the test moves with the module | Update the test import |
| A date string or bounds value changes | Tests move unchanged; full gate | Revert and re-plan |
| A bundle byte changes | Compare the report and backtest hashes | Revert and investigate |
| The move introduces a module cycle | `date` imports only `quantrule_data`; the guard checks the old path | Revert the import edge |

Rollback is the branch. There is no migration, schema, or external state.

## 10. Stop conditions

- A move requires a logic edit or a date-string change.
- A fixture, expected value, command, schema, error string, stdout, stderr, or
  exit code changes.
- A report or backtest bundle hash changes.
- A test fails for a reason outside the move.
- A new dependency or a module-boundary decision outside the plan is required.
- The full gate cannot pass without weakening a test or adding `#[allow]`.

## 11. Traceability

```text
REF-2026-11-06-03 -> #531 -> PR TBD -> verification record
```

The issue is [quant-rule/quant-rule#531](https://github.com/quant-rule/quant-rule/issues/531).
Its scope and done-when criteria are those in sections 3 and 6.

## 12. Verification record

| Check | Result |
|---|---|
| Date module complete | `src/date.rs` is 322 lines; `source/date.rs` is absent; `source/mod.rs` has no date declaration or re-export |
| Source declaration removed | the `source` module exposes only source access; the date types are imported from `crate::date` |
| Guard and tests | `date-module-coupling` added; 21 structure tests pass, including 4 date-module tests |
| Test inventory | 1,312 passed, 0 failed, 8 ignored; unchanged |
| Golden fixture hash | `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1`; unchanged |
| Report bundle hash | `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512`; unchanged |
| Backtest bundle hash | `4173b5055814048597ee5e946da9b117e34dadf5f867de750aaa2f2cf15f085e`; unchanged |
| Full gate | fmt, clippy, test, doc, file size, structure, registry, validation, verification matrix, verification evidence, emitters, command surface, links, terms, legacy, oracle, progression, improvement, research, program, coverage, and diff check pass |
| Clean-worktree verifier pass | at commit `d5ee617`: every gate passes; 1,312 passed, 0 failed, 8 ignored; all three hashes unchanged; the date guard passes. Sequential pass by the same agent, not an independent review |
| Diff scope | 12 code files and 2 tool files; no fixture, expected-value, command, schema, error-string, or output change |
| Assessment status | `In progress`; local and clean-worktree verification complete; post-merge verification pending |
| Loop decision | pending the post-merge verifier pass and re-scan |
