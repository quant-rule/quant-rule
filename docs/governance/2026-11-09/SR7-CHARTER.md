# SR7 Charter — BTC+ETH H3 Crypto Cross-Section

- Date: 2026-11-09 (UTC)
- Program ID: `PROGRAM-2026-11-09-04`
- Status: **Approved; development complete — no candidate; 9 bps and 2024 not opened**
- Plan: [`sr7-h3-crypto.md`](../../execution-plans/2026-11-09/sr7-h3-crypto.md)
- Summary: [`SR7-H3-CRYPTO.md`](../../improvement/2026-11-09/SR7-H3-CRYPTO.md)
- Tier: strategy
- Model baseline: `MODEL-2026-11-04-04..05` (1m 4 ATR compression expansion)
- Predecessor: [`SR6-VWAP.md`](../../improvement/2026-11-09/SR6-VWAP.md)
- Claim type: strategy research charter; no edge or profitability claim

## 1. Premise

SR3 found the H3 rule positive on BTCUSD and inside the 5% drawdown ceiling, but
the sample was only 18 long / 21 short trades. SR4 showed the rule is not a
EURUSD/XAUUSD behavior. This programme tests whether H3 is a **crypto-major**
behavior by adding ETHUSD, a correlated major with the same microstructure. If
ETH confirms, the pooled sample roughly doubles; if ETH fails, the BTC lead stays
BTC-only.

H3 is the 1h compression-breakout fade with the 1h EMA stack aligned to the event
direction and 1h `adx_regime = 0` (weak trend), with no RVOL filter.

## 2. Economic rationale

A compression breakout aligned with the 1h stack is a trend-continuation signal;
fading it is a bet that the breakout is a low-participation liquidity probe when
trend strength is weak. The weak-ADX filter removes the strong-trend breakouts
that continue, and the stack alignment keeps the fade on the side of the
prevailing trend. Crypto majors share the same 24/7 order flow and
participation structure, so ETH is a meaningful cross-crypto control.

The premise fails if ETH is negative or if the pooled sample does not improve
the result. A negative ETH result closes the stack-fade family; it is not a
reason to add a filter.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `strategy` |
| Changed | strategy rule cells only |
| Frozen | data, policy, accepted model, execution, portfolio, analytics |
| Instruments | BTCUSD, ETHUSD |
| Timeframe | 1h execution and 1h state; no 15m or 1m model |
| Data versions | `dukascopy-bid-<instrument>-m1-<year>-01-01_<year>-12-31` |
| Development | `strategy_dev`, 2018-01-01..2022-12-31 |
| Validation | `strategy_validation` 2024; not opened |
| Costs | 6 bps base; 9 bps for a passing side |
| Account | $10,000 per cell; fixed notional; 1x leverage; one position |
| Drawdown | 5% per cell, hard halt; any observed DD > 5% fails the cell |

## 4. Cells

The event is `signal.compression_breakout` (type 3, subtype 7, source 5). The
entry is opposite the event (`contrarian: true`). The 1h EMA stack must equal the
event direction, and 1h `adx_regime` must be 0. Stop 1 ATR, target 3 ATR, time
stop 20 bars.

| Variant | Rule | Primary? |
|---|---|---|
| `h3-long` | Event −1, stack −1, weak ADX, contrarian long | BTC control; ETH primary |
| `h3-short` | Event +1, stack +1, weak ADX, contrarian short | BTC control; ETH primary |

2 instruments × 2 sides × 5 years = 20 base cells. The primary cells are the two
ETH sides; the BTC sides are the SR3 control and must reproduce their prior sign.

## 5. Partitions and data budget

| Partition | Window | Use |
|---|---|---|
| `strategy_dev` | 2018-01-01..2022-12-31 | The only partition read |
| `strategy_validation` | 2024-01-01..2024-12-31 | One-shot, not opened |

The validation partition has not been opened. A validation run requires a
frozen passing side and separate human approval.

## 6. Correction

| Field | Value |
|---|---|
| Method | `bonferroni` |
| Primary cells | 2 (ETH long and short) |
| Trials | 4 instrument-side cells |
| Family-wise alpha | 0.05 |
| Notes | BTC is the control; a negative ETH side fails the premise |

## 7. Green criteria

A side passes only when all of the following hold:

- at least 30 pooled H3 trades across BTC+ETH on that side;
- pooled net P&L > 0 at 6 bps;
- pooled profit factor > 1.10 at 6 bps;
- every cell's observed max drawdown ≤ 5%;
- at least 3 of 5 pooled years positive;
- both instruments non-negative on that side;
- profit factor > 1.0 at 9 bps;
- byte-identical rerun for the passing side.

At least one side must pass. A side that fails is not part of the candidate. The
BTC control must not contradict the SR3 sign; a BTC loss with an ETH gain is not
a cross-crypto confirmation.

## 8. Concurrency and ownership

| Field | Value |
|---|---|
| Max parallel cells | 2 groups |
| Worktree root | `../quant-rule--sr7-h3-crypto` |
| Cell ownership | one worktree and branch for the programme |
| Shared read-only files | source CSVs, model and execution configs |
| Shared write policy | each run writes only its own `data/improvement/2026-11-09/SR7-H3-CRYPTO/` path |

## 9. Stop rule

Stop the programme when:

- the count gate has fewer than 30 H3 events per side pooled;
- any preflight cell fails;
- both sides fail the development gate;
- a passing side fails the 9 bps gate;
- any cell observes a drawdown above 5%;
- the iteration cap is reached;
- the human stops it.

A stopped programme does not open 2024 and is not retried with a new filter or a
new instrument under the same premise.

## 10. Validation access

| Field | Value |
|---|---|
| Access | `none` until a frozen passing side and separate approval |
| Approved by | human, not yet requested |
| Approved on | — |
| Promoted cells | the passing side only |
| Conditions | development and 9 bps pass; config frozen by hash |

## 11. Human approval

- Approved by: human, 2026-11-09 ("apropve")
- Approved on: 2026-11-09
- Scope: the premise, the count gate, the four instrument-side cells, the
  correction, the budget, and the stop rule above
- Not authorized: 2024 access, a post-hoc event/timeframe/target/stop/filter/
  cost change, a new instrument, or a second validation access

## 12. Closeout

| Field | Value |
|---|---|
| Verdict | `no_edge` — H3 long fails trade-count/DD; H3 short fails PF/years/ETH/DD |
| Accepted baseline | none |
| Registry | `docs/improvement/2026-11-09/SR7-H3-CRYPTO.md` |
| Evidence | local run record `data/improvement/2026-11-09/SR7-H3-CRYPTO/` |
| Cleanup | worktree and branch removed after preservation |

### Result

- Count gate: BTC 20/23, ETH 19/19, pooled 39/42; pass.
- H3 long pooled: 27 trades, +$2,289.36, PF 2.098, max DD 5.62%, 5/5 years.
- H3 short pooled: 36 trades, +$147.02, PF 1.067, max DD 5.64%, 1/5 years.
- BTC control reproduces SR3; ETH long is +$670 on 9 trades; ETH short is
  −$576 on 15 trades.
- The 9 bps sensitivity and 2024 validation were not opened.
