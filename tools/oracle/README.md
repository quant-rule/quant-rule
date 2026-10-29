# Progression replay oracle

`progression_replay.py` is the independent replay behind the Progression golden
fixtures. It implements contract §8.4 and §9 in Python from the checked-in
readings and never imports, calls, or reads the Rust implementation. It is a
developer tool, not a shell command. It requires Python 3.10 or newer and uses
only the standard library.

## Inputs

| Input | Role |
|---|---|
| `crates/quantrule-model/tests/fixtures/readings_golden.csv` | The independently generated candles, ATR 14, pivot candidates, and confirmed pivots. The replay does not recompute these readings. |

## Outputs

| Output | Regenerated content |
|---|---|
| `crates/quantrule-model/tests/fixtures/progression_golden.csv` | Every per-bar Progression value and both per-scale leg records. |
| `crates/quantrule-model/tests/fixtures/progression_legs_golden.csv` | The retained-leg ledger in creation order. |
| `crates/quantrule-model/tests/fixtures/dashboard_readings_golden.csv` | The Progression rows (`axis_code 4`); the other axes are preserved byte for byte. |
| `crates/quantrule-model/tests/fixtures/market_report_golden.txt` | The Progression axis statistics, latest state, published-leg line, and chart `stage` fields; every other line is preserved byte for byte. |

The ledger has no comment header, so its file stays byte-identical to its
pre-#197 form. The three fixtures that carry a header name this oracle and its
input.

## Check and write

Run both commands from the repository root:

```bash
python3 tools/oracle/progression_replay.py --check
python3 -m unittest discover -s tools/oracle -p 'test_*.py'
```

`--check` is the default and the CI path. It compares the rendered files with
the checked-in ones and exits nonzero on any difference. `--write` regenerates
them deliberately:

```bash
python3 tools/oracle/progression_replay.py --write
git diff -- crates/quantrule-model/tests/fixtures
```

A mismatch is investigated as a defect in the oracle or in the Rust
implementation. It is never resolved by running `--write` around a data change.
`--write` is appropriate only when a reviewed, intentional contract change
alters the expected values, and the resulting diff is reviewed in full.

## Independence and limits

- The state machine lives in `progression_model.py` and is a pure function of
  its arguments. The CLI owns all file I/O.
- The replay consumes confirmed pivots; level invalidation is not represented
  in the readings fixture, so target revision is exercised on newly confirmed
  levels.
- The author read the Rust implementation while investigating #196. The
  independence is procedural: the replay follows the contract and the D1–D10
  decisions, and a reviewer checks it against those documents. This limitation
  is recorded in the #197 transparency log.
- The oracle covers the 260-bar fixture and its unit scenarios, not the
  full-year series.
