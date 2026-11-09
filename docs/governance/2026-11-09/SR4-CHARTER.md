# SR4 Charter — Multi-Instrument Stack-Aligned 1h Fade

- Date: 2026-11-09 (UTC)
- Program ID: `PROGRAM-2026-11-09-01`
- Status: **Approved; development complete — `no_edge`; 2024 not opened**
- Plan: [`sr4-stack-fade.md`](../../execution-plans/2026-11-09/sr4-stack-fade.md)
- Summary: [`SR4-STACK-FADE.md`](../../improvement/2026-11-09/SR4-STACK-FADE.md)
- Tier: strategy
- Model baseline: `MODEL-2026-11-04-04..05` (1m 4 ATR compression expansion)
- Predecessor: [`SR3-FEATURES.md`](../../improvement/2026-11-08/SR3-FEATURES.md)
- Claim type: strategy research charter; no edge or profitability claim

## 1. Premise

The SR3 feature programme found that the 1h compression-breakout fade is
positive when the event direction equals the 1h EMA stack, but the BTC-only
sample holds only 9–21 trades per side over five years. The next question is
whether the stack-aligned fade is a repeatable market behavior or a BTC-specific
small-sample artifact.

The programme tests the simplest stack-aligned rule on BTCUSD, EURUSD, and
XAUUSD. A larger pooled sample is the point: it can confirm the sign, expose
instrument dependence, and give a defensible trade count without changing the
rule after seeing a result.

## 2. Economic rationale

A compression breakout aligned with the prevailing EMA stack is a
trend-continuation signal. Fading it is a bet that the breakout is a
low-participation liquidity probe rather than the start of a trend. When price
is aligned with the stack but participation is weak and trend strength is low,
the breakout has little follow-through, and the opposite side of the probe can
be paid. The counterparty is the breakout buyer or seller who demands
immediacy; the risk premium is the failed breakout's snap-back.

The premise fails if the fade is positive only on BTC, only in one year, or only
before costs. The programme is designed to falsify it on the other two
instruments and on the pooled sample.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `strategy` |
| Changed | strategy rule cells only |
| Frozen | data, policy, accepted model, execution, portfolio, analytics |
| Instruments | BTCUSD, EURUSD, XAUUSD |
| Timeframe | 1h execution and 1h state; no 15m, no 1m model |
| Data versions | `dukascopy-bid-<instrument>-m1-<year>-01-01_<year>-12-31` |
| Development | `strategy_dev`, 2018-01-01..2022-12-31 |
| Validation | `strategy_validation` 2024; not opened |
| Costs | 6 bps base; 9 bps sensitivity for S1 only |
| Account | $10,000 per cell; fixed notional; 1x leverage; one position |
| Drawdown | 5% per cell, hard halt; any observed DD > 5% fails the cell |

## 4. Cells

The matrix is 3 rules × 2 sides = 6 rule-side variants, crossed with 3
instruments = 18 variant-instrument combinations, one calendar year per cell,
2018–2022 = 90 base cells.

| Variant | Rule | Primary? |
|---|---|---|
| `s1-stack` | Event direction equals the 1h EMA stack; no ADX or RVOL filter | yes |
| `s2-stack-rvol` | `s1-stack` plus `rvol_regime = -1` | control |
| `s3-stack-adx` | `s1-stack` plus `adx_regime = 0` | control |

| Side | Event direction | EMA stack | Entry side |
|---|---:|---:|---|
| long | −1 | −1 | long (`contrarian: true`) |
| short | +1 | +1 | short (`contrarian: true`) |

The event is `signal.compression_breakout` (type 3, subtype 7, source 5).
Stop 1 ATR, target 3 ATR, time stop 20 bars, `event_window_bars = 1`,
`require_contiguous = true`.

The 90 config paths are generated under `config/sr4/runs/`; the matrix is
`config/sr4/matrix.json`. The exact strategy configs are
`config/strategy/sr4-<variant>-<side>.json`.

## 5. Partitions and data budget

| Partition | Window | Use |
|---|---|---|
| `strategy_dev` | 2018-01-01..2022-12-31 | The only partition read |
| `strategy_validation` | 2024-01-01..2024-12-31 | One-shot, not opened |

The validation partition has not been opened for this programme. A validation
run requires a frozen S1 candidate and separate human approval.

## 6. Correction

| Field | Value |
|---|---|
| Method | `bonferroni` |
| Primary cells | 6 (S1, three instruments, two sides) |
| Trials | 18 variants |
| Family-wise alpha | 0.05 |
| Notes | S2 and S3 are controls; they cannot rescue a failed S1 |

## 7. Green criteria

S1 is the primary claim. A side (long or short) passes only when all of the
following hold:

- pooled S1 net P&L > 0 at 6 bps;
- pooled S1 profit factor > 1.10 at 6 bps;
- at least 30 pooled S1 trades on that side;
- at least 2 of 3 instruments positive on that side;
- at least 3 of 5 pooled years positive;
- every S1 cell's observed max drawdown ≤ 5%;
- pooled S1 profit factor > 1.0 at 9 bps;
- byte-identical rerun for the passing side.

At least one side must pass the full gate. The other side must be non-negative
at 6 bps; a negative side is not part of the candidate. S2 and S3 are reported
descriptively and do not change the S1 verdict.

## 8. Concurrency and ownership

| Field | Value |
|---|---|
| Max parallel cells | 4 groups |
| Worktree root | `../quant-rule--sr4-stack-fade` |
| Cell ownership | one worktree and branch for the programme |
| Shared read-only files | source CSVs, model and execution configs |
| Shared write policy | each run writes only its own `data/improvement/2026-11-09/SR4-STACK-FADE/` path |

## 9. Stop rule

Stop the programme when:

- any preflight cell fails;
- S1 fails the pooled development gate;
- S1 fails the 9 bps pool;
- any S1 cell observes a drawdown above 5%;
- the iteration cap is reached;
- the human stops it.

A stopped programme does not open 2024 and does not re-register S1 as a variant.
S2 and S3 are not promoted to primary after an S1 failure.

## 10. Validation access

| Field | Value |
|---|---|
| Access | `none` until a frozen S1 candidate and separate approval |
| Approved by | human, not yet requested |
| Approved on | — |
| Promoted cells | S1 passing side only |
| Conditions | S1 development gate and 9 bps pass; config frozen by hash |

## 11. Human approval

- Approved by: human, 2026-11-09 ("approved proceed")
- Approved on: 2026-11-09
- Scope: the premise, the 18 variants, the correction, the budget, and the stop
  rule above
- Not authorized: 2024 access, a post-hoc event/timeframe/target/stop/filter/
  cost change, or a second validation access

## 12. Closeout

| Field | Value |
|---|---|
| Verdict | `no_edge` — instrument-specific; BTC-only positive; S1 fails 2-of-3 instruments and 5% drawdown |
| Accepted baseline | none |
| Registry | `docs/improvement/2026-11-09/SR4-STACK-FADE.md` |
| Evidence | local run record `data/improvement/2026-11-09/SR4-STACK-FADE/` |
| Cleanup | worktree and branch removed after preservation |

### Result

- 90 / 90 preflight pass; 90 / 90 base cells and qualification records.
- S1 long: 113 trades, +$1,150.76, PF 1.34, 4/5 positive years, 1/3 instruments.
- S1 short: 111 trades, +$411.97, PF 1.16, 3/5 positive years, 1/3 instruments.
- EURUSD and XAUUSD are negative on every variant and side.
- S1 BTC long/short breach 5% drawdown in 2018/2019/2021.
- The 9 bps sensitivity and 2024 validation were not opened.
