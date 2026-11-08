# MR7 Data Validation — COPPERCMDUSD and TSLAUSUSD

- Date: 2026-11-08 (UTC)
- Status: Validated for price-only model research
- Owner: Data Steward
- Data change: [`DATA-2026-11-08-MR7-01`](../../config/2026-11-08/data-change-MR7-01.md)
- Claim type: data quality evidence; no model result

## 1. Sources

| Instrument | File | Header | Rows 2018–2020 | First | Last |
|---|---|---|---:|---|---|
| COPPERCMDUSD | `COPPERCMDUSD_M1_2018-01-01_2024-12-31.csv` | Date,Open,High,Low,Close,Volume | 979,317 | 2018-01-01 23:05 | 2020-12-31 21:14 |
| TSLAUSUSD | `TSLAUSUSD_M1_2018-01-01_2024-12-31.csv` | Date,Open,High,Low,Close,Volume | 292,830 | 2018-01-02 14:30 | 2020-12-31 20:59 |

## 2. Integrity

| Check | COPPERCMDUSD | TSLAUSUSD |
|---|---|---|
| Malformed rows | 0 | 0 |
| Non-increasing timestamps | 0 | 0 |
| Duplicate timestamps | 0 | 0 |
| Zero-volume rows | 904,640 | 0 |
| Rows 2018 / 2019 / 2020 | 333,074 / 310,756 / 335,487 | 97,200 / 97,558 / 98,072 |

Every row parses and the timestamps are strictly increasing. Gaps are recorded
by the data engine, not filled. The source files are immutable; the run records
the per-year data version.

## 3. Volume audit

Monthly median M1 volume, maximum/minimum per year:

| Instrument | 2018 | 2019 | 2020 | Reading |
|---|---:|---:|---:|---|
| COPPERCMDUSD | 0× | 0× | 0× | volume absent |
| TSLAUSUSD | 6.0× | 2.6× | 11,975.4× | 2020 unit/contract shift |

COPPER volume is zero for most rows; TSLA volume is unit-unstable in 2020.
Neither supports a volume, participation, or flow claim. Both are admitted for
price/level events only.

## 4. Partition

The two instruments are added to every declared partition with the aliases
`COPPER` and `TSLA`. `check_partitions.py` passes with 7 instruments and 6
aliases. The holdout flags and windows are unchanged.

## 5. Admissibility

- **Admitted:** price-only model research, including the opening-range
  sweep/reclaim event.
- **Excluded:** volume, participation, and flow events; `MR7.4` uses only the
  five existing instruments.
- **Not a fresh time period:** the development and validation windows are the
  declared partitions; the instruments are a fresh cross-section, not a fresh
  holdout.

## 6. Reproduction

```bash
python3 - <<'PY'
# The audit streams the source CSV, filters 2018-01-01..2020-12-31, and
# counts rows, zero-volume rows, duplicate timestamps, and monthly medians.
PY
python3 tools/check_partitions.py
```

The observed counts match the table above. The source file SHA-256 values are
recorded in the MR7 development record.
