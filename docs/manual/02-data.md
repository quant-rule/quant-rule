# 2. Data

- Status: Current
- Audience: operator preparing candles before a model or coupled run

## Goal

Upload one candle file, prove its content identity, inspect its coverage, and
decide whether the dataset is admissible as an input.

## Before you start

| Need | Value |
|---|---|
| Candle file | OHLCV CSV with a UTC timestamp column |
| Profile | The profile that should own the dataset |
| Stack | Healthy gateway and data service |
| Limit | The upload limit configured by the data service |

A dataset is shared by content. The same bytes in another profile return the
same `data_version`; a changed candle produces a different one.

## Steps

| # | Shell | Browser | Expect |
|---:|---|---|---|
| 1 | `quantrule dataset upload candles.csv --name btc-1m` | **Workbench → Data** → CSV file → Upload | The record and its `data_version` |
| 2 | `quantrule dataset upload candles.csv --tolerant` | **Workbench → Data** → Skip bad rows → Upload | A quality report with accepted and skipped rows |
| 3 | `quantrule dataset list` | **Datasets** → table | One row per dataset, newest records reachable |
| 4 | `quantrule dataset show <data_version>` | **Datasets** → Details | Object key, SHA-256, coverage, bytes, quality, created time |
| 5 | `quantrule inspect candles <data_version>` | **Inspect** → Candles | Candle count, first and last bar, bytes |
| 6 | `quantrule inspect timeframes <data_version>` | **Inspect** → Timeframes | Bars and partial buckets per timeframe |
| 7 | `quantrule inspect events <data_version> --timeframe 5m` | **Inspect** → Events | The published event batch for that timeframe |

Inspection reads the stored bytes and the published records. It never starts a
run, changes a dataset, or constructs a second data version.

## Tolerant upload semantics

The ordinary upload refuses the file on the first invalid row. The tolerant
upload keeps the valid rows and reports what it skipped:

| Field | Meaning |
|---|---|
| `rows_read` | Rows the parser read |
| `accepted` | Rows written to the dataset |
| `rejected` | Rows refused |
| `duplicates` | Repeated timestamps |
| `out_of_order` | Rows that break ascending time |
| `invalid` | Rows with a bad field count, number, or timestamp |
| `skipped` | Line number and stable reason for each skipped row |

A tolerant upload is a deliberate data decision, not a way to hide a bad file.
The quality report is part of the dataset record. Two tolerant uploads of the
same bytes return the same `data_version`.

## What you see

| Field | Meaning |
|---|---|
| `data_version` | Content identity of the normalized dataset |
| `name` | The operator-facing label supplied at upload |
| `candles` | Accepted candle count |
| `first_bar`, `last_bar` | Coverage in UTC seconds |
| `bytes` | Stored object size |
| `object_key`, `sha256` | Object-store location and content hash |
| `quality` | The tolerant report, or `none` |
| `created_at` | Server-assigned UTC creation time |

The timeframe ladder reports complete bars and partial buckets separately. A
partial bucket is not a complete candle and is never silently treated as one.

## What it proves

**Diagnostic.** Upload and inspection prove content identity, coverage, and
parse quality. They do not produce evidence, a canonical digest, or a baseline;
a clean dataset is a prerequisite, not a result.

## Failure

| Code or symptom | Cause | Next action |
|---|---|---|
| `dataset.invalid` | Blank name, bad UTF-8, or a rejected candle | Fix the file or choose the tolerant path deliberately |
| `dataset.not_found` | The `data_version` is unknown in this profile | Re-run `dataset list`; do not construct an id |
| `upload.too_large` | The body exceeds the configured limit | Split the source or raise the limit with the human |
| `service.unavailable` | The data service or object store is down | Read `health`; stop until the dependency returns |
| Identical upload returns the same row | Content addressing, not a failed write | Reuse the existing version |
| Different upload returns a new version | The bytes changed | Record which version the run uses |

## Related

- [First run](01-first-run.md)
- [Model](03-model.md)
- [Data Lifecycle workflow](../workflows/data-lifecycle.md)
