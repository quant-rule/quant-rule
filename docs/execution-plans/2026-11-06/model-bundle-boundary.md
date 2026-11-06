# Third Refactor Cycle — Move the Model-Bundle Writer Out of the Command Layer

- Date: 2026-11-06 (UTC)
- Status: Approved
- Issue: #528 `Execute the third behavior-neutral refactor cycle`
- Assessment: [`REF-2026-11-06-05`](../../refactors/2026-11-06/structure-cycle-3-refactor.md)
- Branch: `refactor/model-bundle-boundary`
- Worktree: `../quant-rule--model-bundle-boundary`
- Claim type: behavior-neutral module move plus error-type extraction. No formula,
  event, strategy, execution, portfolio, analytics, command, schema, or output
  change.

## 1. Intent

Remove the last reverse dependency from a shell stage module to the command
layer. The model-bundle assembly, provenance, and CSV artifact writers move from
`command/report_run` to a neutral `crate::model_bundle` module, and the writer
gets a neutral `ModelBundleError` so its existing error strings, exit codes, and
artifact bytes stay identical. The backtest runner then calls the shared writer
directly instead of going through `command`.

## 2. Current state

| Item | Observed |
|---|---:|
| Baseline commit | `172b65a` |
| Test inventory | 1,309 passed, 0 failed, 8 ignored |
| Golden fixture hash | `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1` |
| Report bundle hash | `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512` |
| Backtest bundle hash | `4173b5055814048597ee5e946da9b117e34dadf5f867de750aaa2f2cf15f085e` |
| Stage-to-command references | 1: `backtest/run.rs:140` calls `crate::command::model_bundle_from` |
| Bundle writer files | `bundle.rs` 275, `provenance.rs` 261, `model_artifacts.rs` 601 |
| Command files | `report_run/mod.rs` 145, `report_run/spec.rs` 94, `command/types.rs` 263 |
| Error variants to move | `ReportError::EmptyPeriod` (`types.rs:144`) used by `bundle.rs:99` and `spec.rs:90`; `RunError::Bundle` (`types.rs:216`) used by `bundle.rs:91` and `bundle.rs:258` |
| Full gate | fmt, clippy, test, doc, file size, structure, registry, validation, verification matrix, verification evidence, emitters, command surface, links, terms, legacy, oracle, progression, improvement, research, program, coverage, and diff check pass |

The structural report is reproducible with
`python3 tools/check_structure.py --report`. The baseline table and the re-scan
evidence are in
[`structure-cycle-3-refactor.md`](../../refactors/2026-11-06/structure-cycle-3-refactor.md).

### Campaign

| Item | Value |
|---|---|
| Baseline commit | `172b65a` |
| Finding list | REF-2026-11-06-05 |
| Named metric | stage modules referencing `crate::command`: 1 -> 0 |
| Cycle cap | 1 |
| Stop rule | stop after cycle-3 verification and re-scan; REF-03 requires a new campaign and an approved plan |

## 3. Scope

Included:

- Add `crates/quantrule-shell/src/model_bundle/` with `mod.rs`, `bundle.rs`,
  `provenance.rs`, `artifacts.rs`, and `error.rs`.
- Move `command/report_run/bundle.rs`, `provenance.rs`, and `model_artifacts.rs`
  into that directory without changing their logic or output.
- Move `period_label` from `command/report_run/mod.rs` to
  `model_bundle/mod.rs`, unchanged.
- Add `ModelBundleError` with `EmptyPeriod`, `Bundle`, and `Output` variants
  that delegate the existing messages exactly.
- Update the bundle writer to return `ModelBundleError` instead of `RunError`.
- Add `RunError::Model(ModelBundleError)`; remove `RunError::Bundle` and
  `ReportError::EmptyPeriod`. Update `Display` and `Error::source` to preserve
  the current strings and chains.
- Update `command/report_run/mod.rs` and `spec.rs` to call the neutral writer
  and map its errors into `RunError::Model`.
- Widen only the visibility the move needs: `model_bundle_from`,
  `model_bundle_with_spec`, `BundleOutput`, `SpecRunArtifacts`,
  `ModelBundleError`, `period_label`, `panel_provenance`, and
  `resolve_spec_path` become `pub(crate)`; sibling-only helpers stay
  `pub(super)`.
- Remove the `model_bundle_from` re-export from `command.rs`; update
  `backtest/run.rs` to import `crate::model_bundle::model_bundle_from`.
- Add `mod model_bundle;` to `main.rs`.
- Rename the structural rule to `stage-command-coupling`, reject any
  `crate::command` reference in the six stage directories, and add
  `artifact-command-coupling` for `request.rs` and `model_bundle/`.
- Add error-message tests for `EmptyPeriod`, `Bundle`, and `Output`.
- Update module documentation, the assessment, this plan, and the AI log.

Excluded:

- REF-2026-11-06-03 (the shared date module).
- REF-2026-11-06-04 and REF-2026-11-06-06 (observations, no cycle).
- Any formula, reading, event, strategy, execution, portfolio, analytics, or
  command behavior change.
- Any fixture, expected-value, command, schema, error-string, or output change.
- Any rename outside the moved functions, types, and the guard rule.
- Any new dependency, `#[allow]`, `unsafe`, `unwrap`, or `expect`.

## 4. Assumptions and uncertainty

- The bundle writer's only command dependency is `RunError` and
  `ReportError::EmptyPeriod`. A neutral error enum can preserve the exact
  strings: "no candle falls inside the requested period {period}", the
  `BundleError` messages, and "cannot write output: {error}".
- The report and backtest bundles are deterministic. The backtest bundle hash was
  identical across two output directories and contains no clock value or
  path-dependent bytes.
- `period_label` is a pure function of `RunConfig`; moving it does not change a
  label.
- `SpecRunArtifacts` and `model_bundle_with_spec` are used only by the
  `report model` command; moving them with the writer is safe.
- `backtest/run.rs` maps the writer failure with `to_string()`; the neutral
  error's `Display` is the same string the old `RunError` produced.
- `RunError::Bundle` is used only by the moved writer, and
  `ReportError::EmptyPeriod` is used only by the moved writer and `spec.rs`.
  The compiler and clippy prove there is no other use.
- `model_bundle` may depend on `config`, `pipeline`, `render_report`, and the
  model crate without a cycle. The guard rejects a `crate::command` import in
  the new module.
- The guard rename from `stage-request-coupling` to `stage-command-coupling`
  reflects the broader invariant; it is a reviewed rename, not a drive-by edit.
- Adding error-message tests raises the test count; no existing test is removed
  or weakened.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Module name | `crate::model_bundle` | One name for the model-bundle artifact contract |
| Layout | `mod.rs`, `bundle.rs`, `provenance.rs`, `artifacts.rs`, `error.rs` | Each file keeps one role; the writer is under 400 lines |
| Error type | `ModelBundleError` | Neutral; preserves the existing strings |
| `RunError` | Add `Model`, remove `Bundle` | The command error wraps the neutral failure |
| `ReportError` | Remove `EmptyPeriod` | The empty-period failure belongs to the bundle writer |
| `period_label` | Move to `model_bundle/mod.rs` | One label definition used by the writer and spec loader |
| Visibility | Only `pub(crate)` for the cross-module entry points; `pub(super)` for sibling helpers | No API expansion; the moved modules keep their narrow surface |
| Guard | Split stage and artifact rules | Rejects any future stage/artifact import of `crate::command` |
| Backtest hash | Sorted-file recursive hash | Reproducible equivalence target |
| Issue | Create after approval | The board tracks the implementation |
| Dependencies | None | No `Cargo.toml` or lockfile change |

## 6. Acceptance criteria

- `crate::model_bundle` holds the moved writer, provenance, artifacts, and
  `period_label`; `command/report_run` contains only `mod.rs`, `parse.rs`, and
  `spec.rs`.
- `grep -rn "crate::command"` over the six stage directories returns nothing.
- `backtest/run.rs` imports `crate::model_bundle::model_bundle_from`.
- `crate::request` and `crate::model_bundle` do not import `crate::command`.
- `RunError` has a `Model(ModelBundleError)` variant and no `Bundle` variant;
  `ReportError` has no `EmptyPeriod` variant.
- The `EmptyPeriod`, `BundleError`, and output error strings are byte-identical;
  the new unit tests and the existing CLI tests prove them.
- `cargo test --workspace --no-fail-fast` passes; 1,309 tests remain plus the new
  error-message tests; no test is removed, skipped, or weakened.
- The golden fixture hash remains `da7389...`.
- The report bundle hash remains `59124f...`.
- The backtest bundle hash remains `4173b505...`.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo doc`, the file-size gate, the structural gate, and every documentation
  and registry gate pass.
- The diff contains only the move, the neutral error, import updates, the guard,
  tests, and documentation.

## 7. Work breakdown

| Step | Work | Paths | Owner |
|---:|---|---|---|
| 1 | Create `model_bundle/`; move the three writer files and `period_label`; add the module doc | `src/model_bundle/*`, `src/command/report_run/mod.rs` | Refactor Implementer |
| 2 | Add `ModelBundleError` and its tests; update the writer's error returns | `src/model_bundle/error.rs`, `bundle.rs` | Refactor Implementer |
| 3 | Update `command/types.rs`, `report_run/mod.rs`, `report_run/spec.rs`, and `command.rs` | command files | Refactor Implementer |
| 4 | Update `backtest/run.rs` and `main.rs`; remove the command re-export | `src/backtest/run.rs`, `src/main.rs` | Refactor Implementer |
| 5 | Broaden the guard and add its tests | `tools/check_structure.py`, `tools/test_check_structure.py` | Refactor Implementer |
| 6 | Run the full gate; compare the three hashes; record the evidence | this plan, assessment | Refactor Verifier |
| 7 | Update the assessment status and links after merge; update the AI log | assessment, AI log | Refactor Implementer / Verifier |

Steps 1 and 2 may be separate commits; no step may change logic to make a path
compile.

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

- `cargo test --workspace --no-fail-fast`: 1,309 passed plus the new error tests,
  0 failed, 8 ignored.
- Golden fixture combined hash: `da7389...`, unchanged.
- Report bundle hash: `59124f...`, unchanged and identical across two output
  directories.
- Backtest bundle hash: `4173b505...`, unchanged and identical across two output
  directories.
- `git diff --stat` shows only the paths named in section 7.
- The CLI non-empty-output refusal test still asserts the same `BundleError`
  string; the new unit tests assert the empty-period, bundle, and output
  messages.
- The new structure tests prove both guards catch a violation and accept a clean
  tree.

Limitations:

- The move proves behavior preservation only for the paths the tests exercise.
  The two bundle hashes and the full gate cover the command surface, not every
  internal call path.
- If one agent performs the implementer, reviewer, and verifier roles, the
  sequential passes are disclosed and are not an independent review.

## 9. Risks and recovery

| Risk | Mitigation | Recovery |
|---|---|---|
| An error string changes | New unit tests plus the CLI refusal test; compare before/after | Revert the error mapping and re-plan |
| A bundle byte changes | Compare the report and backtest hashes | Revert and investigate the move |
| The neutral error loses a source chain | Preserve `BundleError` and `io::Error` as `source()`; test the chain | Restore the delegation |
| A hidden command dependency remains in the moved files | `grep -rn "crate::command"` in `model_bundle`; the guard test | Move the dependency or re-scope |
| The guard false-positives on a clean stage | The negative test uses `crate::model_bundle`; the guard scans only `crate::command` | Narrow the rule |
| REF-03 is pulled into the diff | Scope review and `git diff --stat` | Stop; open a separate cycle |

Rollback is the branch. There is no migration, schema, or external state.

## 10. Stop conditions

- A move requires a logic edit outside the error mapping.
- A fixture, expected value, command, schema, error string, stdout, stderr, or
  exit code changes.
- The report or backtest bundle hash changes.
- A test fails for a reason outside the move.
- A new dependency or a module-boundary decision outside the plan is required.
- The guard cannot pass without also moving REF-03.
- The full gate cannot pass without weakening a test or adding `#[allow]`.

## 11. Traceability

```text
REF-2026-11-06-05 -> #528 -> PR TBD -> verification record
```

The issue is [quant-rule/quant-rule#528](https://github.com/quant-rule/quant-rule/issues/528).
Its scope and done-when criteria are those in sections 3 and 6.

## 12. Verification record

| Check | Result |
|---|---|
| Model-bundle module complete | `src/model_bundle/` is 1,256 lines across five files; `command/report_run/` contains only `mod.rs`, `parse.rs`, and `spec.rs`; `RunError::Model` exists; `RunError::Bundle` and `ReportError::EmptyPeriod` are removed |
| Stage-to-command references | 0 in the six stage directories; 0 in `request.rs` and `model_bundle/` |
| Guard and tests | `stage-command-coupling` and `artifact-command-coupling` added; 17 structure tests pass, including 6 stage/artifact tests |
| Error strings | the `EmptyPeriod`, `BundleError`, and output unit tests pass; the CLI non-empty-output refusal test passes |
| Test inventory | 1,312 passed, 0 failed, 8 ignored (1,309 plus 3 new error-message tests) |
| Golden fixture hash | `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1`; unchanged |
| Report bundle hash | `59124f63a2741697c533e699eeaf191684d84811d354b7778518b4d03ef82512`; unchanged |
| Backtest bundle hash | `4173b5055814048597ee5e946da9b117e34dadf5f867de750aaa2f2cf15f085e`; unchanged |
| Full gate | fmt, clippy, test, doc, file size, structure, registry, validation, verification matrix, verification evidence, emitters, command surface, links, terms, legacy, oracle, progression, improvement, research, program, coverage, and diff check pass |
| Clean-worktree verifier pass | at commit `9dc160a`: every gate passes; 1,312 passed, 0 failed, 8 ignored; all three hashes unchanged; both command-coupling guards pass. Sequential pass by the same agent, not an independent review |
| Diff scope | 11 code files and 2 tool files; no fixture, expected-value, command, schema, error-string, or output change |
| Assessment status | `In progress`; local and clean-worktree verification complete; post-merge verification pending |
| Loop decision | pending the post-merge verifier pass and re-scan |
