# Second Refactor Cycle — Move the Request Vocabulary Out of the Command Layer

- Date: 2026-11-06 (UTC)
- Status: Approved
- Issue: #525 `Execute the second behavior-neutral refactor cycle`
- Assessment: [`REF-2026-11-06-02`](../../refactors/2026-11-06/structure-cycle-2-refactor.md)
- PR: [#526](https://github.com/quant-rule/quant-rule/pull/526)
- Branch: `refactor/shell-request-boundary`
- Worktree: `../quant-rule--shell-request-boundary`
- Claim type: behavior-neutral module move. No formula, event, strategy,
  execution, portfolio, analytics, command, schema, or output change.

## 1. Intent

Remove the reverse dependency from the shell stage modules to the command layer.
The parsed request types move from `crate::command::types` to a neutral
`crate::request` module, so the command layer depends on the stage inputs and the
stage modules no longer depend on the command adapter. The command output, error
strings, exit codes, schemas, fixtures, and bundle bytes stay identical.

## 2. Current state

| Item | Observed |
|---|---:|
| Baseline commit | `7e6ab31` |
| Test inventory | 1,309 passed, 0 failed, 8 ignored |
| Golden fixture hash | `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1` |
| Bundle hash | `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512` |
| `command/types.rs` | 399 lines; argument structs at lines 78-214; `WindowMode` re-export at line 76 |
| Request types crossing the boundary | 7: `AnalyticsEvaluateArgs`, `BacktestRunArgs`, `PortfolioReplayArgs`, `ExecutionRunArgs`, `QualifyArgs`, `StrategyExplainArgs`, `StrategyRunArgs` |
| Stage modules importing them | 6: `analytics/run.rs:13`, `backtest/run.rs:21`, `execution/run.rs:23`, `portfolio/run.rs:22`, `qualification/run.rs:15`, `strategy/run.rs:24` |
| Remaining stage-to-command call | `backtest/run.rs:140` calls `crate::command::model_bundle_from`; recorded as REF-2026-11-06-05 and excluded from this cycle |
| Full gate | fmt, clippy, test, doc, file size, structure, registry, validation, verification matrix, verification evidence, emitters, command surface, links, terms, legacy, oracle, progression, historical improvement and research tooling all pass |

The structural report is reproducible with
`python3 tools/check_structure.py --report`. The baseline table, the re-scan
evidence, and the carried findings are in
[`structure-cycle-2-refactor.md`](../../refactors/2026-11-06/structure-cycle-2-refactor.md).

### Campaign

| Item | Value |
|---|---|
| Baseline commit | `7e6ab31` |
| Finding list | REF-2026-11-06-02 |
| Named metric | stage modules importing `crate::command::types`: 6 -> 0 |
| Cycle cap | 1 |
| Stop rule | stop after cycle-2 verification and re-scan; REF-03 and REF-05 require a new campaign and an approved plan |

## 3. Scope

Included:

- Add `crates/quantrule-shell/src/request.rs` with the eleven parsed argument
  structs: `ModelArgs`, `ReportArgs`, `PolicyCheckArgs`, `StrategyExplainArgs`,
  `StrategyRunArgs`, `ExecutionRunArgs`, `PortfolioReplayArgs`,
  `BacktestRunArgs`, `QualifyArgs`, `AnalyticsEvaluateArgs`, and `FetchArgs`,
  plus `ModelArgs::new`.
- Move the structs and their rustdoc without changing fields, derives,
  visibility, constructors, or behavior.
- Keep `Command`, `HelpTopic`, `ParseError`, `ReportError`, and `RunError` in
  `command/types.rs`; import the request types there for the `Command` enum.
- Remove the `pub(crate) use crate::pipeline::WindowMode;` re-export from
  `command/types.rs`; `command/parse.rs` and `command/tests.rs` import
  `WindowMode` from `crate::pipeline`.
- Update the request-type imports in the six stage modules; leave the
  `crate::command::model_bundle_from` call at `backtest/run.rs:140` unchanged
  because it is REF-2026-11-06-05.
- Update the command parser modules, `command/tests.rs`, `command.rs`, and
  `main.rs` to the new paths.
- Add a `stage-request-coupling` rule to `tools/check_structure.py` and its
  positive and negative tests to `tools/test_check_structure.py`.
- Update module documentation so each module names its new boundary.
- Record the assessment, this plan, and the AI transparency log.

Excluded:

- REF-2026-11-06-03 (the shared date module).
- REF-2026-11-06-05 (the model-bundle writer).
- REF-2026-11-06-04 and REF-2026-11-06-06 (observations, no cycle).
- Any function-signature, field, derive, visibility, error-string, output,
  schema, fixture, command, or engine change.
- Any rename outside the moved argument structs.
- Any new dependency, `#[allow]`, `unsafe`, `unwrap`, or `expect`.
- Any change to the historical assessment or the first cycle's evidence.

## 4. Assumptions and uncertainty

- The stage modules read request fields only; none needs command parsing or
  command-layer behavior. The compiler and clippy catch any missed property.
- `crate::request` may import `std::path::PathBuf`,
  `quantrule_backtest::intent::StrategyPosition`, `quantrule_data::Timeframe`,
  `crate::pipeline::WindowMode`, and `crate::source::{DateRange, Symbol}`
  without creating a cycle. The guard checks that `request` does not import
  `crate::command`.
- The shell tests exercise every command family, so a moved parser still
  produces the same parsed command. The bundle hash covers the `report model`
  output.
- The bundle hash method was not recorded in the first assessment. The new
  method in the re-scan is reproducible; two output directories produced the
  same `59124f...` hash.
- The guard's substring rule covers the current import style. The unit tests
  cover a violating stage import and a clean stage import. A future import
  spelling is a reason to extend the rule, not to widen it now.
- The remaining `backtest/run.rs:140` call is explicitly out of scope; the guard
  must not fail on it. This is a deliberate, recorded boundary for cycle 2, not
  an unexamined exception.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Module name | `crate::request` | One name for the parsed input vocabulary; no catch-all module |
| Types moved | All eleven argument structs | `command/types.rs` keeps one role: the command envelope and failures |
| Error types | Stay in `command/types.rs` | No error-string or variant change |
| `WindowMode` | Import from `crate::pipeline` | Removes an unnecessary command re-export |
| Guard scope | `crate::command::types` in the six stage directories | The guard passes after the move; REF-05 remains a separate finding |
| Bundle hash | Documented sorted-file hash, baseline `59124f...` | The verifier can reproduce the comparison; the old unrecoverable hash is not used |
| Issue | Create after approval | The plan is not an issue; the board tracks the implementation |
| Dependencies | None | No `Cargo.toml` or lockfile change |

## 6. Acceptance criteria

- `crate::request` holds all eleven parsed argument structs and `ModelArgs::new`;
  `command/types.rs` holds no argument struct and no `WindowMode` re-export.
- `grep -rn "crate::command::types"` over the six stage directories returns
  nothing.
- `crate::request` does not import `crate::command`.
- The only stage-to-command call left is `backtest/run.rs:140`, recorded as
  REF-2026-11-06-05.
- `python3 tools/check_structure.py` passes, and its new tests fail on a temp
  stage module that imports `crate::command::types` and on a temp `request.rs`
  that imports `crate::command`.
- `cargo test --workspace --no-fail-fast` remains 1,309 passed, 0 failed, 8
  ignored.
- The golden fixture hash remains
  `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1`.
- The `report model` bundle hash remains
  `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512`.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo doc`, the file-size gate, the structural gate, and every documentation
  and registry gate pass.
- No fixture, expected value, command, schema, error string, or output changes.
- The diff contains only the move, import updates, module declaration, guard,
  tests, and documentation.

## 7. Work breakdown

| Step | Work | Paths | Owner |
|---:|---|---|---|
| 1 | Create `request.rs`; move the eleven argument structs and `ModelArgs::new`; add the module doc and `mod request` | `src/request.rs`, `src/main.rs` | Refactor Implementer |
| 2 | Trim `command/types.rs`; import the request types for `Command`; remove the `WindowMode` re-export | `src/command/types.rs` | Refactor Implementer |
| 3 | Update the request-type import in the six stage modules; leave the `model_bundle_from` call at `backtest/run.rs:140` unchanged | `src/{analytics,backtest,execution,portfolio,qualification,strategy}/run.rs` | Refactor Implementer |
| 4 | Update the command parsers, `command/tests.rs`, `command.rs`, and `run_events`; import `WindowMode` from `crate::pipeline` | `src/command/*.rs`, `src/command/report_run/*.rs`, `src/command.rs` | Refactor Implementer |
| 5 | Add the `stage-request-coupling` rule and its tests | `tools/check_structure.py`, `tools/test_check_structure.py` | Refactor Implementer |
| 6 | Run the full gate and record the evidence in this plan | this plan, assessment | Refactor Verifier |
| 7 | Update the assessment status and links after merge; update the AI transparency log | assessment, AI log | Refactor Implementer / Verifier |

Step 3 and step 4 may be separate commits if that keeps the move reviewable.
No step may change logic to make a path compile.

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

- `cargo test --workspace --no-fail-fast`: 1,309 passed, 0 failed, 8 ignored.
- Golden fixture combined hash:
  `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1`.
- `report model` bundle hash:
  `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512`.
- `git diff --stat` shows only the paths named in section 7.
- The shell tests assert the command surface, usage errors, and error strings;
  no assertion is removed or changed.
- The new structure tests prove the guard catches a violation and accepts a
  clean tree.

Limitations:

- The structural report's refactor-document count and the Markdown and term file
  counts rise by one because this assessment and plan are added. No source
  metric changes.
- The move proves behavior preservation only for the paths the tests exercise.
  The bundle hash and the full gate cover the command surface, not every
  internal call path.
- If one agent performs the implementer, reviewer, and verifier roles, the
  sequential passes are disclosed and are not an independent review.

## 9. Risks and recovery

| Risk | Mitigation | Recovery |
|---|---|---|
| A moved struct changes a field, derive, or visibility | Move verbatim; diff the moved block; clippy and tests | Revert the commit and re-plan |
| A call site keeps the old path and the guard misses it | The compiler catches the import; the guard catches `crate::command::types` | Fix the path; extend the guard test |
| `request` imports `command` and creates a cycle | The guard checks the reverse edge; the compiler checks the module | Move the offending item to a narrower neutral module |
| The guard false-positives on the remaining writer call | The rule targets `crate::command::types`, not all `crate::command` | Narrow the rule or re-scope the cycle; do not widen the diff |
| A fixture or bundle byte changes | Compare the hashes; stop | Revert and investigate the move |
| A command error string changes | The shell tests assert the strings | Revert; the move must not touch error rendering |
| REF-05 or REF-03 is pulled into the diff | Scope review and `git diff --stat` | Stop; open a new cycle |

Rollback is the branch. There is no migration, schema, or external state.

## 10. Stop conditions

- A move requires a logic edit, a signature change, or a new error variant.
- A fixture, expected value, command, schema, error string, stdout, stderr, or
  exit code changes.
- A test fails for a reason outside the move.
- A new dependency or a module-boundary decision outside the plan is required.
- The guard cannot pass without also moving REF-05.
- The full gate cannot pass without weakening a test or adding `#[allow]`.

## 11. Traceability

```text
REF-2026-11-06-02 -> #525 -> #526 -> verification record
```

The issue is [quant-rule/quant-rule#525](https://github.com/quant-rule/quant-rule/issues/525).
Its scope and done-when criteria are those in sections 3 and 6.

## 12. Verification record

| Check | Result |
|---|---|
| Request module complete | `src/request.rs` 151 lines; `command/types.rs` 263 lines; all eleven structs and `ModelArgs::new` moved; `WindowMode` re-export removed |
| Stage imports removed | 0 `crate::command::types` imports in the six stage directories; `backtest/run.rs:140` `model_bundle_from` remains as REF-2026-11-06-05 |
| Guard and tests | `stage-request-coupling` added; 14 structure tests pass, including 3 new tests |
| Test inventory | 1,309 passed, 0 failed, 8 ignored; unchanged |
| Golden fixture hash | `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1`; unchanged |
| Bundle hash | `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512`; unchanged and identical across two output directories |
| Full gate | fmt, clippy, test, doc, file size, structure, registry, validation, verification matrix, verification evidence, emitters, command surface, links, terms, legacy, oracle, progression, improvement, research, program, coverage, and diff check pass |
| Clean-worktree verifier pass | at commit `f6e8c22`: every gate passes; 1,309 passed, 0 failed, 8 ignored; fixture and bundle hashes unchanged; the stage-import guard passes. Sequential pass by the same agent, not an independent review |
| Diff scope | 22 code files and 2 tool files; no fixture, expected-value, command, schema, error-string, or output change |
| Assessment status | `In progress`; local and clean-worktree verification complete; post-merge verification pending |
| Loop decision | pending the post-merge verifier pass and re-scan |
