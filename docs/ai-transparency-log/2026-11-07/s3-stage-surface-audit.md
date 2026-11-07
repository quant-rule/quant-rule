# AI Transparency Log — S3.x Stage-Surface Audit

- Date: 2026-11-07 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: S3.x stage-surface audit
- Audit document: `docs/audits/2026-11-07/s3-stage-surface-audit.md`
- Boundary: Audit Workflow stage 1 only. Read-only against the code and tests;
  no code, test, issue, pull request, or branch changes. The only repository
  writes are this log and the audit document.

## Intent and boundary

The human asked to run the Audit Workflow. The last published audit
(`docs/audits/2026-11-06/current-surface-audit.md`) covered the shell, model,
config, and report surface and explicitly excluded the S3.x stage internals. The
S3.x strategy, execution, portfolio, backtest, qualification, and analytics
commands were merged after that audit. The audit therefore targeted the current
S3.x stage surface at `main` `fbc38a0`, not a trading hypothesis.

The audit did not run a validation partition, did not claim an edge, and did not
treat the fixture runs as evidence of profitability.

## Verification and evidence

The audit ran the repository gate on `fbc38a0` before writing the document:

- `cargo fmt --all -- --check` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test --workspace` passed: 1,312 tests, 0 failed, 8 ignored.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` passed.
- All ten Python contract checks, the structural fitness checker, and
  `git diff --check` passed.

The behavioral findings were reproduced in a scratch directory outside the
repository, using the checked-in fixtures and the built `target/debug/quantrule`
binary. The exact recipes and observed outputs are in the audit document's
"Reproduction notes". The main observations:

- A 15m strategy ran through a 5m `backtest run` with exit 0, 300-second
  decisions, and `"execution_timeframe": "15m"` in the strategy provenance.
- A 15m condition with no context produced 52 holds in `backtest run`, while
  `strategy run` refused it with `a strategy condition names 900s, but it is not
  the execution timeframe or a configured context timeframe`.
- `strategy explain` on the coupled model bundle produced `enter_bracket` at bar
  41 for a diagnostic event, while the coupled decisions at bar 41 were `hold`.
- An unregistered event identity `(1,1,2)` produced 52 holds with exit 0.
- `portfolio replay` opened a short trade under `allow_short: false`.
- A `backtest` root containing `junk.txt` was reused and the file remained.

The code references in each finding were read directly. No test result or
observation in the audit is inferred or invented.

## Learning and corrections

- The diagnostic-event issue was initially a test-gap hypothesis. Running
  `strategy explain` showed an actual `enter_bracket` decision from a
  diagnostic-role event, so it was recorded as a boundary/validation defect
  (`AUD-2026-11-07-04`), not only as a missing test.
- The event-set mismatch was found by comparing the standalone and coupled paths
  on the same model bundle. The first comparison used an event-composite
  strategy and showed only the documented position-field difference; the
  compression-bracket and extended-event case exposed the signal difference.
- `available_bar_index` was considered as a contract finding. It was left as an
  observation because the decision also depends on the current position and
  execution state, which are knowable only at the decision bar; the field is
  conservative, not a demonstrated wrong value.
- A missing data-artifact hash in the manifest was considered as a finding. The
  artifact contract only promises a content hash of each stage provenance, so it
  was recorded as an integrity observation, not a defect.
- The audit did not treat the passing gates as evidence that the semantic
  findings are absent; all findings are cross-stage behavior that the current
  gates do not exercise.

## Limits

- One sequential audit pass; no independent agent, Code Reviewer, Quant
  Validator, or human review is claimed.
- The live source path, the data engine beyond the fixture path, and the model
  formulas were not re-audited except where the strategy handoff depends on
  them.
- The fixture data is one day of candles; the findings are structural and do not
  depend on a market outcome.
- No remediation was implemented. The findings are `Open`; the Planner owns the
  next stage after human approval.
