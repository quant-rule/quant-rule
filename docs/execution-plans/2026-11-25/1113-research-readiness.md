# HA-015 — Research Readiness Gate

- Date: 2026-11-25 (UTC)
- Status: In Progress
- Issue: [#1113](https://github.com/quant-rule/quant-rule/issues/1113)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-25/1113-research-readiness.md`](../../ai-transparency-log/2026-11-25/1113-research-readiness.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md), [End-to-End Verification](../../workflows/end-to-end-verification.md)
- Parent plan: [`docs/execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md`](../2026-11-22/1082-v1.6.0-ra-ha.md)
- Target: `v1.6.0`

## 1. Intent

Make the platform's research readiness explicit and checkable before the GA
release. The gate records the eight RR-01…RR-08 claims, their evidence tier,
their boundary, and the one human-accepted risk for the single-host lab. It also
states that QuantRule is a research system, not a trading system.

This leaf does not run a new backtest, change a model or strategy, or claim an
edge. It turns the existing contracts and the 2026-11-25 HA-014 lab evidence
into one governed record.

## 2. Current state

- The RR-01…RR-08 claims exist only in the v1.6.0 plan's H.3 table; no committed
  matrix, checker, or production-readiness record exists.
- Data partitions, the run contract, multiple-testing evidence, failover and
  restore evidence, pack/signal retention, the remote client path, and queue
  drainage already exist in separate documents and tests.
- The HA-014 lab is `Simulated HA` on one physical host; `Real HA` remains
  HA-22/HA-23.
- The v1.6.0 plan's V22 is `Pending`.

## 3. Scope

Included:

- `docs/config/2026-11-25/research-readiness.json`: `quantrule.research-readiness/v1`,
  eight rows, the research-only system block, and the accepted lab risk.
- `docs/config/2026-11-25/research-readiness.md`: the human-readable gate.
- `research-readiness` governance checker and its unit tests.
- Config index, v1.6.0 plan V22 and current-state refresh, and changelog entry.
- The #1113 target update to `v1.6.0` and the board `In Progress` transition.

Excluded:

- A new lab, soak, real-host, or remote-client run.
- A model, strategy, execution, portfolio, or signal change.
- An edge claim or a change to a green criterion.
- A `Real HA` claim from the single-host lab.

## 4. Assumptions and uncertainty

- The 2026-11-25 HA-014 lab evidence is the current maximum; the single-host
  boundary is already approved in the v1.6.0 plan.
- The evidence paths are locations, not proof that a future operator has run the
  same command; the matrix records the tier honestly.
- The governance checker is a static gate; it does not execute the lab.
- The 72-hour soak remains V20/QA-006 and is not claimed by RR-08.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Evidence record | JSON matrix plus a human-readable MD, mirroring the HA matrix | Approved |
| Status vocabulary | `Current`, `Measured`, `Simulated HA`, `Accepted risk`, `Unverified`, `Retired` | Approved |
| Research-only boundary | `system.kind=research-only`, `trading=false`; the checker refuses otherwise | Approved |
| Simulated lab | Four `Simulated HA` rows; one human-accepted `AR-RESEARCH-SIMULATED-LAB` risk | Approved |
| Real HA | Not claimed; HA-22/HA-23 remain the real-host gate | Approved |
| Scope | Docs and governance only; no model or strategy behavior change | Approved |

## 6. Acceptance criteria

- The matrix has exactly RR-01…RR-08, each with a claim, owner, issue, test,
  evidence, status, tier, boundary, date, and notes.
- `research-readiness` passes on the committed matrix and fails a missing row,
  unknown status, trading system, duplicate row, or missing accepted risk.
- The record states `research-only` and `trading: false`.
- The four simulated rows name the single-host lab boundary and the accepted
  risk; no row claims `Real HA`.
- The v1.6.0 plan's V22 is `Pass` with the matrix and checker evidence.
- `qr-governance check --all` and the governance tests pass.

## 7. Work breakdown

| Step | Work | Role | Paths |
|---:|---|---|---|
| 1 | Write the JSON matrix and MD gate | Implementation | `docs/config/2026-11-25/` |
| 2 | Add the checker and tests | Test Engineer | `crates/quantrule-governance/src/checks/` |
| 3 | Register the checker and run the governance suite | Test Engineer | `checks/mod.rs` |
| 4 | Update the config index, plan V22, and changelog | Implementation | `docs/config/README.md`, parent plan, `CHANGELOG.md` |
| 5 | Record the AI log and move the board item | Orchestrator | `docs/ai-transparency-log/2026-11-25/` |

## 8. Verification

```bash
cargo test -p quantrule-governance
cargo run --quiet -p quantrule-governance -- check --checker research-readiness
cargo run --quiet -p quantrule-governance -- check --all
tools/gate.sh --changed main
```

The full workspace gate is not required for a docs/governance change; the
scoped gate and the governance test suite are the applicable evidence. The lab
is not re-run for this leaf.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Matrix | RR-01…RR-08 exist once each | `research-readiness` checker | Pass | Checker passes on the matrix | Pass | agent | 2026-11-25 |  |
| V02 | Research-only | `kind=research-only`, `trading=false` | checker test | Pass | Unit test refuses `trading=true` | Pass | agent | 2026-11-25 |  |
| V03 | Simulated boundary | Four `Simulated HA` rows and the accepted risk | checker test | Pass | Missing-risk test fails; committed matrix passes | Pass | agent | 2026-11-25 |  |
| V04 | Checker tests | New checker and registry count | `cargo test -p quantrule-governance` | Pass | 139 unit tests pass (170 total with integration tests) | Pass | agent | 2026-11-25 |  |
| V05 | Full checkers | `check --all` accepts the new contract | `check --all` | Pass | Pass | Pass | agent | 2026-11-25 |  |
| V06 | Parent plan | V22 is Pass with the new evidence | parent plan + execution-plans checker | Pass | Pass | Pass | agent | 2026-11-25 |  |
| V07 | Index and changelog | Contract is discoverable | `check --all` | Pass | Pass | Pass | agent | 2026-11-25 |  |
| V08 | Board | #1113 is `In Progress` with the `v1.6.0` target | issue and board | Done | Issue target updated to v1.6.0; project API returned 502, board transition pending | Pending | agent | 2026-11-25 |  |
| V09 | Delivery | PR open, CI green, merged, worktree cleaned | PR and `git worktree list` | Done | Not yet | Pending | agent | 2026-11-25 |  |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| The static matrix outlives its evidence | Each row states its tier and boundary; `Real HA` is never inferred from the lab |
| A future operator treats `Simulated HA` as `Real HA` | The accepted risk and the release note state the boundary |
| The checker becomes a docs-only ritual | It runs from `check --all`; it fails a missing row, status, or risk |
| Scope creep into a soak or real-host run | V20/QA-006 and HA-22/HA-23 remain separate gates |

## 10. Stop conditions

Stop if the matrix cannot state the research-only boundary, if a row would claim
`Real HA` from the single-host lab, if the checker requires a new dependency, or
if the work changes model or strategy behavior.
