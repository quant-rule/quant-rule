# W1 — Out-of-Sample Evidence Validation

- Date: 2026-10-31 (UTC)
- Issue: #280
- Finding: AUD-2026-10-31-31
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W1)
- Validator: Quant Validator role, sequential pass by the implementation agent (not independent)
- Claim type: evaluation-evidence correctness; no strategy edge claim

## Claim

`evidence.out_of_sample` passes only when `oos/portfolio/report.json` is
readable and records a `data_version` different from the main lab's
`portfolio/report.json`. `lab summary` and `lab diagnose` use the same
comparison. A same-version, unreadable, or absent `oos/` lab is `unknown`, not
`pass`.

## Reproduction

| Command | Observed |
|---|---|
| `cargo test -p quantrule-analytics --test diagnosis` | 4 passed, 0 failed |
| `rm -rf target/report-tests && cargo test -p quantrule-shell --test diagnosis` | 5 passed, 0 failed |
| `cargo test -p quantrule-shell --test lab_summary` | 18 passed, 0 failed |
| `cargo test --workspace` | 1506 passed, 0 failed, 7 ignored |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |

The shell test `lab_diagnose_requires_a_distinct_oos_data_version` creates a lab
with `backtest --diagnose`, copies `portfolio/report.json` to
`oos/portfolio/report.json`, runs `lab diagnose`, and reads the
`evidence.out_of_sample` status from `diagnosis.json`:

- same `data_version` -> `unknown`;
- changed `data_version` -> `pass`.

The `lab summary` test asserts the same-version text on the first run and the
distinct-version text after changing the copied report and rerunning with
`--force`. The analytics test covers the fourth state (unreadable) and the
absence state.

## Checks

- Look-ahead bias: not applicable. The check runs after the backtest and reads
  the recorded lab; it does not feed a rule.
- Data snooping and costs: not applicable. This is not a strategy result and no
  threshold was tuned.
- Signal path: the diagnosis is evaluation evidence; the strategy, execution,
  and portfolio engines never read it.
- Determinism: the comparison is a pure read of two JSON documents; the tests
  run over fixed fixtures.

## Residual risk

- The check compares the input-series identity, not the OOS period. A distinct
  `data_version` whose period overlaps the main lab still passes. The audit's
  option B (require a valid OOS report and recorded OOS period) was not
  selected; if period overlap matters, it needs a separate decision.
- The validator pass is sequential, not independent. The human remains the
  final acceptance authority.
