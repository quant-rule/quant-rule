# SR8-A H3 BTC 2023 Archive Buffer

- Date: 2026-11-09 (UTC)
- Status: **Stopped — archive rejected; no H3 run; 2024 not opened**
- Charter: [`SR8-A-CHARTER.md`](../../governance/2026-11-09/SR8-A-CHARTER.md)
- Predecessor: [`SR8-H3-BUFFER.md`](../../improvement/2026-11-09/SR8-H3-BUFFER.md)
- Claim type: strategy robustness buffer; no candidate or edge claim

## 1. Intent

SR8 ran the frozen H3 BTC long/short rule on the excluded 2023 year and found
the chart-endpoint source data-ineligible: 61.3% 1h coverage, 3,378 missing 1h
bars. The cause is the source, not the strategy. The Dukascopy historical
archive supplies complete daily 1m candles for 2023 (365 files, 525,600 rows,
8,760 full 1h hours), decoded and checked against the chart source.

This programme re-runs the **unchanged** frozen H3 long/short rule once on the
corrected 2023 data version. It is a robustness buffer, not a candidate, not a
validation, and not a new rule. 2024 remains closed.

## 2. Current state

- SR8 result: H3 long 0 trades; H3 short 4 trades, −$98.78, PF 0.572 at 6 bps;
  buffer ineligible on 61.3% 1h coverage.
- The 2014–2017 chart source is unavailable; `fetch BTCUSD 2014-01-01
  2014-01-05` returns no candles.
- The Dukascopy archive has BTCUSD from 2017-08 onward, not 2014.
- The 2023 archive file is complete: 525,600 one-minute rows, 8,760 full hours,
  zero duplicate or out-of-order timestamps.
- The archive and chart sources agree on OHLC for all 487,479 overlapping
  minutes; volume differs only by float32 rounding (≤ 2e-6 absolute).
- 2024 validation remains closed.

## 3. Scope

Included:

- Copy the complete 2023 archive CSV into `data/sources/`.
- A data-quality record, a new data version, and a data-change record.
- A reproducible acquisition tool under `tools/improvement/`.
- Frozen H3 data/run configs under `config/sr8a/`.
- Two feasibility preflights, two base cells, two 9 bps cells, and two
  qualification records.
- Buffer review, decision record, registry/baseline/changelog updates.

Excluded:

- 2024 validation access.
- Any change to the H3 rule, filter, stop, target, timeframe, or instrument.
- A partition change; 2023 remains outside every formal partition.
- A second 2023 attempt if this run fails the buffer criteria.
- 2014–2017 data; the archive does not cover it.
- Product code changes to the `fetch` adapter.

## 4. Assumptions and uncertainty

- The archive is the same Dukascopy BID series as the chart source; overlapping
  OHLC matches exactly and volume differs only by float32 rounding.
- The H3 rule does not read volume, so the volume rounding cannot change a
  decision. This is checked against the strategy config.
- One year is a robustness check, not a sample-size solution; the 30-trade floor
  cannot be met by one year, so the 5-trade-per-side buffer floor applies.
- The archive's daily files are complete; missing files are retried and, if
  still missing, the buffer stops as data-ineligible.
- The human approval for the data-source path and the re-run covers the one
  corrected 2023 use described here.

## 5. Decisions

| ID | Decision | Consequence | Approval |
|---|---|---|---|
| D1 | Use the Dukascopy archive, not the chart endpoint | Complete 2023 series; new data version | Human approved the data-source path, 2026-11-09 |
| D2 | Re-run 2023 once under SR8-A | SR8's no-reuse clause is superseded because SR8 produced no valid evidence; 2023 is consumed once by this run | Human approved "obtain a complete BTC 2023 ... source and re-run the frozen H3 test", 2026-11-09 |
| D3 | Keep the SR3/SR7 H3 long/short configs unchanged | The result tests robustness, not a new rule | Frozen; no approval needed |
| D4 | Do not open 2024 | Validation stays one-shot for a future frozen candidate | Human constraint |

## 6. Acceptance criteria

- Source: 525,600 rows; header `Date,Open,High,Low,Close,Volume`; strictly
  increasing; zero duplicates; 8,760 full 1h hours; 100% 1h coverage.
- The archive and chart sources match on OHLC for every overlapping minute.
- Both preflights pass at 6 bps.
- Both base cells, both 9 bps cells, and both qualification records run.
- The buffer criteria are evaluated exactly as in SR8: ≥5 trades per side,
  net > 0 and PF > 1.0 at 6 bps, max DD ≤5%, no halt, PF > 0 at 9 bps.
- 2024 is not opened.
- Rust/Python/documentation gates pass; records are updated.

## 7. Work breakdown

| Step | Work | Owner | Paths |
|---|---|---|---|
| 1 | Write the charter and plan | Quant Researcher / Orchestrator | `docs/governance/2026-11-09/`, this file |
| 2 | Add the archive acquisition tool and tests | Data Steward / Test Engineer | `tools/improvement/dukascopy_archive.py`, `test_dukascopy_archive.py` |
| 3 | Build the 2023 source and quality record | Data Steward | `data/sources/BTCUSD_M1_2023-01-01_2023-12-31.csv`, `docs/reports/2026-11-09/` |
| 4 | Add the data-change record and version | Data Steward | `docs/config/2026-11-09/data-change-SR8A-01.md` |
| 5 | Add frozen configs | Quant Developer | `config/sr8a/` |
| 6 | Run preflights and cells | Quant Developer | `data/improvement/2026-11-09/SR8-A-H3-ARCHIVE/` |
| 7 | Review, decide, record | Result Analyst / Quant Validator | run record, `docs/improvement/2026-11-09/` |
| 8 | Update registry, baselines, changelog | Orchestrator | `docs/improvement/`, `CHANGELOG.md` |
| 9 | Verify, commit, PR, merge, clean up | Engineering | branch `feat/sr8a-h3-archive` |

## 8. Verification

- `python3 tools/improvement/test_dukascopy_archive.py`
- Source audit: row count, ordering, duplicates, full 1h hours, overlapping
  OHLC equality against the chart source.
- `python3 tools/improvement/feasibility.py` on both cells at 6 bps.
- `target/release/quantrule backtest run` at 6 and 9 bps.
- `target/release/quantrule qualify` with `config/qualification/SR7-DEV.json`.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
  `cargo doc` with warnings denied.
- `python3 -m unittest discover -s tools/improvement -p 'test_*.py'`.
- Repository checks and `git diff --check`.

## 9. Risks and recovery

- **Data identity.** The archive is a different extraction of the same provider
  series. Mitigation: exact OHLC match on all overlapping minutes; volume
  rounding only; the H3 rule does not read volume.
- **Local-only source.** The CSV is not committed. Mitigation: record its
  SHA-256, the URL pattern, the decoder, and the acquisition tool so it can be
  rebuilt.
- **Reuse of 2023.** The original charter barred reuse. Mitigation: this plan
  and the charter amendment are approved before the run; SR8's partial result
  remains recorded; 2023 is consumed once by SR8-A.
- **Underpowered buffer.** One year may still produce fewer than 5 trades per
  side. That is a bounded result, not a reason to relax the floor.
- **Archive outage.** Missing daily files are retried; if any remain missing,
  the run stops as data-ineligible rather than filling gaps.

## 10. Stop conditions

Stop when:

- the source fails the 95% 1h coverage gate;
- an archive day is missing and cannot be recovered;
- the archive and chart OHLC differ beyond volume rounding;
- a preflight fails;
- the H3 configs are not byte-identical to the frozen SR7 configs;
- 2024 access is requested;
- the human stops the programme.

A buffer criteria failure is a result, not a stop before the run. If either side
fails, the stack-fade family closes and 2023 is not reused.

## 11. Outcome

| Check | Observed |
|---|---|
| Archive days | 365 / 365 downloaded |
| Rows | 525,600 |
| Row-based 1h coverage | 8,760 / 8,760 = 100% |
| Flat zero-volume bars | 38,948 (7.41%) |
| Longest flat run | 1,260 minutes |
| Archive-only minutes | 38,121 |
| Archive-only flat placeholders | 38,093 (99.93%) |
| Hours full only in the archive | 3,406, all containing a flat bar |
| Merged genuine-minute coverage | 61.4% |
| OHLC mismatches vs chart source | 615 / 487,479; max $105.40 |
| Archive tick probe (2023-07-29 12h) | 0 bytes |
| H3 run | not run |
| 2024 | not opened |

The archive's row count is complete, but the added rows are flat zero-volume
placeholders, not market data. The source is rejected; the SR8-A buffer stops as
data-ineligible. No H3 outcome was produced, and 2023 was not consumed by a
strategy run.

The next step is a genuine 24/7 BTC 1m source (for example a spot exchange
export) or stopping the stack-fade family. Both require a new human decision.
