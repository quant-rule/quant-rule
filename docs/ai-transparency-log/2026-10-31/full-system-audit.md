# AI Transparency Log — Full-System Audit

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: `full-system-audit`
- Audit: [`docs/audits/2026-10-31/full-system-audit.md`](../../audits/2026-10-31/full-system-audit.md)
- Plan: [`docs/execution-plans/2026-10-31/full-system-audit.md`](../../execution-plans/2026-10-31/full-system-audit.md)
- Branch: `docs/full-audit-2026-10-31`
- Baseline: last audited head `c8b1dc1`; head `e34693f`

## Intent and boundary

The human asked to run the full audit workflow. The work was a read-only audit
of the current system at `e34693f`, with the delta since the last audited head
(`c8b1dc1`) reviewed in full, followed by a sequential Planner pass that wrote a
remediation plan. The human approved the plan and D1–D17 via "continue" on
2026-10-31; the Planner then created the issue tree (#308–#318). The audit pass
itself did not modify code or tests, and no commit, push, or pull request was
made.

The work started in the project root on `main`. A dedicated worktree
`quant-rule--full-audit-2026-10-31` and branch `docs/full-audit-2026-10-31`
were created from `e34693f` before any artifact was written; the session was
moved into that worktree.

## Implemented

- Inspected the Git state, the worktrees, the last audited head, the merged PR
  sequence, the audit format, the prior audit and plan, and the current
  architecture baseline.
- Ran the global gates at `e34693f`: fmt, clippy with warnings denied, the full
  workspace test suite, rustdoc with warnings denied, and `git diff --check`.
- Reviewed the `c8b1dc1..e34693f` delta: 70 files, +3,873/−273, covering the
  ten prior findings, the architecture baseline, the BTCUSD example move, and
  documentation cleanup.
- Ran five read-only exploration passes (separate subagent sessions) over the
  data/store, model, backtest, shell, and analytics/docs areas. They returned
  candidate findings with `file:line` evidence and positive observations.
- Re-read every accepted candidate against the code. Rejected or corrected
  candidates that did not survive:
  - The entry-bar stop candidate claimed a target could be pre-empted; its
    example candle had `open < low`, which `Candle::new` rejects. Because a
    valid candle's low is at or below its open, the stop is always touched when
    the pre-entry open is beyond it. The accepted finding is the stop reference
    price (AUD-48), Minor, not a Critical/Major wrong-exit finding.
  - The `lab_evidence` unreadable branch is real code, but the W1 record's
    coverage claim is only about the analytics status mapping; the file-reader
    branch is a test gap, not a false behavior claim.
  - The `report_versions` constants are low-exposure on `backtest --diagnose`
    because the same binary writes the bundles; the finding is Info, not Major.
- Built a throwaway probe outside the repository
  (`/tmp/opencode/execution-probe`) against the local crates. It reproduced the
  entry-bar stop reference: entry 100.02, stop 100.01, bar open 100.0, low
  99.99 → stop reference 100.0, price 99.98; the stop price is 100.01, so the
  correct price is 99.99.
- Wrote 29 findings (7 Major, 18 Minor, 4 Info) with evidence, impact, options,
  recommendation, and a closure test.
- Ran the sequential Planner pass: re-read every finding at `e34693f`, grouped
  the findings into eleven work items, wrote decisions D1–D17, acceptance
  criteria, traceability, verification, risks, and stop conditions.
- After approval, created the issue tree (#308–#318), added the items to the
  board, and set W1 (#308) to `In Progress` with start date 2026-10-31. No code
  or test changed; the findings are `Planned`.

## Verification and evidence

Commands actually run, with observed results:

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 1,518 passed, 0 failed, 7 ignored; 47 suites |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |
| `cargo run` in `/tmp/opencode/execution-probe` | Entry reference 100.0/price 100.02; stop reference 100.0/price 99.98; position flat |
| `rg`, `sed`, `git diff`, `git show` on the named files | The `file:line` evidence in the audit |
| Relative-link check over `docs/**/*.md` | 4 broken links, all in `docs/releases/2026-10-28/v0.2.0.md` |

The full gate log is `/tmp/opencode/audit-gates-full-2026-10-31.log` (scratch;
not committed). The probe is not part of the repository. No live Dukascopy
request was made, no full-year market source was present, and no performance
benchmark was run.

## Learning

- The prior remediation holds: all ten findings from
  AUD-2026-10-31-31..40 are fixed and tested at `e34693f`.
- The most consequential new defects are evaluation and provenance defects,
  not trading-engine defects: output-path collisions, a strategy-run envelope
  that records the wrong period, two definitions of an independent rerun, a
  correlation check that tests two of six pairs, Unknown checks with failure
  text, a research export that skips its validation table, and an architecture
  baseline that omits a real crate edge.
- Re-verification changed a severity. The subagent's entry-bar example was
  invalid, and the claimed target pre-emption does not exist. A code trace that
  sounds plausible is not enough; the accepted finding is narrower and now has
  runtime evidence.
- The diagnosis's missing-value convention is still inconsistent: the prior
  audit fixed `strategy-exit-reasons.csv` but not `checks.csv` or
  `model-health.csv`. The same pattern appears in the OOS fix: the shared
  comparison was centralized, but the adjacent rerun comparison was not.
- The architecture baseline was written from the intended graph, not the
  manifests. A living baseline needs a mechanical guard, or it will drift on
  the next dependency.
- I considered reporting the strategy-run empty success as a Major code defect.
  The W6b2 validation intentionally changed the behavior; the defect is the
  stale README and the missing zero-bar signal, so the finding is Minor and the
  plan offers both the documentation fix and the error restoration as D2.
- I considered reporting `max_drawdown`/halt as a code defect requiring a
  halt at fills. Whether the halt is mark-only is a policy decision; the audit
  records the inconsistency and the plan presents both options as D10.
- I considered reporting the store's `AUTOINCREMENT` ids as a determinism
  defect. The ids are database-local and not part of the dataset identity; the
  finding is Info, with the query-output and `application_id` options grouped
  under AUD-67.
- I did not treat the passing gates as evidence that the findings are false.
  The suite passes because the defective paths are not covered; the audit
  names the missing guards instead of weakening the gate result.

## Role disclosure and limitations

The five exploration passes were separate read-only subagent sessions of the
same model family. The consolidation, severity assignment, and re-verification
were performed by the main agent. The Planner pass is a sequential role pass
by the same agent. This is not an independent-agent or independent-model
judgment.

No live provider request, full-year data reproduction, adversarial store
migration, pixel-level chart review, or performance benchmark was performed.
The board was read back after the issue tree was created. The audit and plan are
committed on `docs/full-audit-2026-10-31` and opened as PR #320; W1 is
`fix/308-output-path-collision`, PR #319.

## Approval and issue tree

- 2026-10-31 (UTC): the human approved the plan and D1–D17 as recommended via
  "continue".
- The Planner created #308–#318 for W1–W11, added the items to the QuantRule
  project board, and set W1 (#308) to `In Progress` with start date
  2026-10-31. The other leaves are `Todo`.
- Board read-back: `#308 In Progress Issue Open 2026-10-31`; `#309`–`#318`
  `Todo Issue Open`. No field the project workflow owns was set by hand; the
  repository `project-sync.sh` was run with the current token to populate
  `Item type` and `State`.
