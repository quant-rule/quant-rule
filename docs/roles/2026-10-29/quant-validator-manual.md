# Quant Validator Manual

- Date: 2026-10-29 (UTC)
- Status: Current
- Normative source:
  [`AGENTS.md` section 10](../../../AGENTS.md#10-agentic-engineering-and-role-definitions)
- Companion: [Quant roles](quant-roles.md),
  [full-pipeline tutorial](../../tutorials/2026-10-29/full-pipeline.md)
- Scope: candles-only, rule-based signals. Statistics serves audit only.
- Claim type: procedural. This manual makes no profitability claim.
- Shell entry: `help roles` for the role map; `help tutorial` for the pipeline.

This is the operating manual for the Quant Validator role. `AGENTS.md` section
10 is normative; if this manual disagrees with it, `AGENTS.md` wins. The
validator reproduces the result independently, audits the evidence, records the
limitations, and hands a verdict to the human. The validator does not modify the
work, waive a control, or approve a result.

## 1. Role card

| Item | Content |
|---|---|
| **Purpose** | Independently reproduce a result and decide whether the evidence supports the conclusion. |
| **Inputs** | The researcher's protocol, the developer's implementation and run, the recorded data version and configuration, the model and report contracts, and the raw data. |
| **Outputs** | A validator lab result, a validation record with commands and observed results, a verdict, residual risks, and a handoff packet to the human. |
| **Stop conditions** | A non-reproducible run; a missing data version or configuration; an unrecorded split or variation; a request to change the criteria; a profitability claim without out-of-sample evidence; a request to modify the work or approve a trade-off. |
| **Handoff** | The validation record and verdict to the human for acceptance. |

## 2. When the role is required

| Change | Validator required? |
|---|---|
| A strategy result is about to be called promising | Yes |
| A model or strategy behavior change is about to be merged | Yes |
| A release changes signal generation | Yes |
| A new strategy hypothesis reaches a candidate result | Yes |
| A bug fix changes model or strategy output | Yes |
| A bug fix does not change model or strategy output | No |
| Documentation or tooling only | No |

The validator is required before a strategy result is called promising, before a
model or strategy behavior change is merged, and before a release that changes
signal generation. A validation verdict is evidence, not approval; the human
remains the final authority.

## 3. Operating procedure

### 3.1 Freeze the criteria

Read the researcher's protocol before reading the result. Record the criteria
that will be used:

- The hypothesis and the rule.
- The data version and coverage.
- The in-sample, out-of-sample, or walk-forward split.
- The metrics and the pass/fail criterion.
- The cost assumptions.
- Every variation the researcher recorded.
- The stop conditions.

Do not change a criterion to turn a failure into a pass. If the criteria are
missing or contradictory, stop and return the work to the researcher.

```bash
cat data/notes/researcher-protocol.md
```

### 3.2 Verify the inputs and provenance

The data version is the caller-supplied identity of the input series: source,
timeframe, and coverage. It must match every report and provenance file.

```bash
QR=./target/release/quantrule
CFG=config/examples/solusd-2025/run.json
DV=dukascopy-bid-solusd-m1-2025-01-01_2025-12-31
lab_d=data/labs/lab-result-2026-10-30T12-30-00Z
lab_v=data/labs/lab-result-2026-10-30T13-00-00Z

cat "$lab_d/summary.txt"

python3 - <<'PY'
import json
for b in ["model", "strategy", "execution", "portfolio", "analytical"]:
    path = f"data/labs/lab-result-2026-10-30T12-30-00Z/{b}/provenance.json"
    d = json.load(open(path))
    print(b, d["data_version"], d["report_version"],
          d["requested_period"], d["actual_period"], d["config_files"])
PY
```

Check:

- The data version matches the protocol.
- The requested and actual periods match the split.
- The configuration files are the approved ones.
- The lab has all five bundles and every `provenance.json`.
- The root summary's checks are present and honest.

### 3.3 Reproduce independently

Use a fresh lab folder and the recorded data version and configuration. Do not
copy the developer's lab and call it a reproduction.

```bash
"$QR" backtest --config "$CFG" --out "$lab_v"
"$QR" lab summary "$lab_v" \
  --role validator \
  --data-version "$DV" \
  --config "$CFG" \
  --created-utc 2026-10-30T13:00:00Z \
  --operator human
```

Replace the timestamp and lab path with the work item's own. An agent passes
`--operator agent`.

Compare every bundle with the developer's lab:

```bash
for bundle in model strategy execution portfolio analytical; do
  diff -r "$lab_d/$bundle" "$lab_v/$bundle" > /dev/null && echo "$bundle reproduced"
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

If a bundle differs, stop. Record the first difference and hand it back to the
developer. Do not repair the bundle, rerun until it matches, or waive the check.

### 3.4 Audit the evidence

The audit is read-only. Run the relevant tests and inspect the reports and
configuration.

#### Look-ahead and availability

```bash
cargo test -p quantrule-model
cargo test -p quantrule-backtest
```

Inspect the event records for availability:

```bash
"$QR" events data/sources/SOLUSD_M1_2025-01-01_2025-02-28.csv --window latest \
  > data/scratch/validator-events.ndjson

python3 - <<'PY'
import json
bad = []
for line in open("data/scratch/validator-events.ndjson"):
    e = json.loads(line)
    if e["available_bar"] < e["occurred_bar"]:
        bad.append(e)
print("events before availability:", len(bad))
PY
```

Expected:

```text
events before availability: 0
```

This is a sample check; the model's causality tests cover the full property.
Check that the strategy uses closed bars and that execution fills at the next
open, not the signal bar's close.

#### Survivorship and data snooping

- The example uses one symbol and one continuous series. There is no universe
  selection, so survivorship is not introduced by a list of survivors. State
  this explicitly; do not claim a survivorship check that was not performed.
- Count the variations in the protocol. If the researcher tried several
  thresholds, samples, or metrics, the result is selected and must be reported
  as such.
- A threshold chosen after seeing the result is data snooping. Stop and return
  the work to the researcher.

#### Split leakage

- The split must be stated before the run.
- The out-of-sample period must not overlap the in-sample period.
- The full-year example has no out-of-sample period. The summary's
  `[ ] out-of-sample evidence` is correct; a profitability claim is not
  supported.
- Walk-forward windows must be adjacent and must not reuse future data.

#### Costs, fills, and risk controls

Read the configuration and the portfolio and execution summaries:

```bash
cat config/examples/solusd-2025/execution.json
cat config/examples/solusd-2025/portfolio.json
cat "$lab_v/execution/summary.txt"
cat "$lab_v/portfolio/summary.txt"
grep -E 'final_equity|max_drawdown|trades|halted|commission|slippage|funding' \
  "$lab_v/summary.txt"
```

Check:

| Item | What to verify |
|---|---|
| Commission | Rate, base, and per-operation setting match the protocol. |
| Slippage | Basis points match the protocol. |
| Funding | Rate and interval match the protocol. |
| Fills | Next-open fills, ATR stop and target, tie-break rule. |
| Rejections | Zero-volume, missing ATR, open-position, and end-of-data policies are visible. |
| Position limit | One position, fixed notional, leverage. |
| Drawdown | Limit and action; whether the run halted and where. |
| Exposure and capacity | No hidden leverage; the notional is consistent with the account. |

The example result is:

```text
  final_equity: 8010.410952
  max_drawdown: 0.200542
  trades: 100
  halted: true
  commission: 1822.737614
  slippage: 6.772630
  funding_paid: 1.896363
  funding_received: 3.696679
```

`max_drawdown: 0.200542` exceeds the configured `0.2` limit, so the run halts.
The final equity is after the halt, not a full-period result. Commission is the
largest cost. These are configured assumptions, not live fills.

### 3.5 Read the result and the checks

The root summary has seven sections: Identity, Artifacts, Observed result,
Checks, Role notes, Limitations, and Next. Read the Checks section literally:

```text
[x] five report bundles present
[x] provenance.json present (5/5)
[x] data version matches report.json
[x] data version matches provenance.json (5/5)
[x] report versions present (5/5)
[ ] out-of-sample evidence
[ ] independent rerun
```

The `[ ] independent rerun` line is conservative: the command does not know
that the validator ran a second backtest. Record the reproduction evidence in
the validation record, not by editing the summary.

### 3.6 Write the validation record and verdict

The record contains the commands, the observed results, the audit findings, the
limitations, and a verdict. Do not write a narrative that is not backed by an
observed command.

```bash
cat > data/notes/validator-record.md <<'EOF'
Reproduced: yes. A third run matched the developer bundles byte for byte.
Data version: dukascopy-bid-solusd-m1-2025-01-01_2025-12-31.
Config: config/examples/solusd-2025/run.json.
Split: in-sample only; no out-of-sample evidence.
Look-ahead: causal tests pass; no event before its availability bar in the sample.
Survivorship: one symbol and one continuous series; no universe selection.
Data snooping: no unrecorded variation; no threshold change after the run.
Costs: commission 0.1% of equity per operation, slippage 2 bps, funding 0.0001 per 8h.
Fills: next-open; ATR stop 2.0 and target 3.0; stop wins on a tie.
Risk: max_drawdown 0.200542 exceeded the 0.2 limit; the run halted; the result is truncated.
Verdict: the evidence supports reproducibility, not a profitability claim.
Residual risk: in-sample, halt, cost assumptions, same engine and machine.
EOF

"$QR" lab summary "$lab_v" \
  --role validator \
  --data-version "$DV" \
  --config "$CFG" \
  --created-utc 2026-10-30T13:00:00Z \
  --notes data/notes/validator-record.md \
  --force \
  --operator human
```

Verdict categories:

| Verdict | Meaning |
|---|---|
| **Reproduced** | The bundles match the candidate run on this machine and engine. |
| **Not reproduced** | A bundle differs; the result is not reproducible as recorded. |
| **Evidence sufficient for the stated claim** | The claim is limited to what the evidence supports, such as reproducibility or activity. |
| **Evidence insufficient** | A required input, split, cost, or control is missing. |
| **Rejected** | A bias, leak, control failure, or unrecorded variation invalidates the result. |

A verdict of "reproduced" is not a verdict of "profitable." A same-engine rerun
on the same machine is a reproduction, not an independent model judgment.

### 3.7 Hand off to the human

Complete the checklist in section 5 and the packet in section 8. The validator
does not repair the work, waive a control, or approve a trade-off. The human
accepts, revises, or rejects.

## 4. Artifacts and lab result folder

| Artifact | Path | Purpose |
|---|---|---|
| Validator lab | `data/labs/lab-result-YYYY-MM-DDTHH-MM-SSZ/` | Five report bundles plus the root summary. |
| Validation record | `data/notes/validator-record.md` | Commands, observed results, audit findings, verdict, residual risk. |
| Bundle comparison | The command output or a log | The five `reproduced` lines, or the first difference. |
| Event sample | `data/scratch/validator-events.ndjson` | The availability check sample. |
| Handoff packet | The work item or the packet below | The validator's record for the human. |

The validator lab is a separate folder from the researcher's and developer's
labs. The root summary records the validator role and the validation notes.

## 5. Checks before handoff

| Check | Condition |
|---|---|
| Criteria frozen | The hypothesis, metrics, split, costs, and pass/fail criterion are the researcher's. |
| Inputs verified | The data version, configuration, periods, and configuration files match the protocol. |
| Independent run | A fresh backtest produced the validator lab; the developer's lab was not reused. |
| Bundles reproduce | All five bundles match the developer's bundles byte for byte. |
| Look-ahead audited | Causal tests pass; no event precedes its availability bar in the sample. |
| Survivorship addressed | The universe and series selection are stated; no unperformed check is claimed. |
| Data snooping addressed | Every variation is recorded; no post-hoc threshold. |
| Split audited | In-sample and out-of-sample are separated; a missing split is labeled. |
| Costs audited | Commission, slippage, funding, fills, and risk controls match the configuration. |
| Halt noted | A halted run is reported as truncated and not as a full-period result. |
| Verdict limited | The verdict matches the evidence; no profitability claim without out-of-sample evidence. |
| Residual risk recorded | The halt, sample, costs, engine, and machine limits are stated. |
| No approval | The validator does not approve, merge, or waive a control. |

## 6. Agent instructions

1. Read `AGENTS.md` section 10, this manual, the researcher's protocol, and the
   developer's record.
2. Do not modify the code, the configuration, the lab, or the criteria.
3. Verify the data version and configuration before reproducing.
4. Run a fresh backtest; do not reuse the developer's lab.
5. Compare all five bundles; if one differs, stop and report.
6. Run the causal and determinism tests and the availability sample.
7. Audit the split, variations, costs, fills, and risk controls.
8. Record the actual commands and observed results; do not invent evidence.
9. Write a verdict limited to what the evidence supports. Do not call a result
   profitable without out-of-sample evidence.
10. Pass `--operator agent` to `lab summary` when an agent produces the lab.
11. Hand the packet to the human. Do not approve, merge, release, or deploy.

## 7. Human instructions

Review the following before accepting the result:

- The data version and configuration match the protocol.
- The validator ran an independent backtest, not a copy of the developer's lab.
- All five bundles reproduce.
- The split is in-sample, out-of-sample, or walk-forward, and the gap is visible.
- The variations are recorded; no threshold was chosen after the result.
- The costs, fills, and risk controls match the configuration.
- A halted run is reported as truncated.
- The verdict is limited to the evidence.
- The residual risks are stated.

Decide one of:

| Decision | Meaning |
|---|---|
| **Accept** | The evidence supports the stated, limited conclusion. |
| **Revise** | The work needs a correction, a missing control, or more evidence. |
| **Reject** | The evidence does not support the conclusion or the risk is unacceptable. |

The validator's verdict is evidence, not approval. The human remains the final
authority.

## 8. Handoff packet template

```text
Handoff packet - Validator to Human
Work item:
Issue:
Researcher protocol:
Developer lab:
Validator lab:
Data version:
Config:
Reproduced:
Split:
Costs:
Look-ahead:
Survivorship:
Data snooping:
Halt:
Verdict:
Residual risks:
Decision requested:
```

## 9. Common failure modes

| Failure | Consequence | Correction |
|---|---|---|
| Treating an in-sample run as validation | The result is called promising without out-of-sample evidence. | Label the split and limit the verdict. |
| Ignoring costs | The result overstates the edge. | Audit commission, slippage, and funding against the configuration. |
| Not checking the split | Tuning and evaluation overlap. | Verify the periods before reading the result. |
| Not checking the halt | A truncated run is read as a full-period result. | Report `halted: true` and the drawdown limit. |
| Calling a same-engine rerun independent | The verdict overstates independence. | State the engine and machine limits. |
| Changing the criteria to pass | The validation is meaningless. | Freeze the criteria; reject or return the work. |
| Waiving a control | A known risk is hidden. | Report the failure; the human decides. |
| Claiming profitability | The evidence does not support it. | Limit the verdict to reproducibility or activity. |
| Not recording limitations | The result is read as stronger than it is. | Record the residual risks in the validation record. |
| Modifying the work under review | The validator becomes the author. | Read-only by default; hand findings back. |
| Reusing the developer's lab | The run is not independent. | Run a fresh backtest from the recorded inputs. |
| Ignoring unrecorded variations | Data snooping is hidden. | Count the variations; stop if the protocol is incomplete. |

## 10. Normative links

- [`AGENTS.md` section 10](../../../AGENTS.md#10-agentic-engineering-and-role-definitions)
- [Quant roles overview](quant-roles.md)
- [Full-pipeline tutorial](../../tutorials/2026-10-29/full-pipeline.md)
- [Four-axis analytical model](../../model/2026-10-21/four-axis-analytical-model.md)
- [Research output contract](../../analytics/2026-10-24/research-output.md)
- [Per-engine configuration](../../config/2026-10-28/per-engine-config.md)
- [Per-engine reports](../../reports/2026-10-28/per-engine-reports.md)
