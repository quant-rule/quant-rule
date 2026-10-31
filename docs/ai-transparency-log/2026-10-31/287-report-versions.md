# AI Transparency Log — Five Report Versions

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #287, branch `fix/287-report-versions`
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W4)
- Audit: `docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md` (AUD-2026-10-31-34)

## Intent and boundary

W4's first leaf fixes the Minor finding that `identity.report_versions` passed
with one to four versions, although its threshold says five. The change is
confined to the diagnosis identity check and its tests. It does not change a
threshold, a model or strategy rule, execution, or portfolio accounting.

## Implemented

- Added `REPORT_BUNDLES`, the five expected names: model, strategy, execution,
  portfolio, analytical.
- The check now passes only when the list has exactly five entries, every
  expected bundle is present, and every version is non-empty.
- The evidence names the observed bundles, or says "0 of 5" when empty.
- Added an analytics test for the five expected bundles, four bundles, a wrong
  bundle, and an empty version.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --test diagnosis` | 7 passed, 0 failed |
| `cargo test --workspace` | 1510 passed, 0 failed, 7 ignored |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The shell diagnosis fixture supplies all five bundles and still passes; the
short and wrong lists are exercised through the analytics input.

## Learning

- The old check tested presence, not identity: five copies of one bundle would
  have passed if each version was non-empty. Matching the expected names makes
  the check about the five bundles the contract names.
- The shell hard-codes the five versions, so no shell path produces a short
  list. The defect is reachable through the public analytics input and any
  future caller, which is why the regression test lives at that boundary.
- The count in the evidence now lists the observed names, so a failing check
  says which bundle is missing or misnamed.

## Limitations

- No independent verifier; this is a sequential implementation pass.
- The tests use constructed `ReportVersion` lists, not a full lab.
- The change is evaluation evidence; no strategy result changes.
