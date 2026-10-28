# AI Transparency Log — Market-Report Verification

- Date: 2026-10-27 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#152](https://github.com/quant-rule/quant-rule/issues/152)
- Container: [#148](https://github.com/quant-rule/quant-rule/issues/148)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-27/141-market-analysis-and-backtest.md`](../../execution-plans/2026-10-27/141-market-analysis-and-backtest.md)
- Contract: [`docs/model/2026-10-27/market-report.md`](../../model/2026-10-27/market-report.md)
- Branch/worktree: `test/152-market-report` in `quant-rule--152-market-report`
- PR: pending
- Status: Verification complete; documentation close-out pending the pull request.

## Intent and boundary

A4 closes the market-analysis container with evidence: a full-year SOLUSD 2025
`market-report` run, two-run byte comparison, coverage and gap inspection,
artifact review, and a check that the descriptive path publishes no execution
field. It adds no command, no model field, no test fixture, and no dependency.
The run uses the merged A3 binary; the shell still only loads, renders, and
writes, and the model still owns every published number.

The fetched CSV and the generated artifacts stay outside Git: `data/` is
ignored, and `reports/` is a local working directory. The plan and this log
carry the evidence, not the data.

## Work performed

1. Built `quantrule-shell` in release mode at `c380698` (merge of PR #155).
2. Fetched `SOL/USD` M1 for 2025-01-01..2025-12-31 from Dukascopy BID into
   `data/SOLUSD_M1_2025-01-01_2025-12-31.csv`.
3. Ran `market-report` twice with identical arguments into `reports/run-a` and
   `reports/run-b`, then compared every artifact with `sha256sum` and `diff -r`.
4. Ran the command with explicit `--from 2025-01-01 --to 2025-12-31` into
   `reports/run-range` to confirm the requested period is recorded and the
   model body is unchanged.
5. Ran with `--json` and compared stdout with the written `report.json`.
6. Inspected the JSON envelope, the chart metadata, the coverage table, the
   event totals, and the latest state; searched the text and JSON for execution
   vocabulary.
7. Confirmed the help listing names the command, its arguments, and the
   extended-range limit.

## Verification and evidence

### Fetch

```text
symbol=SOL/USD timeframe=1m start=2025-01-01 end=2025-12-31
rows=522633 accepted=522633 rejected=0 pages=18
first=2025-01-01T14:05:00Z last=2025-12-31T14:58:00Z
csv=data/SOLUSD_M1_2025-01-01_2025-12-31.csv
```

The over-cap warning appeared on stderr, as designed: the file exceeds the
100,000-row batch cap and is accepted by `market-report` up to 600,000 rows.

### Two-run comparison

Both runs used:

```text
market-report data/SOLUSD_M1_2025-01-01_2025-12-31.csv \
  --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 \
  --out reports/run-a|reports/run-b
```

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| `summary.txt` | 4,464 | `84cff6c06bd336fa548a83d04faf68e4504f3517f289e18b8e670a363e3d6dd3` |
| `report.json` | 2,300,490 | `7b2a26dc41d607c8bb8e7c8dd7896ee6ff9c19c25bf81e8840024e9137181ac2` |
| `price-axes.png` | 26,331 | `acb33a8c935fbe20d49ee8363a660423b9db76d717e8de0deec1b4f186fc2594` |
| `events.png` | 16,249 | `a15587e0f044e610de2ac73b4bc492b1300ad363bb787c2fb1e48a261b3beea6` |

`diff -r reports/run-a reports/run-b` reported no difference. The four hashes
are identical across the two runs.

Run 1 took 19.05 s wall and 795,184 KiB peak RSS; run 2 took 18.90 s and
795,248 KiB. The earlier D2 benchmark measured 16.1 s and 822 MiB on its own
input; the difference is input and build variance, not a contract change.

### Requested period and coverage

The explicit-range run recorded `requested_period: 2025-01-01..2025-12-31` and
the same actual coverage. Its `market_report` body is identical to the
full-file run's; only `requested_period` differs, as the contract requires.
The report's actual period is `2025-01-01T14:05:00Z..2025-12-31T14:58:00Z`,
the provider's first and last bars.

| Timeframe | Bars | Expected | Missing | Share |
|---|---:|---:|---:|---:|
| 1m | 522,633 | 524,214 | 1,581 | 0.9970 |
| 5m | 104,364 | 104,842 | 478 | 0.9954 |
| 15m | 34,664 | 34,946 | 282 | 0.9919 |
| 1h | 8,535 | 8,735 | 200 | 0.9771 |

The missing bars are provider gaps in the fetched series, counted per
timeframe; the report never infers coverage from the requested dates. The
model ran over 670,196 bars in total.

### Artifacts

- `report.json` envelope: `model_version` `0.1.0`, `view_contract` `2026-10-27`,
  `data_version` `dukascopy-bid-solusd-m1-2025-01-01_2025-12-31`, requested
  period, and the `market_report` body.
- Events total 1,190,518: 842,285 level, 15,320 divergence, 332,913 signal.
- The chart uses H1: 8,535 source bars, bucket size 36, 238 buckets, which is
  within the 240-bucket bound. Each bucket keeps the last published axis value.
- `price-axes.png` is 1280×960 and `events.png` is 1280×720, both 8-bit RGB PNG.
- `--json` stdout is byte-identical to `report.json`
  (`867f67b06afeda84f97da1a9f1bd8185ef2eee9c0ec7c86ce47b2b0f53234b33` for the
  explicit-range run).
- A key and word search over `summary.txt` and `report.json` found no `trade`,
  `fill`, `order`, `position`, `equity`, `drawdown`, `pnl`, `profit`, `loss`,
  or `strategy` field or word. The report applies no strategy and publishes no
  execution result.

### Help and README

`quantrule help` lists `market-report <csv-path>`, `--data-version <id>` as
required, the optional date and output arguments, and the 600,000-candle
extended limit. The README's market-report contract matches the merged command
and the model contract.

## Learning

- The report body is independent of how the period is requested; only the
  `requested_period` field changes. That is the correct separation, so the
  verification compares the body and the envelope separately instead of
  comparing the whole file across different arguments.
- The two-run hash check must use the same arguments. Comparing a full-file run
  with an explicit-range run would fail on `requested_period` and would not be
  a determinism defect.
- The fetched series starts at 14:05 UTC on 2025-01-01, so the report's actual
  period, not the requested dates, is the coverage evidence.
- Rejected: committing the 522,633-row CSV or the 2.3 MB JSON as fixtures. The
  test suite already covers determinism over checked-in fixtures; the full-year
  evidence belongs in the plan and this log.
- Rejected: adding a shell-side coverage or hash calculation. The model view
  publishes coverage; the verification reads the artifacts and compares bytes.
