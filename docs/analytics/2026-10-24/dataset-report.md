# Research Dataset Verification

- Date: 2026-10-24 (UTC)
- Issue: [#82](https://github.com/quant-rule/quant-rule/issues/82) — test: verify the research dataset
- Contract: [`docs/analytics/2026-10-24/research-output.md`](research-output.md)
- Test: `crates/quantrule-analytics/tests/research_verification.rs`

## 1. Scope

The verified dataset is the layer export implemented by #80: `m1.csv`, `layer1.csv`, `layer2.csv`, `layer3.csv`, `manifest.json`, `data_dictionary.csv`, `data_dictionary.md`, `codes.csv`, and `README.md`. The event, leg, level, and consensus files are the dataset's record contract and belong to the store epic (#64); they are not part of this export.

## 2. Verification table

| Check | Test | Result |
|---|---|---|
| Layer 1/2/3 row counts | `the_layer_counts_match_the_inputs` | 600 M1 + 120 M5 + 40 M15 + 10 H1 = 770 Layer 1 rows; 770 Layer 2; 600 Layer 3 |
| Layer 2 derives from Layer 1 | `layer_two_derives_from_every_layer_one_metric` | every row's four resolutions equal the mapping of its own metrics |
| Resolution range and bucket | `every_resolution_is_in_range_and_matches_its_bucket` | every continuous value is in `[-1, +1]`; every ordinal equals the bucket; null pairs carry a reason |
| As-of alignment | `every_block_is_the_latest_closed_higher_bar` | every block is the latest closed bar; every higher close is at or before the base close; staleness matches |
| Byte-identical export | `the_export_is_byte_identical_and_complete` | two renders are equal |
| Column widths | `the_export_is_byte_identical_and_complete` | 61 / 73 / 356 columns; every row has the header width |
| Dictionary completeness | `the_export_is_byte_identical_and_complete` | 496 rows: 6 raw + 61 + 73 + 356 |
| Code table | `the_export_is_byte_identical_and_complete` | 95 rows; the resolution, reason, and subtype registries are present |
| Sample analysis from the CSVs | `a_sample_analysis_matches_the_exported_rows` | the M1 direction count, mean, ordinal histogram, and direction/momentum correlation recomputed from `layer2.csv` match the in-memory summary |
| Causality | `the_dataset_is_causal` | the first 300 Layer 3 rows of a 300-bar build equal the first 300 rows of a 360-bar build |

The tests build their own deterministic M1 series through `TimeframeSet::full`, so they do not read the shell's fixtures or the network.

## 3. 100k benchmark

Command:

```bash
cargo test -p quantrule-analytics --release --test research_verification \
  a_100k_dataset_export_benchmark -- --ignored --nocapture
```

Release result:

| Metric | Value |
|---|---|
| M1 candles | 100,000 |
| Layer 1 rows | 128,332 |
| Layer 2 rows | 128,332 |
| Layer 3 rows | 100,000 |
| Layer build | 4.122 s |
| Export render | 4.105 s |
| `layer3.csv` | 228,808,595 bytes |
| All files | 368,567,856 bytes |

The render holds the files in memory; the byte figures are the output size, not the process peak. The shell writes each file through a temporary sibling and an atomic rename.

## 4. Limitations

- The record files are defined in the contract but are not written by this export; their persistence belongs to the store epic (#64).
- The sample analysis reads `layer2.csv`, not a SQLite store. The store's own verification (#74) repeats the check on persisted tables.
- The 100k run reports the build and render time and the output size; it does not instrument the allocator's peak.
- The dataset contains no wall-clock value; the manifest records the caller's data version and the contract dates only.

## 5. Reproduce

```bash
cargo test -p quantrule-analytics --test research_verification
cargo test -p quantrule-analytics --release --test research_verification \
  a_100k_dataset_export_benchmark -- --ignored --nocapture
cargo run -q -p quantrule-shell -- research export crates/quantrule-shell/tests/fixtures/state_m1.csv demo \
  --data-version test:demo:60:0-260 --root target/research-demo
cargo run -q -p quantrule-shell -- research summary crates/quantrule-shell/tests/fixtures/state_m1.csv
```
