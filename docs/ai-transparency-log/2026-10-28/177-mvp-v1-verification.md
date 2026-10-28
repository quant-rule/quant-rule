# AI Transparency Log — MVP v1 Verification

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#177](https://github.com/quant-rule/quant-rule/issues/177)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `test/177-mvp-v1-verification` in `quant-rule--177-mvp-v1-verification`
- PR: pending
- Status: Verification complete; two criteria unresolved; PR not yet open.

## Intent and boundary

B4 evaluates the MVP v1 acceptance criteria against what the release binary
actually wrote, not against the code's intent. It runs the full-year SOLUSD 2025
config twice, compares every artifact, and records coverage, the portfolio
result, the execution counts, the analytical layer counts, and the three
boundary checks. It changes no engine, no command, and no report format.

The only repository change is documentation: the verification report, this log,
and the plan's status and evidence sections. The measurement artifacts live under
`data/`, which is gitignored.

## Work performed

1. Built the release binary and ran the full-year config twice into separate
   roots, recording wall time and peak RSS with `/usr/bin/time -v`.
2. Compared the two roots with `diff -r` and `sha256sum`, and compared the
   printed `--json` output of two runs that used the same output root.
3. Ran `report portfolio` into a fresh root and confirmed it created only
   `portfolio/`.
4. Ran `market-report` for the same data and period and compared its four
   artifacts with the model bundle byte for byte.
5. Ran `strategy-run` over the full year and scanned it for fill, position,
   cash, equity, P&L, quantity, commission, and funding fields.
6. Ran a config that names a missing source and confirmed the failure left no
   root behind.
7. Re-ran `fmt`, `clippy`, `test`, and `doc`, and the ledger and config suites.
8. Updated the plan's acceptance table with the observed evidence, marked it
   Verified, and recorded the two unresolved criteria.

## Findings

- **The 2 GiB peak-RSS budget is not met.** The full-year run peaks at
  3,890,928 KiB and 4,032,920 KiB on the two runs, with wall time inside budget
  at 29.14 s and 28.90 s. The plan explicitly allows the analytical pass to be
  split out when it does not fit, so this is a known, named gap rather than an
  unknown one. Filed as [#189](https://github.com/quant-rule/quant-rule/issues/189)
  and the release is blocked until it is met or a higher budget is approved.
- **The analytical bundle ignores `analytics.json.timeframes` since B3.**
  `build_layers_from_run` builds layers for every timeframe in the model run, so
  a configured subset would still be reported for the whole ladder. The example
  config lists all four timeframes, so the MVP run's numbers are unaffected, and
  every artifact remains consistent with itself. It is a config-contract defect,
  and it is in #189's scope because that work rewrites the analytical pass.
- The full-year run halts in January at a 20.315% drawdown. The remaining eleven
  months hold a flat equity, so their monthly returns are a measured `0`; this is
  the latching halt working, not a missing measurement.
- 2,258 of 2,354 orders are rejected: 1,133 `no_position` (the strategy's exit
  after a bracket already closed the position) and 1,125 `missing_quantity` (the
  halt blocks new entries after January). Both are the contract's recorded
  behavior, and `pending_at_end` is 0.

## Decisions and corrections

- **A rejected order is evidence, not a failure.** The strategy keeps its own
  position state, so a bracket exit leaves it thinking a position is open and it
  later emits an exit; execution records `no_position`. B4 reports the count
  rather than treating the run as broken. Changing the strategy's state model is
  outside this work item.
- **The verification report is a new document.** The plan's acceptance table has
  the criterion-by-criterion evidence, but the measurements and the method need
  a home that is not the plan. `docs/reports/2026-10-28/mvp-v1-verification.md`
  holds them; the plan's table links the work item and the observed values.
- **The printed-output check uses one output root.** Comparing two runs with
  different roots would compare paths, not output. Two runs to the same root,
  with `cmp` on stdout, is the contract's case.
- **The plan is not marked fully verified.** Two criteria are open (RSS and the
  tag), and the status line says so. B4 records the gap instead of rounding it
  up.
- The first attempt at the full-year run failed because the worktree has no
  `data/` directory. `data/` is gitignored and not part of the worktree, so a
  symlink to the main checkout was created for the measurements and excluded
  locally through `git rev-parse --git-path info/exclude`, leaving `.gitignore`
  untouched.

## Verification and evidence

Commands were run in `quant-rule--177-mvp-v1-verification` on 2026-10-28. The
numbers are in the verification report; the table below is the command-level
summary.

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,346 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo doc --no-deps` | Passed; exit 0; no warning |
| `backtest` twice, `diff -r` | Identical |
| `sha256sum` over 29 files | Identical |
| `cmp` of two `--json` runs, same root | Identical |
| `cmp` of the four model artifacts vs `market-report` | Identical |
| `report portfolio` in a fresh root | Created only `portfolio/` |

## Learning

- Determinism has two surfaces: the files and the printed output. Only the
  second one catches a path, an ordering, or a formatting drift in stdout.
- A budget is a criterion like any other. The run is fast enough and correct,
  but 3.9 GiB is not 2 GiB, and the honest state is "unresolved with a work
  item".
- The full-year result is dominated by one January drawdown: the halt then
  freezes the account for eleven months. That makes the monthly returns table
  mostly zeros, which is easy to misread as missing data; the report says why.
