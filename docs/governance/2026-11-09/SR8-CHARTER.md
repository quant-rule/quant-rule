# SR8 Charter — H3 BTC 2023 Buffer Test

- Date: 2026-11-09 (UTC)
- Program ID: `PROGRAM-2026-11-09-05`
- Status: **Stopped — 2023 buffer data-ineligible (61.3% 1h coverage); 2024 not opened**
- Plan: [`sr8-h3-buffer.md`](../../execution-plans/2026-11-09/sr8-h3-buffer.md)
- Summary: [`SR8-H3-BUFFER.md`](../../improvement/2026-11-09/SR8-H3-BUFFER.md)
- Tier: strategy
- Model baseline: `MODEL-2026-11-04-04..05`
- Predecessor: [`SR7-H3-CRYPTO.md`](../../improvement/2026-11-09/SR7-H3-CRYPTO.md)
- Claim type: strategy robustness buffer; no candidate or edge claim

## 1. Premise

The H3 BTC 1h fade is the only positive, drawdown-compliant lead: 18 long /
21 short trades over 2018–2022, PF 2.23/1.73, max DD 4.41%/4.17%, positive at
9 bps. It is underpowered. The intended 2014–2017 data expansion is unavailable:
the source returned no BTC candles for 2014-01-01..2014-01-05, so the approved
fallback is a one-shot 2023 buffer test.

2023 is excluded from every formal partition. This programme consumes it once as
a robustness buffer, not as a validation and not as a candidate. The frozen H3
long/short configs are run unchanged on 2023 BTCUSD.

## 2. Economic rationale

The H3 rule fades a compression breakout aligned with the 1h EMA stack when 1h
ADX is weak. The 2023 year is a different regime from 2018–2022 and was not used
in any development, so a positive result is evidence that the BTC sign is not
confined to the development window. A negative result closes the stack-fade
family.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `strategy` |
| Changed | strategy evaluation only; no rule change |
| Frozen | data source, policy, model, execution, portfolio, strategy |
| Instrument | BTCUSD |
| Timeframe | 1h execution and 1h state |
| Data version | `dukascopy-bid-btcusd-m1-2023-01-01_2023-12-31` |
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

- it reads only 2023 BTCUSD 1m;
- it does not add 2023 to a partition;
- it does not open 2024;
- the buffer is consumed once and cannot be used again for this hypothesis.

The 2014–2017 availability check failed: `fetch BTCUSD 2014-01-01 2014-01-05`
returned no candles.

## 7. Stop rule

Stop when the 2023 data coverage is incomplete, a preflight fails, either side
misses the buffer criteria, or the human stops it. A stopped programme does not
open 2024 and does not re-use 2023 for this hypothesis.

## 8. Human approval

- Approved by: human, 2026-11-09 ("aprove")
- Approved on: 2026-11-09
- Scope: the 2023 one-shot buffer, the frozen H3 long/short configs, 6/9 bps,
  and the buffer criteria above
- Not authorized: 2024 access, a rule/config change, or a second 2023 use

## 9. Closeout

| Field | Value |
|---|---|
| Verdict | `buffer_ineligible` — 2023 1h coverage 61.3%, below the 95% gate |
| Accepted baseline | none |
| Registry | `docs/improvement/2026-11-09/SR8-H3-BUFFER.md` |
| Evidence | local run record `data/improvement/2026-11-09/SR8-H3-BUFFER/` |
| Cleanup | worktree and branch removed after preservation |

### Result

- 2014–2017 source returned no candles.
- 2023 1h coverage: 61.3% (5,354 bars, 3,378 missing).
- H3 long 0 trades; H3 short 4 trades, −$98.78 at 6 bps.
- Buffer ineligible; no no-edge conclusion; 2024 not opened.
