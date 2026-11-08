# SR2 Charter — Dual Payoff/Timeframe Fade

- Date: 2026-11-08 (UTC)
- Status: **Approved; development complete — `no_edge`; validation not opened**
- Plan: [`sr2-dual-design.md`](../../execution-plans/2026-11-08/sr2-dual-design.md)
- Tier: strategy
- Model baseline: `MODEL-2026-11-04-04..05` (1m 4 ATR compression expansion)
- Claim type: strategy research charter; no edge or profitability claim

## 1. Premise

SR1 showed that a 4 ATR stop against a 1 ATR target needs an 80% win rate
before costs and is structurally infeasible at 1m. SR2 keeps the same fade
entry — opposite the first compression breakout — but uses a positive
reward/risk so the strategy does not need a win rate above 50%.

Two pre-registered designs are tested in parallel:

| Design | Timeframe | Stop | Target | Breakeven at 9 bps, worst instrument |
|---|---|---:|---:|---:|
| A | 1h | 1 ATR | 3 ATR | 42.6% (EUR) |
| B | 15m | 1 ATR | 4 ATR | 48.9% (EUR) |

The 9 bps figures are from the 2018 feasibility reports; the gate itself uses
the 6 bps base cost.

## 2. Strategy rule

| Item | Value |
|---|---|
| Engine | `event_composite` |
| Event | `signal.compression_breakout` `(type 3, subtype 7, source 5)` |
| Direction match | `0` (either side) |
| Entry side | Opposite the matched event (`contrarian: true`) |
| Stop | 1 ATR |
| Target | Design A: 3 ATR; Design B: 4 ATR |
| Time stop | 20 bars |
| Additional conditions | None |
| Execution | Next execution bar, existing path |

The entry is the same in both designs; only the execution timeframe and the
target differ. The two designs are one programme, not two searches.

## 3. Model baseline scope

The accepted baseline supplies the compression-breakout event definition and
the model implementation. The baseline's accepted claim is 1m. The 15m and 1h
uses are a **strategy hypothesis extension**; they are not a model promotion
and do not claim a 15m/1h model baseline. A strategy result at those timeframes
is evidence for this hypothesis only.

`signal.compression_breakout` is registered `context` and its claim is
direction-neutral. The strategy uses its direction code as the sole entry; this
is the same explicit exception recorded for SR1. The result does not promote
the event to a directional `signal`.

## 4. Data and protocol

| Item | Value |
|---|---|
| Instruments | BTCUSD, EURUSD, XAUUSD |
| Development | `strategy_dev`, 2018-01-01..2022-12-31 |
| Validation | `strategy_validation` 2024; one-shot, not opened now |
| Folds | One calendar year per run, 2018–2022 |
| Account | $10,000 per cell; fixed notional; 1x leverage; one position |
| Drawdown ceiling | 10% per cell, hard halt |
| Costs | 6 bps round trip: 2 bps commission per operation + 1 bps slippage per fill |
| Sensitivity | 9 bps round trip: 3 bps commission per operation + 1.5 bps slippage per fill |
| Cells | 2 designs × 3 instruments × 5 folds = 30 base cells |

## 5. Green criteria

Development, per design:

- total return > 0;
- profit factor > 1.10;
- max drawdown ≤ 0.10;
- at least 600 closed trades;
- at least one closed trade per trading day on average across the portfolio;
- at least 60% of trading days with a trade;
- at least 2 of 3 instruments positive;
- at least 3 of 5 folds positive;
- profit factor > 1.0 at 9 bps;
- byte-identical rerun for a passing design.

Validation (one-shot 2024, only after freeze and human approval): total return
> 0; profit factor > 1.0; max drawdown ≤ 0.10; at least 100 trades; at least
2 of 3 instruments positive.

A design that fails development is `no_edge`; the rule is not tuned. The two
designs are compared as pre-registered cells; the programme may accept at most
one candidate for validation.

## 6. Stop rule

Stop a design when the feasibility preflight fails, the base development
criteria fail, the 9 bps sensitivity fails, the machine cannot sustain the
parallel run, or the human stops it. Stop the programme when both designs
fail; do not re-register a failed design as a variant.

## 7. Roles and gates

| Role | Responsibility |
|---|---|
| Quant Researcher | Premise, dual designs, metrics, thresholds |
| Quant Developer | Configs, preflight, matrix, runs |
| Quant Validator | Preflight gate, cost and causality checks, reproduction |
| Human | Approve the designs, 2024 access, and any baseline |

Sequential same-agent passes; not independent agents.

## 8. Not authorized

- The 2024 validation before a freeze and separate human approval.
- A model promotion at 15m/1h.
- A post-hoc change to the event, side, stop, target, timeframe, cost, or
  universe.
- A live or prop-firm claim.

## 9. Result

- 30 / 30 base cells completed.
- Design A (1h, 1:3): +0.63% pooled, profit factor 1.063, drawdown 10.06%,
  541 trades, 0.31 trades/day, 1/3 instruments positive.
- Design B (15m, 1:4): -5.38% pooled, profit factor 0.637, drawdown 10.56%,
  1,452 trades, 0.82 trades/day, 0/3 instruments positive.
- Both designs fail the development criteria. The 9 bps sensitivity and the
  2024 validation were not opened; no strategy baseline is claimed.
