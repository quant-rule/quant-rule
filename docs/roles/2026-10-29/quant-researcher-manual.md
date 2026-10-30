# Quant Researcher Manual

- Date: 2026-10-29 (UTC)
- Status: Current
- Normative source:
  [`AGENTS.md` section 10](../../../AGENTS.md#10-agentic-engineering-and-role-definitions)
- Companion: [Quant roles](quant-roles.md),
  [full-pipeline tutorial](../../tutorials/2026-10-29/full-pipeline.md)
- Scope: candles-only, rule-based signals. Statistical, machine-learning, and
  neural-network signal generation are out of scope.
- Claim type: procedural. This manual makes no profitability claim.

This is the operating manual for the Quant Researcher role. `AGENTS.md` section
10 is normative; if this manual disagrees with it, `AGENTS.md` wins. The
tutorial shows the complete shell pipeline; this manual states what the
researcher decides, records, and hands to the Quant Developer.

## 1. Role card

| Item | Content |
|---|---|
| **Purpose** | Turn a market question into a deterministic, rule-based hypothesis and a pre-registered experiment. |
| **Inputs** | The market question, the data coverage, the analytical model contract, the available run configuration, and prior evidence. |
| **Outputs** | A written hypothesis, a protocol with metrics and a split, cost assumptions, pass/fail criteria, a variation log, a researcher lab result, and a handoff packet. |
| **Stop conditions** | A threshold or criterion change after seeing results; an unrecorded sample or variation; a profitability claim without out-of-sample evidence; a request to write production code or approve the result. |
| **Handoff** | The protocol and the lab result to the Quant Developer. |

## 2. When the role is required

| Change | Researcher required? |
|---|---|
| New strategy hypothesis | Yes |
| New evaluation metric or pass/fail criterion | Yes |
| Claim about model behavior that affects signals | Yes |
| Strategy threshold or configuration change | Yes |
| Release that changes signal generation | Yes, when a behavior or performance claim is made |
| Bug fix that does not change model or strategy output | No |
| Documentation or tooling only | No |

The researcher does not write production code and does not certify the result.
The role is required when the question is new, when the evidence standard
changes, or when a claim will be made about model or strategy behavior.

## 3. Operating procedure

### 3.1 State the question and the hypothesis

Write the question as a deterministic rule over OHLCV candles and the
configured timeframe ladder. State what the rule does, which readings and axes
it uses, and what observable result would support or reject the hypothesis.

- The hypothesis is a statement about the rule's behavior, not a profit target.
- The rule must be expressible with the model's published numbers. Do not
  introduce a statistical, machine-learning, or neural-network signal.
- State the expected direction of the effect, the sample, and the condition
  that would count as a failure.
- If the question cannot be answered from the model's published outputs, stop
  and narrow it before running anything.

Example activity-and-risk hypothesis: "On the full-year SOLUSD example, the
`axis-agreement` rule produces at least 100 M15 entries, and the portfolio halts
at the configured 20% drawdown limit." This is a testable claim about activity
and risk control. It is not a claim that the rule is profitable.

### 3.2 Fix the data, the sample, and the split

The data version is the caller-supplied identity of the input series: source,
timeframe, and coverage. The engine records it and never infers it from the
path.

| Input | Maximum M1 candles | Use |
|---|---:|---|
| `market-state`, `market-dashboard`, `market-chart`, `axis`, `events`, `legs`, `model-series`, `store`, `research` | 100,000 | Two-month slice |
| `market-report`, `strategy-run` | 600,000 | Either slice |
| `report`, `backtest` | `data.json.max_candles`, at most 600,000 | Full year |

Set the working variables from the repository root:

```bash
QR=./target/release/quantrule
CSV2=data/SOLUSD_M1_2025-01-01_2025-02-28.csv
CSV=data/SOLUSD_M1_2025-01-01_2025-12-31.csv
DV2=tutorial-solusd-m1-2025-01-01_2025-02-28
DV=dukascopy-bid-solusd-m1-2025-01-01_2025-12-31
CFG=config/examples/solusd-2025/run.json
```

Inspect the source before choosing the sample:

```bash
"$QR" candles-tolerant "$CSV2" > /dev/null
"$QR" timeframes "$CSV2" > /dev/null 2> data/researcher-timeframes.txt
grep '^aggregation' data/researcher-timeframes.txt
```

Expected from `candles-tolerant` on the two-month slice:

```text
quality rows_read=84108 accepted=84108 rejected=0 duplicate=0 out_of_order=0 invalid=0 evicted=83108 retained=1000
```

Expected from the `grep`:

```text
aggregation timeframe=1m retained=1000 partial=0 gaps=7
aggregation timeframe=5m retained=1000 partial=7 gaps=7
aggregation timeframe=15m retained=1000 partial=8 gaps=7
aggregation timeframe=1h retained=1000 partial=8 gaps=6
```

Record the split before running:

- **In-sample:** the period used to form or tune the hypothesis.
- **Out-of-sample:** a period not used to form or tune the hypothesis.
- **Walk-forward:** a sequence of adjacent in-sample and out-of-sample windows.
- **No split:** allowed for a descriptive or pipeline exercise, but the result
  cannot support a profitability claim.

The example run uses 2025-01-01..2025-12-31 with no out-of-sample period. That is
enough to exercise the pipeline and record an observed result. It is not enough
to call the strategy promising.

### 3.3 Pre-register the metrics, costs, and pass/fail criteria

Write the metric table before the run. Use the model's published fields and the
report values; do not invent a metric after seeing the output.

| Metric group | Fields | Source |
|---|---|---|
| Activity | `strategy_entries`, `strategy_exits`, `trades` | Strategy and portfolio reports |
| Risk | `max_drawdown`, `halted` | Portfolio report |
| Costs | `commission`, `slippage`, `funding_paid`, `funding_received` | Portfolio report |
| Descriptive | axis counts, ranges, ordinal histograms, relations | `research summary`, `market-report` |

Record the costs from the configuration, not from memory:

```bash
cat config/examples/solusd-2025/execution.json
cat config/examples/solusd-2025/portfolio.json
```

For the example configuration:

| Cost | Assumption |
|---|---|
| Commission | `0.1%` of equity per operation |
| Slippage | `2` bps |
| Funding | `0.0001` per 8 hours |
| Drawdown limit | `0.2`, action `halt` |
| Sizing | Fixed notional `10000`, one position, no leverage |

The pass/fail criterion is part of the protocol. A result that misses the
criterion is reported as a miss. The criterion is not revised after the run.

### 3.4 Write the protocol before running

The protocol is a file in the repository or in the work item's notes. It
contains the question, hypothesis, data version, sample, split, metrics, costs,
pass/fail criterion, every planned variation, and the stop conditions.

```bash
cat > data/researcher-protocol.md <<'EOF'
Question: does the axis-agreement rule stay active on the full-year example?
Hypothesis: at least 100 M15 entries, and the portfolio halts at the 20% drawdown limit.
Data version: dukascopy-bid-solusd-m1-2025-01-01_2025-12-31.
Sample: 2025-01-01..2025-12-31, full-year example.
Split: in-sample only; no out-of-sample evidence.
Metrics: strategy_entries, strategy_exits, trades, halted, max_drawdown,
commission, slippage, funding_paid, funding_received.
Costs: commission 0.1% of equity per operation, slippage 2 bps, funding 0.0001 per 8h.
Pass: strategy_entries >= 100 and halted == true.
Variations: none. No threshold changes after the run.
Stop: stop if the data version or config changes, or if the result needs a new criterion.
EOF
```

An agent records the protocol before executing the experiment. A human reviews
it before authorizing the run. The tutorial shows the full run before the role
lab only to make the pipeline concrete; in a real work item, the protocol comes
first.

### 3.5 Run the descriptive research

The research commands are descriptive. They do not place orders or compute P&L.

```bash
"$QR" research summary "$CSV2" > data/researcher-summary.txt
wc -l < data/researcher-summary.txt
"$QR" strategy-run "$CSV2" \
  --strategy axis-agreement \
  --data-version "$DV2" \
  > data/researcher-strategy.ndjson
wc -l < data/researcher-strategy.ndjson
```

Expected: 41 summary lines (16 axis lines, 24 relation lines, one header) and
363 strategy records (182 entries, 181 exits) on the two-month slice. Count the
record types explicitly:

```bash
python3 - <<'PY'
import json
from collections import Counter
counts = Counter()
for line in open("data/researcher-strategy.ndjson"):
    counts[json.loads(line)["type"]] += 1
print(dict(counts))
PY
```

Expected:

```text
{'entry': 182, 'exit': 181}
```

The four axes are read separately. A cross-axis agreement is a count, not an
average. Ordinal codes are counted, never averaged. A null is a missing
measurement with a reason code, never a zero.

### 3.6 Run the experiment

The full run uses the example configuration and the full-year file. Choose a new
UTC timestamp for the lab folder; the folder format is
`YYYY-MM-DDTHH-MM-SSZ` and `--created-utc` uses `YYYY-MM-DDTHH:MM:SSZ`.

```bash
lab=data/lab-result-2026-10-30T12-00-00Z
"$QR" backtest --config "$CFG" --out "$lab"
"$QR" lab summary "$lab" \
  --role researcher \
  --data-version "$DV" \
  --config "$CFG" \
  --created-utc 2026-10-30T12:00:00Z \
  --notes data/researcher-protocol.md \
  --operator human
```

Expected: five bundle paths, then `wrote <lab>/summary.txt`. The example result
is:

| Field | Value |
|---|---|
| `trades` | `48` |
| `final_equity` | `8016.143910` |
| `total_return` | `-0.198386` |
| `max_drawdown` | `0.203150` |
| `halted` | `true` |
| `commission` | `885.610499` |
| `slippage` | `3.994526` |
| `funding_paid` | `4.003485` |
| `funding_received` | `1.993983` |
| `strategy_entries` | `1173` |
| `strategy_exits` | `1173` |

The example halts at the configured 20% drawdown limit. That is an observed
result and a risk finding, not evidence that a different configuration would
perform better.

### 3.7 Interpret and record

Separate the evidence into:

- **In-sample:** what the run measured on the sample used to form the
  hypothesis.
- **Out-of-sample:** what a later, untouched sample measured. If there is none,
  say so.
- **Descriptive:** counts, distributions, and relations that do not support a
  profitability claim.
- **Residual risk:** the halt, the cost assumptions, the data coverage, and the
  limits of a single run.

Record every variation that was tried, including variations that were rejected.
Do not change the threshold or the criterion to turn a miss into a pass. If a
new question appears, write a new protocol; do not fold it into the old one.

### 3.8 Hand off

Run the checklist in section 5, complete the packet in section 8, and hand the
protocol and the lab to the Quant Developer. The researcher does not implement
the strategy and does not certify the result.

## 4. Artifacts and lab result folder

| Artifact | Path | Purpose |
|---|---|---|
| Protocol | `data/researcher-protocol.md` | The pre-registered question, metrics, split, costs, and pass/fail criterion. |
| Research summary | `data/researcher-summary.txt` | Axis counts, ranges, histograms, and relations. |
| Strategy records | `data/researcher-strategy.ndjson` | Entries and exits; no fills or P&L. |
| Lab result | `data/lab-result-YYYY-MM-DDTHH-MM-SSZ/` | Five report bundles plus the root summary. |
| Root summary | `<lab>/summary.txt` | Identity, artifacts, observed result, checks, role notes, limitations, and next. |
| Handoff packet | In the work item or the packet below | The researcher's decision record for the developer. |

A lab result folder contains:

```text
model/ strategy/ execution/ portfolio/ analytical/ summary.txt
```

Each bundle contains `report.json`, `summary.txt`, and `provenance.json` plus its
own artifacts. The root summary is not a sixth engine and does not recompute the
model.

## 5. Checks before handoff

| Check | Condition |
|---|---|
| Protocol written first | The protocol predates the run and contains the metrics, split, costs, and pass/fail criterion. |
| Data version recorded | The data version is explicit and matches every report and provenance file. |
| Split stated | In-sample, out-of-sample, or walk-forward is named. A missing split is labeled as no split. |
| Costs recorded | Commission, slippage, funding, and drawdown policy come from the configuration. |
| Metrics pre-registered | Every reported metric appears in the protocol; no post-hoc metric is introduced as a pass condition. |
| Variations recorded | Every threshold, configuration, or sample variation is listed, including rejected ones. |
| Axes separate | Direction, Momentum, Structure, and Progression are reported separately; no average or single score is used. |
| Nulls handled | A null is a missing measurement with a reason code, never a zero. |
| Lab summary complete | The five bundles, provenance, data version, and checks are present. |
| No profitability claim | A single in-sample run is not called promising, validated, or profitable. |
| Handoff packet complete | The packet names the data version, config, lab, metrics, observed result, limitations, and requested next step. |

## 6. Agent instructions

1. Read `AGENTS.md` section 10, this manual, and the [roles overview](quant-roles.md).
2. Do not write production code, change a threshold, or change a configuration.
3. Write the protocol before running the experiment.
4. Run only the commands in the procedure. Record the exact command and the
   observed output; do not summarize a result you did not observe.
5. Stop if the data version, configuration, or sample is missing; if a command
   exceeds its input cap; or if the result requires a new decision.
6. Do not self-approve. Hand the packet to the Quant Developer.
7. Pass `--operator agent` to `lab summary` when an agent produces the lab.
8. Do not call a result promising or profitable without out-of-sample evidence.
9. Do not change the criterion, the threshold, or the sample to improve the
   result. Record the miss and hand it off.

## 7. Human instructions

Review the following before authorizing the developer:

- The question is answerable from the model's published outputs.
- The hypothesis is a deterministic rule over OHLCV, not a statistical or
  machine-learning signal.
- The data version and coverage are correct.
- The split is stated before the run, and the out-of-sample gap is visible.
- The metrics and pass/fail criterion are pre-registered.
- The costs are the configured assumptions, not optimistic guesses.
- Every variation is recorded.
- The observed result is separated from the interpretation.
- The limitations include the halt, the sample, the costs, and the absence of
  out-of-sample evidence.

Decide one of:

| Decision | Meaning |
|---|---|
| **Authorize developer** | The protocol is complete and the next role may reproduce it. |
| **Revise** | The question, sample, metrics, or costs need a change before implementation. |
| **Reject** | The hypothesis is out of scope, untestable, or not worth the run. |

A researcher's pass on an activity metric is not a profitability verdict. A
validator's verdict is evidence, not acceptance.

## 8. Handoff packet template

```text
Handoff packet - Researcher to Developer
Work item:
Issue:
Question:
Hypothesis:
Data version:
Config:
Sample:
Split:
Metrics:
Costs:
Pass/fail:
Variations:
Lab:
Observed result:
Limitations:
Requested from developer:
```

## 9. Common failure modes

| Failure | Consequence | Correction |
|---|---|---|
| Changing a threshold after seeing results | The result is tuned to the sample and cannot be trusted. | Freeze the protocol; record the variation as a new experiment. |
| Reporting only in-sample evidence | The result is called promising without validation. | State the split; require out-of-sample evidence before a promise. |
| Ignoring costs | The result overstates the edge. | Use the configured commission, slippage, and funding; report them. |
| Averaging the four axes | The axes describe different things; the average hides disagreement. | Report each axis; use counts for agreement. |
| Treating a null as zero | A missing measurement becomes a false observation. | Keep the null and its reason code; exclude it from samples. |
| Using the full-year file in a 100k-capped command | The command refuses the input or the workflow stalls. | Use the two-month slice for capped commands. |
| Claiming profitability from one run | A single backtest is not validation. | Report reproducibility and descriptive evidence only. |
| Not recording the data version | The run cannot be reproduced. | Use the caller-supplied identity and keep it with the lab. |
| Adding a metric after the run | The pass/fail criterion moves to fit the result. | Pre-register metrics; label post-hoc analysis as exploratory. |
| Writing production code | The researcher crosses into the developer role. | Hand the protocol to the developer. |

## 10. Normative links

- [`AGENTS.md` section 10](../../../AGENTS.md#10-agentic-engineering-and-role-definitions)
- [Quant roles overview](quant-roles.md)
- [Full-pipeline tutorial](../../tutorials/2026-10-29/full-pipeline.md)
- [Four-axis analytical model](../../model/2026-10-21/four-axis-analytical-model.md)
- [Research output contract](../../analytics/2026-10-24/research-output.md)
- [Per-engine configuration](../../config/2026-10-28/per-engine-config.md)
- [Per-engine reports](../../reports/2026-10-28/per-engine-reports.md)
