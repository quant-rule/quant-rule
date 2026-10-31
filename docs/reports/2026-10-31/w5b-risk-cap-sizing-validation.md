# W5b Risk Cap and Sizing Cause Validation

- Date: 2026-10-31 (UTC)
- Work item: #263 (W5b)
- Findings: AUD-2026-10-31-14, -26
- Plan: `docs/execution-plans/2026-10-31/audit-unified-ladder.md` (W5)
- Validator: Quant Validator (AI), sequential read-only pass
- Baseline: `main` before W5b; fixed branch `fix/263-risk-cap-sizing`
- Claim type: program correctness fix; no edge claim

## Scope

The validator checked that a risk-capped fill is a multiple of `quantity_step`,
that a cap below one step is rejected, that a short-disabled sizing failure
carries its own reason, and that the regenerated runner goldens reflect only the
intended step change.

## Reproduction

| Check | Baseline | Fixed |
|---|---|---|
| Capped entry with step 0.1 | Non-step quantity | Step multiple |
| Cap below one step | Could fill a partial step | `risk_limit` rejection |
| Short entry with `allow_short=false` | `missing_quantity` only | `short_not_allowed` in the report |
| Runner golden | Old quantities/metrics | Regenerated; decisions unchanged |

The fail-before check disabled the cap step and then the sizing-cause push. The
step test and the short-disabled test failed; with both restored, they passed.

## Commands and observed results

| Command | Observed |
|---|---|
| `cargo test -p quantrule-backtest --lib a_risk_capped_fill_respects_the_quantity_step` | Passed |
| `cargo test -p quantrule-backtest --lib a_rejected_short_is_never_an_open_strategy_position` | Passed |
| `cargo test -p quantrule-backtest --lib runner::tests::the_runner_matches_the_golden_run` | Passed; 6 trades, metrics match |
| `cargo test -p quantrule-shell --bin quantrule the_execution_summary_carries_sizing_rejections` | Passed |
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | 45 test-result-ok suites, 0 failures |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |

The runner decisions fixture is unchanged. Trade quantities now end on the
0.0001 grid; final equity moved from `9409.959422030162` to
`9409.959666975237`, and commission from `115.73367028451358` to
`115.7336718084858`. Those deltas are the intended step-flooring effect.

## Risks and limitations

- The validator and implementer are the same agent and model. This is a
  sequential pass, not an independent review.
- The full-year SOLUSD source is not committed; the evidence is the checked-in
  runner fixture and unit tests.
- The step floor changes all capped quantities and downstream equity by design.
  The decision sequence is unchanged; a future strategy result must use the new
  goldens.
- The report now carries a sizing-rejection count map; a reader that assumes the
  old schema must read the new field.

## Verdict

The evidence supports moving AUD-2026-10-31-14 and -26 to `Fixed`. It is not a
profitability or edge claim.
