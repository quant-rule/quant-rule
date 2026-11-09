# SR6 Charter — 15m VWAP Rejection with 1h Stack Context

- Date: 2026-11-09 (UTC)
- Program ID: `PROGRAM-2026-11-09-03`
- Status: **Approved; development complete — no candidate; 9 bps and 2024 not opened**
- Plan: [`sr6-vwap.md`](../../execution-plans/2026-11-09/sr6-vwap.md)
- Summary: [`SR6-VWAP.md`](../../improvement/2026-11-09/SR6-VWAP.md)
- Tier: strategy
- Model baseline: `MODEL-2026-11-04-04..05` (1m 4 ATR compression expansion)
- Predecessor: [`SR5-ABSORPTION.md`](../../improvement/2026-11-09/SR5-ABSORPTION.md)
- Claim type: strategy research charter; no edge or profitability claim

## 1. Premise

SR4 found the stack-aligned compression fade BTC-only; SR5 found the
absorption-rejection event too rare. The next test uses a different 15m signal
with a large sample: `signal.vwap_rejection` (code 28). Price rejects the
session VWAP and reverses away from it; when the rejection direction equals the
1h EMA stack, the trade is a pullback in the 1h trend rather than a countertrend
guess.

The read-only census found 4,822 long / 4,281 short A1 events over 2018–2022,
so the count gate should pass. The backtest decides whether the event has a
cost-aware follow-through.

## 2. Economic rationale

The session VWAP is the volume-weighted price paid by the session's
participants. A push through VWAP that fails and closes back on the prior side
traps the late aggressor; the rejection is the visible failure of that push.
When the 1h stack agrees, the rejection is a pullback into the trend rather than
a reversal against it. The counterparty is the trader who paid up for the
breakout and is forced out on the snap-back.

The premise fails if the rejection has no cost-aware follow-through or if the
1h stack filter does not improve the event-only control.

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
| Costs | 6 bps base; 9 bps V1 sensitivity |
| Account | $10,000 per cell; fixed notional; 1x leverage; one position |
| Drawdown | 5% per cell, hard halt; any observed DD > 5% fails the cell |

## 4. Cells

The event is `signal.vwap_rejection` (type 3, subtype 14, source 19). The entry
side is the event direction. Stop 1 ATR, target 3 ATR, time stop 20 bars,
`event_window_bars = 1`, `require_contiguous = true`.

| Variant | Rule | Primary? |
|---|---|---|
| `v1-stack` | VWAP rejection plus 1h `ema_stack` equal to the event direction | yes |
| `v2-stack-adx` | V1 plus 1h `adx_regime = 1` | control |
| `v3-event` | VWAP rejection only, no 1h context | control |

Long and short are separate. 3 rules × 2 sides × 5 years = 30 base cells.
V1 is the primary claim; V2 and V3 cannot rescue V1.

## 5. Count gate

Before the first backtest cell, a read-only count gate runs on the 15m/1h model
bundle for 2018–2022. It counts V1 events by side, using the event's
`available_bar_index` and the latest 1h bar whose close is at or before the 15m
bar's close.

- V1 must have at least 30 events on each side.
- If either side is below 30, the programme stops as `underpowered` before any
  backtest.
- The census forecast is 4,822 long / 4,281 short; the gate confirms it on the
  recorded bundles.

## 6. Partitions and data budget

| Partition | Window | Use |
|---|---|---|
| `strategy_dev` | 2018-01-01..2022-12-31 | The only partition read |
| `strategy_validation` | 2024-01-01..2024-12-31 | One-shot, not opened |

The validation partition has not been opened for this programme. A validation
run requires a frozen V1 candidate and separate human approval.

## 7. Correction

| Field | Value |
|---|---|
| Method | `bonferroni` |
| Primary cells | 2 (V1 long and short) |
| Trials | 6 variants |
| Family-wise alpha | 0.05 |
| Notes | V2 and V3 are controls; they cannot rescue V1 |

## 8. Green criteria

V1 is the primary claim. A side (long or short) passes only when all of the
following hold:

- at least 30 V1 trades on that side;
- net P&L > 0 at 6 bps;
- profit factor > 1.10 at 6 bps;
- every V1 cell's observed max drawdown ≤ 5%;
- at least 3 of 5 years positive;
- profit factor > 1.0 at 9 bps;
- byte-identical rerun for the passing side.

At least one side must pass the full gate. The other side must be non-negative
at 6 bps; a negative side is not part of the candidate. V2 and V3 are reported
descriptively and do not change the V1 verdict.

## 9. Concurrency and ownership

| Field | Value |
|---|---|
| Max parallel cells | 3 groups |
| Worktree root | `../quant-rule--sr6-vwap` |
| Cell ownership | one worktree and branch for the programme |
| Shared read-only files | source CSV, model and execution configs |
| Shared write policy | each run writes only its own `data/improvement/2026-11-09/SR6-VWAP/` path |

## 10. Stop rule

Stop the programme when:

- the count gate has fewer than 30 V1 events on either side;
- any preflight cell fails;
- V1 fails the development gate;
- V1 fails the 9 bps gate;
- any V1 cell observes a drawdown above 5%;
- the iteration cap is reached;
- the human stops it.

A stopped programme does not open 2024 and does not re-register V1 as a variant.
V2 and V3 are not promoted to primary after a V1 failure.

## 11. Validation access

| Field | Value |
|---|---|
| Access | `none` until a frozen V1 candidate and separate approval |
| Approved by | human, not yet requested |
| Approved on | — |
| Promoted cells | V1 passing side only |
| Conditions | V1 development gate and 9 bps pass; config frozen by hash |

## 12. Human approval

- Approved by: human, 2026-11-09 ("aprove")
- Approved on: 2026-11-09
- Scope: the premise, the count gate, the six variants, the correction, the
  budget, and the stop rule above
- Not authorized: 2024 access, a post-hoc event/timeframe/target/stop/filter/
  cost change, or a second validation access

## 13. Closeout

| Field | Value |
|---|---|
| Verdict | `no_edge` — V1 long/short fail net, PF, and drawdown; every cell halted |
| Accepted baseline | none |
| Registry | `docs/improvement/2026-11-09/SR6-VWAP.md` |
| Evidence | local run record `data/improvement/2026-11-09/SR6-VWAP/` |
| Cleanup | worktree and branch removed after preservation |

### Result

- Count gate: 4,822 long / 4,281 short; pass.
- V1 long: 229 trades, −$242.14, PF 0.966, max DD 6.37%.
- V1 short: 154 trades, −$719.73, PF 0.853, max DD 5.61%.
- Every V1/V2/V3 cell halted in every year.
- V2/V3 do not rescue V1.
- The 9 bps sensitivity and 2024 validation were not opened.
- The event-composite parser was widened to registered signal subtypes; diagnostics remain rejected.
