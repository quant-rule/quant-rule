# AI Transparency Log — SR8-A H3 Archive

- Date: 2026-11-09 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `sr8a-h3-archive`
- Plan: [`sr8a-h3-archive.md`](../../execution-plans/2026-11-09/sr8a-h3-archive.md)
- Charter: [`SR8-A-CHARTER.md`](../../governance/2026-11-09/SR8-A-CHARTER.md)
- Record: [`SR8-A-H3-ARCHIVE.md`](../../improvement/2026-11-09/SR8-A-H3-ARCHIVE.md)
- Status: stopped; archive rejected; no H3 run; 2024 not opened
- Claim type: strategy robustness buffer; no candidate or edge claim

## Intent and boundary

The human approved the data-source path after SR8's 2023 buffer was
data-ineligible. The task was to obtain a complete BTC 2023 1m source and re-run
the frozen H3 long/short test. The Dukascopy historical archive was selected
because the chart endpoint's 2023 series has only 61.3% 1h coverage.

## Implementation

- Checked the 2014–2017 chart source: no candles; the archive starts in 2017-08.
- Downloaded 365 daily Dukascopy archive files for BTCUSD 2023.
- Decoded the 24-byte big-endian records and built a 525,600-row CSV.
- Added `tools/improvement/dukascopy_archive.py` and 9 unit tests.
- Audited flat zero-volume bars, full hours, archive-only minutes, and OHLC
  differences against the chart source.
- Stopped before any H3 backtest when the archive showed 38,948 flat
  placeholders and only 61.4% genuine-minute 1h coverage.
- Did not open 2024.

## Verification and evidence

| Check | Observed |
|---|---|
| Archive days | 365 / 365 |
| Rows | 525,600 |
| Duplicates / order | 0 / strictly increasing |
| Row-based 1h coverage | 100% |
| Flat zero-volume bars | 38,948 (7.41%) |
| Longest flat run | 1,260 minutes |
| Archive-only minutes | 38,121 |
| Archive-only flat placeholders | 38,093 |
| Hours full only in the archive | 3,406; all contain a flat bar |
| Merged genuine-minute 1h coverage | 61.4% |
| OHLC mismatches vs chart source | 615; max $105.40 |
| Archive tick probe (2023-07-29 12h) | 0 bytes |
| H3 backtest | not run |
| 2024 | not opened |
| `python3 -m unittest discover -s tools/improvement -p 'test_dukascopy_archive.py'` | 9 pass |

## Learning

- A 100% row count is not market coverage. The archive fills missing minutes
  with flat zero-volume candles, and the 1h bucket builder cannot tell them
  apart from real candles.
- The Dukascopy candle archive and the chart endpoint are different feeds. They
  overlap on 487,479 minutes, but 615 OHLC values differ and long archive
  stretches are stale.
- The archive tick file for a gap hour can be empty, confirming a missing feed
  rather than a no-trade market.
- Merging the 28 genuine archive-only minutes does not raise coverage; the 2023
  chart series remains the best Dukascopy has at 61.4%.
- The H3 family remains underpowered; no strategy evidence was produced here.

## Rejected or corrected

- Did not treat the archive's 100% row coverage as admissible.
- Did not run the H3 backtest on synthetic flat bars.
- Did not lower the 95% coverage gate or merge placeholders silently.
- Did not claim a no-edge result; the buffer was not run.
- Did not open 2024.
- Did not claim independent Quant Developer or Quant Validator review; the
  passes were sequential same-agent passes.
