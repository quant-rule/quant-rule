# MVP v1 — Verification Report

- Date: 2026-10-28 (UTC)
- Issue: [#177](https://github.com/quant-rule/quant-rule/issues/177)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md) §9
- Status: Verified — 2026-10-28; re-verified 2026-10-29 after [#189](https://github.com/quant-rule/quant-rule/issues/189) and [#196](https://github.com/quant-rule/quant-rule/issues/196). Both criteria that were unresolved on 2026-10-28 are now closed; see §7.

## 1. Method

The release binary (`cargo build --release -p quantrule`) was run against the
checked-in SOLUSD 2025 example config. Nothing was patched to make a check pass:
every number below is copied from the artifacts the binary wrote. The data file
is `data/SOLUSD_M1_2025-01-01_2025-12-31.csv`, version
`dukascopy-bid-solusd-m1-2025-01-01_2025-12-31`.

## 2. Full-year run

| Run | Root | Wall time | Peak RSS | Exit |
|---|---|---|---|---|
| A | `data/verification/run-a` | 29.14 s | 3,890,928 KiB | 0 |
| B | `data/verification/run-b` | 28.90 s | 4,032,920 KiB | 0 |

These are the 2026-10-28 measurements, before #189. They are retained as the
historical record; the current `main` is re-measured in §7.1. On 2026-10-28 the
wall time was inside the 60 s budget, the peak RSS was not inside the 2 GiB
budget, and that criterion was tracked as [#189](https://github.com/quant-rule/quant-rule/issues/189).

Coverage and period, from `model/summary.txt`:

| Timeframe | Bars | Expected | Missing | Share | First | Last |
|---|---|---|---|---|---|---|
| 1m | 522,633 | 524,214 | 1,581 | 0.9970 | 2025-01-01T14:05Z | 2025-12-31T14:58Z |
| 5m | 104,364 | 104,842 | 478 | 0.9954 | 2025-01-01T14:05Z | 2025-12-31T14:50Z |
| 15m | 34,664 | 34,946 | 282 | 0.9919 | 2025-01-01T14:15Z | 2025-12-31T14:30Z |
| 1h | 8,535 | 8,735 | 200 | 0.9771 | 2025-01-01T15:00Z | 2025-12-31T13:00Z |

Portfolio result, from `portfolio/summary.txt`:

| Field | Value |
|---|---|
| `initial_cash` | 10,000 |
| `final_equity` | 8,016.143909900684 |
| `total_return` | −0.19838560900993163 |
| `realized_pnl` | −1,983.85609009931 |
| `gross_pnl` | −1,096.236088911805 |
| `commission` | 885.6104992983051 |
| `funding_paid` / `funding_received` | 4.0034852442 / 1.993983355 |
| `max_drawdown` | 0.20315025809884202 |
| `max_drawdown_duration_bars` | 34,462 |
| `trades` / `wins` / `losses` | 48 / 9 / 39 |
| `profit_factor` | 0.2971484365808004 |
| `exposure` | 0.0023544132639136315 |
| `halted` / `open_position` | true / null |

The sample bracket is `fee_rate: 0.0`, so `execution_fees` is 0 and commission
alone carries the transaction cost. `realized_pnl + unrealized_pnl = net_pnl`
holds, and `final_equity = initial_cash + net_pnl` holds to the printed digits.

The run halts on 2025-01-31 at a 20.315% drawdown, above the 0.20 limit. The
halt latches: the remaining eleven months hold the flat equity, so their monthly
returns are a measured `0` rather than a missing value, and the position is
`null`. `max_drawdown_duration_bars` keeps counting because the equity never
recovers to its peak.

Execution result, from `execution/summary.txt`:

| Field | Value |
|---|---|
| `orders` | 2,354 |
| `filled` / `rejected` / `pending_at_end` | 96 / 2,258 / 0 |
| `fills` / `entries` / `exits` | 96 / 48 / 48 |
| `fill_reasons` | entry 48, exit 40, stop 3, target 5 |
| `rejections` | `missing_quantity` 1,125, `no_position` 1,133 |
| `slippage` | total 3.994526343140028, average 0.04160964940770862 |
| `fees` | 0 |

Analytical result, from `analytical/summary.txt`: `layer1_rows` 670,196,
`layer2_rows` 670,196, `layer3_rows` 522,633, plus 16 axis rows and 24 relation
rows in the two CSVs.

## 3. Reproducibility

| Check | Command | Observed |
|---|---|---|
| Tree identity | `diff -r data/verification/run-a data/verification/run-b` | No output |
| File hashes | `sha256sum` over the 29 files, roots normalized | Identical |
| Printed output | Two runs to the same `--out`, `cmp` of stdout | Identical |
| Artifact count | `find … -type f \| wc -l` | 29 |

The two roots differ only in their path, so the first two checks compare file
content. The stdout check uses one output root for both runs, which is the
contract's "same command, same data, same output" case.

## 4. Bundle availability

| Check | Observed |
|---|---|
| `backtest` artifacts per bundle | model 5, strategy 5, execution 7, portfolio 7, analytical 5 |
| Fresh-root `report portfolio` | Created only `portfolio/`, with the seven contract files |
| `report model` vs `market-report` | `summary.txt`, `report.json`, `price-axes.png`, `events.png` byte-identical |
| `strategy-run` fields | 2,346 NDJSON lines; no fill, cash, equity, P&L, quantity, commission, or funding field |
| Config failure | Missing source exited 1 and left no run root |

## 5. Engine-level evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-backtest portfolio::ledger_tests` | 12 passed, 0 failed |
| `cargo test -p quantrule-backtest portfolio::tests` | 48 passed, 0 failed |
| `cargo test -p quantrule-shell config::tests` | 12 passed, 0 failed |
| `cargo test` | 1,346 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo fmt --all -- --check` | Passed, no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed, no warning |
| `cargo doc --no-deps` | Passed, no warning |

The ledger tests cover the commission base, funding at an aligned boundary, the
latching halt, equity-fraction sizing, the quantity step, and the golden trade
and equity fixtures. The config tests cover every documented rejection. The full
suite covers the five bundles, the report boundaries, determinism, and causality.

## 6. Criteria that were unresolved on 2026-10-28

| Criterion | State on 2026-10-28 | Resolution |
|---|---|---|
| 2 GiB peak RSS | Unresolved: 3.71–3.85 GiB measured. [#189](https://github.com/quant-rule/quant-rule/issues/189) | Resolved by #189; re-measured at 900,300 / 900,816 KiB on 2026-10-29 (§7.1) |
| `v0.2.0` tag on `main` | Pending V2, blocked by the criterion above | Done: tag `v0.2.0` and release “MVP v1” exist (§7.6) |

The plan's §9.5 fallback — run the analytical pass after the pipeline is
released — was a production change, not a verification step, so B4 recorded the
measurement and filed the work item rather than patching the runner here. #189
applied that fallback.

## 7. Post-release re-verification — 2026-10-29

This section verifies `main` at `6c1e705`, which includes #189 and #196. The
`v0.2.0` tag points to commit `42e0021` (annotated tag object `ef844f6`) and does
not include #196; #196 is a post-release fix intended for the next minor
release. The measurements below are from the current `main`, not from the tagged
commit.

### 7.1 Full-year run

| Run | Root | Wall time | Peak RSS | Exit |
|---|---|---|---|---|
| A | `/tmp/opencode/verify177-a` | 21.08 s | 900,300 KiB | 0 |
| B | `/tmp/opencode/verify177-b` | 21.37 s | 900,816 KiB | 0 |

Both runs are inside the 60 s and 2 GiB budgets. The increase over the pre-#196
baseline measurement (811,512 KiB) is the per-bar published Progression leg
record added by #196.

Coverage and the portfolio and execution results are unchanged from §2. The
analytical layer counts are unchanged; its Progression axis values differ as
described in §7.7.

### 7.2 Reproducibility

| Check | Observed |
|---|---|
| `diff -rq` between runs A and B | No output |
| `sha256sum` over the 29 files | Identical; 29 hashes |
| Two runs to the same `--out`, `cmp` of stdout | Identical |
| Artifact count | 29 |

### 7.3 Bundle availability and boundaries

| Check | Observed |
|---|---|
| `backtest` artifacts per bundle | model 5, strategy 5, execution 7, portfolio 7, analytical 5 |
| PNG artifacts | 6 valid PNGs: model 1280×960 and 1280×720; strategy 1280×720; execution 1280×720; portfolio two 1280×480 |
| Fresh-root `report portfolio` | Created only `portfolio/`, with the seven contract files |
| Non-empty run root | `backtest` exited 1 with `output directory ... must be absent or empty`; the existing file was untouched |
| Missing source | Exited 1 with `cannot open ...: No such file or directory`; no run root was created |

### 7.4 Model compatibility

`report model --config ... --out ...` and `market-report <csv> --data-version ...
--from 2025-01-01 --to 2025-12-31 --out ...` produced byte-identical
`summary.txt`, `report.json`, `price-axes.png`, and `events.png`. The period
arguments are required for the comparison because the model bundle carries the
configured requested period and the legacy command defaults to `full file`.

### 7.5 Strategy boundary

`strategy-run` printed 2,346 NDJSON lines. The union of keys is `bar`,
`datetime`, `direction`, `momentum_force`, `reason`, `side`, `side_label`,
`strategy`, `structure_score`, `timeframe`, `timeframe_secs`, and `type`; none
of the fill, position, cash, equity, P&L, quantity, commission, or funding keys
appears.

### 7.6 Checks and release

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,350 passed, 0 failed, 6 ignored |
| `cargo doc --no-deps` | Passed; no warning |
| Workspace version | `0.2.0` |
| `v0.2.0` tag | Commit `42e0021` (annotated tag `ef844f6`); release title “MVP v1” |

### 7.7 Effect of #196

The strategy, execution, and portfolio bundles are byte-identical to the
pre-#196 baseline. The model and analytical bundles differ, as intended: #196
publishes the newest origin-confirmed Progression leg and adds the continuous
`progression_progress` chart field. A field-level diff of the model report found
changes only in the Progression axis statistics, the Progression chart fields,
and the latest Progression leg; the analytical axes changed only in the four
Progression rows, and the relations only in the 12 Progression pairs. On the
SOLUSD 2025 H1 series the stage is
available on 3,911 of 8,535 bars (45.8%), up from 52 bars (0.6%). The remaining
4,624 null stages are 4,589 invalid denominator, 27 unstable target, 4 no
target, and 4 unconfirmed origin. The three Progression goldens are temporarily
not independent of the implementation; #197 rebuilds the replay.

## 8. Limitations

- The 2026-10-29 re-verification measures `main` after #196, not the `v0.2.0`
  tag. The tag's own verification is §2–§5.
- The Progression goldens are regenerated from the implementation until #197
  restores the independent replay. The fixture headers say so.
- The memory and wall-time figures are from one machine and one input; they are
  evidence for the budget, not a general performance guarantee.
