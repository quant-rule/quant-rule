# MR5 Data Steward Review — Validation Freshness and Instruments

- Date: 2026-11-08 (UTC)
- Status: Proposed; input to the MR5 charter
- Owner: Data Steward
- Reviewer: Quant Validator (read-only challenge)
- Partition contract: [`data/partitions.json`](../../../data/partitions.json)
- Claim type: data-admissibility and freshness review; no model result

## 1. Question

Is the existing `model_validation` window fresh enough for the first MR5
confirmation, and can an additional instrument give the low-frequency cells more
power without breaking the partition contract?

## 2. Current state

| Item | State |
|---|---|
| `model_dev` | 2018-01-01..2020-12-31 |
| `model_validation` | 2021-01-01..2022-12-31 |
| Excluded | 2023 from every partition |
| Strategy validation | 2024; cannot be used for a model baseline |
| Portfolio holdout | 2025; cannot be used for a model baseline |
| Declared instruments | BTCUSD, ETHUSD, XAUUSD, EURUSD, ESANDP500 |
| Prior model validation use | Three accepted compression baselines; the M2, M3, and M4 programmes did not open validation |

The model validation partition is **not fresh**. Any MR5 confirmation is a
further one-shot use of the same 2021–2022 window. The Quant Validator must
record that residual data-snooping risk, and the human must accept it.

## 3. Instrument review

| Instrument | Partition status | Prior model use | Is it fresh for MR5? |
|---|---|---|---|
| XAUUSD | Declared | Compression baselines and M2–M4 | No |
| BTCUSD | Declared | Compression baselines and M2–M4 | No |
| ESANDP500 | Declared | Compression baselines and M2–M4 | No |
| EURUSD | Declared | M2–M4 | Not fully fresh |
| ETHUSD | Declared | Earlier compression work and the ETH strategy line | Not fully fresh; price-only use is still useful for replication |

ETHUSD is already in `data/partitions.json`, so adding it to an MR5 grid is not
a partition change. It adds a fifth price series for the price-only theses and
raises the replication count. It does **not** create a fresh holdout, because
the earlier compression work touched ETH. The volume family remains
inadmissible for ETH on 2018–2020, so MR5 must remain price-only if ETH is
included.

## 4. Fresh-instrument options

| Option | Work required | Consequence |
|---|---|---|
| Add ETHUSD to MR5 | Grid/instrument list only; it is already declared | More cross-sectional sample; not a fresh holdout |
| Add a new instrument (for example AAPL, COPPER, or another M1 source with 2021–2022 coverage) | Data Lifecycle: source, alias, partition, coverage, admissibility, data-change record | A genuinely fresh cross-sectional holdout; the strongest option for a 10-cell confirmation |
| Use 2024 for model validation | Partition change | Not allowed; 2024 is the strategy validation partition |
| Use 2025 for model validation | Partition change | Not allowed; 2025 is the portfolio holdout |
| Repair and use 2023 | Data Lifecycle | 2023 is excluded by the workflow reset; a repair is a new decision |

The local source workspace contains additional M1 files, but an instrument is
not admissible until the Data Steward records its source, version, aliases,
coverage, and quality. An unlisted CSV is not a dataset.

## 5. Recommendation

1. **Run the first MR5 confirmation with five primary base cells** on the
   existing declared instruments. Use XAU, BTC, S&P 500, EURUSD, and ETHUSD for
   the price-only theses; ETHUSD is already declared and adds replication.
2. **Disclose the validation reuse.** The 2021–2022 window was used by the
   compression baselines; the MR5 confirmation is not a fresh holdout.
3. **Do not run a 10-cell confirmation on the current data.** The MDE worksheet
   shows the low-frequency cells are not powered at ten cells. A 10-cell
   confirmation needs a fresh instrument with 2021–2022 coverage, added through
   the Data Lifecycle workflow, before the charter is approved.
4. **Keep the volume family out of MR5.** ETH and BTC volume are not admissible
   on the development partition; the MR5 theses are price-only.

## 6. Limitations

- This review does not add an instrument or change a partition. It records a
  recommendation and the work a change would require.
- The ETHUSD freshness depends on the exact scope of the earlier compression
  evidence; it is not a guaranteed fresh holdout.
- The source files are local workspace data. Their hashes and versions are
  recorded at the Data Lifecycle step, not in this review.
- A fresh instrument does not remove the need for the correction; it adds
  independent sample, not certainty.
