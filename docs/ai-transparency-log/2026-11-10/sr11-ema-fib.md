# AI Transparency Log — SR11 Trend-Aligned EMA/Fibonacci Entry

- Date: 2026-11-10 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash
- Work item: [`SR11-CHARTER`](../../governance/2026-11-10/SR11-CHARTER.md)
- Result: [`SR11`](../../improvement/2026-11-10/SR11.md)
- Claim type: strategy research; no edge claim.

## Intent and boundary

SR10 was stopped early because its 2018 primary giveback cell failed the PF and
drawdown criteria. This entry records the new SR11 hypothesis: the same-bar
EMA/Fibonacci entry plus `ema_stack` and `structure_state` alignment. It is a new
hypothesis, not a post-hoc change to SR10. The human approved the premise,
development access, and the development-only `M1.2` exception; 2024 stayed
closed.

## Implemented

- SR10 result record `docs/improvement/2026-11-10/SR10.md`.
- SR11 charter `docs/governance/2026-11-10/SR11-CHARTER.md`.
- Four strategy configs: long/short × structure/force-loss exits.
- 15m execution, 1-bar event window, 1 ATR stop, 3 ATR target, 50-bar time stop.
- Feasibility preflight at 6 and 9 bps: pass.
- Full 40 year-cell matrix: four cells × 2018–2022 × 6/9 bps.

## Runtime change

The first 2018 cell took 8m47 because the run computed the 1m and 15m models.
The AI proposed restricting the model ladder to 15m with
`"model_timeframes": ["15m"]`, leaving the 1m source and data version unchanged.
The human did not request this change; it is a run-configuration optimization,
not a strategy change.

Verification: for SR11.1 2018, the 15m rows of `readings.csv`, `events.csv`,
`levels.csv`, and `legs.csv` were byte-identical to the full-ladder SR10 2018
model. Runtime fell to 13–22s per year-cell. The restriction is therefore
behavior-neutral for the 15m strategy.

## Evidence

- SR10 2018: long giveback 19 trades, PF 0.939, DD 5.31%; long structure 33
  trades, PF 1.039, DD 5.45%. Both failed.
- SR11 preflight 2018: 6 bps breakeven 0.287; 9 bps 0.306; both pass.
- SR11 full matrix, 6 bps:

| Cell | Trades | PF | Payoff | Win rate | Max DD | Halted |
|---|---:|---:|---:|---:|---:|---:|
| SR11.1 long structure | 123 | 0.680 | 2.31 | 22.8% | 5.91% | 5/5 |
| SR11.2 short structure | 86 | 0.692 | 2.14 | 24.4% | 7.03% | 5/5 |
| SR11.3 long force | 174 | 0.597 | 1.61 | 27.0% | 5.33% | 5/5 |
| SR11.4 short force | 111 | 0.453 | 0.98 | 31.5% | 7.86% | 5/5 |

- At 9 bps the PF range is 0.444–0.688; every cell is negative.
- Realized payoff is below the designed 3:1 in every cell, and every cell's win
  rate is below the breakeven rate implied by its realized payoff.
- Every year-cell halted at the 5% drawdown limit.

## Raw-edge diagnostic

The human approved a raw-edge diagnostic before any sizing change. The AI added
`tools/research/raw_edge.py` (stdlib only, deterministic) and
`tools/research/test_raw_edge.py` (10 unit tests). The tool reconstructs the
entry from the model bundle, enters at the next bar open, and measures forward
returns, MFE/MAE, and a 3:1 barrier with no stop, target, halt, or position
state. Controls are events only, states only, and every bar.

The diagnostic ran on the five 2018–2022 model bundles:

| Side | Signals | 20-bar net | 20-bar hit | Target | Stop | Mean R |
|---|---:|---:|---:|---:|---:|---:|
| Long | 2,338 | −0.0502% | 48.1% | 20.2% | 70.1% | −0.0075 |
| Short | 2,269 | −0.0188% | 47.0% | 20.1% | 69.4% | −0.0212 |

Non-overlapping 95% intervals include zero, and the point estimates are
negative. The full signal does not beat the event-only, states-only, or
unconditional controls on mean R. Targets are reached in about 20% of signals,
below the 25% breakeven for a 3:1 bracket. Average 20-bar MFE is 2.72 ATR long
and 2.84 ATR short, below the 3 ATR target; average MAE is 2.91 and 2.49 ATR, far
beyond the 1 ATR stop.

The AI recommended stopping the EMA/Fibonacci line and not running the 0.5x
sizing diagnostic. The human's approval covered the raw-edge diagnostic; the
stop decision is recorded in the SR11 decision record.

## Learning

- SR10's 3-bar window allowed repeated entries in a cluster; the giveback exit
  underperformed the structure control.
- SR11's stricter same-bar and trend-alignment filter did not fix the edge. The
  structure exit is the better base, but its payoff is about 2.2 and its win rate
  about 23–24%, both below the required combination.
- The force exit is clearly worse: it cuts winners and collapses payoff.
- The 5% hard halt and full-notional 1 ATR stop bind in every year. The halt
  truncates the sample, but the pre-halt evidence is already below breakeven, so
  sizing alone would not create an edge.
- The hypothesis is stopped, not retuned.
- The raw edge is flat to negative and does not beat a random bar. The state
  filter is not adding value: the full conjunction is worse than the event-only
  control. The 3 ATR target sits beyond the average favorable excursion and the
  1 ATR stop sits inside the average adverse one.
- A 0.5x sizing run would only scale a zero/negative edge; it was not run. The
  EMA/Fibonacci line is stopped.

## Verification

- `cargo fmt`, `cargo clippy --all-targets`, and `cargo test` were not rerun for
  this record because no Rust code changed.
- `cd tools/research && python3 -m unittest test_raw_edge.py`: 10 tests, all
  pass.
- Raw-edge diagnostic: completed over five 2018–2022 model bundles; output under
  `data/improvement/2026-11-10/SR11/raw-edge/`.
- All 40 `backtest run` invocations exited 0 and wrote a manifest.
- 2024 was not accessed.

## Not claimed

- No strategy result, edge, or validation.
- No baseline.
- No change to SR10 or its result.
