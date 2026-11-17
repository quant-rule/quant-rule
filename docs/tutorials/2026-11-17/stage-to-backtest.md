# Tutorial — Stage to Backtest

- Date: 2026-11-17 (UTC)
- Status: Current
- Audience: operator moving from diagnostic stages to a coupled evidence run
- Related: [Shell HTTP tutorial](../2026-11-14/shell-http.md),
  [End-to-end verification tutorial](end-to-end-verification.md),
  [Engine Execution workflow](../../workflows/engine-execution.md),
  [Backtest workflow](../../workflows/backtest.md)

This tutorial runs the model, strategy, and portfolio stages in order, then
submits a promoted coupled run that reuses the frozen model artifact. The stages
are diagnostic. The coupled run is evidence.

## 1. Prerequisites

| Item | Value |
|---|---|
| Stack | `make up`, healthy |
| Profile | The profile that will own every stage and run |
| Dataset | A `data_version` with enough history |
| Config | A valid run config that carries a strategy |
| Shell | `QUANTRULE_API_URL=http://localhost:8080/api/v1` |

```bash
make up
export QUANTRULE_API_URL=http://localhost:8080/api/v1
quantrule health
quantrule profile use default
quantrule dataset upload config/examples/verify-mini/candles.csv --name stage-to-backtest
quantrule config list
quantrule config show <config_id>
```

Use the `data_version` printed by the upload and a `valid` config id.

## 2. Model stage

```bash
quantrule stage run model <data_version> <config_id>
quantrule stage list --kind model --status succeeded --limit 20
quantrule stage show <model_stage_run_id>
quantrule stage fetch <model_stage_run_id> ./stage-model
```

`stage show` prints `evidence: false` and an `artifact_id`. That id is the
`model_artifact_id` the next stage and the promoted run name.

| What | Expect |
|---|---|
| Submit | `stage_run_id`, `state: queued`, `created: true` |
| Detail | `evidence: false`, provenance, artifact id, file list |
| Artifact | Model bundle copied byte-for-byte; `report.json`, `readings.csv`, `events.csv`, `levels.csv`, `legs.csv`, `summary.txt`, `provenance.json` |
| Fetch | Every file checked against its recorded size and SHA-256 |

## 3. Strategy stage

```bash
quantrule stage run strategy <model_artifact_id> <config_id>
quantrule stage show <strategy_stage_run_id>
quantrule stage explain <strategy_stage_run_id> --bar 12
quantrule stage fetch <strategy_stage_run_id> ./stage-strategy
```

The strategy stage derives the dataset from the succeeded model artifact. The
decisions bundle is exactly `decisions.ndjson`, `summary.txt`, and
`provenance.json`; it carries no fills, cash, P&L, exposure, drawdown, or
canonical digest.

The standalone strategy simulates its own position. The coupled run gives the
strategy the position execution actually holds, so a rejected entry can make the
two decision sequences differ. That divergence is deliberate.

## 4. Portfolio stage

```bash
quantrule stage run portfolio <decisions_artifact_id> <config_id>
quantrule stage show <portfolio_stage_run_id>
quantrule stage fetch <portfolio_stage_run_id> ./stage-portfolio
```

The portfolio stage derives dataset and model identity from the succeeded
strategy artifact. The artifact has two prefixed bundles:

| Prefix | Files |
|---|---|
| `execution/` | `orders.csv`, `fills.csv`, `rejections.csv`, `summary.txt`, `provenance.json` |
| `portfolio/` | `trades.csv`, `equity.csv`, `funding.csv`, `risk_events.csv`, `metrics.json`, `summary.txt`, `provenance.json` |

Read `portfolio/metrics.json` for fees, funding, drawdown, and exposure. The
published ledger invariant is `final_equity = initial_cash + net_pnl`. The P&L is
diagnostic accounting, not evidence.

## 5. Promoted coupled run

```bash
quantrule run submit <data_version> <config_id> --model-artifact <model_artifact_id>
quantrule run status <run_id>
quantrule run fetch <run_id> ./backtest-bundle
quantrule verify <run_id>
```

| Submission | Model | Strategy | Execution | Portfolio |
|---|---|---|---|---|
| Full run | Computed | Computed | Computed | Computed |
| Promoted run | Reused byte-for-byte | Recomputed | Recomputed | Recomputed |

The run record carries `model_artifact_id`. Its model files and canonical digest
must equal a full run with the same inputs. A frozen artifact that is missing,
non-succeeded, or mismatched is refused with `run.model_artifact_invalid`.

## 6. Expected output

| Step | Expect |
|---|---|
| `run status` | `status: succeeded`, four stages done, provenance, artifact list, digest |
| `run fetch` | One line per artifact, each hash matching the record |
| `verify` | `run_id` and the canonical digest |

The digest covers `results.json`, `*.csv`, and `*.ndjson`; it excludes
`provenance.json`, manifests, summaries, logs, and timestamps. One run is not
reproducibility evidence. Two equivalent runs that print the same digest are; see
the [end-to-end tutorial](end-to-end-verification.md).

## 7. Failure actions

| Code or symptom | Cause | Next action |
|---|---|---|
| `stage.not_found` | Unknown stage or upstream artifact | Re-read the stage list; use this profile |
| `stage.invalid` | Wrong artifact kind, config, or provenance | Read the message; fix the input |
| `stage.conflict` | Terminal stage or no artifact yet | Read `stage show`; wait or submit again |
| `config.invalid` | Config or strategy validation failed | Fix the config; do not bypass the validator |
| `run.model_artifact_invalid` | Frozen artifact missing, failed, or mismatched | Re-run the model stage or choose a matching artifact |
| `run.not_found` | Unknown run id | Re-read `run list` |
| `run.conflict` | Cancel after a terminal state | Read the run; do not retry |
| `service.unavailable` | A required service is down | Read `health`; stop |
| Artifact hash mismatch | Storage or transfer defect | Re-fetch; do not use the bytes |

## 8. What this proves

| Path | Proves |
|---|---|
| Model stage | The model ran and published a verifiable diagnostic artifact |
| Strategy stage | The rule ran and published explainable decisions |
| Portfolio stage | Frozen decisions replayed through execution and accounting |
| Promoted run | The coupled evidence path reuses the model and recomputes the rest |
| `verify` | The recorded canonical digest is readable and intact |

None of the stages is a baseline, an edge, or a qualification verdict. The
promoted run is the evidence unit; the human owns any later acceptance.
