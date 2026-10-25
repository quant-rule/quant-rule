# Research Dataset Verification

- Date: 2026-10-25 (UTC)
- Dataset: `harden-analytics`
- Data version: `test:harden-analytics:60:0-260`
- Source: `crates/quantrule-shell/tests/fixtures/state_m1.csv`
- Root: `data`

## Generated files

`data/processed/harden-analytics/` contains:

| File | Bytes |
|---|---:|
| `codes.csv` | 5,792 |
| `data_dictionary.csv` | 58,655 |
| `data_dictionary.md` | 65,574 |
| `layer1.csv` | 117,787 |
| `layer2.csv` | 135,068 |
| `layer3.csv` | 314,967 |
| `manifest.json` | 775 |
| `README.md` | 962 |

The manifest records `layer1=333`, `layer2=333`, `layer3=260`, and `raw.60=260` rows.

## Commands and evidence

```bash
cargo run -q -p quantrule-shell -- research export \
  crates/quantrule-shell/tests/fixtures/state_m1.csv harden-analytics \
  --data-version test:harden-analytics:60:0-260 --root data
```

```bash
cargo run -q -p quantrule-shell -- research summary \
  crates/quantrule-shell/tests/fixtures/state_m1.csv
```

Two exports to the same root were compared byte for byte and were identical. Two summary runs produced identical output.

## Limitations

- `data/` is gitignored; this artifact is local evidence, not a committed dataset.
- The source is the 260-bar shell fixture, not a market-history dataset. The 100k path remains covered by the ignored benchmark.
- The manifest paths are relative to the export root and change when the root changes; byte-identity is checked by exporting twice to the same root.
