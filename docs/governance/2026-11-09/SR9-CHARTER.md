# SR9 Charter — 15m Structure-Transition Campaign

- Date: 2026-11-09 (UTC)
- Program ID: `PROGRAM-2026-11-09-07`
- Status: **Stopped — primary cells underpowered; no candidate**
- Plan: [`sr9-15m-structure.md`](../../execution-plans/2026-11-09/sr9-15m-structure.md)
- Tier: strategy
- Model: current model with `vol_regime`, `structure_state`, and `signal.breakout_retest_hold`
- Claim type: strategy research; no edge claim

## 1. Premise

A 15m structure-transition strategy can capture the displacement between an old
and a new structure: enter when a compression breakout is confirmed by a retest
that holds, filter by confirmed structure, trend alignment, momentum, and normal
volatility, and exit when the structure flips.

The premise is tested on BTCUSD, XAUUSD, and EURUSD. It is not a search for a
passing configuration.

## 2. Economic rationale

The trade spans the transition rather than one 15m candle, so the gross move can
be larger than the round-trip cost. The retest is intended to reduce false
breaks. The structure flip is the exit, with a 1 ATR stop and a 50-bar time stop
as backstops.

## 3. Frozen inputs

| Field | Value |
|---|---|
| Tier | strategy |
| Changed | strategy evaluation only |
| Frozen | data, policy, model, execution, portfolio |
| Instruments | BTCUSD, XAUUSD, EURUSD |
| Timeframe | 15m execution and 15m context |
| Data partition | `strategy_dev` 2018–2022, per instrument-year |
| Costs | 6 bps base; 9 bps sensitivity |
| Account | $10,000 per cell; fixed notional; 1x leverage; one position |
| Drawdown | 5% per cell, hard halt |
| Validation | 2024; not opened |

## 4. Cells

Primary cells: 3 instruments × 2 sides.

Long entry:

- `signal.breakout_retest_hold` direction +1;
- `structure_state = +1`;
- `ema_stack = +1`;
- `macd_state = +1`;
- `vol_regime = 1`.

Short: mirror.

Exit: `structure_state` against the position; 1 ATR stop; 50-bar time stop;
6:1 ATR target as a backstop.

Control: the same rule with `signal.compression_breakout` (the break-only event)
instead of the retest event. The control is descriptive and is not a candidate.

## 5. Criteria

Each primary cell:

- at least 30 trades over 2018–2022;
- PF > 1.1 at 6 bps;
- PF > 1.0 at 9 bps;
- max drawdown ≤ 5%;
- no halt;
- at least 2 of 3 instruments positive per side.

The 9 bps run is a sensitivity check; it is not a second chance.

## 6. Multiple testing

Six primary cells; twelve planned trials (six base, six sensitivity). Bonferroni
correction at family alpha 0.05. Controls and ablations are not candidates.

## 7. Stop rule

Stop when a primary cell is underpowered, a primary cell fails the criteria, the
data coverage is inadequate for a cell, or the human stops the programme. A
stopped programme does not promote a control and does not re-register a variant
with the same premise.

## 8. Human approval

- Approved by: human, 2026-11-09 (“approved proceed with the full implementation”)
- Scope: the six primary cells, the break-only control, 6 bps, and the stop rule
- Not authorized: 2024 access, post-hoc filter changes, or control promotion

## 9. Closeout

| Field | Value |
|---|---|
| Verdict | `no_candidate_underpowered` |
| Accepted baseline | none |
| Registry | `docs/improvement/2026-11-09/SR9-15M-STRUCTURE.md` |
| Evidence | `data/improvement/2026-11-09/SR9-15M-STRUCTURE/` |
| 2024 | not opened |
