# Unified Ladder Audit Remediation Plan

- Date: 2026-10-31 (UTC)
- Status: Approved
- Work item: `audit-unified-ladder`
- Audit: [`docs/audits/2026-10-31/unified-ladder-audit.md`](../../audits/2026-10-31/unified-ladder-audit.md)
- AI transparency log: [`docs/ai-transparency-log/2026-10-31/audit-unified-ladder.md`](../../ai-transparency-log/2026-10-31/audit-unified-ladder.md)
- Baseline: `main` at `7c865de`
- Human approval: approved on 2026-10-31 (UTC); decisions D1–D12 accepted as recommended
- Claim type: program remediation plan; no strategy edge claim

## 1. Intent

Turn the 2026-10-31 unified-ladder audit into a reviewable remediation plan.
The plan fixes one Critical execution defect and the Major
migration/traceability, H4, strategy-context, ledger-accounting, and
model/analytics contract defects found at the baseline. It also resolves or
explicitly defers the Minor and Info findings.

The audit is read-only evidence. This plan is the only implementation
authorization after the human approves it.

## 2. Current state

- The nine-phase unified ladder program is merged at `7c865de`.
- Global gates pass: `fmt`, `clippy`, `test --workspace` (45 suites), and
  `doc` with warnings denied.
- The audit found 1 Critical, 7 Major, 16 Minor, and 6 Info findings.
- The Critical finding is a pending entry that fills on the first supplied bar
  after a data gap, using the pre-gap ATR.
- The Major findings are the v2 migration message, 4h path commands, inert
  strategy-context config, exit-quantity reconciliation, stale current
  docs/status, the execution-v3/v2 contract conflict, and the Layer 3 block
  schema.
- No code, tests, issues, or PRs were changed by the audit.

## 3. Scope

Included:

- The findings in the audit, grouped into eight work items.
- Regression tests that fail before each accepted fix and pass after it.
- Quant Validator records for the execution, ledger, model-order, and
  analytics-schema changes.
- Documentation, help, contract, status, experiment, and validator-record
  corrections.
- A post-fix control regression run to detect output drift; no new edge search.

Explicit non-goals:

- New strategy hypotheses, thresholds, or fitted weights.
- Statistical, machine-learning, or neural-network signal generation.
- Risk-based sizing, structural brackets, live trading, or broker integration.
- A new dependency or an `unsafe` block.
- Rewriting dated historical records outside the current-status surface.
- Implementing execution v3 behavior unless the human selects that decision.

## 4. Assumptions and uncertainty

- The Critical gap fix may change historical backtest outputs; the change is a
  correctness fix, so goldens are regenerated deliberately and a Quant Validator
  records the delta.
- The audit’s probe used a synthetic bar sequence; the fix must also cover the
  runner and a sparse CSV integration case.
- Some findings are contract choices, not mechanical fixes: partial exits,
  `context_lost` on missing context, dashboard forming-leg averages, and intent
  expiry/slippage wiring. Those decisions are listed in section 6 and stop for
  human direction.
- The full-year SOLUSD source is not committed, so post-fix regression evidence
  uses the checked-in fixtures and a documented fetch where needed.
- A finding may become stale if the current code changed by the time of
  implementation; the Planner re-verifies before opening issues.

## 5. Decisions

Each decision is proposed; none is approved until the human accepts this plan.

| ID | Decision | Options | Recommendation | Approval |
|---|---|---|---|---|
| D1 | Gap fill policy | A reject non-contiguous pending fill; B document next-available fill and re-anchor ATR; C cancel pending on gap | A | Pending |
| D2 | Execution schema | A v2 current, v3 target; B implement v3 now | A | Pending |
| D3 | Strategy context fields | A document inert fields; B use Structure score/kind in the rule; C remove fields | A | Pending |
| D4 | Partial exits | A reject quantity mismatch; B support partial exits; C clamp | A | Pending |
| D5 | Missing context while open | A hold; B forced exit documented; C policy switch | A or C | Pending |
| D6 | Layer 3 blocks | A fixed configured blocks with nulls; B variable width documented | A | Pending |
| D7 | H4 path commands | A build H4; B remove 4h from their parsers/help | A | Pending |
| D8 | Event batch order | A preserve detector order; B explicit contract rank; C narrow contract | A or B | Pending |
| D9 | Dashboard forming-leg average | A amend contract; B gate and regenerate golden | A or B | Pending |
| D10 | Intent expiry/slippage | A add config fields; B remove fields/example claim | A or B | Pending |
| D11 | Risk cap step | A step-aware cap; B conservative pre-size; C document override | A | Pending |
| D12 | Info reporting fields | A fix; B defer with a dated note | A for W5/W7, B for W8 | Pending |

## 6. Work breakdown

| Work item | Findings | Deliverable | Roles | Depends on |
|---|---|---|---|---|
| W1 | 01 | Gap-safe pending fills | Quant Developer, Test, Quant Validator, Code Reviewer | Approved 2026-10-31 |
| W2 | 02, 07, 10, 19 | Schema/migration truthfulness | Implementation, Test, Code Reviewer | Approved 2026-10-31 |
| W3 | 03, 06 (H4 parts), 29, 30 | 4h reachability and H4 docs/tests | Implementation, Test, Code Reviewer | Approved 2026-10-31 |
| W4 | 04, 12, 13, 20, 22 | Strategy context contract and safety | Quant Researcher (D3/D5/D10), Quant Developer, Test, Quant Validator | Approved 2026-10-31 |
| W5 | 05, 14, 15, 16, 17, 25, 26, 27 | Execution/portfolio invariants | Quant Developer, Test, Quant Validator, Code Reviewer | Approved 2026-10-31 |
| W6 | 08, 09, 11, 23, 24 | Model/analytics output contracts | Quant Developer, Test, Quant Validator | Approved 2026-10-31 |
| W7 | 06 (docs/status), 07, 18, 29 | Current docs/help/experiment consistency | Implementation, Test, Code Reviewer | Approved 2026-10-31 |
| W8 | 21, 28 | Boundary and monthly-return observations | Implementation, Test | Approved 2026-10-31 |

### Issue tree

The Planner created the work-item issues after approval. W2–W7 are tracked as
work items now; their proposed sub-issues are created when each work item starts,
so one leaf is in flight at a time.

| Work item | Issue |
|---|---|
| W1 | [#247](https://github.com/quant-rule/quant-rule/issues/247) |
| W2 | [#248](https://github.com/quant-rule/quant-rule/issues/248) |
| W3 | [#249](https://github.com/quant-rule/quant-rule/issues/249) |
| W4 | [#250](https://github.com/quant-rule/quant-rule/issues/250) |
| W5 | [#251](https://github.com/quant-rule/quant-rule/issues/251) |
| W6 | [#252](https://github.com/quant-rule/quant-rule/issues/252) |
| W7 | [#253](https://github.com/quant-rule/quant-rule/issues/253) |
| W8 | [#254](https://github.com/quant-rule/quant-rule/issues/254) |
| Parent | [#255](https://github.com/quant-rule/quant-rule/issues/255) |

Audit/plan PR: [#256](https://github.com/quant-rule/quant-rule/pull/256).
W1 PR: [#257](https://github.com/quant-rule/quant-rule/pull/257).

### W1 — Gap-safe pending fills (Critical)

- Reject a pending entry when the next supplied bar is not exactly one
  execution timeframe after the decision bar.
- Decide and document the pending-exit policy in the same change; an exit must
  not silently cross a gap either.
- Keep the rejection in the execution records; leave the position unchanged.
- Add a fail-before test with a missing execution bar, plus a runner sparse-CSV
  integration test.
- Quant Validator reproduces the rejection and checks that no position, fill,
  trade, or equity change occurs.
- Implementation evidence: pending entries and exits are rejected with
  `non_contiguous_bar`; the bracket still evaluates on the gap bar. The
  fail-before run failed 2 engine tests and 1 runner test; the fixed probe
  reports `fill count=0`. Validator record:
  `docs/reports/2026-10-31/w1-gap-fill-validation.md`. Issue #247, PR #257.

Proposed issue: **Reject pending fills across non-contiguous execution bars**.

### W2 — Schema and migration truthfulness

- Move the strategy schema check before field validation so a real v2 file gets
  the documented migration message; test with the historical v2 shape.
- Make execution v2 the current contract and mark v3 as planned, or implement
  v3 by separate approval; remove the “v2 retired” contradiction.
- Accept an absent `model.json.timeframes` as `{}`, or document it as required.
- Reconcile `no_trade.missing_context` (default vs required); recommend required
  and update the contract.
- Add parser tests for each chosen rule.
- Implementation evidence: schema-before-fields, optional `timeframes`, v2
  current/v3 target, and `missing_context` required. The real-v2 and
  absent-`timeframes` tests fail before and pass after; the two documentation
  findings are pinned by tests that match the strict contract. Issue #248.

Proposed sub-issues:
1. **Make the strategy v2 migration message reachable**
2. **Reconcile the execution v2/v3 contract status**
3. **Fix model `timeframes` and strategy `missing_context` optionality**

### W3 — 4h reachability and H4 docs/tests

- Build H4 for `model-series` and `market-chart` path commands, or remove the
  advertised option; recommend build.
- Update README/help/contracts to five series, H4 bucket rules, and the
  path-command contract.
- Add H4 CLI tests, a complete/partial/gap H4 fixture case, and a 4h
  `model-series` row assertion.
- Update `docs/config/2026-10-30/data-v3.md` relative path and the stale M15
  statement.

Proposed sub-issues:
1. **Make 4h reachable from model-series and market-chart**
2. **Align H4 docs, help, and tests with the data engine**

### W4 — Strategy context contract and safety

- Document the inert Structure weight, `kind` literals, stage histogram, and
  agreement codes as diagnostic-only, or approve a rule change (D3).
- Reject weight sets whose per-axis sum is not finite.
- Validate that a `LadderContext` belongs to the input bar/timeframe; use the
  model’s `COMPACT_STATE_THRESHOLD` for the neutral band.
- Decide and document the missing-context exit policy (D5).
- Decide whether `valid_until`/`max_slippage_bps` become strategy config fields
  or are removed from the shipped claim (D10).
- Add a context artifact to the strategy bundle (selected bar identity, missing
  timeframes, axis shares/agreements, conflict/late counts) or explicitly
  narrow the ADR’s audit claim.
- Add multi-context and open-position missing-context golden rows.

Proposed sub-issues:
1. **Document or use the inert strategy context fields**
2. **Guard context weights, identity, and neutral band**
3. **Persist the as-of context in the strategy bundle**
4. **Decide and test the missing-context exit policy**
5. **Wire or remove intent expiry and slippage**

### W5 — Execution/portfolio invariants

- Reject an exit fill whose quantity differs from the open position; add the
  invariant `final_equity = initial_cash + net_pnl` over a full close.
- Make the risk-envelope cap respect `quantity_step`.
- Reject a second `submit` on one bar or emit an explicit supersede record.
- Update `drawdown`/`peak_equity` on every equity change, or document mark-only.
- Decide the exit commission base (fill-time equity vs prior mark) and test a
  gap exit.
- Record the capped fill quantity in the order record; reconcile order, fill,
  position, and trade.
- Carry the `SizingRejection` cause into the report.
- Make `slippage.total` a quote-currency total or rename it.

Proposed sub-issues:
1. **Reject mismatched exit quantities**
2. **Make the risk cap step-aware**
3. **Guard one pending order per bar**
4. **Update portfolio equity state on every change**
5. **Reconcile capped fill records and sizing reasons**
6. **Fix slippage total units**

### W6 — Model/analytics output contracts

- Preserve the contract event order in `TimeframeModel::events` or narrow the
  contract; add a bar-28/bar-37 test.
- Pass the configured ladder to Layer 3 and emit null blocks for empty higher
  series; assert fixed width on a short source.
- Resolve the dashboard forming-leg average contract/golden conflict.
- Validate or reject an out-of-range divergence direction in the public
  aggregate.
- Decide whether a zero-bar context series is missing context or a preflight
  error; make `backtest` and `strategy-run` consistent.

Proposed sub-issues:
1. **Fix event batch order**
2. **Fix Layer 3 fixed block schema**
3. **Resolve the dashboard forming-leg contract**
4. **Guard the aggregate direction code**
5. **Unify missing context-series behavior**

### W7 — Current docs/status/help/experiment consistency

- Regenerate the README backtest envelope for the current 5m example or mark it
  historical; add the current fetch/data-version evidence.
- Update current status lines (PR #246 merged, Phase 1/4/9 status, context
  implemented, H4 support).
- Fix `help all` tutorial label, `help store ingest` `unchanged`, migration
  table 15m→5m, tutorial “executed” heading, `data-v3` path, strategy-v3 short
  momentum wording, and `improvement-loop` one-timeframe policy.
- Mark `EXP-2026-10-30-01` historical or archive its v2 fixture.
- Add an errata for the EXP-02 determinism rerun and record bundle hashes for
  the validator’s byte-identical claim.
- Add a current portfolio-schema section or annotate the historical doc.

Proposed sub-issues:
1. **Regenerate current README/status examples**
2. **Fix help, migration, tutorial, and contract drift**
3. **Archive or annotate stale experiment and validator evidence**

### W8 — Boundary and monthly-return observations

- Replace `crate::strategy::Side` with `crate::side::Side` in portfolio and add
  a boundary test.
- Carry the last available equity across an empty month or document the
  current rule; add a Jan+Mar test.

Proposed issue: **Fix portfolio boundary import and monthly-return gap**.

## 7. Traceability

| Finding | Work item | Proposed sub-issue | Verification |
|---|---|---|---|
| AUD-2026-10-31-01 | W1 | Gap-safe pending fills | Missing-bar rejection test; no position/fill/trade |
| AUD-2026-10-31-02 | W2 | v2 migration message | Real v2 fixture prints the migration sentence |
| AUD-2026-10-31-03 | W3 | 4h path commands | 4h model-series rows; 4h chart PNG |
| AUD-2026-10-31-04 | W4 | Inert context fields | Two structure weights: identical decisions + doc |
| AUD-2026-10-31-05 | W5 | Exit quantity rejection | Mismatch error; equity invariant |
| AUD-2026-10-31-06 | W3/W7 | H4/docs/status | README envelope; five-series help; status greps |
| AUD-2026-10-31-07 | W2 | Execution v2/v3 status | v2 passes; v3 documented target |
| AUD-2026-10-31-08 | W6 | Layer 3 blocks | Short source exports fixed blocks |
| AUD-2026-10-31-09 | W6 | Event order | Bar-28/bar-37 order test |
| AUD-2026-10-31-10 | W2 | Model optionality | Config without `timeframes` parses |
| AUD-2026-10-31-11 | W6 | Dashboard contract | Forming-leg test matches contract |
| AUD-2026-10-31-12 | W4 | Missing-context exit | Open-position golden row |
| AUD-2026-10-31-13 | W4 | Weight sum guard | `1e308` weights rejected |
| AUD-2026-10-31-14 | W5 | Step-aware cap | Capped quantity is a step multiple |
| AUD-2026-10-31-15 | W5 | One pending order | Two submits yield one order |
| AUD-2026-10-31-16 | W5 | Equity state update | Post-fill drawdown formula |
| AUD-2026-10-31-17 | W5 | Commission base | Gap-exit commission test |
| AUD-2026-10-31-18 | W7 | Strategy-run bounds | Config period honored |
| AUD-2026-10-31-19 | W2 | `missing_context` | Missing-key test |
| AUD-2026-10-31-20 | W4 | Context identity/band/audit | Mispair test; context artifact |
| AUD-2026-10-31-21 | W8 | Portfolio boundary | No portfolio→strategy import |
| AUD-2026-10-31-22 | W4 | Intent fields | Config-driven expiry/slippage test |
| AUD-2026-10-31-23 | W6 | Aggregate guard | Out-of-range direction no panic |
| AUD-2026-10-31-24 | W6 | Zero-bar context | Consistent command behavior |
| AUD-2026-10-31-25 | W5 | Capped fill record | Order/fill/position agree |
| AUD-2026-10-31-26 | W5 | Sizing causes | Distinct report reason |
| AUD-2026-10-31-27 | W5 | Slippage total | Currency total test |
| AUD-2026-10-31-28 | W8 | Monthly return | Jan+Mar test |
| AUD-2026-10-31-29 | W3/W7 | Portfolio schema/M15 | Current doc contains schema |
| AUD-2026-10-31-30 | W3 | H4 coverage | New H4 tests fail first |

## 8. Acceptance criteria

- Every Critical and Major finding is fixed with a regression guard, or the
  human records a deferral.
- Every accepted Minor finding is fixed with a test or a documented reason a
  test is impossible.
- Info findings are fixed or explicitly deferred in the audit.
- `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo doc --no-deps` with warnings denied, and
  `git diff --check` pass.
- A post-fix control regression run is recorded; it is not an edge claim.
- Quant Validator records exist for W1, W5, and W6.
- The audit findings move to `Fixed`; the Improvement Verifier alone moves them
  to `Verified`.
- The transparency log
  `docs/ai-transparency-log/2026-10-31/audit-unified-ladder.md` records the
  audit and plan work and links back to this plan.
- No new dependency or `unsafe` block is introduced.
- No open Critical finding is released without explicit human residual-risk
  acceptance.

## 9. Verification

- Fail-before/pass-after tests for each accepted finding where feasible.
- Sparse-CSV integration for the gap fix.
- Ledger invariant test for full closes.
- Multi-context and H4 fixtures.
- 4h CLI tests.
- Event-order integration test.
- Layer 3 fixed-width test.
- Config parser tests for migration/optionality.
- Documentation link check and rendered help goldens.
- Post-fix control backtest and byte-identical rerun.
- Quant Validator reproduction for execution/ledger/model/analytics changes.

Limitations: the full-year SOLUSD source is not committed; the post-fix
regression uses checked-in fixtures unless the documented fetch is run. The
validator is a sequential same-agent pass, not independent.

## 10. Risks and recovery

| Risk | Mitigation | Recovery |
|---|---|---|
| Gap fix changes historical outputs | Deliberate golden update; validator delta | Revert the work item; keep the audit open |
| Partial-exit rejection breaks a caller | Search all ledger callers; typed error | Add a partial-exit contract only by approval |
| H4 path commands change output | Help/README in the same change | Revert to option B (remove 4h from path commands) |
| Event order changes goldens | Regenerate deliberately; compare values | Narrow the contract by approval |
| Layer 3 schema change breaks readers | Fixed-width test; version note | Revert and document variable width |
| Context field cleanup breaks configs | New schema only if approved | Keep fields and document diagnostics |
| Scope creep | One sub-issue per PR | Stop and ask |
| Overfitting | No threshold tuning; no edge claim | Stop the loop |

## 11. Stop conditions

Stop and request human direction if:

- D1–D12 require a choice not covered by this plan.
- A fix changes strategy/model behavior beyond a program defect.
- Partial exits, `context_lost`, dashboard averages, or intent wiring need a
  product decision.
- A test cannot fail before the fix or cannot be made to pass without weakening
  it.
- A new dependency, schema version, or migration is required.
- The audit evidence becomes stale or a finding is contradicted by current code.
- The human has not approved this plan.

## 12. Approval

This plan is **Approved** as of 2026-10-31 (UTC). The human approved the plan and
decisions D1–D12 as recommended.

After approval the Planner creates the fix issues/sub-issues, and execution
proceeds under AGENTS section 13. Commits, pushes, and pull requests remain
separate Git approval gates; they are not authorized by plan approval alone.
