# Remediation and Diagnosis-v1 Audit Improvement Verification

- Date: 2026-10-31 (UTC)
- Work item: `audit-remediation-diagnosis-v1`
- Audit: [`docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md`](../../improvement/INVALIDATIONS.md)
- Plan: [`docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md`](../../execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md)
- Verifier: Improvement Verifier (AI), sequential read-only pass in a dedicated worktree
- Baseline: merged `main` at `476d00a`
- Toolchain: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; `cargo 1.99.1 (5f94df478 2026-08-27)`
- Claim type: audit closure; no strategy edge claim

## Method

The verifier read each finding's expected invariant and verification in the
audit, located the fix and its regression guard on `main` at `476d00a`,
inspected each fix PR's merge diff, ran the targeted tests, reproduced the
fail-before claim where the fix permits it, and checked the consumers and
contracts the change should have updated. Fail-before checks used throwaway
detached worktrees under `/tmp/opencode` with the regression test copied from
the fixing PR; no committed code or test was modified for them.

The verification worktree is `quant-rule--verify-remediation-diagnosis-v1` on
branch `docs/verify-remediation-diagnosis-v1`, created from `main` at `476d00a`.
This record and the `Status` column in the audit are the verifier's file
changes; the post-verification corrections below were made by the orchestrator
after this pass.

## Findings

| Finding | Fix (PR) | Reproduction command(s) | Observed result | Verdict | Residual risk |
|---|---|---|---|---|---|
| AUD-2026-10-31-31 | #294 | `cargo test -p quantrule-shell --test diagnosis lab_diagnose_requires_a_distinct_oos_data_version`; `cargo test -p quantrule-shell --test lab_summary out_of_sample_and_rerun_evidence_are_checked`; `cargo test -p quantrule-analytics --test diagnosis out_of_sample_evidence_requires_a_distinct_data_version` | Passed. Shell `lab diagnose`: same `data_version` -> `unknown`, distinct -> `pass`. Shell `lab summary` prints the same-version and distinct-version lines and requires `--force` for the second write. Analytics maps all four states: absent/unreadable/same -> `unknown`, distinct -> `pass`. `lab_evidence` is the single comparison used by both commands. Fail-before at `714c678^1`: the same-version shell test returned `pass`. | Verified | A distinct `data_version` string does not prove the OOS period is disjoint; the data version is caller-set. The audit's option B (period check) was not selected. |
| AUD-2026-10-31-32 | #296 | `cargo test -p quantrule-analytics --test diagnosis a_run_without_1m_reports_the_gap_check_unknown a_present_1m_series_with_no_gaps_passes_with_zero` | Passed. 1m absent -> `unknown`, `value == None`, evidence "no 1m series", triage "Add a 1m series"; 1m present with no gaps -> `pass`, `0.0`. Fail-before at `265341f^1`: the new test failed with `left: Pass, right: Unknown`. Text, JSON (`null`), histogram, and chart consumers handle the `None`. | Verified | A present-but-empty 1m series still produces an empty gap list and `pass` with `0.0`; the finding covered an absent series, not an empty one. |
| AUD-2026-10-31-33 | #297 | `cargo test -p quantrule-model --lib out_of_range_divergence_directions_do_not_panic_or_count` | Passed. `i8::MAX`, `i8::MIN`, and `2` are ignored; `-1`, `0`, and `1` are counted (six divergence timeframes, direction agreement share `1/3`). Fail-before at `43be6f9^1`: the test panicked with "attempt to add with overflow" at `aggregate/events.rs:131`. | Verified | The public snapshot still accepts any `i8`; the guard ignores an out-of-range value at the aggregate boundary. A validated newtype was not selected. |
| AUD-2026-10-31-34 | #299 | `cargo test -p quantrule-analytics --test diagnosis report_versions_require_the_five_expected_bundles` | Passed. The five expected bundles pass; four versions, a wrong bundle, and an empty version fail. Fail-before at `8a59727^1`: the new test failed with `left: Pass, right: Fail`. | Verified | None material. The check requires `len() == 5` plus the five expected names, so an extra or wrong bundle cannot pass. |
| AUD-2026-10-31-35 | #301 | `cargo test -p quantrule-analytics --test diagnosis a_zero_trade_exit_reason_has_no_average_hold a_traded_exit_reason_keeps_its_average_hold` | Passed. Zero-trade reasons are `None` and render an empty CSV cell; a stop trade with a four-bar hold yields `Some(4.0)` and `4.000000`. The old `0.0` assignment is visible in the diff. A literal fail-before cannot compile because the field changed from `f64` to `Option<f64>`; a temporary checkout confirmed the type error. | Verified | The shell JSON `null` branch is covered by the analytics projection rather than an end-to-end shell run. The plan's W5 contract note (empty CSV cell vs JSON `null`) is missing; see plan status below. |
| AUD-2026-10-31-36 | #300 | `cargo test -p quantrule-shell --test diagnosis manifest_lists_every_artifact_including_itself` | Passed. The manifest header equals the 17 artifact names, the row set equals the 17 files including `manifest.txt`, and every declared byte length matches the file length. Fail-before at `ab16a82^1`: the new test failed because `manifest.txt` was absent from the rows. | Verified | None material. The fixed-point length loop is bounded by the manifest's growing byte count. |
| AUD-2026-10-31-37 | #302 | `cargo test -p quantrule-analytics --test diagnosis a_long_fill_above_the_entry_bar_clamps_mfe_to_zero a_short_fill_below_the_entry_bar_clamps_mfe_to_zero an_in_range_fill_keeps_its_measured_excursions` | Passed. Out-of-range long and short fills give `mfe = 0` and `mae = -3`, with the measured prices retained; an in-range fill keeps `mfe = 1`, `mae = -1`. Fail-before at `b82019c^1`: the long test failed with `left: Some(-1.0), right: Some(0.0)`. | Verified | Option A clamps the excursion and therefore hides that the fill was outside the entry bar's range. That is the approved D7=A contract choice, not a regression. |
| AUD-2026-10-31-38 | #303 | `cargo test -p quantrule-analytics --test diagnosis the_daily_funnel_counts_open_entry_fills` | Passed. Two entry fills on different days, including one still open, produce two daily points with `entry_fills = 1` each; the daily sum equals `funnel.entry_fills` (2). Fail-before at `b82019c^1`: the new test failed with `left: 1, right: 2`. | Verified | The shell fixture's open-position path is not asserted end-to-end; the analytics input is the tested boundary. |
| AUD-2026-10-31-39 | #304 | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed. The `lab.rs` module header names `lab summary`, `lab diagnose`, and `lab list` and states the shared boundary. | Verified | None. |
| AUD-2026-10-31-40 | #305 | `cargo test -p quantrule-shell --test diagnosis lab_list_validates_a_present_config_hash` | Passed. A 16-byte multi-byte hash returns an error without panicking; a 16-hex hash lists; an absent provenance still lists with an empty hash. Fail-before at `476d00a^1`: the new test observed exit code `101` (panic at the byte slice). The README `lab list` contract documents the validation. | Verified | One malformed lab now fails the whole `lab list` command; that is the approved option B behavior. |

## Per-finding detail

Only findings with a nuance beyond the table are expanded.

### AUD-2026-10-31-31

`lab_evidence::out_of_sample` distinguishes `Absent`, `Unreadable`,
`SameDataVersion`, and `DistinctDataVersion`. Both the shell diagnosis
projection and `lab summary` call it, so the two commands cannot disagree. The
analytics check maps only `DistinctDataVersion` to `pass`. The README `lab
summary` paragraph was updated; the diagnosis contract already required a
different data version.

### AUD-2026-10-31-32

`DataQuality.raw_gaps` is now `Option<Vec<Gap>>`. Every consumer was updated:
the text summary, `diagnosis.json` (`null`), the gap histogram, and the
data-quality chart. The plan's W2 text did not require a contract edit because
the contract's general rule already says a missing measurement is `unknown`.

### AUD-2026-10-31-33

The fail-before was reproduced in a detached worktree at `43be6f9^1` with the
fixing PR's test file copied in; it panicked at `aggregate/events.rs:131:52`
with "attempt to add with overflow", matching the validator record. The current
guard uses `checked_add(1)`, then `usize::try_from`, then a length filter.

### AUD-2026-10-31-35

The fail-before is not runnable as written because the old field was `f64` and
the new test asserts `Option`. The old assignment and the new assertion are both
visible in the PR diff, and a temporary checkout confirmed the test does not
compile against the old type. The shell JSON projection serializes the
`Option<f64>` as `null`; the CSV writer emits an empty cell; the text summary
uses `-`.

### AUD-2026-10-31-37

The fix keeps the measured favourable and adverse prices and clamps only the
excursions. The in-range test is the no-change guard. The diagnosis contract's
non-negative/non-positive invariant is now true for every fill, including one
outside the entry bar's range.

### AUD-2026-10-31-38

The daily series reads `input.fills` filtered to `reason == "entry"` rather than
the closed-trade list. Days that have an entry fill but no entry attempt are
also emitted. The aggregate `funnel.entry_fills` uses the same fill list, so the
two reconcile by construction.

## Global gates at `476d00a`

Run in the verification worktree after the targeted tests:

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 1518 passed, 0 failed, 7 ignored across 47 test-result suites |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The full test log is `/tmp/opencode/verify-tests.log` (scratch; not committed).
The counts include 35 doctests. The 7 ignored tests are pre-existing and were
not added by the remediation.

## Plan acceptance

The finding-level acceptance criteria are met: every behavioral finding has a
regression guard that fails before and passes after (finding 35's fail-before is
a type change, documented in its detail block), finding 39 is verified by
inspection and `cargo doc`, the targeted behaviors match the audit's
verification columns, and the global gates pass.

Two plan deliverables were not present on `main` at the time of this pass:

1. The W5 work breakdown required the diagnosis contract to state that a
   missing numeric CSV value is an empty cell while `diagnosis.json` uses
   `null`. `docs/reports/2026-10-31/diagnosis-v1.md` was updated for the
   manifest row by #300 but not for the missing-value convention.
2. The W3 work breakdown required the Improvement Verifier to correct AUD-23's
   verification cell in `docs/audits/2026-10-31/unified-ladder-audit.md` to
   cite the new guard test. The cell still reads "Out-of-range direction test
   does not panic" and does not cite
   `out_of_range_divergence_directions_do_not_panic_or_count`.

Plan bookkeeping also remains: the traceability table in section 8 still says
`Planned` for all ten findings. That is part of the plan's own closure update,
not a product defect.

Because of those documentation items, the plan could not be marked `Verified`
in full at the time of this pass. The verification mandate for this pass covered
this audit's `Status` column and this record only, so no other file was edited.
The human can accept the items as follow-ups or authorize a one-line correction
to each and then mark the plan `Verified`.

## Post-verification corrections

After this pass, the human authorized the three documentation corrections. On
2026-10-31 the orchestrator added the missing-value note to the diagnosis
contract, cited the W3 guard test in AUD-23, and marked the ten traceability
rows and the plan `Verified`. These edits are outside this verifier pass; the
verdict above is based on the merged code at `476d00a`.

## Limitations

- This is a separate session from the audit, plan, and implementation passes,
  but the same model family (DeepSeek V4.1 Flash). It is a sequential verifier
  pass, not a human-independent review or independent model judgment.
- The Quant Validator records for W1, W3, and W5 were inspected; this pass did
  not commission an independent Quant Validator. Their own records already
  disclose that they are sequential passes by the implementation agent.
- No live market data was used. The evidence is committed fixtures, generated
  diagnosis artifacts, and targeted unit/integration tests.
- Fail-before reproductions used throwaway worktrees at the fixing PRs' first
  parents. They are not part of the committed tree and are not a substitute for
  the current-tree regression guards. One initial fail-before build for finding
  31 hit a linker bus error because `/tmp` was full from the throwaway target
  directories; after removing them the same command reproduced the expected
  fail-before.
- The project board and issue state were not inspected or changed; this pass is
  repo-only.

## Verdict

All ten findings, AUD-2026-10-31-31 through -40, are `Verified`. No finding is
returned for remediation. The two plan documentation deliverables above remain
for the human to accept or assign before the plan is marked `Verified`.
