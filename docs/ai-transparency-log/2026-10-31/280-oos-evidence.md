# AI Transparency Log — Out-of-Sample Evidence

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #280, branch `fix/280-oos-evidence`
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W1)
- Audit: `docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md` (AUD-2026-10-31-31)

## Intent and boundary

The human approved the remediation plan on 2026-10-31. W1 fixes the Major
finding: `evidence.out_of_sample` passed when `oos/portfolio/report.json`
existed, without reading its `data_version`.

The change is confined to evaluation evidence. It does not touch the signal
path, a strategy rule, an indicator, execution, portfolio accounting, or a
diagnosis threshold. It adds no dependency and no I/O outside the shell.

## Implemented

- Added `OutOfSampleStatus` to the analytics diagnosis vocabulary with four
  states: absent, unreadable, same data version, distinct data version.
- Added `crates/quantrule-shell/src/lab_evidence.rs` as the one comparison of
  the main and `oos/` portfolio reports. `lab summary` and `lab diagnose` both
  call it, so the two commands cannot disagree.
- `evidence.out_of_sample` passes only for a distinct data version; the other
  three states are `unknown`, each with its own evidence text and no value.
- Added an analytics test across all four states and a shell test that copies
  the main report into `oos/`, then changes its `data_version`.
- Extended the `lab summary` test to assert the same-version line and the
  distinct-version line on one lab.
- Updated the README `lab summary` contract to state the data-version rule.
- Made `lab_list_prints_a_json_array` create its own root directory. A clean
  worktree showed the test depended on another test having written
  `target/report-tests`; that is a test-isolation defect, not part of the
  finding, and the one-line change is recorded here and in the pull request.

## Verification and evidence

Commands actually run, with observed results:

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --test diagnosis` | 4 passed, 0 failed |
| `cargo test -p quantrule-shell --test diagnosis` (after removing `target/report-tests`) | 5 passed, 0 failed |
| `cargo test -p quantrule-shell --test lab_summary` | 18 passed, 0 failed |
| `cargo test --workspace` | 1506 passed, 0 failed, 7 ignored |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The shell test `lab_diagnose_requires_a_distinct_oos_data_version` reads
`diagnosis.json` and asserts `unknown` for the same version and `pass` for a
changed one. The analytics test asserts the evidence text and the null value for
all four states.

## Learning

- The diagnosis had the correct comparison in `lab_summary` but a different,
  weaker check in `diagnosis.rs`. Sharing the comparison removed the
  duplication and the disagreement, rather than adding a second copy.
- A fresh worktree is a useful test-isolation check: the existing lab-list test
  passed only because a previous run had created `target/report-tests`. The
  fix is test-only.
- The remaining limitation is deliberate: the check compares the input-series
  identity, not the OOS period. A distinct `data_version` whose period overlaps
  the main lab still passes. The audit's option B (require a valid OOS report
  and period) was not selected, so this is recorded as residual risk rather
  than hidden.

## Limitations

- The Quant Validator record is a sequential pass by the implementation agent;
  it is not an independent model judgment.
- The tests use checked-in fixtures and a small generated lab, not live market
  data.
- The check is evaluation evidence; no engine reads it, so no strategy result
  changes.
