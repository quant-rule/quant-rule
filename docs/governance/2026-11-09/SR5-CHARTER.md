# SR5 Charter — 15m Absorption Rejection with 1h Stack Context

- Date: 2026-11-09 (UTC)
- Program ID: `PROGRAM-2026-11-09-02`
- Status: **Approved; count gate failed — underpowered; no backtest**
- Plan: [`sr5-absorption.md`](../../execution-plans/2026-11-09/sr5-absorption.md)
- Summary: [`SR5-ABSORPTION.md`](../../improvement/2026-11-09/SR5-ABSORPTION.md)
- Tier: strategy
- Model baseline: `MODEL-2026-11-04-04..05` (1m 4 ATR compression expansion)
- Predecessor: [`SR4-STACK-FADE.md`](../../improvement/2026-11-09/SR4-STACK-FADE.md)
- Claim type: strategy research charter; no edge or profitability claim

## 1. Premise

SR4 showed that the stack-aligned compression fade is BTC-only and too small to
support a robust result. The next question is whether a different event family
on BTCUSD produces a larger, cost-aware sample with a clearer mechanism.

The programme tests a 15m absorption rejection with a 1h stack context. A
high-volume, narrow-range bar at VWAP or the profile POC absorbs the aggressor;
a close back inside the absorption bar's range is a rejection, and the fade of
that rejection is the entry. The 1h EMA stack must agree with the rejection
direction, so the 15m trade is a pullback in the 1h trend rather than a
countertrend guess.

## 2. Economic rationale

A high-volume, narrow-range bar at a reference price marks absorption: the
aggressor pays for immediacy, but price does not progress. When the next bar
closes back inside the absorption range, the aggressor is trapped; the
counterparty is the late breakout buyer or seller. The 1h stack filters for the
side where the absorption is a pullback rather than a reversal against the
prevailing trend. The edge, if it exists, is the rejection's snap-back after
the failed push.

The premise fails if the event is too rare, if the rejection has no cost-aware
follow-through, or if the 1h stack filter does not improve the event-only
control.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `strategy` |
| Changed | strategy rule cells only |
| Frozen | data, policy, accepted model, execution, portfolio, analytics |
| Instrument | BTCUSD |
| Execution | 15m |
| Context | 1h |
| Data versions | `dukascopy-bid-btcusd-m1-<year>-01-01_<year>-12-31` |
| Development | `strategy_dev`, 2018-01-01..2022-12-31 |
| Validation | `strategy_validation` 2024; not opened |
| Costs | 6 bps base; 9 bps A1 sensitivity |
| Account | $10,000 per cell; fixed notional; 1x leverage; one position |
| Drawdown | 5% per cell, hard halt; any observed DD > 5% fails the cell |

## 4. Cells

The event is `signal.absorption_rejection` (type 3, subtype 12, source 22).
The entry side is the event direction. Stop 1 ATR, target 3 ATR, time stop 20
bars, `event_window_bars = 1`, `require_contiguous = true`.

| Variant | Rule | Primary? |
|---|---|---|
| `a1-stack` | Absorption rejection plus 1h `ema_stack` equal to the event direction | yes |
| `a2-stack-adx` | A1 plus 1h `adx_regime = 1` | control |
| `a3-event` | Absorption rejection only, no 1h context | control |

Long and short are separate. 3 rules × 2 sides × 5 years = 30 base cells.
A1 is the primary claim; A2 and A3 are controls and cannot rescue A1.

## 5. Count gate

Before the first backtest cell, a read-only count gate runs on the 15m/1h model
bundle for 2018–2022. It counts A1 events by side, using the event's
`available_bar_index` and the latest 1h bar whose close is at or before the 15m
bar's close.

- A1 must have at least 30 events on each side.
- If either side is below 30, the programme stops as `underpowered` before any
  backtest.
- The gate also reports the A2 and A3 counts for context, but they do not
  change the stop rule.

## 6. Partitions and data budget

| Partition | Window | Use |
|---|---|---|
| `strategy_dev` | 2018-01-01..2022-12-31 | The only partition read |
| `strategy_validation` | 2024-01-01..2024-12-31 | One-shot, not opened |

The validation partition has not been opened for this programme. A validation
run requires a frozen A1 candidate and separate human approval.

## 7. Correction

| Field | Value |
|---|---|
| Method | `bonferroni` |
| Primary cells | 2 (A1 long and short) |
| Trials | 6 variants |
| Family-wise alpha | 0.05 |
| Notes | A2 and A3 are controls; they cannot rescue A1 |

## 8. Green criteria

A1 is the primary claim. A side (long or short) passes only when all of the
following hold:

- at least 30 A1 trades on that side;
- net P&L > 0 at 6 bps;
- profit factor > 1.10 at 6 bps;
- every A1 cell's observed max drawdown ≤ 5%;
- at least 3 of 5 years positive;
- profit factor > 1.0 at 9 bps;
- byte-identical rerun for the passing side.

At least one side must pass the full gate. The other side must be non-negative
at 6 bps; a negative side is not part of the candidate. A2 and A3 are reported
descriptively and do not change the A1 verdict.

## 9. Concurrency and ownership

| Field | Value |
|---|---|
| Max parallel cells | 3 groups |
| Worktree root | `../quant-rule--sr5-absorption` |
| Cell ownership | one worktree and branch for the programme |
| Shared read-only files | source CSV, model and execution configs |
| Shared write policy | each run writes only its own `data/improvement/2026-11-09/SR5-ABSORPTION/` path |

## 10. Stop rule

Stop the programme when:

- the count gate has fewer than 30 A1 events on either side;
- any preflight cell fails;
- A1 fails the development gate;
- A1 fails the 9 bps gate;
- any A1 cell observes a drawdown above 5%;
- the iteration cap is reached;
- the human stops it.

A stopped programme does not open 2024 and does not re-register A1 as a variant.
A2 and A3 are not promoted to primary after an A1 failure.

## 11. Validation access

| Field | Value |
|---|---|
| Access | `none` until a frozen A1 candidate and separate approval |
| Approved by | human, not yet requested |
| Approved on | — |
| Promoted cells | A1 passing side only |
| Conditions | A1 development gate and 9 bps pass; config frozen by hash |

## 12. Human approval

- Approved by: human, 2026-11-09 ("approved proceed")
- Approved on: 2026-11-09
- Scope: the premise, the count gate, the six variants, the correction, the
  budget, and the stop rule above
- Not authorized: 2024 access, a post-hoc event/timeframe/target/stop/filter/
  cost change, or a second validation access

## 13. Closeout

| Field | Value |
|---|---|
| Verdict | `underpowered` — A1 has 14 long / 12 short, below the 30-per-side gate |
| Accepted baseline | none |
| Registry | `docs/improvement/2026-11-09/SR5-ABSORPTION.md` |
| Evidence | local run record `data/improvement/2026-11-09/SR5-ABSORPTION/` |
| Cleanup | worktree and branch removed after preservation |

### Result

- A1 count: 14 long, 12 short.
- A2 count: 7 long, 8 short.
- A3 event-only count: 30 long, 32 short.
- 2018 and 2021 have zero A1 events; 2019 carries 15 of the 26 A1 events.
- No backtest, no 9 bps, no 2024.
