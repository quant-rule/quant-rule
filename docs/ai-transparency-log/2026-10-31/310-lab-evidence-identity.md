# AI Transparency Log — Lab Evidence and Identity

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #310, branch `fix/310-lab-evidence-identity`
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W3)
- Audit: `docs/audits/2026-10-31/full-system-audit.md` (AUD-2026-10-31-43, -60)

## Intent and boundary

The human approved the full-system audit plan on 2026-10-31. W3 closes two
evaluation-evidence findings: `lab summary` and `lab diagnose` used different
definitions of an independent rerun, and neither validated every provenance
identity field against its own report. It also stops `lab list` from turning a
missing halt state into `false`.

The change is confined to the shell's lab evidence, lab identity, lab list
rendering, documentation, and tests. It does not touch the signal path, a
strategy rule, an indicator, execution, portfolio accounting, or a diagnosis
threshold. It adds no dependency.

## Implemented

- Added `lab_evidence::RerunStatus` with four states: absent, unreadable,
  differs, matches. A rerun matches only when both portfolio reports and both
  provenance files are readable, the config hashes match, and the five summary
  fields reproduce.
- `lab summary` and `lab diagnose` both use that one comparison. A differing
  rerun is `unknown` in the diagnosis and `[ ] independent rerun (rerun/
  differs)` in the summary; an unreadable rerun is a distinct state.
- `lab summary` now compares each provenance `data_version` and `model_version`
  with its own report, in addition to the existing `--data-version` and config
  hash checks.
- `lab diagnose` now compares each provenance `data_version` and
  `model_version` with its own report before the cross-bundle identity checks.
- `lab list` publishes `halted` as `Option<bool>`: `yes`/`no` in the table,
  `null` in JSON, and `—` for a missing value.
- Updated the README, `help all`, the lab-summary help fixture, and the
  diagnosis contract to state the rerun rule and the missing-halt rendering.
- Added five unit tests for the four rerun states, three lab-summary tests, and
  three diagnosis/lab-list tests.

## Verification and evidence

Commands actually run, with observed results:

| Command | Observed |
|---|---|
| `cargo test -p quantrule-shell --bin quantrule lab_evidence` | 5 passed, 0 failed |
| `cargo test -p quantrule-shell --test lab_summary` | 21 passed, 0 failed |
| `cargo test -p quantrule-shell --test diagnosis` | 10 passed, 0 failed |
| `cargo test -p quantrule-shell --test cli help` | 15 passed, 0 failed |
| `cargo test -p quantrule-analytics --test diagnosis` | 13 passed, 0 failed |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace` | 1,539 passed, 0 failed, 7 ignored; 47 suites |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The cross-command test creates a lab, copies the portfolio into `rerun/`, checks
that the diagnosis passes with a matching config, changes the rerun provenance
`config_hash`, then checks that the diagnosis reports `unknown` and
`lab summary` reports `rerun/ differs` for the same lab.

## Learning and rejected suggestions

- The two commands disagreed because the summary had the config-hash check and
  the diagnosis did not. Moving the comparison into `lab_evidence` removed the
  duplicate rule; adding a state enum made "unreadable" distinct from "differs".
- The summary fixture is a synthetic `test:v1` lab summarized with the BTCUSD
  example config. I did not add a `--data-version == config.data.data_version`
  check: it would reject that established fixture and is not the defect the
  audit describes. The new checks compare provenance to report and keep the
  existing `--data-version` and config-hash checks.
- The old `lab list` used `unwrap_or(false)`, so a malformed lab looked like a
  valid non-halted result. The field is now nullable end to end; the text table
  uses `—` rather than inventing a boolean.

## Limitations

- The change is evaluation evidence; no model, strategy, execution, or portfolio
  behavior changes. No Quant Validator record is required.
- The tests use checked-in fixtures and a small generated lab, not live market
  data.
- The rerun comparison uses the five published summary fields and the config
  hash; it does not compare every report field or the rerun period.
