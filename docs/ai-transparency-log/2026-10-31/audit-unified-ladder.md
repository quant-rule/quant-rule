# AI Transparency Log — Unified Ladder Audit

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: `audit-unified-ladder`
- Plan: [`docs/execution-plans/2026-10-31/audit-unified-ladder.md`](../../execution-plans/2026-10-31/audit-unified-ladder.md)
- Audit: [`docs/audits/2026-10-31/unified-ladder-audit.md`](../../audits/2026-10-31/unified-ladder-audit.md)

## Intent and boundary

The human asked to run the full audit workflow against the merged unified-ladder
system. The work was a read-only audit followed by a Planner remediation plan
marked `Proposed`. The baseline was `main` at `7c865de`, the merge of PR #246.

The audit did not modify code or tests. It created no fix issues before
approval. On 2026-10-31 the human approved the plan and decisions D1–D12 as
recommended; the Planner then created remediation issues #247–#255. The
audit/plan work was committed as `8643bf0` and opened as PR #256. W1 was
implemented and committed as `7d29dcc` under #247, then opened as PR #257.
Neither PR was merged. W2–W8 have not started.

The audit covered five areas: strategy/config/context/runner; execution,
portfolio, and the TradeIntent boundary; data/model/timeframe/causality; shell,
reports, help, and docs; and global test/doc gates. Store migrations, live
fetching, release tooling, and the full CLI were not exhaustively exercised and
are listed as unexamined.

## Implemented

- Read the current architecture, the unified-ladder plan, the audit format, and
  the prior audit/remediation artifacts before inspecting code.
- Ran four bounded read-only exploration passes (explore subagents) over the
  five audit areas. They returned candidate findings with file:line evidence;
  the main agent re-checked each candidate before accepting it.
- Ran the global gates at the baseline: `fmt`, `clippy --all-targets`, the full
  workspace test suite, rustdoc with warnings denied, and `git diff --check`.
- Re-ran focused CLI reproductions for the v2 migration error, the 4h path
  commands, the `model.json` `timeframes` requirement, event ordering, the
  Layer 3 width, and the stale-doc greps.
- Built a temporary probe outside the repository (`/tmp/opencode/audit-probe`)
  against the local crates to reproduce the gap fill and exit-quantity
  findings. The probe is not part of the repository.
- Consolidated 30 findings (1 Critical, 7 Major, 16 Minor, 6 Info) into the
  dated audit document, each with evidence, impact, options, recommendation,
  and verification.
- Wrote the proposed remediation plan with eight work items (W1–W8), a
  finding-to-work-item traceability table, twelve open decisions (D1–D12),
  acceptance criteria, verification, risks, and stop conditions.
- After human approval, marked the plan `Approved`, moved findings to `Planned`,
  and created issues #247–#255: seven work items, one docs work item, and one
  parent. W1 is `In Progress` with a `2026-10-31` start date.
- W1 was then implemented under #247 on
  `fix/247-noncontiguous-pending-fill`; its own log is
  `docs/ai-transparency-log/2026-10-31/247-noncontiguous-pending-fill.md`. This
  entry records the audit and plan work, not an independent verification of that
  implementation.
- Committed the audit, plan, and this log as `8643bf0`; opened PR #256. W1 was
  committed as `7d29dcc` and opened as PR #257.
- Recorded this log. No repository artifact other than the audit, the plan, and
  this log was created by the audit work item itself.

## Verification and evidence

Commands actually run in this worktree:

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Passed, no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed, no warnings |
| `cargo test --workspace` | Passed; 45 test-result-ok suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |
| `policy check` on a copy of the SOLUSD example with a v2 `strategy.json` | `$.long_threshold: expected a known field`, not the documented migration message |
| `model-series ... --timeframe 4h --bars 5` | Exit 0, 0 stdout bytes, 0 stderr bytes |
| `policy check` after removing `model.json.timeframes` | `$.timeframes: expected a required field` |
| `help all` / `help tutorial` | `help all` labels the tutorial “full-pipeline tutorial map” |
| `research export` on `single.csv` | Layer 3 has 155 columns; no higher-timeframe blocks |
| `events` on `state_m1.csv` | Bar 28 emits the minor-pivot test before the EMA20 break; bar 37 emits a Structure signal before Momentum |
| Probe `EX-01` | `gapped fill count=1 bar=Some(1) time=Some(1800)`: a pending entry filled across a missing 900s bar |
| Probe `PF-01` | A 100-unit entry plus a 50-unit exit was accepted; `equity=5460.03` while `initial_cash + net_pnl=10960.03`; the trade claimed quantity 100 |
| Stale-doc greps | Found the H4/four-series claim, “context pending”, “Phase 1 pending”, and “PR #246 open” |
| Relative link check over the audit, plan, and log | 4 links checked, 0 broken |
| Issue creation | #247–#254 created under parent #255; #247 moved to `Done` when PR #257 opened |
| Board read-back | #247 `Done`; #255 `In Progress`; #248–#254 `Todo`; #247 start date `2026-10-31` |
| Pull requests | Audit/plan PR #256; W1 PR #257; neither merged |

The probe output is the runtime evidence for the Critical gap fill and the Major
ledger exit-quantity defect. The remaining findings are code inspection plus the
listed commands. The full-year SOLUSD source was not present, so the experiment
and validator records were inspected, not reproduced.

## Role disclosure

The four exploration passes were separate read-only subagent sessions; the
consolidation, severity assignment, and focused reproductions were performed by
the main agent. This is a sequential role pass, not an independent-agent or
independent-model review. The Improvement Verifier has not verified any finding:
AUD-2026-10-31-01 is `Fixed` pending its PR, the rest are `Planned`, and W2–W8
have not started.

## Learning and rejected suggestions

- Rejected filing every candidate finding the subagents returned. Several were
  re-checked against the code and dropped or merged when the evidence did not
  hold or the behavior was already documented.
- Rejected treating the passing global gates as evidence that the findings are
  false. The suites pass because the defective paths are not covered; the audit
  records the uncovered paths instead of weakening the gate result.
- Rejected calling the 4h path-command behavior a documentation-only issue. The
  command exits 0 with no output, which is a silent wrong result, so it is Major.
- Rejected treating the ledger exit-quantity defect as Critical. The shipped
  runner currently closes the full position, so the corruption is latent for the
  product but reachable through the public library API; the finding is Major.
- Rejected making the contract choices inside the plan. Partial exits, the
  missing-context exit, the dashboard forming-leg average, intent expiry, and v3
  execution are product decisions listed as D4, D5, D9, D10, and D2, not
  decisions the Planner may settle.
- Stopped at the approval gate before creating fix issues, then created the
  issue tree only after the human approved the plan. Issue creation is not
  implementation; no code or test changed in this work item.
