# Remaining MVP Audit Remediation — Execution Plan

- Date: 2026-10-30 (UTC)
- Status: **Merged as `d9ada57`** — closure verification found the AUD-06 small-step regression; the fix is tracked in `docs/execution-plans/2026-10-30/aud-06-step-precision.md`.
- Work item: `audit-remaining-mvp-remediation` (no issue number assigned)
- Baseline: `origin/main` at `63eff7d` (tree identical to `refactor/architecture-hardening` at `3808685`)
- Toolchain: Rust 1.99.1, edition 2024
- Related:
  - `docs/audits/2026-10-30/mvp-pre-release-audit.md`
  - `docs/audits/2026-10-30/aud-2026-10-30-02-validation.md`
  - `docs/execution-plans/2026-10-30/architecture-hardening.md`
  - `docs/audits/2026-10-30/architecture-audit.md`

> The local `main` checkout is at `9f0d0d5` and is behind `origin/main`. The
> architecture-hardening worktree has the same tree as `origin/main`, so this
> re-audit uses that tree without changing either branch.

## 1. Intent

Close the remaining MVP audit findings after the architecture-hardening merge,
without reopening the refactor. The stateless-strategy change fixed the central
AUD-02 state divergence, but its closure is incomplete and the other Critical/
Major findings from the original audit are still open. The intended outcome is a
backtest whose strategy, execution, and portfolio evidence agree; a strategy
report that says whether it is virtual or executed; and a data/portfolio path
whose validity and risk rules hold on every supported command.

## 2. Re-audit result

### 2.1 Finding status on `origin/main` (`63eff7d`)

| ID | Severity | Status | Re-audit result |
|---|---|---|---|
| AUD-2026-10-30-01 | Critical | Verified | Native higher-timeframe sources are rejected by the ladder consumers. |
| AUD-2026-10-30-02 | Critical | Fixed, closure incomplete | The strategy is stateless and the executed walk has no virtual position. The exit-side guard required by the verification is absent, and the strategy artifact does not say whether it is virtual or executed. |
| AUD-2026-10-30-03 | Major | Open | `apply_bracket` still checks the stop before a gap through the target. |
| AUD-2026-10-30-04 | Major | Open | `FixedNotional` still ignores current equity; config validation still compares against initial cash. The audit’s 100× unit example is config-unreachable (`max_position_fraction` is capped at 1.0); the reachable defect is post-drawdown or gap-up over-leverage, e.g. equity 800 with notional 1,000. |
| AUD-2026-10-30-05 | Major | Open | The data engine still accepts negative prices; the ledger still requires positive prices. |
| AUD-2026-10-30-06 | Major | Open | Quantity sizing still floors `raw / step` with binary `f64`. |
| AUD-2026-10-30-07 | Minor | Open | `store remove` still uses create-capable `Store::open`. |
| AUD-2026-10-30-08 | Minor | Open | The tolerant reader still classifies an invalid duplicate as `invalid`. |
| AUD-2026-10-30-09 | Minor | Open | Bucket volume is still an unchecked sum. |
| AUD-2026-10-30-10 | Minor | Open | Help still says `Input cap: 100000` while latest accepts more. |
| AUD-2026-10-30-11 | Info | Guard present; audit Open | The strategy-input boundary test exists; the audit document should move to Verified/Closed. |

### 2.2 AUD-02 closure state

The stateless strategy is a genuine fix for the original state divergence:

- `crates/quantrule-backtest/src/strategy/agreement.rs:78-108` decides from
  `StrategyInput` plus an explicit `Option<Side>`; the strategy carries no
  virtual position.
- `crates/quantrule-backtest/src/runner/mod.rs:150-190` passes the execution
  position after `advance` and fills, and builds the executed walk.
- The validation record reports 7,528 entry decisions but only 43 executed
  entries; the executed walk never carries a short and the strategy bundle's
  end position is null.

Two closure gaps remain:

1. **Exit-side check is missing.** `crates/quantrule-backtest/src/execution/engine.rs:351-383`
   matches `OrderIntent::Exit { side, .. }` but closes `self.position` without
   comparing `side` to the open position. A mismatched intent would close the
   wrong side rather than reject. The audit verification text says “exit side is
   checked”; it is not.
2. **The strategy artifact is ambiguous.** The standalone `report strategy`
   writes a virtual walk; the `backtest` strategy bundle writes the executed
   walk. Both use the same artifact names and schema, and neither `report.json`
   nor `summary.txt` says which mode produced the `open_position` field. The
   README also says `report strategy` reads the executed backtest walk, which is
   false (`crates/quantrule-shell/src/command/report_run.rs:144` uses
   `walk_strategy`; only `backtest.rs:142` uses the run's executed walk).

The validation record explicitly treats the decision-only report as a residual.
The audit's verification sentence “allow_short=false cannot report a short it
did not execute” is therefore not literally met if “report” means the strategy
decision list: the 4,019 short entry decisions remain in that list even though
only 43 entries executed. The report needs either an execution marker and
executed counts, or an explicit human decision that decision lists are allowed
to contain rejected entries and the audit verification text must be amended.

### 2.3 New residuals found in the re-audit

| ID | Severity | Finding |
|---|---|---|
| AUD-02-R1 | Major | Exit intent side is ignored by execution; the required guard and test are missing. |
| AUD-02-R2 | Major | Strategy report lacks `execution_state`/executed counts; virtual and executed bundles are indistinguishable. |
| AUD-02-R3 | Major | `AGENTS.md` §2/§3 says the strategy reads only published axis values and bar identity; the stateless strategy now receives execution position. The normative contract needs an explicit clarification, or the design needs review. |
| DOC-01 | Minor | README says `report strategy` reads the executed backtest walk; it is virtual. `report_run.rs` module doc is stale (“R1 … other three not implemented”). README’s `report model` byte-identity claim omits “default model settings”. |
| PROV-01 | Minor | `lab summary` validates the data version but not `config_hash` across the five bundles; mixed-config bundles can be combined. |

### 2.4 Checks observed on the current tree

- `cargo test --workspace --lib --bins --tests`: all unit and integration suites
  passed.
- `cargo test --workspace --doc`: passed when `TMPDIR` was moved off the 2.8 GiB
  `/tmp` tmpfs. The first attempt failed in the linker with a Bus error/disk
  quota, not in a doctest.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- `cargo doc --workspace --no-deps`: clean.

## 3. Scope

### In scope

- AUD-02 closure: exit-side guard, execution-mode marker and executed counts,
  contract clarification, tests, validation.
- AUD-03: gap-through-target bracket precedence.
- AUD-04: fixed-notional leverage/equity envelope.
- AUD-05: one non-positive-price contract across commands.
- AUD-06: exact quantity-step floor.
- AUD-07, AUD-08, AUD-09, AUD-10: minor fixes.
- AUD-11: audit-record closure only; the boundary guard exists.
- DOC-01 and PROV-01: documentation/provenance consistency.

### Out of scope

- The deferred architecture-hardening Phases 4–5 (args split, bundle split,
  CLI-test split, `lab_summary` split, optional store-validation move). They are
  maintainability work, not MVP correctness.
- New strategies, new indicators, new dependencies, `unsafe`, new I/O surfaces.
- Changing command names, arguments, or output contracts except the strategy
  report’s explicit execution-mode/executed-count fields (decision D1).
- Changing policy/strategy semantics beyond the findings above.
- Performance optimization.

## 4. Assumptions and uncertainty

| ID | Assumption | If wrong |
|---|---|---|
| A1 | The human accepts the stateless strategy as the target design. | Replan; option A reconciliation would replace Phase 1. |
| A2 | The strategy report may keep rejected entry decisions as decisions, provided the artifact labels the mode and publishes executed counts. | Add outcome filtering, a larger report-contract change. |
| A3 | Non-positive prices are not required by any supported instrument in v1. | Choose AUD-05 option B (support them in the portfolio) and replan the portfolio changes. |
| A4 | The policy `max_position_fraction` must be enforced at fill time, not only at config load. | Enforce only `leverage = 1.0` at fill and document the policy-fraction gap. |
| A5 | A tolerance-based quantity floor is acceptable; no decimal dependency is needed. | Introduce a checked decimal/tolerance helper; no new dependency unless approved. |
| A6 | The architecture-hardening merge is the new baseline; no further refactor is in flight. | Stop and re-baseline. |
| A7 | No issue numbers exist yet; the plan uses a slug until issues are created. | Rename the plan and branches to `<issue>-<slug>`. |

## 5. Decisions required

| ID | Decision | Options | Recommendation |
|---|---|---|---|
| D1 | AUD-02 report semantics | **A:** keep decision-only records; add `execution_state` (`virtual`/`executed`) and executed entry/exit counts. **B:** filter the strategy report to executed decisions. **C:** keep the current artifacts and amend the audit verification text. | **A**. It is truthful, preserves the decision report, and shows why decision entries differ from fills. |
| D2 | Exit-side guard | Add `RejectReason::WrongExitSide` and reject a mismatched `OrderIntent::Exit`, or leave the engine permissive because the stateless runner cannot produce a mismatch. | Add the guard. The audit requires it and the engine is a public internal API. |
| D3 | Strategy boundary contract | Update `AGENTS.md`/ADR to allow the runner-supplied position parameter, or revert to a stateful strategy. | Update the contract: position is the strategy’s own state, supplied explicitly; it is not market data, cash, P&L, or a statistic. |
| D4 | AUD-03 gap policy | **A:** resolve a gap at the open before the intrabar stop-first tie rule. **B:** keep stop-first even for gaps. **C:** reject ambiguous bars. | **A**. The open proves the target was reached first. |
| D5 | AUD-04 enforcement | **A:** pass current-equity × leverage × policy fraction as an entry envelope and cap the fill quantity to it; reject only when the envelope is non-positive. **B:** enforce only current-equity × leverage at sizing. **C:** remove the leverage claim from fixed notional. | **A** (cap). A strict rejection makes the shipped example config untradeable because its fixed notional equals initial cash and slippage alone exceeds the limit. |
| D6 | AUD-05 price contract | **A:** reject non-positive prices in the data engine. **B:** support them in the portfolio. **C:** document the split. | **A** for the v1 positive-price universe. |
| D7 | AUD-06 floor | **A:** tolerance-aware floor. **B:** document the one-step rule. **C:** reject non-exact steps. | **A**. |
| D8 | PROV-01 | Add a `config_hash` equality check across the five bundles; or leave provenance inspection to the human. | Add the check; it is cheap and protects experiment reproducibility. |

## 6. Acceptance criteria

### 6.1 AUD-02 closure

- A mismatched `OrderIntent::Exit` is rejected with `WrongExitSide`, schedules no
  order, and leaves execution and ledger unchanged.
- The backtest strategy `report.json` and `summary.txt` carry
  `execution_state: "executed"` and executed entry/exit counts;
  the standalone `report strategy` carries `execution_state: "virtual"` and no
  executed counts.
- The full-year `allow_short=false` run has no short executed position, no
  strategy exit for a short it never held, and strategy `open_position` equals
  the ledger/execution end position.
- The audit verification text matches the implementation, or the human records
  that decision-only rejection lists are accepted.
- `AGENTS.md`/ADR and the strategy trait docs agree on the position parameter.
- Regression tests fail before the change and pass after it.
- Quant Researcher, Quant Developer, and Quant Validator records are present;
  the validator reproduces the run from the recorded data/config.
- AUD-02 moves to `Fixed` only when the above is true; the Improvement Verifier
  then moves it to `Verified`.

### 6.2 AUD-03

- A bar that opens beyond the target and later dips below the stop fills
  `TakeProfit` at the open.
- A bar whose open is between stop and target and touches both still fills the
  stop under `tie_break: stop_first`.
- The change is covered by a unit test and a deliberate golden update; no other
  fill behavior changes.

### 6.3 AUD-04

- A fixed-notional entry whose fill notional exceeds the current equity ×
  leverage × policy fraction is capped to the envelope, never over-leveraged.
- The check holds after a drawdown and after a gap-up fill; a non-positive
  envelope rejects with `risk_limit`.
- The shipped example config remains tradeable.
- The policy fraction is either passed into the engine through an explicit
  risk-envelope parameter or the chosen option B is documented.
- Existing example configs still validate; a config-schema change, if any, is
  documented and tested.

### 6.4 AUD-05

- A zero-price or negative-price file is accepted or rejected identically by
  `candles`, `candles-tolerant`, `timeframes`, `market-state`, `market-report`,
  `store ingest`, `research`, and `backtest`.
- The data-engine contract test matches the chosen behavior; the ledger’s
  positive-price checks remain consistent.
- No supported positive-price fixture changes.

### 6.5 AUD-06

- Exact step boundaries produce the exact quantity: `3.0 / 5.0` with step `0.1`
  yields `0.6`, and `0.3 / 3.0` with step `0.1` yields `0.1`.
- A quantity below one step is still rejected; no zero quantity is produced.
- Deterministic boundary fixtures and property-style cases cover values just
  below and above a step.

### 6.6 Minors and records

- AUD-07: `store remove` on a missing database exits with `DatasetNotFound` and
  creates no file or directory.
- AUD-08: an invalid duplicate/out-of-order row is counted under its ordering
  reason, not `invalid`.
- AUD-09: an overflowing volume sum reports the bucket/aggregation error, not an
  innocent later row.
- AUD-10: help, README, and the observed `--window latest` behavior agree.
- AUD-11: the audit record is updated to Verified/Closed; no code change unless
  the boundary test needs strengthening.
- DOC-01: the stale README/module-doc statements are corrected.
- PROV-01: `lab summary` fails when the five provenance files carry different
  `config_hash` values for the same data version.

## 7. Work breakdown

### Phase 0 — Records, baseline, and issues

| Step | Work | Paths | Role |
|---|---|---|---|
| 0.1 | Create the tracking issue and one sub-issue per phase; no invented numbers | GitHub | Orchestrator |
| 0.2 | Write this plan of record and the AI transparency log; update the audit with the re-audit status | `docs/execution-plans/2026-10-30/`, `docs/ai-transparency-log/2026-10-30/`, `docs/audits/2026-10-30/` | Planner / Orchestrator |
| 0.3 | Create a dedicated worktree from `origin/main`; one branch per phase | repository | Orchestrator |
| 0.4 | Record the baseline checks and the `/tmp` doctest limitation in the plan/AI log | repository | Test / Release |

**Acceptance:** plan approved, issues created, worktree ready, baseline recorded.

### Phase 1 — AUD-02 closure (Critical)

| Step | Work | Paths | Role |
|---|---|---|---|
| 1.1 | Add `RejectReason::WrongExitSide`; reject a mismatched exit intent; update exhaustive matches and docs | `crates/quantrule-backtest/src/execution/order.rs`, `engine.rs`, `crates/quantrule-shell/src/command/report_run/bundles.rs` | Quant Developer |
| 1.2 | Add `execution_state` and executed counts to the strategy report; keep decision records; distinguish standalone virtual vs backtest executed | `crates/quantrule-shell/src/command/report_run.rs`, `report_run/bundles.rs`, `command/backtest.rs` | Quant Developer |
| 1.3 | Update `AGENTS.md`/ADR/strategy docs for the position parameter | `AGENTS.md`, `docs/adr/2026-10-30/`, `docs/reports/2026-10-28/per-engine-reports.md` | Architect |
| 1.4 | Add fail-before/pass-after tests: exit side, `allow_short=false`, bracket exit, zero-volume exit, final open position, byte determinism | `crates/quantrule-backtest/src/runner/tests.rs`, `crates/quantrule-backtest/tests/`, shell tests | Test / Quant Developer |
| 1.5 | Reproduce the full-year run; record strategy vs execution vs portfolio counts | repository | Quant Validator |
| 1.6 | Update AUD-02 verification text/status and the validation record | `docs/audits/2026-10-30/` | Improvement Verifier |

**Acceptance:** §6.1. **Rollback:** revert Phase 1; AUD-02 returns to Fixed with the residual documented.

### Phase 2 — AUD-04 leverage envelope (Major, risk first after AUD-02)

| Step | Work | Paths | Role |
|---|---|---|---|
| 2.1 | Define the risk-envelope input (current equity, leverage, policy fraction) and the rejection reason | `portfolio/`, `runner/`, shell config | Architect / Quant Developer |
| 2.2 | Pass the current-equity envelope to execution and cap the fill quantity; reject only when the envelope is non-positive | `execution/`, `runner/mod.rs` | Quant Developer |
| 2.3 | Tests: post-drawdown, gap-up, exact boundary, fixed notional vs equity fraction | `crates/quantrule-backtest/tests/`, shell config tests | Test |
| 2.4 | Quant Validator reproduction; document the changed risk behavior | docs | Quant Validator |

**Acceptance:** §6.3. **Rollback:** revert Phase 2; the old config behavior returns.

### Phase 3 — AUD-03 gap-through-target (Major)

| Step | Work | Paths | Role |
|---|---|---|---|
| 3.1 | Resolve the open-gap case before the intrabar stop/target tie rule | `crates/quantrule-backtest/src/execution/engine.rs` | Quant Developer |
| 3.2 | Tests: gap through target, gap through stop, both touched inside the bar, exact open at level | `execution/engine_tests.rs` | Test |
| 3.3 | Deliberate golden update if the example changes; record the before/after | fixtures | Quant Validator |

**Acceptance:** §6.2. **Rollback:** revert Phase 3.

### Phase 4 — AUD-05 non-positive prices (Major)

| Step | Work | Paths | Role |
|---|---|---|---|
| 4.1 | Add the positive-price invariant to `Candle::new`/`parse`; update the negative-price test and docs | `crates/quantrule-data/src/candle.rs` | Quant Developer |
| 4.2 | Prove every command rejects the same file; update fixtures and error text | data/shell tests | Test |
| 4.3 | Check the store/research/chart paths propagate the data error before writing | shell tests | Test |

**Acceptance:** §6.4. **Rollback:** revert Phase 4; the old split returns.

### Phase 5 — AUD-06 quantity floor (Major)

| Step | Work | Paths | Role |
|---|---|---|---|
| 5.1 | Add a tolerance-aware step calculation or a checked exact-step helper | `crates/quantrule-backtest/src/portfolio/sizing.rs` | Quant Developer |
| 5.2 | Tests: exact boundaries, just-below, just-above, below one step, large/small steps | `portfolio/tests.rs` | Test |
| 5.3 | Quant Validator confirms no under-size and no spurious rejection | repository | Quant Validator |

**Acceptance:** §6.5. **Rollback:** revert Phase 5.

### Phase 6 — Minors, AUD-11, and provenance

| Step | Work | Paths | Role |
|---|---|---|---|
| 6.1 | AUD-07: open existing DB read-write only; missing DB creates nothing | `command/store.rs`, `store/` | Implementation |
| 6.2 | AUD-08: classify ordering before candle validity | `crates/quantrule-shell/src/csv.rs` | Implementation |
| 6.3 | AUD-09: checked bucket volume sum; bucket-level error | `crates/quantrule-data/src/bucket.rs`, `timeframe_set.rs` | Implementation |
| 6.4 | AUD-10: state the latest-window exception in help/README, including `market-state`, `axis`, `events`, `legs`, `model-series` | `command/help.rs`, README | Docs |
| 6.5 | AUD-11: mark the boundary guard Verified/Closed | audit doc | Improvement Verifier |
| 6.6 | DOC-01: fix the `report strategy` executed-walk claim, stale module doc, and byte-identity qualifier | README, `report_run.rs`, docs | Docs |
| 6.7 | PROV-01: compare `config_hash` across the five bundles; add a mixed-config test | `lab_summary.rs`, tests | Test / Implementation |

**Acceptance:** §6.6. **Rollback:** revert Phase 6; no engine behavior change.

### Phase 7 — Final verification and closure

| Step | Work | Paths | Role |
|---|---|---|---|
| 7.1 | Run the full verification matrix on the merged tree | repository | Test / Release |
| 7.2 | Run the example backtest and a targeted `allow_short=false` run; compare bundles | repository | Quant Validator |
| 7.3 | Update the audit statuses, validation records, and AI log | docs | Improvement Verifier |
| 7.4 | Verify no open Critical/Major finding remains, or record the human’s residual-risk acceptance | audit | Human |

**Acceptance:** all criteria; no open Critical/Major without explicit acceptance.

## 8. Verification

Run after every phase, from the phase worktree:

| Check | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all -- --check` | No diff |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | No warnings |
| Tests | `cargo test --workspace --lib --bins --tests` | All pass |
| Docs tests | `TMPDIR=<disk-backed dir> cargo test --workspace --doc` | All pass |
| Docs build | `cargo doc --workspace --no-deps` | No warnings |
| Boundary tests | model/backtest/shell architecture tests | Pass |
| Example run | `backtest --config config/examples/solusd-2025/run.json --out <tmp>` | Five bundles; strategy `execution_state` and counts; no virtual short |
| Risk case | `allow_short=false` fixture | No short position; executed/rejected counts visible |
| Determinism | Two identical runs | Byte-identical bundles |
| Finding-specific | AUD-03/04/05/06 tests | Fail before, pass after |

The `/tmp` doctest failure observed in this re-audit was environmental: `/tmp` is
a 2.8 GiB tmpfs and was full. Use a disk-backed `TMPDIR` for doctests and record
the distinction rather than treating it as a code failure.

## 9. Risks and recovery

| Risk | Likelihood | Impact | Mitigation | Recovery |
|---|---|---|---|---|
| AUD-02 report contract change | Medium | Medium | Add only `execution_state`/counts; keep existing fields | Revert Phase 1 |
| AUD-04 config/API change | Medium | High | Add an explicit risk envelope; validate example configs; Quant Validator | Revert Phase 3 |
| AUD-03 changes P&L | Medium | High | Deliberate golden update; explain gap rule | Revert Phase 3 |
| AUD-05 rejects previously accepted files | Medium | Medium | Decide the universe; update error text and tests | Revert Phase 4 or choose option B |
| AUD-06 tolerance changes sizing | Low | Medium | Exact-boundary tests; no silent cap | Revert Phase 5 |
| AUD-09 API change in bucket push | Low | Medium | Propagate a typed error; no panic | Revert Phase 6 |
| Minors regress command text | Low | Low | CLI/golden tests | Revert Phase 6 |
| Parallel sessions conflict | Medium | Medium | One worktree, one branch/PR per phase; sequential merges | Rebase/replan, never force-push |

## 10. Stop conditions

1. The human does not approve D1–D8 or the AUD-02 report-contract change.
2. A golden fixture changes outside the intended finding.
3. A phase needs a new dependency, crate, `unsafe`, or I/O surface.
4. A command name, argument, or unrelated output contract must change.
5. The policy/strategy/config schema change from AUD-04 or AUD-05 is not approved.
6. The Quant Validator cannot reproduce a phase or finds look-ahead, cost, or state leakage.
7. The current architecture-hardening merge is superseded by a newer baseline.
8. A test/clippy failure cannot be fixed within the phase scope.

## 11. Traceability

| Finding | Phase | Acceptance | Verification |
|---|---|---|---|
| AUD-02 / R1 / R2 / R3 | 1 | Exit guard; execution-mode marker; contract clarification; no virtual short | Regression tests; full-year run; Quant Validator |
| AUD-03 | 3 | Gap-through-target fills target at open | Unit test; golden diff |
| AUD-04 | 2 | Over-envelope fill capped; current-equity check | Fill/risk-envelope tests; Quant Validator |
| AUD-05 | 4 | One price-validity contract | Cross-command test |
| AUD-06 | 5 | Exact step boundaries | Boundary fixtures |
| AUD-07 | 6 | Missing DB creates nothing | Shell test |
| AUD-08 | 6 | Duplicate counted as duplicate | CSV test |
| AUD-09 | 6 | Bucket-level overflow error | Data test |
| AUD-10 | 6 | Help/README match behavior | Help golden |
| AUD-11 | 6.5 | Boundary guard recorded | Audit update |
| DOC-01 | 6.6 | Docs corrected | Review |
| PROV-01 | 6.7 | Mixed config hashes fail | Lab-summary test |

## 12. Roles and approvals

| Role | Responsibility |
|---|---|
| Orchestrator | Sequence phases, collect evidence, stop at gates |
| Architect | AUD-02 boundary/ADR, AUD-04 risk-envelope shape |
| Implementation | Code changes within each approved phase |
| Test Engineer | Regression, causality, determinism, boundary, CLI tests |
| Code Reviewer | Full diff, scope, conventions |
| Quant Researcher | Confirm the strategy semantics for AUD-02 |
| Quant Developer | Strategy/execution/portfolio changes |
| Quant Validator | Reproduce AUD-02/03/04/05/06; check look-ahead, costs, state |
| Improvement Verifier | Verify AUD-02/03/04/05/06/11 closure |
| Human | Approve the plan, decisions D1–D8, commits, push, PR, merge |

Per AGENTS §10, the quant passes may be sequential by one agent; disclose that
they are not independent-agent or independent-model judgments.

## 13. Sequence and estimate

| Phase | Branch (proposed) | Depends on | Estimate |
|---|---|---|---:|
| 0 Records | `docs/audit-remaining-remediation` | — | 0.5 day |
| 1 AUD-02 closure | `fix/aud-02-strategy-execution-closure` | 0 | 1–2 days |
| 2 AUD-04 | `fix/aud-04-fixed-notional-envelope` | 1 | 1–2 days |
| 3 AUD-03 | `fix/aud-03-gap-through-target` | 1 | 1 day |
| 4 AUD-05 | `fix/aud-05-non-positive-prices` | 1 | 1 day |
| 5 AUD-06 | `fix/aud-06-quantity-floor` | 1 | 0.5–1 day |
| 6 Minors/provenance | `fix/aud-minor-remediation` | 1–5 | 1 day |
| 7 Closure | no branch | all | 0.5 day |

Total: **6–9 working days**, sequential. Phases 2–5 touch different engines but
are kept sequential by the default one-work-item-in-flight rule.

## 14. Artifacts

- This plan, copied to
  `docs/execution-plans/2026-10-30/<issue>-audit-remaining-mvp-remediation.md`
  after issue creation and approval.
- AI transparency log:
  `docs/ai-transparency-log/2026-10-30/<issue>-audit-remaining-mvp-remediation.md`.
- Updated audit findings and validation records under `docs/audits/2026-10-30/`.
- Phase PRs with the verification table and observed results.

## 15. Implementation evidence (2026-10-30)

The plan is implemented in the `fix/audit-remaining-mvp-remediation` worktree
from `origin/main` at `63eff7d`. The implementation is committed on the task branch at `f38e298`;
push, PR, and merge remain gated by AGENTS §9.

| Phase | Status | Evidence |
|---|---|---|
| 0 Records | Complete | This plan, ADR `docs/adr/2026-10-30/stateless-strategy-position-parameter.md`, audit and AI-log updates |
| 1 AUD-02 closure | Complete | `WrongExitSide`; `execution_state` plus `executed_entries`/`executed_exits`; `executed_counts`; full-year strategy and execution counts agree at 65/65; exit-side and stateless-position tests |
| 2 AUD-04 | Complete (cap) | Current-equity × leverage × policy fraction envelope; fill quantity capped; `risk_limit` only for a non-positive envelope; engine and runner tests |
| 3 AUD-03 | Complete | Open-gap precedence before the intrabar stop-first rule; long and short gap tests |
| 4 AUD-05 | Complete | Open/close must be positive; data, CLI, and ladder tests |
| 5 AUD-06 | Complete | Relative tolerance plus step-aware quantity normalization; exact-boundary and small-step tests |
| 6 Minors | Complete | Existing-DB remove; tolerant ordering precedence; bucket volume error; latest-window docs; `config_hash` check; help/README updates |
| 7 Closure | Pending PR/Verifier | Audit findings marked Fixed in this worktree; Improvement Verifier has not yet marked Verified |

### Verification observed

- `cargo test --workspace --lib --bins --tests`: 40 suites pass, 0 failed
  (the 100k store benchmark remains ignored by design).
- `cargo test --workspace --doc`: 77 doctests pass across four doctest binaries.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo fmt --all -- --check`: clean.
- Full-year `backtest` with the SOLUSD 2025 data and example config: five bundles
  written in 4m24s. Strategy report: `execution_state: executed`,
  `executed_entries: 65`, `executed_exits: 65`; execution report: 65 entries,
  65 exits, 130 fills; portfolio: 65 trades, final equity 8302.124215,
  max drawdown 0.204439, halted true.
- `lab summary` on that run: five bundles and provenance present, data versions
  match, config hashes match, strategy executed counts match execution.
- Full-year reproducibility: two `allow_short=false` runs from the same config
  and data into different output roots produced 29 files each; `diff -rq`
  reported no differences.

### Deviation from the proposed decision

D5 proposed rejection of an over-envelope entry. The implemented fix caps the
fill quantity to the current-equity envelope instead. A strict rejection made the
shipped example config (fixed notional equal to initial cash, with any positive
gap or slippage) untradeable. The audit’s option A explicitly allowed “cap or
reject”; capping preserves the entry, keeps the strategy state consistent, and
still prevents over-leverage. This is recorded for human acceptance.

### Remaining gates

- The original remediation PR is merged; the AUD-06 regression fix has its own plan and branch.
- Quant Validator reproduction of the full-year run is recorded as a sequential
  pass by the implementing agent, not an independent-agent review.
- The Improvement Verifier must move the fixed findings from `Fixed` to
  `Verified` after merge.
