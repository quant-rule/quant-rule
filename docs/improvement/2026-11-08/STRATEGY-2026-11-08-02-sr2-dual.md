# STRATEGY-2026-11-08-02 — SR2 Dual Design, No Edge

- Date: 2026-11-08 (UTC)
- Status: **Development complete; `no_edge`; 9 bps not run; validation not opened; strategy tier closed**
- Tier: strategy
- Charter: [`SR2-CHARTER.md`](../../governance/2026-11-08/SR2-CHARTER.md)
- Plan: [`sr2-dual-design.md`](../../execution-plans/2026-11-08/sr2-dual-design.md)
- Run record: `data/improvement/2026-11-08/STRATEGY-2026-11-08-02/`
- Baseline: `MODEL-2026-11-04-04..05`
- Claim type: strategy research; no edge or profitability claim

## 1. Outcome

Both pre-registered designs fail the development criteria. **The programme
decision is `no_edge`; no candidate is frozen, the 9 bps sensitivity was not
run, `strategy_validation` 2024 was not opened, and the strategy tier is
closed.**

| Item | Design A | Design B |
|---|---|---|
| Timeframe / payoff | 1h, 1 ATR stop / 3 ATR target | 15m, 1 ATR stop / 4 ATR target |
| Cells | 15 / 15 | 15 / 15 |
| Pooled return | +0.63% | -5.38% |
| Profit factor | 1.063 | 0.637 |
| Max cell drawdown | 10.06% | 10.56% |
| Trades | 541 | 1,452 |
| Trades per day | 0.31 | 0.82 |
| Positive instruments | 1 of 3 | 0 of 3 |
| Positive folds | 3 of 5 | 1 of 5 |
| Verdict | fail | fail |

## 2. Protocol

| Item | Value |
|---|---|
| Entry | Opposite `signal.compression_breakout` `(3, 7, 5)`, `contrarian: true` |
| Design A | 1h, 1 ATR stop, 3 ATR target, 20-bar time stop |
| Design B | 15m, 1 ATR stop, 4 ATR target, 20-bar time stop |
| Instruments | BTCUSD, EURUSD, XAUUSD |
| Partition | `strategy_dev`, 2018-01-01..2022-12-31 |
| Folds | One calendar year per cell, 2018–2022 |
| Account | $10,000 per cell, fixed notional, 1x leverage, one position |
| Drawdown | 10% per cell, hard halt |
| Costs | 6 bps base; 9 bps sensitivity |
| Cells | 2 designs × 3 instruments × 5 folds = 30 |

The accepted model baseline supplies the compression event definition. The 15m
and 1h uses are a strategy hypothesis extension, not a model promotion. The
event is registered `context`; the sole-entry use is the same disclosed SR1
exception.

## 3. Preflight

All 30 cells pass the 6 bps feasibility gate. The 9 bps diagnostic showed
EURUSD 2019 1h at 51.3% breakeven (A) and 64.3% (B), but the gate uses the
declared 6 bps base cost; 9 bps is a development sensitivity, not the admission
rule.

## 4. Pooled results

| Metric | A | B |
|---|---:|---:|
| Trades | 541 | 1,452 |
| Net P&L | +951.67 | -8,075.24 |
| Total return | +0.006344 | -0.053835 |
| Max cell drawdown | 0.100569 | 0.105598 |
| Profit factor | 1.063441 | 0.637235 |
| Halted cells | 1 | 5 |
| Trades per day | 0.306863 | 0.820803 |
| Active-day share | 0.268293 | 0.520633 |

## 5. Criteria

| Criterion | A | B |
|---|---|---|
| Total return > 0 | pass | fail |
| Profit factor > 1.10 | fail | fail |
| Max drawdown <= 0.10 | fail (10.06%) | fail (10.56%) |
| Trades >= 600 | fail (541) | pass (1,452) |
| Trades per day >= 1 | fail (0.31) | fail (0.82) |
| Active-day share >= 0.60 | fail (0.27) | fail (0.52) |
| Positive instruments >= 2/3 | fail (1) | fail (0) |
| Positive folds >= 3/5 | pass (3) | fail (1) |
| 9 bps profit factor > 1.0 | not run | not run |

## 6. What the result says

- Design A's positive pooled return is entirely BTCUSD; EURUSD and XAUUSD are
  negative in every fold. The result is not robust across instruments.
- Design B's only positive cell is BTCUSD 2018; every other cell is negative and
  five cells halt at the 10% ceiling.
- The first BTCUSD 2018 cells looked promising (+5.3% A, +19.5% B), but the
  result did not replicate. One cell is not evidence.
- The 1 ATR stop / 3–4 ATR target passes the 50% breakeven screen but does not
  create the directional edge needed to reach that breakeven. The feasibility
  screen is necessary, not sufficient.
- The daily activity floor fails for both designs. A hard daily quota would not
  create a positive edge; it would only force more trades.
- The 9 bps sensitivity was not run because both designs failed the base
  criteria. Running it would not change the decision.

## 7. Freeze and validation

No candidate is frozen. The 2024 `strategy_validation` partition was not
opened. A failed design is not re-registered as a variant with another stop,
target, timeframe, or filter.

## 8. Quant validation

- 30 / 30 preflight cells pass at 6 bps.
- 30 / 30 base cells complete; 30 per-cell qualification records.
- The reduced model ladder (execution timeframe only) matches the full-ladder
  A/B BTCUSD 2018 metrics exactly; the model output at the selected timeframe is
  unchanged.
- The 6 bps cost identity is 2 bps commission per operation + 1 bps slippage per
  fill; the 9 bps pair is 3 bps + 1.5 bps.
- The Quant Developer and Quant Validator passes are sequential same-agent
  passes, not independent agents.
- The 9 bps sensitivity and byte-identical rerun were not evaluated because the
  base failed.

## 9. Limitations

- The 15m/1h model use is a strategy extension beyond the accepted 1m baseline;
  it is not a model promotion.
- `signal.compression_breakout` is registered `context`; the sole-entry use is a
  disclosed exception.
- The parallel run was restarted by the environment; the final run used a
  detached, resumable process and a reduced model ladder.
- No 2024 evidence exists at development time.

## 10. Links

- [SR2-A BTCUSD robustness audit](SR2-BTC-AUDIT.md) — the BTC lead is fragile
  and concentrated; no candidate.

- [Charter](../../governance/2026-11-08/SR2-CHARTER.md)
- [Plan](../../execution-plans/2026-11-08/sr2-dual-design.md)
- [AI transparency log](../../ai-transparency-log/2026-11-08/sr2-dual-design.md)
- [Feasibility preflight](../../../tools/improvement/feasibility.py)
- [Baselines](../baselines.md)
- [Run index](../index.md)
