# AUD-06 Step Precision Regression

- Date: 2026-10-30 (UTC)
- Status: **Verified** — merged as `a006a17`; independent closure pass reproduced the small and non-grid step cases.
- Baseline: `origin/main` at `d9ada57`
- Finding: `AUD-2026-10-30-06`

## Intent

Remove the fixed twelve-decimal quantity normalization introduced by the AUD-06
fix. It over-sizes or rejects valid quantities whose step is below `1e-12` or
not on a twelve-decimal grid. Normalize at the configured step's own decimal
precision instead.

## Scope

- `crates/quantrule-backtest/src/portfolio/sizing.rs`
- `crates/quantrule-backtest/src/portfolio/tests.rs`
- The AUD-06 audit detail and the quantity-step contract text.
- No command, API, or dependency change.

## Acceptance

- `notional=5e-13, step=1e-13` returns `5e-13`.
- `notional=4e-13, step=1e-13` returns `4e-13`.
- `notional=5e-13, step=2.5e-13` returns `5e-13`.
- A quantity just below one step still rejects; a quantity above one step floors
  to the step.
- Steps at `1e-15`, `1e-18`, `1e-19`, and a true non-grid step remain valid and
  are not rounded to an integer quantity.
- The existing `0.1` boundary tests still pass.
- Full workspace tests, doctests, clippy, fmt, and doc pass.

## Verification

- Targeted `portfolio::tests` sizing tests.
- Full workspace suite and doctests.
- AUD-06 is `Verified`; independent closure pass reproduced every acceptance case.

## Evidence

- Fix commit: `f07de31`.
- Merge commit: `a006a17` (PR #225).
- PR checks: `sync` and `verify` passed; post-merge `main` CI passed.
- Full verification: 40 suites / 1,450 tests, 77 doctests, clippy, fmt, and doc clean.
- Independent closure: `5e-13/1e-13`, `4e-13/1e-13`, `5e-13/2.5e-13`,
  `1.5e-12/1.5e-12`, `5e-15/1e-15`, `5e-18/1e-18`, `5e-19/1e-19`, and a true
  non-grid step all passed; a just-below-step quantity rejected.
- Limitation: the closure verifier was a separate agent session of the same
  model family, not an independent-model judgment.
