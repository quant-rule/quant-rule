# Simulation harness

`tools/simulation/` builds chart-first, hindsight-labeled research runs on a
recorded M1.3 model bundle. A simulation run is exploration: it proposes a
pattern and checks it on a later, unmarked window. It is not a backtest, a fill
simulation, or evidence of an edge.

The workflow is [`docs/workflows/simulation-research.md`](../../docs/workflows/simulation-research.md).
The execution plan is
[`docs/execution-plans/2026-11-10/simulation-runs.md`](../../docs/execution-plans/2026-11-10/simulation-runs.md).

## Tools

| File | Responsibility |
|---|---|
| `bundle.py` | Load one M1.3 bundle or merge per-year reports; validate readings, events, levels, and the M1.3 extended columns |
| `window_select.py` | Deterministic, seeded, year-stratified window selection with overlap exclusion |
| `render.py` | Clean chart with candles, volume, EMAs, one indicator panel, event markers, and labels for the marks |
| `marks.py` | The annotator's seven-column mark schema, validation, and atomic round-trip |
| `oracle.py` | Deterministic hindsight labeler: best good and most tempting bad entry per side |
| `review.py` | Mark counts, MFE/MAE to the rule exit, candle-shape context, and the candidate-rule table |
| `check.py` | Event study on the unmarked window: filtered vs. unfiltered control, 6 bps proxy, half-years |
| `contact_sheet.py` | Not a separate file; `render.contact_sheet` builds the 3x4 sheet |
| `run.py` | Orchestrates select -> render -> oracle -> review for one run |

`window_select.py` is not named `select.py` because a module of that name
shadows the Python standard library `select` module used by `subprocess`.

## One run

```bash
python3 tools/simulation/run.py \
  --run-config data/simulation/2026-11-10/SIM-02-level-rejection/config.json \
  --out data/simulation/2026-11-10/SIM-02-level-rejection
```

The run writes:

```text
manifest.json       premise, model, registry, bundles, seed, criteria, hashes
windows.json        deterministic window selection
charts/             PNGs + manifest.json with the plot geometry + index.csv
marks-ai.csv        the seven-column marks
marks-detail.csv    entry/exit bars, ATR, MFE/MAE, perfect exit, good/bad
review.md           counts, round trips, context, candidate rule
reproduce.md        the command and input identity
decision.md         the decision skeleton
contact-sheet.png   3x4 review sheet
check.json/check.md after check.py
```

`marks-ai.csv` uses the same schema as the human annotator:

```csv
chart,bar,marked_time_utc,action,quality,side,note
sim-02-w001.png,34567,2018-01-05T18:45:00Z,entry,good,long,SIM-02 good long ...
```

The chart manifest repeats `plot_left`, `plot_width`, `price_top`,
`price_bottom`, `price_low`, `price_high`, `candles`, and the per-bar
`[bar, timestamp, high, low]` rows. The existing click annotator can open the
same `charts/` directory and correct the AI marks.

## The check

```bash
python3 tools/simulation/check.py \
  --check-config data/simulation/2026-11-10/SIM-02-level-rejection/check-config.json \
  --out data/simulation/2026-11-10/SIM-02-level-rejection
```

The check applies the event and filter at the event's `available_bar_index`,
enters at the next bar open, exits at the close after the declared horizon, and
subtracts the cost proxy. It compares the filtered events with the unfiltered
event control and records the half-year means. A survivor has to beat the
control on mean and hit rate and hold in at least three half-years; otherwise
the run is `no_pattern`. The check never reads 2023–2025.

## M1.3 report

The model report is capped at 600,000 source candles, so the 2018–2022 1m
source is reported per year:

```bash
for year in 2018 2019 2020 2021 2022; do
  ./target/release/quantrule report model \
    --config data/simulation/_data/M1.3-2018-2022/config/run-$year.json \
    --out data/simulation/_data/M1.3-2018-2022/report-$year
done
```

Each report is 15m only (`model_timeframes: ["15m"]`) and carries the 121 M1.3
reading columns, the Fib/band events, and the corrected sequence availability.
The run config lists the five `model/` directories; `bundle.load_bundles` gives
them one global bar identity, then filters to the label window.

## Tests

```bash
python3 -m unittest discover -s tools/simulation -p 'test_*.py'
```

The tests cover the bundle contract, deterministic selection, mark validation
and round-trip, the oracle criteria on synthetic paths, PNG determinism, the
review math, the check mechanics, and one end-to-end fixture run. Pillow is
required for rendering; the render and run tests skip when it is absent.

## Boundaries

- Read-only over recorded model artifacts; no model, strategy, or execution
  recomputation.
- The marks use future bars by design. They are labels, not signals.
- The check is a screen. A survivor goes to Strategy Research with costs, a
  frozen config, and a pre-registration.
- No new dependency; Pillow is the only non-stdlib import.
- No wall clock in chart output. The same seed, window, and marks produce the
  same windows, marks, and PNG bytes.
