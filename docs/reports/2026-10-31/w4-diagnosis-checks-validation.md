# W4 — Diagnosis Checks Validation

- Date: 2026-10-31 (UTC)
- Issue: #311
- Findings: AUD-2026-10-31-44, -45, -50, -68
- Plan: `docs/execution-plans/2026-10-31/full-system-audit.md` (W4)
- Validator: Quant Validator role, sequential pass by the implementation agent (not independent)
- Claim type: diagnosis evaluation evidence; no strategy edge claim

## Claim

The diagnosis now measures the largest of all six pairwise axis correlations,
names the worst pair in the check evidence, gives `unknown` checks measurement
wording rather than failure wording, writes an empty CSV cell for a missing
number, and reads the five report versions from the written bundles on
`backtest --diagnose`.

No model axis value, strategy decision, execution fill, portfolio record, or
diagnosis threshold changes. The correlations and triage are evaluation output;
no engine reads them.

## Reproduction

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --lib diagnosis::compute::tests` | 5 passed, 0 failed |
| `cargo test -p quantrule-analytics --test diagnosis` | 15 passed, 0 failed |
| `cargo test -p quantrule-shell --bin quantrule diagnosis::tests` | 1 passed, 0 failed |
| `cargo test -p quantrule-shell --test diagnosis` | 10 passed, 0 failed |
| `cargo test --workspace` | 1,543 passed, 0 failed, 7 ignored; 47 suites |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The regression tests are:

- `axis_correlation_uses_the_worst_of_six_pairs` — a crafted 0.9
  Momentum/Structure correlation is selected over lower pairs.
- `unknown_checks_do_not_assert_a_failure` — a no-trade, no-fill, no-limit
  diagnosis contains no negative-expectancy, slippage-exceeds, costs-dominate, or
  limit-breached claim, and contains the measure/record wording.
- `missing_diagnosis_values_are_empty_csv_cells` — an unknown check value and a
  null-only model-health axis render empty CSV cells.
- `report_versions_are_read_from_the_bundle_files` — five crafted
  bundle-specific versions are read back, not replaced by the binary constants.

## Checks

- Look-ahead bias: not applicable; the diagnosis runs after the completed
  backtest and the correlations use the published per-bar values in bar order.
- Survivorship: not applicable; the diagnosis reads one lab.
- Split leakage: not applicable; no model fitting or split.
- Data snooping: the check thresholds are unchanged; the fix measures the pair
  the contract always claimed.
- Costs/capacity: not applicable; no execution or portfolio behavior changes.
- Determinism: the byte-identical diagnosis test still passes; the new
  correlations and check evidence are accumulated in row order.
- Signal path: no model, strategy, execution, or portfolio engine reads the
  diagnosis.

## Limitations

- The pass is sequential, by the implementation agent; it is not an independent
  model judgment.
- The tests use deterministic fixtures and a small generated run, not live
  market data.
- The six correlations are diagnostics; a high correlation is evidence to
  inspect, not a trading rule.
