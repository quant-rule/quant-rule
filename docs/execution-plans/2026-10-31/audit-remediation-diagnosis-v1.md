# Remediation and Diagnosis-v1 Audit Remediation Plan

- Date: 2026-10-31 (UTC)
- Status: Approved
- Work item: `audit-remediation-diagnosis-v1`
- Audit: [`docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md`](../../audits/2026-10-31/remediation-and-diagnosis-v1-audit.md)
- AI transparency log: [`docs/ai-transparency-log/2026-10-31/audit-remediation-diagnosis-v1.md`](../../ai-transparency-log/2026-10-31/audit-remediation-diagnosis-v1.md)
- Baseline: `main` at `c8b1dc1` (merge of PR #279)
- Human approval: approved on 2026-10-31 (UTC) via "proceed"; decisions D1–D10 accepted as recommended
- Claim type: program remediation plan; no strategy edge claim

## 1. Intent

Turn the 2026-10-31 remediation-and-diagnosis-v1 audit into a reviewable
remediation plan. The audit found 1 Major and 9 Minor findings at `c8b1dc1`. The
Major finding lets a same-data `oos/` folder pass the out-of-sample evidence
check; the Minors cover missing-measurement handling, one incomplete model
guard, artifact and identity completeness, diagnosis measurement semantics, and
two shell defects.

The audit is read-only evidence. This plan is the only implementation
authorization, and only after the human approves it. It does not authorize
commits, pushes, or PRs (AGENTS.md section 9).

## 2. Current state

- `main` is at `c8b1dc1`; the audited diff is `7c865de..c8b1dc1`.
- Global gates pass at the head: fmt, clippy with warnings denied,
  `cargo test --workspace` (1504 passed, 0 failed, 7 ignored), rustdoc with
  warnings denied, and `git diff --check`.
- The audit records 10 findings: 1 Major and 9 Minor. All are `Planned`.
- The audit document and its AI transparency log are written but uncommitted in
  the worktree `quant-rule--audit-remediation-diagnosis-v1`.
- The issue tree is created: W1–W7 are #280–#286, and the W4/W5 sub-issues are
  #287–#291. W1 (#280) is `In Progress`.

### 2.1 Planner verification

Each finding was re-read against `c8b1dc1`. None is stale, duplicate, or
already fixed.

| Finding | Re-verified at | Result |
|---|---|---|
| 31 | `crates/quantrule-shell/src/diagnosis.rs:175,545-551`; `compute.rs:1248-1263`; `lab_summary.rs:542-549` | Confirmed: the check reads only file existence while the correct data-version comparison exists in `lab_summary` |
| 32 | `compute.rs:145,983-1011` | Confirmed: an absent 1m series becomes an empty gap list and passes with `0.0` |
| 33 | `crates/quantrule-model/src/aggregate/events.rs:131`; `events/mod.rs:76` | Confirmed: `direction + 1` is `i8` arithmetic; no `i8::MAX`/`i8::MIN` test exists in `aggregate/tests.rs` |
| 34 | `compute.rs:917-936` | Confirmed: pass condition is non-empty list plus non-empty versions, not five bundles |
| 35 | `compute.rs:500-504`; `types.rs:288`; `text.rs:566-585` | Confirmed: zero-trade reason publishes `0.0`; the missing-value convention differs between JSON and CSV (see 2.2) |
| 36 | `crates/quantrule-shell/src/diagnosis.rs:878-897`; `tests/diagnosis.rs:73-97` | Confirmed: header counts `artifacts.len() + 2`, the table writes `artifacts.len() + 1` rows |
| 37 | `compute.rs:883-897`; `crates/quantrule-backtest/src/execution/config.rs:44-60` | Confirmed: MFE/MAE are fill-relative and slippage has no upper bound |
| 38 | `compute.rs:583-613,384`; `types.rs:626-645` | Confirmed: daily fills come from closed trades; `input.fills` already carries `time` and `reason` |
| 39 | `crates/quantrule-shell/src/command/lab.rs:1-5` | Confirmed: the module doc names only `lab summary` |
| 40 | `lab.rs:616-626,717` | Confirmed: `config_hash` is unvalidated and sliced by byte length |

### 2.2 Planner corrections to the audit

- Finding 35: the audit's closure test says "null in the CSV". The established
  CSV convention for a missing number is an empty cell (`text.rs`, `trades_csv`
  and `portfolio-monthly.csv`); `diagnosis.json` uses `null`. The finding
  stands; the closure test is corrected to `None` in the struct, `null` in
  JSON, and an empty cell in CSV.
- Finding 36: the audit's evidence named a generated directory that exists only
  after a run. The defect is reproducible from the code arithmetic above and by
  the artifact-set test, which writes
  `target/report-tests/diagnosis-command/diagnosis`.
- Finding 33: the prior audit's AUD-23 cell claims an out-of-range direction
  test. No such test exists. The finding stands; AUD-23's verification cell is
  corrected when the guard is closed.

## 3. Scope

Included:

- The 10 findings, grouped into seven work items.
- A regression guard for every accepted fix that fails before and passes after.
- Contract, module, and command-reference updates that follow the behavior.
- Quant Validator records for W1, W3, and W5.
- An AUD-23 verification correction when the W3 guard closes.

Explicit non-goals:

- New strategy hypotheses, thresholds, fitted weights, or signal-path changes.
- Statistical, machine-learning, or neural-network signal generation.
- New dependencies, `unsafe`, performance work, or live trading.
- Diagnosis threshold changes; a failing check remains evidence, not a rule.
- Rewriting dated historical records beyond the AUD-23 verification correction.
- Creating fix issues or starting implementation before this plan is approved.

## 4. Assumptions and uncertainty

- Finding 37 is a contract decision (D7). If B is chosen, the code and the
  contract change together; if A is chosen, the clamp hides a fill outside the
  entry bar's range.
- Finding 31 needs four distinguishable evidence states: no `oos/` lab,
  unreadable report, same data version, distinct data version. An enum is the
  preferred shape; the exact name is an implementation choice.
- No current fixture has a zero-trade exit reason, an entry fill outside the
  bar range, or a 1m-absent run. Each closure test needs a small explicit
  fixture.
- Diagnosis artifacts are regenerated, not stored; no checked-in golden embeds
  the affected values. The byte-identical test remains the determinism guard.
- `lab list` tolerates a missing provenance today; W7 must not turn that into
  an error.
- A finding may become stale before its issue opens. Re-verify against `main`
  before starting each work item.

## 5. Decisions

Each decision is proposed; none is approved until the human accepts this plan.

| ID | Finding | Decision | Options | Recommendation | Approval |
|---|---|---|---|---|---|
| D1 | 31 | Out-of-sample evidence | A compare `data_version` and distinguish absence/unreadable/same/distinct; B also require a valid OOS report and period; C require a declared OOS version in config | A | Pending |
| D2 | 32 | Absent 1m series | A `None` -> `Unknown` with no value; B require a 1m series at config load; C measure the fastest available timeframe and label it | A | Pending |
| D3 | 33 | Divergence direction guard | A `checked_add(1)` then `try_from`; B explicit `-1/0/1/_` match; C validated newtype | A | Pending |
| D4 | 34 | Report versions | A require `len() == 5`; B require the five expected bundle names with non-empty versions; C fail on any missing version | B | Pending |
| D5 | 35 | Zero-trade average hold | A `Option<f64>` -> JSON null, CSV empty; B NaN; C omit the row | A | Pending |
| D6 | 36 | Manifest self-row | A add the `manifest.txt` row with its final byte length; B reword the header; C drop the count | A | Pending |
| D7 | 37 | MFE/MAE sign | A clamp at zero, conforming to the current contract; B amend the contract to fill-relative signs; C measure from the reference/open price | A for conformance; B if fill-relative excursions are intended | Pending |
| D8 | 38 | Daily entry fills | A build from `input.fills` filtered to `entry`; B add the open position; C document the closed-trade basis | A | Pending |
| D9 | 39 | `lab.rs` module doc | A update the module doc; B split the module per command | A | Pending |
| D10 | 40 | `lab list` config hash | A display-only `get(..12)`; B validate a present hash as 16 ASCII hex and keep A as defense; C skip malformed labs | B + A, scoped so an absent hash is still listed | Pending |

## 6. Work breakdown

| Work item | Findings | Severity | Deliverable | Roles | Depends on |
|---|---|---|---|---|---|
| W1 | 31 | Major | Data-version OOS evidence | Implementation, Test, Quant Validator, Code Reviewer | — |
| W2 | 32 | Minor | Null-safe 1m gap check | Implementation, Test, Code Reviewer | — |
| W3 | 33 | Minor | Overflow-safe direction guard | Quant Developer, Test, Quant Validator, Code Reviewer | — |
| W4 | 34, 36 | Minor | Complete identity and manifest checks | Implementation, Test, Code Reviewer | — |
| W5 | 35, 37, 38 | Minor | Correct diagnosis measurement semantics | Quant Developer, Test, Quant Validator, Code Reviewer | D7 |
| W6 | 39 | Minor | Current `lab` module doc | Implementation, Code Reviewer | — |
| W7 | 40 | Minor | Safe `lab list` hash handling | Implementation, Test, Code Reviewer | D10 |

Recommended order: W1, W2, W3, W4, W5, W6, W7. W1, W4, and W5 touch
`diagnosis/types.rs` and `compute.rs`; they run one at a time so the leaves do
not contend for the same files. Only one work item is `In Progress` at a time.

### Issue tree

| Work item | Issue | Sub-issues |
|---|---|---|
| W1 | [#280](https://github.com/quant-rule/quant-rule/issues/280) | — |
| W2 | [#281](https://github.com/quant-rule/quant-rule/issues/281) | — |
| W3 | [#282](https://github.com/quant-rule/quant-rule/issues/282) | — |
| W4 | [#283](https://github.com/quant-rule/quant-rule/issues/283) | [#287](https://github.com/quant-rule/quant-rule/issues/287), [#288](https://github.com/quant-rule/quant-rule/issues/288) |
| W5 | [#284](https://github.com/quant-rule/quant-rule/issues/284) | [#289](https://github.com/quant-rule/quant-rule/issues/289), [#290](https://github.com/quant-rule/quant-rule/issues/290), [#291](https://github.com/quant-rule/quant-rule/issues/291) |
| W6 | [#285](https://github.com/quant-rule/quant-rule/issues/285) | — |
| W7 | [#286](https://github.com/quant-rule/quant-rule/issues/286) | — |

### W1 — Data-version OOS evidence (Major)

- Replace the file-existence boolean with a four-state evidence value: no
  `oos/` lab, unreadable `oos/` report, same data version, distinct data
  version.
- Implement the comparison once in the shell and use it from both
  `lab_summary` and `diagnosis`, so the two commands cannot disagree.
- Pass the check only for a distinct data version; publish `Unknown` for the
  other three, with evidence that names the case.
- Closure test: an `oos/` report with the same `data_version` gives `unknown`;
  a distinct `data_version` gives `pass`; absent and unreadable give `unknown`
  with distinct evidence. Analytics unit tests cover all four states.
- Validator record: `docs/reports/2026-10-31/w1-oos-evidence-validation.md`.
- Implementation evidence (#280, PR #294): `OutOfSampleStatus` and the shared
  `lab_evidence` comparison are implemented on `fix/280-oos-evidence`; the
  analytics and shell tests cover the four states. Gates pass (1506 tests).
- Paths: `crates/quantrule-shell/src/diagnosis.rs`,
  `crates/quantrule-shell/src/lab_summary.rs`,
  `crates/quantrule-analytics/src/diagnosis/types.rs`,
  `crates/quantrule-analytics/src/diagnosis/compute.rs`, and the shell and
  analytics diagnosis tests.

Issue: [#280](https://github.com/quant-rule/quant-rule/issues/280) — **Derive out-of-sample evidence from the data version**.

### W2 — Null-safe 1m gap check

- Make the 1m gap list optional so an absent series is distinct from a series
  with no gaps; render the gap count as unavailable, not zero.
- `data.long_gap`: absent 1m -> `Unknown`, no value, evidence "no 1m series";
  present with gaps -> current threshold; present with no gaps -> `Pass` with
  `0.0`.
- Keep the histogram and chart rendering empty for an absent series.
- Closure test: a `ModelRun` without 1m gives `unknown` and `value == None`.
- No Quant Validator record: the check is data quality, not a model or
  strategy measurement.
- Implementation evidence (#281, PR #296): the optional gap list, the unknown
  check, and the text/chart/JSON updates are implemented on
  `fix/281-1m-gap-unknown`; gates pass (1508 tests).
- Paths: `crates/quantrule-analytics/src/diagnosis/types.rs`,
  `crates/quantrule-analytics/src/diagnosis/compute.rs`,
  `crates/quantrule-analytics/src/diagnosis/text.rs`,
  `crates/quantrule-shell/src/diagnosis.rs`,
  `crates/quantrule-shell/src/chart/diagnosis/quality.rs`, and the analytics
  diagnosis tests.

Issue: [#281](https://github.com/quant-rule/quant-rule/issues/281) — **Report an absent 1m series as unknown in the gap check**.

### W3 — Overflow-safe direction guard

- Replace `direction + 1` with checked arithmetic before the index conversion,
  or an explicit `-1/0/1` match (D3).
- Keep every in-range direction count unchanged.
- Regression cases: `i8::MAX`, `i8::MIN`, `2`, and the three valid codes.
- Closure test: a debug build with `direction_code = i8::MAX` does not panic
  and does not increment any direction count.
- Validator record: `docs/reports/2026-10-31/w3-direction-guard-validation.md`;
  existing model goldens are the no-change evidence.
- On closure, the Improvement Verifier corrects AUD-23's verification cell to
  cite the new test.
- Implementation evidence (#282, PR #297): the checked guard and the
  out-of-range test are implemented on `fix/282-direction-guard`; gates pass
  (1509 tests). The fail-before run reproduced the overflow.
- Paths: `crates/quantrule-model/src/aggregate/events.rs` and
  `crates/quantrule-model/src/aggregate/tests.rs`.

Issue: [#282](https://github.com/quant-rule/quant-rule/issues/282) — **Guard the divergence direction index without overflow**.

### W4 — Complete identity and manifest checks

- 34: require the five expected bundle names (`model`, `strategy`, `execution`,
  `portfolio`, `analytical`) with non-empty versions; the check fails on a
  missing, extra, or wrong bundle.
- 36: add the `manifest.txt` row with its final byte length, computed by fixed
  point iteration; keep the `ARTIFACTS` order (manifest before
  `diagnosis.json`) so the table matches the artifact list.
- Update the diagnosis contract's artifact table to state that the manifest
  lists every artifact, including itself, with its byte length.
- Closure tests: four versions -> `fail`, five expected -> `pass`; the manifest
  name set equals the 17 `ARTIFACTS` names and every declared byte length
  equals the file's length.
- No Quant Validator record: artifact completeness, not measurement semantics.
- Implementation evidence (#287, PR #299; #288, PR #300): the five-bundle
  requirement and the manifest self-row are implemented on
  `fix/287-report-versions` and `fix/288-manifest-row`; gates pass (1511 tests).
- Paths: `crates/quantrule-analytics/src/diagnosis/compute.rs`,
  `crates/quantrule-shell/src/diagnosis.rs`,
  `crates/quantrule-shell/tests/diagnosis.rs`,
  `docs/reports/2026-10-31/diagnosis-v1.md`, and the analytics diagnosis tests.

Issue: [#283](https://github.com/quant-rule/quant-rule/issues/283) — **Make diagnosis artifact and identity checks complete**.

Sub-issues created with the work item:

1. [#287](https://github.com/quant-rule/quant-rule/issues/287) — **Require the five bundle versions in `identity.report_versions`**
2. [#288](https://github.com/quant-rule/quant-rule/issues/288) — **List `manifest.txt` in its own manifest**

### W5 — Diagnosis measurement semantics

- 35: `ExitReasonStats.average_hold_bars` becomes `Option<f64>`; a zero-trade
  reason publishes `None`, JSON `null`, and an empty CSV cell.
- 37: implement D7. Under A, clamp MFE at zero and MAE at zero while keeping
  the measured favorable and adverse prices; add the property
  `mfe >= 0` and `mae <= 0` when present. Under B, amend the contract and
  assert the documented sign rule instead.
- 38: build the daily entry fills from `input.fills` with `reason == "entry"`,
  keyed by `time`; the daily sum for a run must equal `funnel.entry_fills`.
- Update the diagnosis contract's artifact table to state that a missing
  numeric CSV value is an empty cell while `diagnosis.json` uses `null`.
- Closure tests: a zero-trade reason is `None`/null/empty; an entry fill above
  the bar high keeps MFE non-negative under A; one open entry on a day without
  a closed trade counts as `entry_fills = 1`.
- Validator record:
  `docs/reports/2026-10-31/w5-diagnosis-measurements-validation.md`.
- Implementation evidence (#289, PR #301): the optional average hold, its
  formatters, and the tests are implemented on `fix/289-null-hold`; gates pass
  (1513 tests).
- Implementation evidence (#290, PR #302): the MFE/MAE clamp and its three
  tests are implemented on `fix/290-mfe-clamp`; gates pass (1516 tests).
- Implementation evidence (#291, PR #303): the daily entry fills read the fill
  list and exclude non-entry fills; gates pass (1517 tests). W5 is complete.
- Paths: `crates/quantrule-analytics/src/diagnosis/types.rs`,
  `crates/quantrule-analytics/src/diagnosis/compute.rs`,
  `crates/quantrule-analytics/src/diagnosis/text.rs`,
  `crates/quantrule-shell/src/diagnosis.rs`,
  `docs/reports/2026-10-31/diagnosis-v1.md`, and the analytics diagnosis tests.

Issue: [#284](https://github.com/quant-rule/quant-rule/issues/284) — **Correct diagnosis measurement semantics**.

Sub-issues created with the work item:

1. [#289](https://github.com/quant-rule/quant-rule/issues/289) — **Publish a null average hold for a zero-trade exit reason**
2. [#290](https://github.com/quant-rule/quant-rule/issues/290) — **Keep enriched MFE and MAE within the documented sign**
3. [#291](https://github.com/quant-rule/quant-rule/issues/291) — **Count open entry fills in the daily funnel**

### W6 — Current `lab` module doc

- Update the `lab.rs` module header to name `lab summary`, `lab diagnose`, and
  `lab list`, and state the shared boundary: parse the lab folder, delegate
  reading and rendering, never open a source or compute a model value.
- Verification: `cargo doc` builds and the header names the three commands.
- Paths: `crates/quantrule-shell/src/command/lab.rs`.

Issue: [#285](https://github.com/quant-rule/quant-rule/issues/285) — **Update the `lab` module documentation**.

### W7 — Safe `lab list` hash handling

- Validate a present `config_hash` as 16 ASCII hex characters and return a
  typed `RunError` naming the lab when it is not; a missing provenance or hash
  keeps today's empty display.
- Use `get(..12).unwrap_or(...)` for the display as a second defense.
- Update the README `lab list` contract to state the validation.
- Closure tests: a multi-byte hash returns an error and does not panic; a
  16-hex hash lists; a lab without provenance still lists.
- Paths: `crates/quantrule-shell/src/command/lab.rs`, `README.md`, and the
  shell diagnosis lab tests.

Issue: [#286](https://github.com/quant-rule/quant-rule/issues/286) — **Validate the `lab list` config hash without panicking**.

## 7. Acceptance criteria

- Every accepted finding has a regression guard that fails before the fix and
  passes after it, or a written reason why a fail-before test is impossible.
- W1: a same-version `oos/` lab yields `evidence.out_of_sample = unknown`; a
  distinct version yields `pass`; `lab summary` and `lab diagnose` agree.
- W2: a run without a 1m series yields `data.long_gap = unknown` with no value.
- W3: `i8::MAX`, `i8::MIN`, and `2` do not panic; in-range aggregate output is
  unchanged against the existing goldens.
- W4: four report versions fail the identity check; the manifest lists all 17
  artifacts with matching byte lengths.
- W5: a zero-trade reason is `None`/null/empty; MFE and MAE respect the
  documented sign under D7=A; the daily entry fills reconcile with
  `funnel.entry_fills`.
- W6: the module header names `lab summary`, `lab diagnose`, and `lab list`.
- W7: a present non-hex `config_hash` returns an error without panicking; an
  absent hash still lists.
- The global gates in section 9 pass and no test is weakened, removed, or
  ignored.
- The audit findings move to `Fixed` with PR links, then to `Verified` only
  after the Improvement Verifier reproduces them.

## 8. Traceability

| Finding | Work item | Issue | Closure verification | Status |
|---|---|---|---|---|
| AUD-2026-10-31-31 | W1 | [#280](https://github.com/quant-rule/quant-rule/issues/280) | Same-version `oos/` -> `unknown`; distinct -> `pass` | Planned |
| AUD-2026-10-31-32 | W2 | [#281](https://github.com/quant-rule/quant-rule/issues/281) | 1m-absent run -> `unknown`, `value == None` | Planned |
| AUD-2026-10-31-33 | W3 | [#282](https://github.com/quant-rule/quant-rule/issues/282) | `i8::MAX`/`i8::MIN`/`2` do not panic; counts unchanged | Planned |
| AUD-2026-10-31-34 | W4 | [#287](https://github.com/quant-rule/quant-rule/issues/287) | Four versions -> `fail`; five expected -> `pass` | Planned |
| AUD-2026-10-31-35 | W5 | [#289](https://github.com/quant-rule/quant-rule/issues/289) | Struct `None`, JSON `null`, CSV empty cell | Planned |
| AUD-2026-10-31-36 | W4 | [#288](https://github.com/quant-rule/quant-rule/issues/288) | Manifest names == 17 artifacts; byte lengths match | Planned |
| AUD-2026-10-31-37 | W5 | [#290](https://github.com/quant-rule/quant-rule/issues/290) | Fill outside bar -> `mfe >= 0`, `mae <= 0` (D7=A) | Planned |
| AUD-2026-10-31-38 | W5 | [#291](https://github.com/quant-rule/quant-rule/issues/291) | Open entry on day D -> `entry_fills = 1` on D | Planned |
| AUD-2026-10-31-39 | W6 | [#285](https://github.com/quant-rule/quant-rule/issues/285) | Header names the three commands | Planned |
| AUD-2026-10-31-40 | W7 | [#286](https://github.com/quant-rule/quant-rule/issues/286) | Multi-byte hash -> error, no panic | Planned |

No finding is deferred. No finding is marked `Won't fix`; only the human can
decide that.

## 9. Verification

Global gates, run at the end of each work item and before its pull request:

| Check | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all -- --check` | exit 0 |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0 |
| Tests | `cargo test --workspace` | all pass, no ignored addition |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| Whitespace | `git diff --check` | clean |

Targeted evidence:

- W1: `cargo test -p quantrule-shell --test diagnosis` and the analytics
  diagnosis tests, plus the four-state unit test.
- W2: analytics diagnosis unit test for a 1m-absent `ModelRun`; the
  `10-data-quality.png` render test still passes.
- W3: `cargo test -p quantrule-model`; the existing aggregate goldens are
  unchanged.
- W4: analytics tests for the identity check; the shell artifact-set test plus
  a new manifest table assertion.
- W5: analytics tests for exit reasons, excursions, and the daily funnel; the
  byte-identical diagnosis test still passes.
- W6: `cargo doc`.
- W7: a shell test with a crafted lab root; `lab list --json` still parses.

The Quant Validator reproduces W1, W3, and W5 from committed fixtures and
records the commands and observed results. W2, W4, W6, and W7 are covered by
tests and code review only. No performance benchmark is required: the diff
adds no performance-sensitive path.

Limitations: the closure tests use unit and integration fixtures, not live
market data; the Quant Validator's reproduction is a sequential pass, not an
independent model judgment, and it is recorded as such.

## 10. Risks and recovery

- W1 changes a diagnosis input type and the `lab summary` evidence text. The
  ripple is limited to the shell projection, the analytics tests, and the
  `lab-summary.txt` fixture; the fixture's absent-`oos/` line stays valid.
- W2 changes `data_quality.raw_gaps` to null when 1m is absent. Update the text
  summary, JSON projection, and chart together; an empty list remains the
  present-with-no-gaps case.
- W3 must be a no-op for valid directions. If any in-range aggregate count
  changes, stop: that is a behavior change outside the guard.
- W5-35 changes a public analytics field; update the shell JSON projection and
  every consumer. W5-38 changes the daily series and the funnel chart; assert
  the daily/aggregate reconciliation. W5-37 is bounded by D7.
- W7 changes `lab list` error behavior only for a present malformed hash. A
  lab without provenance must still list; the existing JSON test is the guard.
- Recovery: one revertable pull request per issue; diagnosis artifacts are
  regenerated by rerunning the command; there is no stored schema or migration.

## 11. Stop conditions

Stop and ask the human when:

- A decision in section 5 is not accepted as recommended, especially D7.
- A fix requires a new dependency, an `unsafe` block, a signal-path change, or
  a command-contract change beyond W7's documented error.
- An in-range model aggregate changes under W3.
- A finding no longer reproduces, or the fix reveals a wider defect.
- The audit/plan commit, push, or pull request has not been authorized.

## 12. Approval

Approved on 2026-10-31 (UTC) via the human's "proceed"; D1–D10 accepted as
recommended. The approval authorizes W1–W7 and the issue tree. It does not
authorize commits, pushes, merges, or pull requests (AGENTS.md section 9). The
Planner created the issue tree (#280–#291) and marked the audit findings
`Planned`; the work proceeds one leaf at a time in its own worktree.
