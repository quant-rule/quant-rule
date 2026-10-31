# Run the Full Pipeline

- Date: 2026-10-29 (UTC)
- Status: Historical; superseded by [`docs/tutorials/2026-10-30/unified-ladder.md`](../2026-10-30/unified-ladder.md) for the current `ladder-context` strategy
- Applies to: QuantRule v0.3.0, model 0.3.0
- Last verified: 2026-10-30 against the v0.3.0 release
- Audience: a human, or an agent acting as Quant Researcher, Quant Developer, or
  Quant Validator
- Claim type: procedural. This tutorial makes no profitability claim.

This dated tutorial describes the retired `axis-agreement` strategy. It is
preserved as a record of the v0.3.0 procedure; use the 2026-10-30 unified-ladder
tutorial for the current command surface.

Note (2026-10-31): the example config named below was renamed to
`config/examples/btcusd-2024/`. This record keeps the original SOLUSD path and
observed values; they describe the run that was actually performed.

It runs every QuantRule stage through shell commands: the data
engine, the four-axis model, the research engine, the strategy, the execution
and portfolio engines, the SQLite store, and the whole-lab summary. It also
contains one lab for each quant role. Every product step is a command; the
build, test, and Git gates are marked separately.

## 1. What the lab produces

One lab result is one folder:

```text
data/lab-result-YYYY-MM-DDTHH-MM-SSZ/
  model/          summary.txt report.json price-axes.png events.png provenance.json
  strategy/       summary.txt report.json decisions.csv price-decisions.png provenance.json
  execution/      summary.txt report.json orders.csv fills.csv rejections.csv price-fills.png provenance.json
  portfolio/      summary.txt report.json trades.csv equity.csv equity.png drawdown.png provenance.json
  analytical/     summary.txt report.json axes.csv relations.csv provenance.json
  summary.txt
```

The five directories are **report bundles** from one `backtest`. The root
`summary.txt` describes the whole lab. It is not a sixth engine, and it does not
recompute the model.

| Term | Meaning in this tutorial |
|---|---|
| **Report bundle** | One engine's directory under the lab. Its `report.json` is the machine-readable report; `summary.txt` is the human-readable view; `provenance.json` records the inputs and artifacts. |
| **Data version** | The caller-supplied identity of the input series: source, timeframe, and coverage. The engine records it and never infers it from a path. |
| **Lab result folder** | `data/lab-result-YYYY-MM-DDTHH-MM-SSZ` in UTC. It holds the five bundles and the root `summary.txt`. |
| **Availability** | The bar on which a measurement becomes knowable. A confirmed pivot or divergence records both when it occurred and when it became available. |

The pipeline is:

```text
INPUT -> DATA ENGINE -> MODEL ENGINE -> OUTPUT
```

The shell wraps the pipeline. `fetch` reads a source; the data engine builds the
timeframe ladder; the model engine publishes numbers; strategies consume those
numbers. The model publishes Direction, Momentum, Structure, and Progression
separately and never averages them into one score. A cross-axis agreement is a
count.

## 2. Before you start

### Actor labels

| Label | Meaning |
|---|---|
| **Either** | A human or an agent runs the command. |
| **Agent** | The agent runs the command and records the output; a human reviews the checkpoint. |
| **Human** | The human makes the decision or accepts the handoff. |
| **Developer gate** | Outside the product shell: build, tests, code review, Git. |

### Build the binary

Run from the repository root:

```bash
cargo build --release -p quantrule-shell
QR=./target/release/quantrule
"$QR" help | head -3
```

Expected:

```text
QuantRule shell commands:
  candles <csv-path>            Read a CSV and print the retained candle window as NDJSON.
  candles-tolerant <csv-path>   Read a CSV, skip bad rows, and report quality on stderr.
```

`QR` is used so every command is copy-pasteable without changing `PATH`. If
`quantrule` is already on `PATH`, use it instead.

Keep one shell session. `QR` is set here; `CSV` is set at the start of section 3;
`lab`, `lab2`, and `lab3` are set in sections 7 and 9. Running a later section in
a fresh shell leaves those variables unset.

### Get the two data files

The examples use two slices of the same SOLUSD series:

| File | M1 candles | Used for |
|---|---:|---|
| `data/SOLUSD_M1_2025-01-01_2025-02-28.csv` | 84,108 | The 100,000-candle-capped commands |
| `data/SOLUSD_M1_2025-01-01_2025-12-31.csv` | 522,633 | The full-year backtest |

If the files are absent, fetch them:

```bash
test -f data/SOLUSD_M1_2025-01-01_2025-02-28.csv || \
  "$QR" fetch sol/usd 2025-01-01 2025-02-28 \
    --csv data/SOLUSD_M1_2025-01-01_2025-02-28.csv

test -f data/SOLUSD_M1_2025-01-01_2025-12-31.csv || \
  "$QR" fetch sol/usd 2025-01-01 2025-12-31 \
    --csv data/SOLUSD_M1_2025-01-01_2025-12-31.csv
```

The fetch summary has this shape:

```text
symbol=SOL/USD timeframe=1m start=2025-01-01 end=2025-02-28
rows=<n> accepted=<n> rejected=<n> pages=<n>
first=<UTC timestamp> last=<UTC timestamp>
csv=data/SOLUSD_M1_2025-01-01_2025-02-28.csv
png=none
```

The row and page counts come from the provider. The two-month file used in this
tutorial has 84,108 data rows. If the network is unavailable and the full-year
file exists, slice it locally:

```bash
awk -F, 'NR==1 || $1 < "2025-03-01"' \
  data/SOLUSD_M1_2025-01-01_2025-12-31.csv \
  > data/SOLUSD_M1_2025-01-01_2025-02-28.csv
```

`data/` is not committed. The CSV contract is the header
`Date,Open,High,Low,Close,Volume`, UTC timestamps in
`YYYY-MM-DD HH:MM:SS+00:00`, and strictly increasing rows for the strict reader.

### Know the input caps

| Commands | Maximum M1 candles | Tutorial input |
|---|---:|---|
| `market-state`, `market-dashboard`, `market-chart`, `axis`, `events`, `legs`, `model-series`, `store`, `research` | 100,000 | Two-month slice |
| `market-report`, `strategy-run` | 600,000 | Either slice |
| `report`, `backtest` | `data.json.max_candles`, at most 600,000 | Full year, 600,000 |
| `candles`, `candles-tolerant`, `timeframes` | Reads the file; displays 1,000 per series | Two-month slice |

An over-cap input fails; it is never silently truncated.

### Determinism rules

- The same input and the same configuration produce the same bytes.
- All timestamps are UTC. The folder uses `YYYY-MM-DDTHH-MM-SSZ`; `--created-utc`
  uses `YYYY-MM-DDTHH:MM:SSZ`.
- The engine never reads the clock. The shell passes `--created-utc` explicitly.
- The caller supplies `--data-version`; the engine records it and never derives
  it from the path.
- The model and research computations read only explicit inputs: no network, no
  clock, no ambient configuration. The store is shell-owned; its database path
  and dataset identity are explicit arguments.
- Redirect large NDJSON to a file before slicing it. A pipe that closes early
  makes the writer fail with `Broken pipe`.

### Interactive or non-interactive

Every command below can be typed at the interactive prompt. The non-interactive
form is `quantrule <command> ...`; the output is identical. Interactive errors
keep the session open; non-interactive errors exit with status 1. Type `quit` to
exit the interactive shell.

### Help

The shell has one entry point for the reference: `help` prints the short map;
`help <command>` prints one command contract; `help all` prints every command and
option. `help glossary`, `help tutorial`, and `help roles` are the reference
topics. The tutorial's section names match the `help tutorial` map.

**Checkpoint.** `"$QR" help` lists the commands and both data files exist. Stop
if the binary is missing or a file is absent and the fetch cannot run.

## 3. Data engine

Actor: Either.

The shell performs source access. The data engine normalizes candles, builds the
timeframe ladder, orders it, and reports gaps. It never fetches.

```bash
CSV=data/SOLUSD_M1_2025-01-01_2025-02-28.csv

"$QR" candles "$CSV" > data/tutorial-candles.ndjson
wc -l < data/tutorial-candles.ndjson
head -1 data/tutorial-candles.ndjson
```

Expected:

```text
1000
{"series_index":83108,"window_index":0,"datetime":"2025-02-28T07:20:00Z","open":128.73,"high":128.77,"low":128.57,"close":128.67,"volume":0.0528}
```

`candles` reads strictly. A malformed row, duplicate timestamp, or out-of-order
timestamp aborts the command before any NDJSON is written. The displayed window
is the last 1,000 candles.

```bash
"$QR" candles-tolerant "$CSV" > /dev/null
```

Expected on stderr:

```text
quality rows_read=84108 accepted=84108 rejected=0 duplicate=0 out_of_order=0 invalid=0 evicted=83108 retained=1000
```

`candles-tolerant` skips row-level failures and reports every skipped row. A
run with skipped rows still exits 0; a missing file or bad header aborts.

```bash
"$QR" timeframes "$CSV" \
  > data/tutorial-timeframes.ndjson \
  2> data/tutorial-timeframes.txt
wc -l < data/tutorial-timeframes.ndjson
grep '^aggregation' data/tutorial-timeframes.txt
```

Expected:

```text
4000
aggregation timeframe=1m retained=1000 partial=0 gaps=7
aggregation timeframe=5m retained=1000 partial=7 gaps=7
aggregation timeframe=15m retained=1000 partial=8 gaps=7
aggregation timeframe=1h retained=1000 partial=8 gaps=6
```

The report also contains `partial` and `gap` lines. M1 is the source series;
M5, M15, and H1 are derived by the data engine. A partial bucket is dropped and
counted, never printed as a candle. A gap is reported, never filled.

**Checkpoint.** 84,108 rows read, zero rejected, four series of 1,000 candles,
and the four aggregation lines above. Stop if the counts differ or the command
aborts.

## 4. Model engine

Actor: Either.

The model consumes closed candles and publishes numbers. The shell maps stable
codes to words; it never recomputes an axis.

### Dashboard

```bash
"$QR" market-dashboard "$CSV" --window latest \
  > data/tutorial-dashboard.txt
wc -l < data/tutorial-dashboard.txt
sed -n '1,12p' data/tutorial-dashboard.txt
```

Expected: 45 lines, beginning with:

```text
market-dashboard  window=latest  ladder=1m,5m,15m,1h  model=0.3.0  view=2026-10-25
as-of  1m 2025-02-28T23:59:00Z | 5m 2025-02-28T23:55:00Z | 15m 2025-02-28T23:45:00Z | 1h 2025-02-28T23:00:00Z
cross-TF agreement  2/3 axes have a strict majority (direction bullish 3/4, momentum no majority, structure bullish 3/4) · coverage 4/4
churn  max 20 state changes (1h progression) · threshold 6 · history 4/4 (complete) · class Unstable

      DIRECTION -1..+1             MOMENTUM decel..accel        STRUCTURE bear..bull         PROGRESSION against..terminal
legend  ● position · small • medium ● large  ■ solid □ dashed  — unavailable (r<code>)
  1m  ────────●── 100% ●■          ──────●────  37% •■          ──────────● 100% ●■          ────────●──    — —■
  5m  ─────────●─  60% •■          ────●──────  76% ●■          ──────────● 100% ●■          — r3          4% ·□
  15m ──────────●  37% •■          ──────●────  66% •■          ──────────● 100% ●■          ────────●──  46% •■
  1h  ───●─────── 100% ●■          ─────●─────  83% ●■          ─────●─────  15% ·■          ●──────────  43% •■
```

The four tracks are separate. The agreement line is a count over Direction,
Momentum, and Structure, not an average. A missing value is `—` with a reason
code; a valid zero is printed as `0%`.

### State

```bash
"$QR" market-state "$CSV" --window latest > data/tutorial-state.json
python3 - <<'PY'
import json
d = json.load(open("data/tutorial-state.json"))
print(sorted(d))
print(d["ladder"])
print([t["timeframe"] for t in d["timeframes"]])
PY
```

Expected:

```text
['aggregates', 'consensus', 'ladder', 'timeframes']
['1m', '5m', '15m', '1h']
['1m', '5m', '15m', '1h']
```

The object carries the per-timeframe axes, diagnostics, context, consensus
counts, event summary, and the cross-timeframe aggregates.

### One axis

```bash
"$QR" axis "$CSV" direction --window latest \
  > data/tutorial-axis-direction.ndjson
wc -l < data/tutorial-axis-direction.ndjson
head -1 data/tutorial-axis-direction.ndjson
```

Expected: four lines, one per timeframe, beginning with:

```json
{"axis":"direction","bar":999,"percentile":90.07936507936508,"readiness":2,"readiness_label":"ready","reason":null,"reason_label":null,"state":1,"state_age_bars":47,"state_changes_50":1,"state_label":"bullish","timeframe":"1m","value":0.6815270379850848}
```

`direction`, `momentum`, `structure`, and `progression` are the four valid axis
names. Each publishes its continuous value and its own state codes.

### Events and legs

```bash
"$QR" events "$CSV" --window latest > data/tutorial-events.ndjson
wc -l < data/tutorial-events.ndjson
head -1 data/tutorial-events.ndjson

"$QR" legs "$CSV" --window latest > data/tutorial-legs.ndjson
wc -l < data/tutorial-legs.ndjson
```

Expected:

```text
7095
{"available_bar":15,"available_time":"2025-02-28T07:35:00Z","direction":-1,"occurred_bar":15,"occurred_time":"2025-02-28T07:35:00Z","primary_value":127.57,"reference_code":1001,"reference_value":127.59,"source":7,"source_label":"minor_pivots","subtype":2,"subtype_label":"cross","timeframe":"1m","type":1,"type_label":"level"}
8
```

There are three event types: **level**, **divergence**, and **signal**. An event
is appended when it becomes knowable, with both its occurrence and availability
bars. `legs` prints two lines per timeframe, one for each pivot scale. A leg's
stage can be null while its origin is unconfirmed or its target is unstable;
that is a missing measurement, not a zero.

### Model series

```bash
"$QR" model-series "$CSV" --timeframe 1h --bars 2 --window latest \
  > data/tutorial-model-series.ndjson
wc -l < data/tutorial-model-series.ndjson
python3 - <<'PY'
import json
with open("data/tutorial-model-series.ndjson") as f:
    row = json.loads(f.readline())
print(sorted(row))
PY
```

Expected:

```text
2
['bar', 'close', 'consensus', 'context', 'datetime', 'direction', 'high', 'low', 'momentum', 'open', 'progression', 'readings', 'series_index', 'structure', 'timeframe', 'volume', 'window_len', 'window_start']
```

One line is one complete model bar: the candle, six readings, the four axes with
their diagnostics, context, and consensus. Events stay separate because one bar
can carry several events.

### Chart

```bash
"$QR" market-chart "$CSV" \
  --png data/tutorial-market-chart.png \
  --timeframe 1h --candles 240 --window latest
```

Expected:

```text
png=data/tutorial-market-chart.png timeframe=1h candles=240 bytes=21922
```

The top panel shows candles and volume; the four panels below show the
continuous axis resolutions. A missing axis value is a gap, never a zero.

### Full-period report

```bash
"$QR" market-report "$CSV" \
  --data-version tutorial-solusd-m1-2025-01-01_2025-02-28 \
  --out data/tutorial-market-report
```

The output directory must be absent or empty. Expected: four artifact paths in
contract order.

```text
data/tutorial-market-report/summary.txt
data/tutorial-market-report/report.json
data/tutorial-market-report/price-axes.png
data/tutorial-market-report/events.png
```

The report covers the whole file, not the 1,000-candle tail:

```bash
sed -n '1,9p' data/tutorial-market-report/summary.txt
```

Expected:

```text
QuantRule market report
data_version: tutorial-solusd-m1-2025-01-01_2025-02-28
requested_period: full file
actual_period: 2025-01-01T14:05:00Z..2025-02-28T23:59:00Z
ladder: 1m 5m 15m 1h

[coverage]
timeframe      bars  expected  missing   share first                 last
1m            84108     84115        7  0.9999 2025-01-01T14:05:00Z  2025-02-28T23:59:00Z
5m            16816     16823        7  0.9996 2025-01-01T14:05:00Z  2025-02-28T23:55:00Z
```

The JSON envelope carries `data_version`, `model_version`, `requested_period`,
`view_contract`, and `market_report`. A missing value is `null`; the report
contains no order, fill, position, trade, equity, or P&L field.

**Checkpoint.** The dashboard has four tracks; `market-state` has four keys and
the 1m/5m/15m/1h ladder; `axis` prints four lines; `events` prints 7,095 lines
for the latest window; `legs` prints eight; `market-report` writes four files.
Stop if a JSON key is missing or an output path is wrong.

## 5. Research engine

Actor: Either.

The research engine turns the model into a dataset for analysis. It does not
place orders or compute P&L. The input cap is 100,000 M1 candles, so the
two-month slice is the correct input here.

```bash
"$QR" research export "$CSV" tutorial-solusd \
  --data-version tutorial-solusd-m1-2025-01-01_2025-02-28 \
  --root data/tutorial-research
```

Expected:

```text
exported tutorial-solusd
  data version: tutorial-solusd-m1-2025-01-01_2025-02-28
  raw rows: 84108
  layer1 rows: 107918
  layer2 rows: 107918
  layer3 rows: 84108
  files: 9
```

The export writes the raw M1 file and three layers, plus the manifest, data
dictionary, code table, and README:

```bash
find data/tutorial-research -maxdepth 3 -type f | sort
```

Expected: 9 files under `data/tutorial-research/raw/tutorial-solusd/` and
`data/tutorial-research/processed/tutorial-solusd/`.

`layer1` is one row per ladder bar; `layer2` carries the axis resolutions;
`layer3` is the as-of higher-timeframe block and the cross-timeframe aggregates.
The manifest records the data version, model and contract versions,
configuration, row counts, and data file list.

```bash
"$QR" research summary "$CSV" > data/tutorial-research-summary.txt
wc -l < data/tutorial-research-summary.txt
sed -n '1,4p' data/tutorial-research-summary.txt
```

Expected: 41 lines. The first lines:

```text
summary layer1=107918 layer2=107918 layer3=84108
axis timeframe=60 axis=direction n=83909 null=199 min=-1.0 max=1.0 mean=-0.015446425224179794 ordinals=8344,8574,9224,8525,7964,8485,9040,8332,7314,8107 ordinal_null=199
axis timeframe=60 axis=momentum n=84094 null=14 min=-0.863040067164551 max=0.8488611883078991 mean=-0.0034355844945665537 ordinals=8,405,3832,14063,24503,23704,13383,3723,464,9 ordinal_null=14
axis timeframe=60 axis=structure n=84043 null=65 min=-1.0 max=1.0 mean=-0.007317682614851921 ordinals=26265,0,649,0,0,30813,0,683,0,25633 ordinal_null=65
```

There are 16 axis lines (four timeframes times four axes) and 24 relation lines
(six axis pairs per timeframe). Ordinal codes are counted, never averaged; no
line combines the axes.

**Checkpoint.** `research export` prints `files: 9` and
`layer3 rows: 84108`; `research summary` has 41 lines. Stop if the export
reports a different row count.

## 6. Strategy

Actor: Either.

`strategy-run` stops at the decision. It creates no order, fill, position,
equity, trade, or P&L record. The strategy reads the M15 series and accepts up
to 600,000 M1 candles.

```bash
"$QR" strategy-run "$CSV" \
  --strategy axis-agreement \
  --data-version tutorial-solusd-m1-2025-01-01_2025-02-28 \
  > data/tutorial-strategy.ndjson
wc -l < data/tutorial-strategy.ndjson
head -1 data/tutorial-strategy.ndjson
```

Expected:

```text
363
{"bar":199,"datetime":"2025-01-03T16:00:00Z","direction":1.0,"momentum_force":0.38369666485977433,"side":1,"side_label":"long","strategy":"axis-agreement","structure_score":1.0,"timeframe":"15m","timeframe_secs":900,"type":"entry"}
```

Count the record types:

```bash
python3 - <<'PY'
import json
from collections import Counter
counts = Counter()
for line in open("data/tutorial-strategy.ndjson"):
    counts[json.loads(line)["type"]] += 1
print(dict(counts))
PY
```

Expected:

```text
{'entry': 182, 'exit': 181}
```

`axis-agreement` enters long when Direction, Momentum force, and Structure score
are all above `+0.1`, and short when all are below `-0.1`. Equality at a
threshold is neutral. It exits when agreement is lost, never reverses on the
same bar, and never exits at the end of the data. A Model Signal event is not a
strategy entry or exit and is never relabelled as one.

**Checkpoint.** 363 records, 182 entries, 181 exits, and the first record is an
entry at `2025-01-03T16:00:00Z`. Stop if the counts differ.

## 7. Full run

Actor: Either. The human reviews the observed result before the role labs.

The full run uses the example configuration and the full-year file. The example
is illustrative; it is not a recommendation and not out-of-sample validation.

```bash
lab=data/lab-result-2026-10-30T12-00-00Z
Before the run, validate the human envelope and read the strategy in trader
words. The run config is `run v2` and names `policy.json`; the strategy owns the
stop and reward-to-risk.

```bash
"$QR" policy check --config config/examples/solusd-2025/run.json
"$QR" strategy explain --config config/examples/solusd-2025/run.json
```

Expected: `policy ok ...` and the strategy thesis, entry, exit, and risk lines.

"$QR" backtest --config config/examples/solusd-2025/run.json --out "$lab"
```

Expected: five bundle paths in contract order.

```text
data/lab-result-2026-10-30T12-00-00Z/model
data/lab-result-2026-10-30T12-00-00Z/strategy
data/lab-result-2026-10-30T12-00-00Z/execution
data/lab-result-2026-10-30T12-00-00Z/portfolio
data/lab-result-2026-10-30T12-00-00Z/analytical
```

A non-empty output root is refused before any write. On failure, a partial
bundle is removed. The model is computed once; the analytical bundle summarizes
that model instead of recomputing it.

```bash
"$QR" lab summary "$lab" \
  --role researcher \
  --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 \
  --config config/examples/solusd-2025/run.json \
  --created-utc 2026-10-30T12:00:00Z
```

Expected:

```text
wrote data/lab-result-2026-10-30T12-00-00Z/summary.txt
```

The command records `human` as the operator by default. An agent adds
`--operator agent` to every `lab summary` invocation, so the lab says who
produced it.

The root summary has seven sections: Identity, Artifacts, Observed result,
Checks, Role notes, Limitations, and Next. The worked example produces:

| Field | Value |
|---|---|
| `requested_period` | `2025-01-01..2025-12-31` |
| `actual_period` | `2025-01-01T14:15:00Z..2025-12-31T14:30:00Z` |
| `final_equity` | `8016.143910` |
| `total_return` | `-0.198386` |
| `net_pnl` | `-1983.856090` |
| `max_drawdown` | `0.203150` |
| `trades` | `48` |
| `halted` | `true` |
| `commission` | `885.610499` |
| `execution_fees` | `0.000000` |
| `slippage` | `3.994526` |
| `funding_paid` | `4.003485` |
| `funding_received` | `1.993983` |
| `strategy_entries` | `1173` |
| `strategy_exits` | `1173` |
| `analytical_layer3_rows` | `522633` |
| `model_events` | `level=842285 divergence=15320 signal=332913 total=1190518` |

The example run halts at the configured 20% drawdown limit. `halted: true` is
an observed result, not a claim that the strategy is unprofitable or that a
different configuration would do better.

The summary's Checks section is deliberately conservative:

```text
Checks
  [x] five report bundles present
  [x] provenance.json present (5/5)
  [x] data version matches report.json
  [x] data version matches provenance.json (5/5)
  [x] report versions present (5/5)
  [ ] out-of-sample evidence
  [ ] independent rerun
```

One bundle can also be written alone:

```bash
"$QR" report portfolio \
  --config config/examples/solusd-2025/run.json \
  --out data/tutorial-portfolio-report
```

Expected: seven artifact paths under
`data/tutorial-portfolio-report/portfolio/`: `summary.txt`, `report.json`,
`trades.csv`, `equity.csv`, `equity.png`, `drawdown.png`, and
`provenance.json`.

Check determinism of the root summary:

```bash
first=$(sha256sum "$lab/summary.txt" | cut -d' ' -f1)
"$QR" lab summary "$lab" \
  --role researcher \
  --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 \
  --config config/examples/solusd-2025/run.json \
  --created-utc 2026-10-30T12:00:00Z \
  --force > /dev/null
second=$(sha256sum "$lab/summary.txt" | cut -d' ' -f1)
test "$first" = "$second" && echo "summary stable"
```

Expected:

```text
summary stable
```

The summary is a copy of published values. It does not read the clock and does
not recompute the model.

**Checkpoint.** Five bundles, 29 bundle artifacts, one root `summary.txt`, the
observed values above, and `summary stable`. Stop if the run halts for a
different reason, a bundle is missing, or the hash changes.

## 8. Store

Actor: Either. The store is shell-owned; the engines remain pure.

```bash
"$QR" store init --db data/tutorial.db
```

Expected:

```text
store data/tutorial.db schema=2
```

Use a fresh database path for the first run. A same-name dataset with a
different data version is a conflict until it is removed explicitly.

```bash
"$QR" store ingest "$CSV" \
  --dataset solusd-2025-q1 \
  --data-version tutorial-solusd-m1-2025-01-01_2025-02-28 \
  --db data/tutorial.db
```

Expected:

```text
ingested solusd-2025-q1 bars=107918 timeframes=4
```

`bars=107918` is the ladder bar count, not the M1 row count. The 84,108 M1 rows
become 107,918 bars across 1m, 5m, 15m, and 1h. The ingest writes the candles,
readings, four-axis metrics, resolutions, consensus rows, Layer 3 rows, pivots,
levels, events, and retained legs in one transaction. The two-month database is
about 176 MB.

```bash
"$QR" store verify "$CSV" \
  --dataset solusd-2025-q1 \
  --data-version tutorial-solusd-m1-2025-01-01_2025-02-28 \
  --db data/tutorial.db
```

Expected:

```text
verified solusd-2025-q1 bars=107918 tables=7
```

`store verify` rebuilds the source and compares every stored table. It opens the
store read-only and never repairs it. The first mismatch is a deterministic
error.

```bash
"$QR" store list --db data/tutorial.db
```

Expected: one header row and one data row.

```text
name,data_version,model_version,structure_scale_code,m1_bars,layer3_materialized,bar_rows,consensus_rows,layer3_rows,pivot_rows,level_rows,event_rows,leg_rows
solusd-2025-q1,tutorial-solusd-m1-2025-01-01_2025-02-28,0.3.0,2,84108,1,107918,107918,84108,126405,34542,192405,7232
```

```bash
"$QR" store show solusd-2025-q1 event --limit 2 --db data/tutorial.db
```

Expected: a header and two rows.

```text
dataset_id,id,timeframe_secs,event_type_code,event_subtype_code,source_code,occurred_bar_index,available_bar_index,direction_code,reference_code,primary_value,reference_value
1,1,60,1,1,7,14,14,0,1003,189.96,189.96
1,2,60,1,1,7,15,15,0,1003,189.96,189.96
```

Query commands open an existing database read-only. A missing file is an error
and is never created. `NULL` is an empty CSV field; a float is the shortest
round-trip decimal.

```bash
"$QR" db series solusd-2025-q1 --timeframe 1h --limit 2 --db data/tutorial.db
```

Expected:

```json
{"dataset":"solusd-2025-q1","timeframe_secs":3600,"bar_index":0,"open_time":"2025-01-01T15:00:00Z","open":190.26,"high":191.47,"low":189.34,"close":191.23,"volume":22.236000000000004}
{"dataset":"solusd-2025-q1","timeframe_secs":3600,"bar_index":1,"open_time":"2025-01-01T16:00:00Z","open":191.22,"high":191.6,"low":189.42,"close":189.88,"volume":29.255999999999997}
```

The wide tables are CSV:

```bash
"$QR" db bars solusd-2025-q1 --limit 1 --db data/tutorial.db \
  > data/tutorial-db-bars.csv
"$QR" db metrics solusd-2025-q1 --limit 1 --db data/tutorial.db \
  > data/tutorial-db-metrics.csv
"$QR" db layer3 solusd-2025-q1 --limit 1 --db data/tutorial.db \
  > data/tutorial-db-layer3.csv

awk -F, '{print "bars " NF; exit}' data/tutorial-db-bars.csv
awk -F, '{print "metrics " NF; exit}' data/tutorial-db-metrics.csv
awk -F, '{print "layer3 " NF; exit}' data/tutorial-db-layer3.csv
```

Expected:

```text
bars 74
metrics 61
layer3 285
```

Optional cleanup removes the dataset but keeps the database file:

```bash
"$QR" store remove solusd-2025-q1 --db data/tutorial.db
```

Expected:

```text
removed solusd-2025-q1 bars=107918
```

**Checkpoint.** Init prints `schema=2`; ingest and verify both print
`bars=107918`; the store list row matches; the table widths are 74, 61, and 285
columns. Stop if verify prints a mismatch; do not repair the store in place.

## 9. Role labs

The three labs use the same pipeline and different responsibilities. Each lab
uses its own lab result folder, so a summary is never overwritten by accident.
The commands repeat the full-year run on purpose: reproducibility is part of the
procedure.

### 9.1 Quant Researcher lab

| Role card | |
|---|---|
| **Purpose** | Turn a market question into a deterministic hypothesis and a pre-registered protocol. |
| **Inputs** | The data coverage, the model contract, the example configuration. |
| **Outputs** | A hypothesis, metrics, a split, cost assumptions, pass/fail criteria, and a researcher lab summary. |
| **Stop conditions** | A threshold change after seeing results; a new sample that was not pre-registered; a profitability claim from one run. |
| **Handoff** | The protocol and the lab to the Quant Developer. |

The two-month commands above are a pipeline pilot. The researcher's experiment
is the full-year lab from section 7. This tutorial shows the full run first so
the pipeline is concrete; in a real work item, the researcher writes the
protocol before the run. The lab demonstrates the record, not the chronology.

Write the protocol before re-reading the Observed result:

```bash
mkdir -p data/tutorial-role-notes
cat > data/tutorial-role-notes/researcher.md <<'EOF'
Hypothesis: the axis-agreement rule is active on the full-year SOLUSD example:
at least 100 M15 entries, and the portfolio halts at the configured 20% drawdown.
Protocol: one run of config/examples/solusd-2025/run.json over 2025-01-01..2025-12-31.
Metrics: strategy_entries, strategy_exits, trades, halted, max_drawdown.
Split: in-sample only; no out-of-sample evidence.
Costs: commission 0.1% of equity per operation, slippage 2 bps, funding 0.0001 per 8h.
Pass: strategy_entries >= 100 and halted == true.
Variations: none. No threshold changes after the run.
EOF
```

Add the notes to the researcher summary:

```bash
"$QR" lab summary "$lab" \
  --role researcher \
  --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 \
  --config config/examples/solusd-2025/run.json \
  --created-utc 2026-10-30T12:00:00Z \
  --notes data/tutorial-role-notes/researcher.md \
  --force
```

Expected: `wrote ...`, and the Role notes section repeats the file. The observed
result is `strategy_entries: 1173`, `trades: 48`, `halted: true`; the pass
condition is met. That supports the activity hypothesis only. It does not
support a profitability claim, and the Checks section still shows
`[ ] out-of-sample evidence`.

```text
Handoff packet - Researcher to Developer
- work item: full-pipeline tutorial, role lab
- data version: dukascopy-bid-solusd-m1-2025-01-01_2025-12-31
- config: config/examples/solusd-2025/run.json
- lab: data/lab-result-2026-10-30T12-00-00Z
- hypothesis: at least 100 M15 entries; halt at 20% drawdown
- observed: strategy_entries=1173, trades=48, halted=true
- limits: in-sample only; example is illustrative; no profitability claim
- requested from developer: reproduce the run and report deviations
```

**Checkpoint.** The researcher summary contains the protocol verbatim, the
observed values match, and the out-of-sample check is still unchecked. Stop if
the result is presented as promising or validated.

### 9.2 Quant Developer lab

| Role card | |
|---|---|
| **Purpose** | Implement and reproduce the approved protocol deterministically. |
| **Inputs** | The researcher's protocol, the model contract, the configuration schemas. |
| **Outputs** | A passing test run, a second reproducible lab, and reported deviations. |
| **Stop conditions** | A failing test; a changed threshold; a difference between two runs; a scope change outside the protocol. |
| **Handoff** | The implementation record and the lab to the Quant Validator. |

Run the developer gates from the repository root:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo doc --no-deps
```

Expected: no diff, no warnings, all tests pass, and documentation builds without
warnings. These gates are outside the product shell.

Run the protocol a second time into a new lab folder:

```bash
lab2=data/lab-result-2026-10-30T12-30-00Z
"$QR" backtest --config config/examples/solusd-2025/run.json --out "$lab2"
"$QR" lab summary "$lab2" \
  --role developer \
  --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 \
  --config config/examples/solusd-2025/run.json \
  --created-utc 2026-10-30T12:30:00Z
```

Compare every bundle with the researcher's lab:

```bash
for bundle in model strategy execution portfolio analytical; do
  diff -r "$lab/$bundle" "$lab2/$bundle" > /dev/null && echo "$bundle identical"
done
```

Expected:

```text
model identical
strategy identical
execution identical
portfolio identical
analytical identical
```

The root summaries differ in their identity and role sections; the five bundles
do not. The observed result is the same: 48 trades, final equity `8016.143910`,
commission `885.610499`, and `halted: true`.

Add the developer notes:

```bash
cat > data/tutorial-role-notes/developer.md <<'EOF'
Implementation: no code change; the approved configuration is unchanged.
Gates: cargo fmt, cargo clippy, cargo test, cargo doc.
Reproducibility: a second backtest matched the researcher bundles byte for byte.
Deviation: none.
Observed: 48 trades, final equity 8016.143910, halted true.
EOF

"$QR" lab summary "$lab2" \
  --role developer \
  --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 \
  --config config/examples/solusd-2025/run.json \
  --created-utc 2026-10-30T12:30:00Z \
  --notes data/tutorial-role-notes/developer.md \
  --force
```

```text
Handoff packet - Developer to Validator
- work item: full-pipeline tutorial, role lab
- data version: dukascopy-bid-solusd-m1-2025-01-01_2025-12-31
- config: config/examples/solusd-2025/run.json
- lab: data/lab-result-2026-10-30T12-30-00Z
- gates: fmt, clippy, test, doc all pass
- reproducibility: five bundles identical to the researcher lab
- deviation: none
- requested from validator: independent reproduction and cost and bias audit
```

**Checkpoint.** All five bundles are identical and the gates pass. Stop if any
bundle differs, a gate fails, or the run needed a configuration change.

### 9.3 Quant Validator lab

| Role card | |
|---|---|
| **Purpose** | Reproduce the result independently and decide whether the evidence supports the conclusion. |
| **Inputs** | The recorded data version, the configuration, the developer's lab. |
| **Outputs** | A validation record with commands, observed results, limitations, and a verdict. |
| **Stop conditions** | A non-reproducible run; a missing data version or config; a request to change the criteria; a profitability claim without out-of-sample evidence. |
| **Handoff** | The validation record to the human for acceptance. |

Run the protocol a third time:

```bash
lab3=data/lab-result-2026-10-30T13-00-00Z
"$QR" backtest --config config/examples/solusd-2025/run.json --out "$lab3"
"$QR" lab summary "$lab3" \
  --role validator \
  --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 \
  --config config/examples/solusd-2025/run.json \
  --created-utc 2026-10-30T13:00:00Z
```

Compare with the developer's lab:

```bash
for bundle in model strategy execution portfolio analytical; do
  diff -r "$lab2/$bundle" "$lab3/$bundle" > /dev/null && echo "$bundle reproduced"
done
```

Expected:

```text
model reproduced
strategy reproduced
execution reproduced
portfolio reproduced
analytical reproduced
```

Audit the observed result and the configured costs:

```bash
grep -E 'final_equity|max_drawdown|trades|halted|commission|slippage|funding' \
  "$lab3/summary.txt"
```

Expected:

```text
  final_equity: 8016.143910
  max_drawdown: 0.203150
  trades: 48
  halted: true
  commission: 885.610499
  slippage: 3.994526
  funding_paid: 4.003485
  funding_received: 1.993983
```

The audit has four findings:

1. **Reproduction.** The three runs produced byte-identical bundles. That
   supports reproducibility on this machine and this engine.
2. **Split.** The run is in-sample only. There is no out-of-sample or
   walk-forward result, and the summary's `[ ] out-of-sample evidence` is
   correct.
3. **Halt.** `max_drawdown: 0.203150` exceeds the configured `0.2` limit, so the
   run halts. The final equity is after the halt, not a full-period result.
4. **Costs.** Commission is `0.1%` of equity per operation, slippage is `2` bps,
   and funding is `0.0001` per 8 hours. These are configured assumptions, not
   live fills. Commission is the largest cost in this run.

Write the validation record:

```bash
cat > data/tutorial-role-notes/validator.md <<'EOF'
Reproduced: yes. A third run matched the developer bundles byte for byte.
Data version: dukascopy-bid-solusd-m1-2025-01-01_2025-12-31.
Config: config/examples/solusd-2025/run.json.
Split: in-sample only; no out-of-sample evidence.
Costs: commission 0.1% of equity per operation, slippage 2 bps, funding 0.0001 per 8h.
Halt: max_drawdown 0.203150 exceeded the 0.2 limit; the result is truncated.
Verdict: the evidence supports reproducibility, not a profitability claim.
Residual risk: in-sample, halt, cost assumptions, same engine and machine.
EOF

"$QR" lab summary "$lab3" \
  --role validator \
  --data-version dukascopy-bid-solusd-m1-2025-01-01_2025-12-31 \
  --config config/examples/solusd-2025/run.json \
  --created-utc 2026-10-30T13:00:00Z \
  --notes data/tutorial-role-notes/validator.md \
  --force
```

```text
Handoff packet - Validator to Human
- work item: full-pipeline tutorial, role lab
- lab: data/lab-result-2026-10-30T13-00-00Z
- reproduced: yes; five bundles identical to the developer lab
- verdict: reproducibility supported; profitability not supported
- residual risks: in-sample only, halt, configured costs, same engine and machine
- decision requested: accept the lab as a pipeline example, not as a strategy result
```

**Checkpoint.** The third run reproduces, the audit names the halt and the
in-sample split, and the verdict does not claim profitability. Stop if the run
does not reproduce or the notes omit a material limitation.

## 10. What the lab proves and does not prove

| Proves | Does not prove |
|---|---|
| The five engines run in order on the recorded data and configuration. | That the strategy is profitable, has an edge, or should be traded. |
| The five bundles and the root summary are written. | That the result holds out of sample. |
| The summary copies published values without recomputing the model. | That the model's formulas are economically correct. |
| The same inputs and configuration produce the same bytes on a rerun. | That fills, costs, funding, or capacity are realistic. |
| The data version and configuration are recorded for a later run. | That a different machine or engine version produces the same result. |

The Validator lab is an exercise. It is not a production validation record, and
it is not an independent model judgment: the same engine, the same machine, and
the same configuration produced all three runs.

## 11. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| A capped command refuses the full-year file | The input exceeds the command's 100,000-candle cap. | Use the two-month slice, or `--window latest` where the command supports it. |
| `output directory is not empty` | The lab, report, or bundle target already has files. | Choose a new UTC timestamp or remove the old directory after confirming it is disposable. |
| `summary.txt already exists; use --force` | A root summary is already present. | Pass `--force` only when overwriting is intentional. |
| `data version mismatch` | `--data-version` does not match a report or provenance. | Use the value recorded in `provenance.json`; do not invent a new identity for an existing lab. |
| `missing .../report.json` | A bundle did not finish. | Rerun the backtest into a new folder; do not hand-edit a bundle. |
| `Broken pipe` | A pipe such as `head` closed before the writer finished. | Redirect to a file, then slice the file. |
| `cannot open ...csv` | The config resolves its source relative to `data.json`. | Run from the repository root, or use an absolute config path with the data file in place. |
| `fetch` network error | The provider is unreachable. | Retry, or slice an existing full-year file with the `awk` command in section 2. |
| `store verify` mismatch | The store is incomplete, stale, or from another build. | Re-ingest into a fresh database; verification never repairs in place. |
| `schema newer than this binary` | The store was written by a newer build. | Use the matching binary or a fresh database; the store is not migrated backward. |
| `invalid --created-utc` | The value is not `YYYY-MM-DDTHH:MM:SSZ`. | Use the UTC form; the folder uses `YYYY-MM-DDTHH-MM-SSZ`. |
| `strategy-run` fails with an empty-period message | The requested period has no M15 bars. | Widen the period or check the source coverage. |
| `candles` aborts on a row | The strict reader found an invalid, duplicate, or out-of-order row. | Use `candles-tolerant` to inspect the quality report. |
| `market-report` refuses the target | The output path is a file or the directory is not empty. | Use a new or empty directory. |
| `quantrule: command not found` | The binary is not on `PATH`. | Use `./target/release/quantrule` or add `target/release` to `PATH`. |

## 12. Command index

| Command | Section | Purpose | Output |
|---|---|---|---|
| `help` | 2 | Show the map, a command contract, or a reference topic. | Plain text |
| `fetch` | 2 | Fetch native BID candles from the source. | CSV and a summary |
| `candles` | 3 | Read a CSV strictly and print the last 1,000 candles. | NDJSON |
| `candles-tolerant` | 3 | Read a CSV, skip bad rows, report quality. | NDJSON and a quality report |
| `timeframes` | 3 | Build M1/M5/M15/H1 from M1. | NDJSON and an aggregation report |
| `market-state` | 4 | Print the four-timeframe state. | One JSON object |
| `market-dashboard` | 4 | Print the four-axis dashboard. | Fixed-width text or JSON |
| `axis` | 4 | Print one axis per timeframe. | NDJSON |
| `events` | 4 | Print every event in the analyzed window. | NDJSON |
| `legs` | 4 | Print the minor and major leg per timeframe. | NDJSON |
| `model-series` | 4 | Print complete model bars. | NDJSON |
| `market-chart` | 4 | Render candles and axis panels. | Deterministic PNG and a summary line |
| `market-report` | 4 | Write the full-period descriptive report. | Four artifacts |
| `research export` | 5 | Write the research dataset. | Raw M1 plus three layers and metadata |
| `research summary` | 5 | Print axis counts, ranges, histograms, and correlations. | Text |
| `strategy-run` | 6 | Print strategy entries and exits. | NDJSON; no fills or P&L |
| `report` | 7 | Write one config-driven bundle. | One bundle under `<out>/<engine>/` |
| `backtest` | 7 | Run the full pipeline and write five bundles. | Five bundle paths |
| `lab summary` | 7 | Write the root lab summary. | `summary.txt`; prints `wrote <path>` |
| `store init` | 8 | Create or open the SQLite store. | Path and schema version |
| `store ingest` | 8 | Ingest one M1 dataset. | Stored counts |
| `store verify` | 8 | Rebuild and compare every stored table. | `verified ...` or the first mismatch |
| `store list` | 8 | List stored datasets. | CSV |
| `store show` | 8 | Show one stored table. | CSV |
| `store remove` | 8 | Remove one dataset and its rows. | `removed ...`; the file remains |
| `db series` | 8 | Read stored candles. | NDJSON |
| `db bars` | 8 | Read the full bar table. | CSV, 74 columns |
| `db metrics` | 8 | Read metrics and resolutions. | CSV, 61 columns |
| `db layer3` | 8 | Read the materialized Layer 3 rows. | CSV, 285 columns |
| `quit` | 2 | Exit the interactive shell with status 0. | None |

The complete option reference is in the repository
[`README.md`](../../../README.md#commands). The model contract is
[`docs/model/2026-10-21/four-axis-analytical-model.md`](../../model/2026-10-21/four-axis-analytical-model.md),
the research contract is
[`docs/analytics/2026-10-24/research-output.md`](../../analytics/2026-10-24/research-output.md),
and the report contract is
[`docs/reports/2026-10-28/per-engine-reports.md`](../../reports/2026-10-28/per-engine-reports.md).

## Related documents

- [Quant roles](../../roles/2026-10-29/quant-roles.md)
- [AGENTS.md section 10](../../../AGENTS.md#10-agentic-engineering-and-role-definitions)
- [Per-engine configuration](../../config/2026-10-28/per-engine-config.md)
- [Execution plan for this tutorial](../../execution-plans/2026-10-29/208-help-tutorial-role-manuals.md)
- [AI transparency log](../../ai-transparency-log/2026-10-29/208-help-tutorial-role-manuals.md)
