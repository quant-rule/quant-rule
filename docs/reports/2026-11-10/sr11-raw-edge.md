# SR11 Raw-Edge Diagnostic

- Date: 2026-11-10 (UTC)
- Status: **Complete — no positive raw edge; stop the EMA/Fibonacci line**
- Scope: `strategy_dev` 2018-01-01..2022-12-31 only; 2024 closed
- Tool: [`tools/research/raw_edge.py`](../../../tools/research/raw_edge.py)
- Output: `data/improvement/2026-11-10/SR11/raw-edge/`
- Claim type: diagnostic; not a strategy result.

## 1. Question

SR11's bracket failed, but the 5% halt truncated every year. Before changing
sizing, measure whether the entry has any raw forward edge independent of the
stop, target, halt, and position state.

## 2. Method

The signal is reconstructed from the model bundle, not from the strategy's
position-aware decisions:

- Fibonacci test: `event_type 1`, `subtype 1`, `source 56`, direction ±1;
- EMA test: `event_type 1`, `subtype 1`, `source 1`, either direction;
- `ema_stack` and `structure_state` aligned with the Fibonacci direction;
- all four conditions on the same 15m bar;
- entry at the next 15m bar open.

Forward measurements: 1, 3, 5, 10, and 20 bars; MFE/MAE in ATR; a 3 ATR target
against a 1 ATR stop over 20 bars, with stop first on a same-bar tie. Costs are
6 bps round trip. No stop, target, halt, or position state enters the signal.

Controls: the two events without the states, the states without the events, and
every bar. Non-overlapping full signals use a 20-bar gap.

## 3. Results

Full signal, all 2018–2022 signals:

| Side | Signals | 20-bar net | 20-bar hit | Target | Stop | Timeout | Mean R |
|---|---:|---:|---:|---:|---:|---:|---:|
| Long | 2,338 | −0.0502% | 48.1% | 20.2% | 70.1% | 9.7% | −0.0075 |
| Short | 2,269 | −0.0188% | 47.0% | 20.1% | 69.4% | 10.5% | −0.0212 |

Non-overlapping full signals:

| Side | Signals | 20-bar net | 95% CI | Mean R | 95% CI |
|---|---:|---:|---|---:|---|
| Long | 866 | −0.0509% | −0.1585, 0.0568 | −0.0202 | −0.1284, 0.0880 |
| Short | 833 | −0.1023% | −0.2466, 0.0420 | −0.0157 | −0.1248, 0.0934 |

Both intervals include zero. The point estimates are negative.

Controls, 20-bar net and mean R:

| Variant | Signals long | Signals short | Net long | Net short | Mean R long | Mean R short |
|---|---:|---:|---:|---:|---:|---:|
| Full | 2,338 | 2,269 | −0.0502% | −0.0188% | −0.0075 | −0.0212 |
| Events only | 14,522 | 14,952 | −0.0172% | −0.0640% | +0.0623 | +0.0246 |
| States only | 25,359 | 23,390 | −0.0351% | −0.0606% | +0.0036 | +0.0249 |
| All bars | 151,166 | 151,166 | −0.0400% | −0.0800% | +0.0195 | +0.0296 |

The full signal does not beat the event-only, states-only, or unconditional
control on mean R. No variant has a positive 20-bar net return at 6 bps.

## 4. Findings

1. **No raw edge.** Gross 20-bar return is +0.0098% long and +0.0412% short;
   after 6 bps the result is negative. The hit rate is below 50%.
2. **The bracket geometry is wrong for the path.** Average 20-bar MFE is 2.72
   ATR long and 2.84 ATR short, below the 3 ATR target. Average MAE is 2.91 ATR
   long and 2.49 ATR short, far beyond the 1 ATR stop. The target sits beyond the
   typical favorable excursion and the stop sits inside the typical adverse one.
3. **The target rate is below the 3:1 breakeven.** Targets are reached in about
   20% of signals, below the 25% rate a 3:1 bracket needs before costs when
   timeout outcomes are ignored. The stop rate is about 70%.
4. **The state filter does not add value.** The full conjunction is worse than
   the event-only control on both sides. It is also worse than the unconditional
   base rate on mean R. The `ema_stack`/`structure_state` filter is not selecting
   a better subset.
5. **The 5% halt was not hiding an edge.** The raw signal is already flat to
   negative before any halt. A 0.5x sizing diagnostic would only scale that
   result; it would not create an edge.

## 5. Decision

Stop the EMA/Fibonacci line. Do not run the 0.5x sizing diagnostic; do not
promote the event-only control; do not retune SR11. The next hypothesis must
start from a different entry premise or a different confirmation, with its own
pre-registration.

2024 remains closed. No baseline and no edge claim.

## 6. Limitations

- One instrument (Dukascopy BTCUSD 15m) and one development partition.
- The raw-edge tool models bracket outcomes at the target/stop price, not gap
  fills; the execution engine's fills remain the authoritative result.
- Overlapping signals are included in the full-signal row; the non-overlapping
  row is the approximately independent view.
- The confidence intervals are naive per-signal intervals on non-overlapping
  windows, not a block bootstrap.
- This is a diagnostic, not a validation run.

## 7. Reproduction

```sh
DATA=${DATA:-data}
python3 tools/research/raw_edge.py \
  --model "$DATA/improvement/2026-11-10/SR11/artifacts/sr11-1/2018/6bps/backtest/model" \
  --model "$DATA/improvement/2026-11-10/SR11/artifacts/sr11-1/2019/6bps/backtest/model" \
  --model "$DATA/improvement/2026-11-10/SR11/artifacts/sr11-1/2020/6bps/backtest/model" \
  --model "$DATA/improvement/2026-11-10/SR11/artifacts/sr11-1/2021/6bps/backtest/model" \
  --model "$DATA/improvement/2026-11-10/SR11/artifacts/sr11-1/2022/6bps/backtest/model" \
  --out "$DATA/improvement/2026-11-10/SR11/raw-edge" \
  --cost-bps 6 --horizons 1,3,5,10,20 --target-atr 3 --stop-atr 1 --barrier-bars 20
```
