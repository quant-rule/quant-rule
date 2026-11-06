# First Refactor Cycle — Split `report_run` by Its Seams

- Date: 2026-11-06 (UTC)
- Status: Approved
- Issue: #521 `Execute the first behavior-neutral refactor cycle`
- Assessment: [`REF-2026-11-06-01`](../../refactors/2026-11-06/current-structure-refactor.md)
- Branch: `refactor/521-split-report-run`
- Worktree: `../quant-rule--521-split-report-run`
- Claim type: behavior-neutral module split. No formula, event, strategy,
  execution, portfolio, analytics, command, or output change.

## 1. Intent

Split `crates/quantrule-shell/src/command/report_run.rs` along the seams that
already exist in the file: argument parsing, command orchestration, bundle
assembly, provenance assembly, and spec/panel loading. The command keeps the
same output bytes and the same public behavior.

## 2. Current state

| Item | Observed |
|---|---:|
| File lines | 856 |
| Production lines | 824 (tests start at line 825) |
| Responsibilities | parsing, orchestration, bundle assembly, provenance, panel/spec loading, emission |
| Shell tests | `report_command.rs` checks bundle shape, provenance boundary, event ordering, and byte determinism |
| Baseline commit | `73fff19` |
| Test inventory | 1,309 passed, 0 failed, 8 ignored |
| Golden fixture hash | `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1` |
| Bundle manifest hash | `9f4c9d263293a76350f30720830d1a32037deaec1f6a1981f256bb375aca1e54` |

## 3. Scope

Included:

- Create `crates/quantrule-shell/src/command/report_run/` with `mod.rs`,
  `parse.rs`, `bundle.rs`, `provenance.rs`, and `spec.rs`.
- Move the existing functions without changing their logic, signatures, or
  output.
- Move the parser tests with `parse_report_arguments`.
- Keep the module docs accurate for each new module.
- Update only visibility needed for sibling modules.

Excluded:

- REF-2026-11-06-02 (`command::types` coupling).
- REF-2026-11-06-03 (`config` importing source date types).
- Any rename outside the moved functions.
- Any output, provenance, schema, event, or fixture change.
- Any new dependency or abstraction.

## 4. Assumptions and uncertainty

- The shell tests exercise the bundle path, the provenance boundary, the spec
  path, and byte determinism.
- A function move that needs a logic edit is outside this plan; stop and re-plan.
- Visibility changes are limited to `pub(super)` or `pub(crate)` needed by the
  new child modules.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Module layout | `mod.rs`, `parse.rs`, `bundle.rs`, `provenance.rs`, `spec.rs` | Each module has one reason to change |
| Parser tests | Child `parse/tests.rs` or inline in `parse.rs` | Tests stay with the parser |
| Shared labels | Small `pub(super)` helpers in `mod.rs` | No new utility module |
| Visibility | Narrowest possible `pub(super)`/`pub(crate)` | No public API expansion |
| Behavior | Move-only | Fixture and bundle hashes must match |

## 6. Acceptance criteria

- `report_run.rs` is replaced by a directory module; no file in it exceeds 400
  lines.
- `mod.rs` keeps orchestration and emission and stays under 250 lines.
- `parse.rs`, `bundle.rs`, `provenance.rs`, and `spec.rs` each hold one
  responsibility and have module docs.
- The parser tests still pass and remain with the parser.
- Test inventory remains 1,309 passed, 0 failed, 8 ignored.
- Golden fixture hash and bundle manifest hash match the baseline.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo doc`, the file-size gate, the structural gate, and every
  documentation and registry gate pass.
- No fixture, expected value, command, schema, or behavior change.

## 7. Work breakdown

| Step | Work | Paths | Owner |
|---:|---|---|---|
| 1 | Create the directory module and move parsing with its tests | `command/report_run/mod.rs`, `parse.rs` | Refactor Implementer |
| 2 | Move bundle assembly and `BundleOutput`/`SpecRunArtifacts` | `bundle.rs` | Refactor Implementer |
| 3 | Move provenance and label helpers | `provenance.rs` | Refactor Implementer |
| 4 | Move spec and panel loading helpers | `spec.rs` | Refactor Implementer |
| 5 | Keep orchestration and emission in `mod.rs`; fix visibility | `mod.rs` | Refactor Implementer |
| 6 | Run the full gate and record the verification | plan and assessment | Refactor Verifier |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps
python3 tools/check_file_size.py
python3 tools/check_structure.py
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
git diff --check
```

Equivalence evidence:

- `cargo test --workspace --no-fail-fast`: 1,309 passed, 0 failed, 8 ignored.
- Golden fixture combined hash:
  `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1`.
- `report model` over the fixture config: artifact manifest hash
  `9f4c9d263293a76350f30720830d1a32037deaec1f6a1981f256bb375aca1e54`.
- No test assertion removed or changed; only module paths for the moved parser
  tests may change.

## 9. Risks and recovery

| Risk | Mitigation | Recovery |
|---|---|---|
| A moved function changes behavior | Move-only; run the shell bundle tests before and after | Revert the commit and re-plan |
| Visibility widens too far | Use `pub(super)`/`pub(crate)` only | Review the diff and narrow |
| A test loses its module path | Keep parser tests in `parse.rs`/`parse/tests.rs` | Restore the test module |
| A fixture or expected value changes | Stop; the refactor is not behavior-neutral | Revert and investigate |

Rollback is the branch. There is no migration or external state.

## 10. Stop conditions

- A move requires a logic edit.
- A fixture, expected value, command, or schema changes.
- A test fails for a reason outside the move.
- A new dependency or architecture decision is required.
- The full gate cannot pass without weakening a test.

## 11. Traceability

```text
REF-2026-11-06-01 -> #521 -> PR TBD -> verification record
```

## 12. Verification record

Fill in when the cycle is verified.

| Check | Result |
|---|---|
| Module split complete | `mod.rs` 144, `parse.rs` 142, `bundle.rs` 275, `provenance.rs` 261, `spec.rs` 94 lines |
| Largest new file | 275 lines (`bundle.rs`) |
| Test inventory | 1,309 passed, 0 failed, 8 ignored |
| Golden fixture hash | `da738931a8643bc2cc0db915174d713141e894614ee06f0087d9a3247497b5f1`; unchanged |
| Bundle manifest hash | `9f4c9d263293a76350f30720830d1a32037deaec1f6a1981f256bb375aca1e54`; unchanged |
| Full gate | fmt, clippy, test, doc, file size, structure, registry, validation, verification, emitters, command surface, links, terms, legacy, and oracle pass |
| Independent verifier | a clean-worktree pass by the same agent re-ran the full gate and both hashes; the sequential pass is disclosed and does not claim independent review; a separate verifier is still pending |
| Assessment status | `In progress`; moves to `Refactored` after merge and `Verified` after separate verifier closure |
| Loop decision | pre-merge verification complete; final loop decision pending merge and separate verifier closure |
