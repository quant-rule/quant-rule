# MR7 Data Steward Review — Fresh Price-Only Instruments

- Date: 2026-11-08 (UTC)
- Status: Admitted for price-only MR7 cells; volume cells exclude them
- Owner: Data Steward
- Data change: [`DATA-2026-11-08-MR7-01`](../../config/2026-11-08/data-change-MR7-01.md)
- Quality record: [`mr7-data-validation.md`](../../reports/2026-11-08/mr7-data-validation.md)
- Claim type: data admissibility; no model or edge claim

## 1. Question

Can MR7 add a fresh cross-section without repeating the mined five-instrument
sample, and are the candidate instruments admissible for the opening-range
sweep/reclaim event?

## 2. Candidate review

| Instrument | Coverage 2018–2020 | Volume | Price-only admissible? | Volume admissible? |
|---|---|---|---|---|
| COPPERCMDUSD | 979,317 M1 rows; complete | zero on 904,640 rows | yes | no |
| TSLAUSUSD | 292,830 M1 rows; complete | 2020 unit shift (11,975×) | yes | no |
| XAGUSD | complete | correlated with XAU | not selected | not selected |

COPPER and TSLA are different cross-sections from the existing commodity, index,
FX, and crypto set. XAG is not selected because it is too close to XAU for a
replication test.

## 3. Decision

- Admit `COPPERCMDUSD` and `TSLAUSUSD` to all declared partitions as **price-only**
  instruments; aliases `COPPER` and `TSLA`.
- The MR7 event is a price/level mechanism, so it is admissible on both.
- The relative-volume cell `MR7.4` uses only XAU, BTC, S&P 500, EURUSD, and ETH;
  COPPER has no volume and TSLA has a unit shift.
- This is a fresh cross-section, not a fresh time period. The declared
  `model_validation` window remains a reused one-shot resource.

## 4. Why this is not a data rescue

MR7 is a new event, not a re-registration of a failed MR5/MR6 hypothesis. The
fresh instruments are a replication and data-snooping control, not a way to
rescue a failed result. No threshold, window, or event is changed after seeing
an outcome.

## 5. Records

- Partition file: [`data/partitions.json`](../../../data/partitions.json)
- Data change: [`DATA-2026-11-08-MR7-01`](../../config/2026-11-08/data-change-MR7-01.md)
- Quality report: [`mr7-data-validation.md`](../../reports/2026-11-08/mr7-data-validation.md)
- Changelog and AI log: updated with the MR7 programme.

## 6. Limitations

- The two instruments add cross-sectional sample, not a new time period.
- COPPER price rows may include contract rolls; the sweep event is a short-horizon
  price event, and the roll risk is a limitation, not a repaired defect.
- TSLA is an equity with session gaps; the London-session opening range is
  defined on the available bars and may differ from a continuous market.
- Same-agent Data Steward and Quant Validator passes; not independent.
