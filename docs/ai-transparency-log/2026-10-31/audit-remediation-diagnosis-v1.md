# AI Transparency Log — Remediation and Diagnosis-v1 Audit

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: `audit-remediation-diagnosis-v1`
- Audit: [`docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md`](../../improvement/INVALIDATIONS.md)
- Plan: [`docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md`](../../execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md)
- Branch: `docs/audit-remediation-diagnosis-v1`
- Baseline: `main` at `7c865de` (merge of PR #246)
- Head: `main` at `c8b1dc1` (merge of PR #279)

## Intent and boundary

The human asked for a new full audit of everything the code changed since the
last audit baseline, explicitly setting aside the older audit documents as the
subject. The scope was `git diff 7c865de..c8b1dc1`: 146 files, +11,664/−601, 50
commits, covering the remediation of the 30 findings in the unified-ladder audit
(PRs #248–#277) and diagnosis-v1 (PR #279).

The audit was read-only with respect to code, tests, issues, and pull requests.
It wrote one audit document and this log. A sequential Planner pass then
re-verified every finding against `c8b1dc1`, corrected one closure-test detail,
and wrote `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md`.
The human approved the plan on 2026-10-31 via "proceed", accepting D1–D10 as
recommended. The Planner then created issues #280–#291, moved W1 (#280) to
`In Progress`, and later opened it as PR #294. The audit did not change model,
strategy, execution, portfolio, or analytics behavior.

The work started in the project root in Plan mode. The audit document could not
be written there; after the human switched agents and said to proceed, the work
moved to a dedicated worktree and branch. The audit document, plan, and this log
were committed on `docs/audit-remediation-diagnosis-v1` and opened as PR #293.

## Implemented

- Inspected the current Git state, the two worktrees, the branch, the baseline
  history, and the merged PR sequence before reviewing code.
- Read the audit format, the prior unified-ladder audit, the diagnosis-v1
  contract, the execution plan, and the relevant validation records.
- Reviewed the full diff by area: backtest execution and gap handling, portfolio
  ledger and monthly returns, runner and strategy context, model aggregate/event
  order, analytics diagnosis and research export, shell commands/help/config,
  and the diagnosis charts.
- Ran the global gates at the audited head: `cargo fmt --all -- --check`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace`,
  `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`, and `git diff --check`.
- Re-checked candidate findings against the code before accepting them. In
  particular, verified that the gap-aware pending-fill rejection, step-aware risk
  cap, exit-quantity reconciliation, equity-state refresh, monthly-return carry,
  context identity check, context-missing hold, weight-overflow guard, event
  batch order, and Layer 3 fixed-width export are implemented with regression
  tests.
- Dismissed candidates that did not survive review: the hard-coded
  `report_versions` shell projection is consistent with the lab-identity
  validation; the benchmark date buckets align with the ledger's UTC-day equity
  points; the diagnosis charts guard empty series; the diagnosis code uses
  ordered collections and no clock; `lab_summary` already compares the OOS data
  version correctly.
- Wrote 10 findings (1 Major, 9 Minor) with evidence, impact, options,
  recommendation, and a closure test. The Major finding is the false
  `evidence.out_of_sample` pass; the Minors cover the missing-1m gap check, the
  incomplete AUD-23 direction guard, the vacuous report-version check, the
  zero-trade average hold, the manifest self-omission, negative MFE, the daily
  funnel omission, stale module docs, and a `lab list` byte-slice panic.
- Recorded the prior audit's verification gap: AUD-23 claims an out-of-range
  direction test that is not present in the repository.
- Ran the Planner pass: re-read every finding's code path at `c8b1dc1`,
  confirmed none is stale or duplicate, grouped the findings into seven work
  items, and wrote the proposed execution plan with decisions D1–D10 and a
  finding→work-item→issue→verification traceability table.
- Corrected finding 35's closure test during triage: the established CSV
  convention for a missing number is an empty cell, not the literal `null`;
  `diagnosis.json` uses null. The finding and its severity are unchanged.
- Marked the 10 findings `Planned` and linked each to its issue.
- Created the issue tree after approval: W1–W7 are #280–#286; #287–#288 are
  W4's sub-issues; #289–#291 are W5's. Synced the items through
  `.github/scripts/project-sync.sh`, set W1 to `In Progress`, and set its
  `Start date` to 2026-10-31. No field the project workflow owns was set by
  hand.
- W1 (#280) was implemented on `fix/280-oos-evidence` and opened as PR #294;
  its validator record and log are in that branch. The audit document marks
  finding 31 `Fixed`.

## Verification and evidence

Commands actually run, with observed results:

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 1504 passed, 0 failed, 7 ignored |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |
| `cat target/report-tests/diagnosis-command/diagnosis/manifest.txt` | header `artifacts: 17 (including this manifest)`, 16 table rows |
| `ls target/report-tests/diagnosis-command/diagnosis \| wc -l` | 17 files |
| `sed -n '545,551p' crates/quantrule-shell/src/diagnosis.rs` | `has_out_of_sample` returns `is_file()` only |
| `sed -n '542,549p' crates/quantrule-shell/src/lab_summary.rs` | the correct `data_version` comparison |
| `sed -n '1248,1263p' crates/quantrule-analytics/src/diagnosis/compute.rs` | `evidence.out_of_sample` passes on the boolean |
| `sed -n '917,936p' crates/quantrule-analytics/src/diagnosis/compute.rs` | report-version check passes on a non-empty list |
| `sed -n '983,1011p' crates/quantrule-analytics/src/diagnosis/compute.rs` | `data.long_gap` `None` branch passes with `0.0` |
| `sed -n '500,504p' crates/quantrule-analytics/src/diagnosis/compute.rs` | zero-trade `average_hold_bars` is `0.0` |
| `sed -n '883,897p' crates/quantrule-analytics/src/diagnosis/compute.rs` | MFE/MAE are fill-relative and can cross zero |
| `sed -n '583,613p' crates/quantrule-analytics/src/diagnosis/compute.rs` | daily fills come from `input.trades` |
| `sed -n '131p' crates/quantrule-model/src/aggregate/events.rs` | `usize::try_from(direction + 1)` |
| `sed -n '717p' crates/quantrule-shell/src/command/lab.rs` | byte-length slice of `config_hash` |
| `gh issue create` | #280–#291 created; #287–#288 linked under #283, #289–#291 under #284 |
| `gh project item-list 1 --owner quant-rule` | W1, W4, W5 `In Progress`; the other leaves `Todo` |

The global gate log is `/tmp/opencode/audit-gates-2026-10-31.log` (scratch; not
committed). No new probe or test was compiled; findings 31–40 are based on code
inspection and existing fixture/generated output. The audit document names the
regression test each closure needs. The Planner pass re-read the code paths in
the plan's verification table; it ran no new build or test.

## Learning

- The remediation is largely correct. The two highest-risk fixes — rejecting a
  pending fill across a non-contiguous execution bar and reconciling the exit
  quantity to the open position — hold up under code review and have regression
  tests.
- The most consequential new defect is not a trading-engine defect: the
  diagnosis reports OOS evidence from a file's existence. The correct comparison
  already exists in `lab_summary`, which is the cheapest remediation path.
- The prior audit's `Verified` status is not uniform evidence. AUD-23's claimed
  test does not exist, and the guard it added still overflows for `i8::MAX`.
  This is a reminder to treat a verification column as a claim to reproduce, not
  as proof.
- The diagnosis contract's "missing measurement is unknown, never pass" rule is
  applied to checks but not to the 1m-gap branch. The same null-versus-zero
  distinction appears in the zero-trade average hold.
- I considered reporting the hard-coded `report_versions` projection as a
  finding, but the lab-identity validation refuses stale bundles before the
  diagnosis is built, so the projection is consistent for both entry points. I
  kept the narrower finding that the analytics check itself does not enforce the
  five-version threshold.
- I considered reporting the `rerun` check as too weak because it compares only
  five summary fields, but that matches the diagnosis contract's comparison
  fields, so it is not a defect.
- I considered reporting the variable Layer 3 width for a non-default ladder,
  but the contract and the CLI test fix the configured ladder and the short
  source now emits the configured blocks; the remaining width is a function of
  the configuration, not the data length.

## Limitations

- The audit is a sequential read-only pass, and the Planner pass is a sequential
  role pass by the same agent. A separate verifier session was used later for
  closure; it is the same model family, so it is not an independent judgment.
- The prior audit's 30 fixes were reviewed at the diff level and spot-checked;
  not every fix was independently reproduced. AUD-23 is the concrete
  false-verification case.
- The project board could not be inspected: `gh` has no ProjectV2 #1 in this
  repository. Board state is not part of the audit.
- Data files under `data/` and generated `target/` artifacts were not audited as
  code; `target/report-tests` was used only as observed output for the manifest
  finding.
- No performance benchmarks were run. The diff does not add a
  performance-sensitive path that would require one.

## Verification pass

- 2026-10-31 (UTC), OpenCode agent (DeepSeek V4.1 Flash), a separate session with
  fresh context, working read-only against `main` at `476d00a`.
- Intent: closure verification of AUD-2026-10-31-31..40. The verifier inspected
  the fix PRs and regression guards, reproduced the fail-before behavior in
  throwaway worktrees where the fix allowed it, and reran the global gates.
- Evidence: `cargo test --workspace` 1518 passed, 0 failed, 7 ignored; fmt,
  clippy, and rustdoc clean; record:
  `docs/reports/2026-10-31/improvement-verification-remediation-diagnosis-v1.md`.
- Result: all ten findings `Verified`; three documentation gaps found (W5
  contract note, AUD-23 cell, plan traceability). The human authorized their
  correction after the pass, and the plan was marked `Verified`.
- Learning: the regression guards and fail-before behavior hold up end to end;
  the gaps were documentation and bookkeeping, not product defects.
- Limitation: the verifier is a separate session but the same model family, so
  this is not a human-independent judgment.
