# SR8-A Charter — H3 BTC 2023 Archive Buffer

- Date: 2026-11-09 (UTC)
- Program ID: `PROGRAM-2026-11-09-06`
- Status: **Stopped — archive data rejected; no H3 run; 2024 not opened**
- Plan: [`sr8a-h3-archive.md`](../../execution-plans/2026-11-09/sr8a-h3-archive.md)
- Tier: strategy
- Model baseline: `MODEL-2026-11-04-04..05`
- Predecessor: [`SR8-H3-BUFFER.md`](../../improvement/2026-11-09/SR8-H3-BUFFER.md)
- Claim type: strategy robustness buffer; no candidate or edge claim

## 1. Premise

The H3 BTC 1h fade is the only positive, drawdown-compliant lead: 18 long / 21
short trades over 2018–2022, PF 2.23/1.73, max DD 4.41%/4.17%, positive at 9 bps.
It is underpowered. SR8 attempted a 2023 buffer on the chart-endpoint source,
but that source has 61.3% 1h coverage, so the buffer was data-ineligible.

The Dukascopy historical archive supplies complete 2023 1m candles: 525,600
rows and 8,760 full 1h hours. This programme re-runs the frozen H3 long/short
rule once on the corrected data version.

## 2. Economic rationale

The H3 rule fades a compression breakout aligned with the 1h EMA stack when 1h
ADX is weak. 2023 is a different regime from 2018–2022 and was not used in any
development, so a positive result is evidence that the BTC sign is not confined
to the development window. A negative result closes the stack-fade family.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `strategy` |
| Changed | strategy evaluation only; no rule change |
| Frozen | data source family, policy, model, execution, portfolio, strategy |
| Instrument | BTCUSD |
| Timeframe | 1h execution and 1h state |
| Data version | `dukascopy-bid-btcusd-m1-2023-01-01_2023-12-31-archive-v1` |
| Buffer window | 2023-01-01..2023-12-31 |
| Validation | `strategy_validation` 2024; not opened |
| Costs | 6 bps base; 9 bps sensitivity |
| Account | $10,000 per cell; fixed notional; 1x leverage; one position |
| Drawdown | 5% per cell, hard halt; any observed DD > 5% fails |

## 4. Cells

| Cell | Rule | Config |
|---|---|---|
| `h3-long` | Frozen SR3/SR7 H3 long: event −1, stack −1, `adx_regime = 0`, contrarian long | `config/strategy/sr7-h3-long.json` |
| `h3-short` | Frozen SR3/SR7 H3 short: event +1, stack +1, `adx_regime = 0`, contrarian short | `config/strategy/sr7-h3-short.json` |

Both cells run at 6 bps and 9 bps. No filter, stop, target, timeframe, or
instrument change.

## 5. Buffer criteria

This is a robustness gate, not a candidate gate. One year cannot meet the
30-trade floor, so the criteria are:

- each side has at least 5 trades;
- net P&L > 0 at 6 bps;
- profit factor > 1.0 at 6 bps;
- observed max drawdown ≤ 5%;
- no halt;
- profit factor > 0 at 9 bps (the sign survives costs).

If both sides pass, the H3 BTC lead survives the 2023 buffer and the next
question is whether to request 2024 validation. If either side fails, the
stack-fade family is closed.

## 6. Data and partition exception

2023 is listed in `data/partitions.json` under `excluded_windows` and is not
part of any formal partition. This programme is a one-shot human-approved
exception:

- it reads only 2023 BTCUSD 1m from the new archive data version;
- it does not add 2023 to a partition;
- it does not open 2024;
- the buffer is consumed once and cannot be used again for this hypothesis.

The original SR8 charter barred reuse of 2023 after its data-ineligible stop.
SR8 produced no valid evidence, so this programme supersedes that clause for one
corrected run. The SR8 record remains; 2023 is consumed by SR8-A.

## 7. Stop rule

Stop when the 2023 data coverage is incomplete, the archive and chart OHLC
differ beyond volume rounding, a preflight fails, either side misses the buffer
criteria, or the human stops it. A stopped programme does not open 2024 and does
not re-use 2023 for this hypothesis.

## 8. Human approval

- Approved by: human, 2026-11-09 ("aporve")
- Approved on: 2026-11-09
- Scope: obtain a complete BTC 2023 1m source and re-run the frozen H3
  long/short test, with 6/9 bps and the buffer criteria above
- Not authorized: 2024 access, a rule/config change, a second 2023 use, or a
  new strategy variant

## 9. Closeout

| Field | Value |
|---|---|
| Verdict | `archive_rejected` — flat zero-volume placeholders; no H3 run |
| Accepted baseline | none |
| Registry | `docs/improvement/2026-11-09/SR8-A-H3-ARCHIVE.md` |
| Evidence | `docs/reports/2026-11-09/sr8a-data-validation.md`; local CSV |
| Cleanup | worktree and branch removed after preservation |

### Result

- 365 / 365 archive days downloaded; 525,600 rows.
- Row-based 1h coverage is 100%, but 38,948 bars are flat zero-volume
  placeholders and 3,406 hours are complete only because of them.
- Merging only the 28 genuine archive-only minutes leaves 1h coverage at 61.4%.
- The archive is rejected; no H3 backtest was run; 2023 was not consumed by a
  strategy run; 2024 was not opened.
