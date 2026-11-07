# AI Transparency Log — Remove the `allow_short` Rule

- Date: 2026-11-07 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: remove the `allow_short` portfolio rule
- Plan: `docs/execution-plans/2026-11-07/remove-allow-short.md`
- Boundary: portfolio configuration and risk-rule change only. No model,
  strategy, execution, or portfolio formula change beyond the removed short
  gate. No commit, push, pull request, or merge.

## Intent and boundary

The human instructed that QuantRule always operates in both directions and that
the `allow_short` rule must be removed from the repository. The implementation
removed the field from `PortfolioConfig`, removed the `ShortNotAllowed` sizing
rejection and the now-unused side parameter from the sizing API, removed the
field from the shell parser and the portfolio provenance, moved current configs
to `quantrule.portfolio/v2`, and updated the current docs. The audit finding
`AUD-2026-11-07-03` was marked `Won't fix` by the human decision.

The change does not touch the model, event, strategy, execution, or portfolio
formulas other than the removed short gate. It does not implement any other audit
finding.

## Verification and evidence

Observed in the task worktree on 2026-11-07:

- `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`
  pass.
- `cargo test --workspace` passes: 1,312 tests, 0 failed, 8 ignored.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` passes.
- All ten Python contract checks, the structural fitness checker, and
  `git diff --check` pass.
- `portfolio_rejects_the_removed_allow_short_field` passes: a `portfolio/v2`
  config carrying `allow_short` is rejected as an unknown field.
- `replay_accepts_a_short_fill` passes: a short fills bundle is accepted, the
  first trade in `trades.csv` has `side=short`, and the portfolio provenance no
  longer contains `allow_short`.
- The analytics and qualification golden fixtures were regenerated deliberately
  because the portfolio config bytes changed the run config hash, the backtest
  manifest hash, and the stage provenance hashes. The numeric results and table
  rows are unchanged.

## Learning and corrections

- Keeping `allow_short` as a deprecated, ignored field was rejected: it would
  leave the rule in the configuration surface and allow a caller to believe the
  switch still works. The field was removed and the schema moved to `v2`.
- Updating the historical `config/research/` configs was rejected: they are
  evidence of the prior contract and are not loaded by the current shell.
  `config/README.md` records that boundary.
- The old short-rejection unit test was replaced with positive coverage rather
  than deleted without replacement: the binary replay test and the unknown-field
  test cover the new behavior.
- The portfolio provenance change is an artifact change, not only a code change;
  the analytics and qualification golden identities moved with the run config
  hash. The numeric artifacts did not change.

## Limits

- One sequential implementation and self-verification pass; no independent
  Code Reviewer, Quant Validator, or human review is claimed.
- The historical research configs and historical work-item documents retain the
  term `allow_short` as evidence. They are not current contracts.
- No commit, push, pull request, or merge was performed.
