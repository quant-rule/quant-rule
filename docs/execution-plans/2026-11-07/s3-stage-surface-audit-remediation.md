# S3.x Stage-Surface Audit — Remediation Plan

- Date: 2026-11-07 (UTC)
- Status: Approved 2026-11-07; D1-D4 = A; execution in progress
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
| `AUD-2026-11-07-01` | S1 | pending | Timeframe/context regression tests; full gate; Quant Validator record |
| `AUD-2026-11-07-02` | S3 | pending | Cross-path decision equality test; full gate; Quant Validator record |
| `AUD-2026-11-07-04` | S2 | pending | Registry/role rejection tests; `EventStrategyInput` boundary test; full gate; Quant Validator record |
| `AUD-2026-11-07-05` | S4 | pending | Event-window contract test; full gate; Quant Validator record |
| `AUD-2026-11-07-06` | S5 | pending | Manifest field test; full gate |
| `AUD-2026-11-07-07` | S7 | pending | Status/doc read; link and term checks |
| `AUD-2026-11-07-08` | S6 | pending | Causality test fails before the guard; passes after |
| `AUD-2026-11-07-09` | S8 | pending | Non-empty-root refusal test |
| `AUD-2026-11-07-10` | S7 | pending | Evidence-table read; term check |

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
