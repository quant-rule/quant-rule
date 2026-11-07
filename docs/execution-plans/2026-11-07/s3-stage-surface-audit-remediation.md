# S3.x Stage-Surface Audit — Remediation Plan

- Date: 2026-11-07 (UTC)
- Status: PRs #534/#535/#536 open 2026-11-07; checks pass; merge and independent verification pending
- Issue: none (the fix issues are created only after approval; no issue number is
  invented)
- Audit: [`docs/audits/2026-11-07/s3-stage-surface-audit.md`](../../audits/2026-11-07/s3-stage-surface-audit.md)
- Removal precedent: [`docs/execution-plans/2026-11-07/remove-allow-short.md`](remove-allow-short.md)
- Baseline: `feat/remove-allow-short` at `e5fe7de`, which contains the removal
  commit `310307b` on top of the audit baseline `fbc38a0`
- Claim type: remediation plan and verification contract; no edge claim
- Roles: Planner, Quant Researcher, Quant Developer, Quant Validator,
  Implementation, Test Engineer, Code Reviewer

## 1. Intent

Close the nine remaining findings of the S3.x stage-surface audit without
changing the model or strategy formulas beyond the corrections the findings
require. `AUD-2026-11-07-03` is already `Won't fix`: the human removed the
`allow_short` rule, and both directions are always available.

The plan makes each remaining finding reviewable before code changes: it records
the Planner's re-verification against the current tree, the chosen remediation,
the proposed sub-issues, and the regression guard for each finding.

## 2. Current state

The audit recorded three Critical, two Major, and five Minor findings. The
human decision removed the `allow_short` finding (`AUD-2026-11-07-03`,
`Won't fix`). The remaining nine findings were re-read against the branch based
on `e5fe7de`:

| Finding | Severity | Re-verified | Evidence path unchanged by the removal |
|---|---|---|---|
| `AUD-2026-11-07-01` | Critical | Yes | `crates/quantrule-shell/src/backtest/run.rs`, `crates/quantrule-shell/src/strategy/run.rs` |
| `AUD-2026-11-07-02` | Critical | Yes | `crates/quantrule-shell/src/strategy/bundle.rs`, `crates/quantrule-backtest/src/strategy/event_input.rs` |
| `AUD-2026-11-07-04` | Major | Yes | `crates/quantrule-shell/src/config/strategy.rs`, `crates/quantrule-backtest/src/strategy/event.rs`, `bracket.rs` |
| `AUD-2026-11-07-05` | Major | Yes | `crates/quantrule-shell/src/strategy/bundle.rs`, `crates/quantrule-backtest/src/strategy/event_input.rs` |
| `AUD-2026-11-07-06` | Minor | Yes | `crates/quantrule-shell/src/backtest/run.rs` (`manifest_bytes`) |
| `AUD-2026-11-07-07` | Minor | Yes | `docs/execution-plans/2026-11-06/s3-command-surface.md`, `docs/architecture/command-surface.md`, `docs/README.md` |
| `AUD-2026-11-07-08` | Minor | Yes | `crates/quantrule-backtest/src/strategy/event.rs`, `bracket.rs`, shell strategy tests |
| `AUD-2026-11-07-09` | Minor | Yes | `crates/quantrule-shell/src/backtest/run.rs` |
| `AUD-2026-11-07-10` | Minor | Yes | `docs/config/2026-11-06/engine-artifacts.md`, `crates/quantrule-shell/src/strategy/run.rs` |

The removal commit changed the portfolio sizing path and the shared run-config
bytes; it did not change the code paths above. The audit's reproduction recipes
remain valid. The audit document remains the status record; this plan does not
claim that a finding is `Fixed` or `Verified`.

## 3. Scope

### Included

- The nine remaining findings, their fixes, regression tests, and documentation.
- A shared strategy/model validation path for the execution timeframe and the
  condition/context relationship.
- Registry and role validation for strategy event identities, and one canonical
  event stream for the standalone and coupled strategy paths.
- The event-window contract decision and its implementation or documentation.
- The manifest strategy identity, the strategy causality test, the backtest
  target check, and the documentation/evidence-table corrections.
- The full gate and the Quant Validator record for the strategy-behavior fixes.

### Excluded

- `AUD-2026-11-07-03`; the rule is removed and the finding is `Won't fix`.
- Model formulas, event definitions, progression arithmetic, and strategy
  formulas not required by a finding.
- A new strategy hypothesis, a profitability claim, or an edge claim.
- A new dependency, a model schema change, or a new architecture boundary
  without a separate approved decision.
- Creating fix issues, branches, commits, pushes, PRs, or merges before this
  plan is approved.
- Rewriting historical audit/refactor/experiment evidence.

## 4. Assumptions and uncertainty

- The audit document is the finding record; this plan is the Planner's
  verification and proposal. The human approves the plan before any fix.
- `AUD-2026-11-07-05` is a strategy-language decision, not only a code defect:
  the contract says the window is in execution bars, while the implementation
  uses each timeframe's bars. The human must choose conversion or documentation.
- `AUD-2026-11-07-02` and `AUD-2026-11-07-04` are coupled. Restricting the
  bracket to the registered compression-range event removes the observed
  diagnostic bracket, but the two event streams still need one definition.
- The registry is a repository artifact. A parser-level registry check can read
  it from the shell, or a generated static table can live in the engine. The
  plan recommends the generated table if the binary is expected to run outside
  the repository root; the human approves the boundary.
- The fixes to findings 01, 02, 04, and 05 change strategy behavior. They need
  the Quant Researcher, Quant Developer, and Quant Validator under the quant
  role rules. No edge claim is made.
- The test suite is green on the branch: 1,312 tests, 0 failed, 8 ignored. The
  fixes must keep it green and add fail-before/pass-after guards.

## 5. Decisions reserved to the human

| ID | Decision | Options | Planner recommendation |
|---|---|---|---|
| D1 | Event-window semantics for `event_window_bars` | A. Convert the execution-bar window to each context timeframe; B. keep per-timeframe bars and change the contract; C. remove context events from the window | A; it preserves the published “execution bars” meaning. B is acceptable if the human prefers the current behavior as the contract. |
| D2 | Canonical strategy event stream | A. Both paths use `timeframe.all_events()` plus the role gate; B. both paths use the core `timeframe.events()` batch; C. publish a dedicated strategy event view | A if extended events are intended as strategy inputs; B if only the core batch is. The choice must be recorded in the model/strategy contract. |
| D3 | Registry lookup location | A. A generated static registry table in the engine; B. the shell reads `docs/registry/indicators.json` at parse time; C. publish a role code in the event record | A for a self-contained binary; C is the structural long-term guard. |
| D4 | Finding 04 scope | A. Parser validation plus compression-range whitelist; B. also publish/filter a role code at the strategy input | A now; C from D3 as a follow-up if the event schema can carry a role code. |

D1, D2, and D3 must be settled in the plan approval. D4 is a scope choice.

## 6. Remediation by finding

| Finding | Remediation | Regression guard | Quant roles |
|---|---|---|---|
| 01 | Reject a strategy whose declared execution timeframe differs from the run's `execution.fill_timeframe`; validate every condition timeframe against execution plus context before a bundle is written. Share the validator with `strategy explain`/`run`. | A 15m strategy against a 5m run fails; a condition outside execution/context fails; a valid run's decisions are byte-identical. | Researcher, Developer, Validator |
| 02 | Make the standalone and coupled paths consume one canonical event batch. If D2=A, the runner uses `all_events()` and the role gate applies; if D2=B, the bundle reader uses the core batch. | The same model bundle, strategy, and bar produce the same decision through `strategy explain`, `strategy run`, and `backtest run`, including an extended-event fixture. | Researcher, Developer, Validator |
| 04 | Validate every configured `(event_type, subtype, source)` against the registry; reject unregistered and diagnostic identities; restrict the compression bracket to the registered compression-range event; add a boundary test over `EventStrategyInput`. | An unregistered config is rejected at parse; a diagnostic bracket config is rejected; a boundary test cannot match a diagnostic event. | Researcher, Developer, Validator |
| 05 | Implement D1. If A, convert the execution-bar window to each context timeframe's bar count; add the conversion test. If B, correct the contract and test the per-timeframe behavior. | A 5m/1h fixture with a 20-bar window follows the chosen contract; a later bar cannot enter an earlier window. | Researcher, Developer, Validator |
| 06 | Add `strategy_config_id` to the backtest manifest and assert it matches the strategy provenance. | Manifest test; manifest-only reader can identify the strategy config. | Engineering, Test |
| 07 | Update the current S3.x status lines and index; leave the dated phase records as history. | Current status matches Git; `check_links.py` and `check_terms.py` pass. | Engineering, Docs |
| 08 | Add a strategy causality test that appends, changes, and removes later bars and asserts the earlier decision is unchanged, for both engines. | The test fails if an earlier decision reads a later bar. | Test, Code Reviewer |
| 09 | Check `ensure_available` on the backtest root before the model write. | A root containing an unrelated file is refused; a clean root still runs. | Engineering, Test |
| 10 | Remove the unused `level` and `progression` rows from the evidence table, or add them as an approved rule-language change. | The table lists only emitted kinds; `rg` finds no unsupported evidence kind. | Docs, Engineering |

## 7. Proposed sub-issues

Issue numbers are assigned only after approval. The proposed sub-issues are:

| Sub-issue | Title | Covers | Depends on | Primary paths |
|---|---|---|---|---|
| S1 | Validate the strategy execution timeframe and context | 01 | — | `crates/quantrule-shell/src/backtest/run.rs`, `crates/quantrule-shell/src/strategy/` |
| S2 | Validate strategy event identities against the registry | 04 | D3, D4 | `crates/quantrule-shell/src/config/strategy.rs`, `crates/quantrule-backtest/src/strategy/` |
| S3 | Unify the standalone and coupled strategy event stream | 02 | D2, S2 | `crates/quantrule-shell/src/strategy/bundle.rs`, `crates/quantrule-backtest/src/strategy/event_input.rs` |
| S4 | Resolve and implement the event-window contract | 05 | D1, S3 | `crates/quantrule-shell/src/strategy/`, `crates/quantrule-backtest/src/strategy/`, `docs/config/2026-11-06/strategy-config.md` |
| S5 | Add the strategy config identity to the backtest manifest | 06 | — | `crates/quantrule-shell/src/backtest/run.rs`, manifest tests |
| S6 | Add the strategy causality test | 08 | S3, S4 | `crates/quantrule-backtest/src/strategy/` tests |
| S7 | Correct the S3.x status and evidence documentation | 07, 10 | — | `docs/execution-plans/2026-11-06/s3-command-surface.md`, `docs/architecture/command-surface.md`, `docs/README.md`, `docs/config/2026-11-06/engine-artifacts.md` |
| S8 | Refuse an unrelated file in the backtest root | 09 | — | `crates/quantrule-shell/src/backtest/run.rs`, `crates/quantrule-shell/tests/backtest_command.rs` |

Ordering: S1, S5, S7, and S8 are independent. S2 precedes S3 so the event stream
has one role rule. S3 precedes S4 and S6. S6 is test-only and can follow S4.
S1-S4 run under the Quant roles; S5-S8 are Engineering/Test/Docs work.

## 8. Traceability

| Finding | Sub-issue | PR | Verification |
|---|---|---|---|
| `AUD-2026-11-07-01` | S1 | #536 | Timeframe/context regression tests; full gate; Quant Validator record |
| `AUD-2026-11-07-02` | S3 | #536 | Cross-path decision equality test; full gate; Quant Validator record |
| `AUD-2026-11-07-04` | S2 | #536 | Registry/role rejection tests; `EventStrategyInput` boundary test; full gate; Quant Validator record |
| `AUD-2026-11-07-05` | S4 | #536 | Event-window contract test; full gate; Quant Validator record |
| `AUD-2026-11-07-06` | S5 | #536 | Manifest field test; full gate |
| `AUD-2026-11-07-07` | S7 | #536 | Status/doc read; link and term checks |
| `AUD-2026-11-07-08` | S6 | #536 | Causality test fails before the guard; passes after |
| `AUD-2026-11-07-09` | S8 | #536 | Non-empty-root refusal test |
| `AUD-2026-11-07-10` | S7 | #536 | Evidence-table read; term check |

A finding moves to `Planned` when this plan is approved, `In progress` when its
sub-issue starts, and `Fixed` only after the fix is merged. The Improvement
Verifier alone moves it to `Verified`. `AUD-2026-11-07-03` stays `Won't fix`.

## 9. Acceptance criteria

- Every remaining Critical and Major finding (`01`, `02`, `04`, `05`) is
  `Verified`, or the human records a dated `Won't fix` with residual-risk
  acceptance.
- Every fix has a regression guard that fails before the fix and passes after
  it, or a written reason why a fail-before test is impossible.
- The strategy/run timeframe and condition/context relationship is rejected
  before a bundle is written; a valid run is unchanged.
- The standalone and coupled strategy paths use one event definition and produce
  the same decision for the same model bundle, strategy, and bar.
- A diagnostic or unregistered event identity cannot enter a strategy decision;
  the compression bracket is restricted to the registered range event.
- The event-window contract matches the implementation.
- The manifest names the strategy config identity.
- The strategy causality test exists and covers both engines.
- The backtest root refuses unrelated files.
- The current docs and evidence table match the behavior.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test
  --workspace`, `cargo doc`, the contract checks, the structure check, and
  `git diff --check` pass on the merged tree.
- No test is weakened, skipped, or removed to obtain a pass.
- The Quant Validator record covers S1-S4; no edge claim is made.

## 10. Verification

| Check | Expected |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | pass; new guards included |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `python3 tools/check_command_surface.py` | pass |
| `python3 tools/check_registry.py` | pass |
| `python3 tools/check_validation.py` | pass |
| `python3 tools/check_verification.py` | pass |
| `python3 tools/check_verification_matrix.py` | pass |
| `python3 tools/check_event_emitters.py` | pass |
| `python3 tools/check_links.py` | pass |
| `python3 tools/check_terms.py` | pass |
| `python3 tools/check_legacy.py` | pass |
| `python3 tools/check_file_size.py` | pass |
| `python3 tools/check_structure.py` | pass |
| `git diff --check` | pass |
| Cross-path strategy test | explain, run, and backtest agree on the fixture and on an extended-event fixture |
| Timeframe/context test | 15m strategy against a 5m run is refused; missing context condition is refused |
| Registry/role test | unregistered and diagnostic configs are refused; bracket is compression-range only |
| Event-window test | the chosen contract holds for a 5m/1h fixture with a multi-bar window |
| Manifest test | `strategy_config_id` is present and matches the strategy provenance |
| Causality test | later bars do not change an earlier decision |
| Backtest-root test | an unrelated file is refused |
| Quant Validator | reproduction record for S1-S4 before merge |

## 11. Risks and recovery

- **Strategy behavior changes.** S1-S4 reject configurations and change event
  visibility. The Quant Researcher defines the intended behavior, the Quant
  Developer implements it, and the Quant Validator reproduces it. A change that
  cannot be explained as a correction to the audit finding stops the plan.
- **Event-stream choice.** D2 changes what the strategy can see. The human
  chooses; the plan does not silently select A or B.
- **Registry boundary.** A generated table is a build-time artifact; a shell
  read of `docs/` depends on the working directory. The chosen option is
  recorded in the plan and tested.
- **Event-window semantics.** D1 can change existing strategy decisions. The
  human approves the choice before S4 starts.
- **Test weakening.** The old behavior is not preserved by deleting a test; each
  fix carries a fail-before/pass-after guard.
- **Rollback.** Each sub-issue is a separate branch/PR; the removal branch is
  already isolated. No history is rewritten.

## 12. Stop conditions

- D1, D2, or D3 is not settled by the human.
- A fix requires a model schema change, a new dependency, or an architecture
  boundary not named in this plan.
- A finding no longer reproduces; the Planner marks it `Stale` instead of
  inventing a fix.
- A gate fails for a reason outside the approved scope.
- A commit, push, PR, merge, or issue creation is required without the
  corresponding authorization.
- The Quant Validator cannot reproduce a strategy-behavior fix.

## 13. Approval

This plan is `Approved` on 2026-11-07. The human instruction is “proceed with
the full remediation plan following your recommendations”, which selects
D1 = A, D2 = A, D3 = A, and D4 = A. The fix issues are created after this
approval; implementation follows the work breakdown. Approval authorizes only
the scope in this plan, not an edge claim, a commit, a push, a PR, or a merge.

## 14. Implementation record

Observed on `fix/s3-audit-remediation` based on `2f04ca2` on 2026-11-07. The
work is committed as `cb08c11` and published in PR #536, stacked on PR #535 and
PR #534; it is not merged. The code and test changes are one review batch
because several sub-issues share files; the sub-issue split is recorded in
section 7, not in separate commits.

| Sub-issue | Implementation | Verification |
|---|---|---|
| S1 | Shared `strategy::context::validate_context` checks the strategy execution timeframe, published context timeframes, and condition timeframes; `strategy explain`/`run` and `backtest run` use it. The coupled runner rejects a declared/run mismatch before the model bundle is written. | `backtest_rejects_a_strategy_execution_timeframe_mismatch`; `backtest_rejects_a_condition_outside_execution_and_context`; existing strategy tests unchanged. |
| S2 | Build-time registry table generated from `docs/registry/indicators.json`; unregistered and diagnostic identities are rejected; the compression bracket is restricted to `(3,8,5)`. The build script uses the workspace's existing `serde_json` crate as a build-dependency; no new external crate is added. | `an_unregistered_event_condition_is_rejected`; `a_diagnostic_event_in_the_input_cannot_satisfy_the_rule`; `a_diagnostic_event_is_rejected_as_the_bracket_event`; `an_unregistered_event_is_rejected_as_the_bracket_event`; observed shell errors for the old unregistered and diagnostic configs. |
| S3 | `EventStrategyInput` reads `timeframe.all_events()`, matching the `events.csv` standalone path. | `the_standalone_and_coupled_strategy_decisions_agree_on_the_signal`. |
| S4 | `context_window_bars` converts the execution-bar window to each context timeframe; the bundle reader and the coupled input use it. | `the_context_window_is_expressed_in_execution_bars`; existing 5m/15m fixture unchanged because its window is one bar. |
| S5 | `strategy_config_id` is written to the backtest manifest. | `backtest_writes_the_stage_bundles` asserts it matches the strategy provenance. |
| S6 | A causality test compares a bar-40 snapshot from 60 bars, 80 bars, and a run with a changed later bar; all are equal. | `later_bars_do_not_change_an_earlier_snapshot`. |
| S7 | S3.x status lines and the evidence table were corrected; historical phase lines remain. | `check_links.py`, `check_terms.py`; manual read. |
| S8 | The backtest root is checked with `ensure_available` before the model write. | `backtest_refuses_an_unrelated_file_in_the_root`. |

Gate evidence:

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 1,323 passed, 0 failed, 8 ignored |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| Contract checks and structure check | pass; 603 Markdown files, 365 term-check files, 706 legacy files |
| `git diff --check` | pass |

The analytics and qualification golden fixtures were regenerated because the
manifest gained `strategy_config_id`; only the manifest hash moved. The decision
and accounting values are unchanged.

Outstanding before a fix can be `Fixed`/`Verified`:

- An independent Quant Validator record for S1-S4.
- An independent Code Reviewer pass.
- Merge PRs #534 → #535 → #536 in order, retargeting the stacked bases to `main`
  after each merge. The local fix commit is a batch, not a per-sub-issue split.
- The merged-tree gate.
- The audit findings remain `Open` until the merged fix exists; S1-S8 are
  implementation evidence, not closure evidence.

Deviation from section 7: the implementation is on one remediation branch rather
than one branch per sub-issue. The changes are separable by sub-issue for commit
review; no PR has been opened.

## 15. Sequential review record

These passes were performed by one agent in the same session. They are **not**
independent Code Reviewer or Quant Validator reviews.

### Quant Validator (sequential)

- Scope: S1-S4, the findings that change strategy behavior. No model formula,
  event definition, strategy formula, or validation partition was opened.
- S1 rejects inconsistent configurations before a bundle; a valid fixture run is
  unchanged.
- S2 rejects unregistered/diagnostic identities and restricts the bracket to the
  registered range event; the fixture uses a registered identity and still runs.
- S3 uses one event set; the standalone/coupled decision parity test passes.
- S4 converts the window in execution-bar terms; the one-bar fixture is unchanged
  and the conversion test covers 20 bars at 5m/1h.
- Causality: `later_bars_do_not_change_an_earlier_snapshot`; execution and
  strategy determinism tests remain green.
- Residual risks: the registry table is generated at build time; a family with
  mixed roles is merged with diagnostic dominance; the extended event stream is
  visible to the input but no current engine can act on a diagnostic identity.
- Verdict: pass as a local contract-correction review; no edge claim and no
  independent validation.

### Code Reviewer (sequential)

- The full diff was reviewed: shared context validation, registry build script
  and table, event-stream change, window conversion, manifest field, tests,
  goldens, and docs.
- No unrelated change was found. The golden fixture updates move only the
  manifest hash; the analytics and qualification values are unchanged.
- Residual risks: the build now depends on the repository registry at build time;
  the batch is on one remediation branch rather than one branch per sub-issue; no
  independent review is claimed.

### Outstanding

- An independent Quant Validator record for S1-S4.
- An independent Code Reviewer pass.
- Merge PRs #534 → #535 → #536 in order, retargeting the stacked bases to `main`
  after each merge. The local commit is a batch, not a per-sub-issue split.
