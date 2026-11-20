# Stage, Research, and Operations Surface

- Date: 2026-11-17 (UTC)
- Status: Current; released as v1.3.0. The stage contract and schema are merged (#850), the worker and gateway landed in #851, the model stage and Workbench Data/Model tabs landed in #852, the strategy stage and Workbench Strategy tab landed in #853, the portfolio stage and Workbench Portfolio tab landed in #854, backtest promotion landed in #855, the research ledger and manifest v3 landed in #856, the research commands and Research screen landed in #857, the operations surface and end-to-end verification landed in #858, and the user manual landed in #859
- Part of: [`README.md`](README.md)
- Claim type: Behaviour Claim
- Decision: [ADR-2026-11-16-01](../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)
- Related: [Engine Execution](../workflows/engine-execution.md), [Backtest](../workflows/backtest.md), [End-to-End Verification](../workflows/end-to-end-verification.md), [Command surface](command-surface.md), [HTTP API](http-api.md), [Client](client.md), [Services](services.md)

## 1. Purpose

The stage surface lets the trader run the model, strategy, and portfolio engines
alone for diagnosis and iteration. It is a **diagnostic** path: a stage run has
no canonical digest, is never a baseline, and never enters the evidence path.
The coupled backtest remains the only evidence run.

This document describes the implemented API, commands, services, and client
screens. The status table in §8 names what each slice covers.

## 2. Evidence boundary

| Rule | Meaning |
|---|---|
| Stage = diagnostic | A stage artifact is never evidence, qualification input, or a baseline |
| Model reuse | A backtest may reuse a frozen model artifact; the model bytes must equal a full run |
| Strategy/portfolio recompute | The backtest recomputes strategy, execution, and portfolio; stage decisions and fills are not inputs |
| TTL | Stage artifacts expire under `QR_STAGE_RETENTION_DAYS`; evidence is permanent |
| One evidence command | `run submit` remains the only evidence command; no separate backtest command is added |

## 3. API surface

The implemented base contract is in [`http-api.md`](http-api.md); the stage, research, and operations surface is implemented.

| Method | Path | Purpose |
|---|---|---|
| POST | `/stages/{kind}/runs` | Submit a model, strategy, or portfolio stage run |
| GET | `/stages/runs` | List stage runs by kind, state, and limit |
| GET | `/stages/runs/{id}` | Read stage state, provenance, and artifact |
| GET | `/stages/runs/{id}/artifacts` | List artifact files |
| GET | `/stages/runs/{id}/artifacts/{*name}` | Download one verified artifact |
| POST | `/stages/runs/{id}/cancel` | Cancel a queued or running stage |
| DELETE | `/stages/runs/{id}` | Delete a diagnostic stage artifact |
| POST | `/research/runs` | Create an improvement-run record |
| GET | `/research/runs` | List improvement runs |
| GET | `/research/runs/{id}` | Read one improvement record |
| POST | `/research/runs/{id}/submit` | Submit and link the coupled pipeline run |
| GET | `/research/runs/{id}/compare` | Compare published numbers with a baseline |
| POST | `/research/runs/{id}/decision` | Record the human accept, reject, or no-edge |
| GET | `/research/baselines` | Read the baseline registry |
| GET | `/research/baselines/{tier}` | Read the current tier baseline |
| GET | `/research/invalidations` | Read the invalidation tombstones |
| GET | `/operations` | Read retention days and the last cleanup |

A strategy submission keeps the input
`{kind:"model_artifact", model_artifact_id}`. The server resolves the succeeded
model stage in the active profile and derives `data_version` from its
provenance; a supplied `data_version` that disagrees is refused. An unknown or
non-model artifact returns `stage.not_found`, and a mismatch returns
`stage.invalid`.

A portfolio submission keeps the input
`{kind:"decisions_artifact", decisions_artifact_id}`. The server resolves the
succeeded strategy stage and derives `data_version` and `model_version` from its
provenance; an unknown or non-strategy artifact returns `stage.not_found`, and a
`data_version` or model-version mismatch returns `stage.invalid`.

The gateway proxies `/stages/*`, `/research/*`, and `/operations` with the same profile header, bearer token, path allowlist, and error envelope; it never reaches an engine.
## 4. Command surface

The current 46 commands are in [`command-surface.md`](command-surface.md). The
model stage commands (`stage run model`, `stage list`, `stage show`, `stage
events`, `stage fetch`, `stage cancel`, and `stage delete`) are implemented
(#852); `stage run strategy`, `stage explain`, and `stage run portfolio` are
implemented (#853–#854). Promotion is implemented (#855), the research commands
and Research screen are implemented (#857), and the operations surface is
implemented (#858):

| Command | Purpose |
|---|---|
| `stage run model <data_version> <config_id>` | Compute and store a model artifact |
| `stage run strategy <model_artifact_id> <config_id>` | Compute a decisions artifact |
| `stage run portfolio <decisions_artifact_id> <config_id>` | Compute fills and metrics |
| `stage show <stage_run_id>` | Read the stage summary and provenance |
| `stage events <stage_run_id> --timeframe <tf>` | Read a model stage event batch |
| `stage explain <stage_run_id> --bar <n>` | Explain one strategy decision |
| `stage fetch <stage_run_id> <dir>` | Download the verified artifact set |
| `stage list`, `stage cancel`, `stage delete` | Manage diagnostic stage runs |
| `run submit <data_version> <config_id> [--model-artifact <id>]` | Coupled evidence run with an optional frozen model artifact (#855) |
| `research create/submit/list/show/compare/decide` | Governed improvement-run records (#857) |
| `baseline list/show`, `invalidation list` | Read the registries |

A target command appears in `help` and the README only when it is implemented and
tested. The command-surface checker keeps planned and implemented commands
separate.

`stage show` lists a stage's artifacts; `stage fetch` is the CLI inspection path
for a published bundle such as `portfolio/metrics.json`, including fees,
funding, drawdown, and exposure.

The standalone strategy stage simulates its own position; the coupled backtest
passes the position execution actually holds. When execution rejects an entry,
the coupled strategy sees a flat position on the next bar, while the stage still
holds the position it entered. The decision sequences can therefore differ, and
a stage decision is never fed into a backtest. The divergence is deliberate and
is covered by the worker's rejected-entry test.

## 5. Clients

| Client | Target surface |
|---|---|
| Browser | **Workbench** with Data, Model, Strategy, Portfolio tabs; **Backtests** inbox and detail; **Research** records and decisions; **Operations** health, profiles, verification, retention |
| Desktop | The same browser screens over the gateway URL; no engine and no local data |
| Shell | The command table above, one gateway call per command, `--profile`, `--token`, and `--json` |

Every target screen and command must land in both clients in the same change. A
capability that only one client can reach is a parity gap.

## 6. Service ownership

| Service | Stage responsibility |
|---|---|
| `evidence` | Owns the `stage` schema: `stage.runs`, `stage.submissions`, `stage.artifacts`; stage lifecycle, profile scoping, expiry; reports retention and the last cleanup on `/operations` |
| `backtest` | Stage worker: claims a stage, dispatches to the model, strategy, or portfolio executor, publishes the artifact, publishes `stage.computed` |
| `model` | Computes the model bundle that a model stage publishes; serves model-stage event reads |
| `data` | Supplies the dataset and config bundle; never executes a stage |
| `gateway` | Proxies `/stages/*`, `/research/*`, and `/operations`; validates the profile and token; holds no stage state |
| `simulation` | Unchanged; the research plane never reads a stage |

No service reads another service's schema. The stage store is in the evidence
service; the worker reads it through the `quantrule.internal/v1` contract.

## 7. Lifecycle

```text
submit -> queued -> running -> succeeded
                         |-> failed
                         |-> cancelled
```

A stage submission is content-addressed by kind, input, config hash, model
version, and image digest. A duplicate submission returns the existing stage.
A succeeded stage names its artifact; a failed stage names its error. A killed
worker leaves no visible artifact and no `succeeded` stage. A stage artifact is
profile-scoped and content-addressed.

## 8. Status

| Capability | Status |
|---|---|
| Stage API types, internal contract, `stage` schema, and store | Implemented (#850) |
| Stage worker, queue, retention, and gateway proxy | Implemented (#851) |
| Model stage vertical slice and Workbench Data/Model tabs | Implemented (#852) |
| Strategy stage vertical slice and Workbench Strategy tab | Implemented (#853) |
| Portfolio stage vertical slice and Workbench Portfolio tab | Implemented (#854) |
| Backtest promotion with a frozen model artifact | Implemented (#855) |
| Research ledger and Research screens | Implemented (#856–#857) |
| Operations surface and end-to-end verification | Implemented (#858) |
| Manual chapters and command/GUI reference | Implemented (#859) |

## 9. Related documents

- [ADR-2026-11-16-01](../adr/2026-11-16/01-stage-execution-and-evidence-boundary.md)
- [Stage execution plan](../execution-plans/2026-11-16/842-stage-execution-surface.md)
- [Engine Execution workflow](../workflows/engine-execution.md)
- [Backtest workflow](../workflows/backtest.md)
- [Command surface](command-surface.md)
- [HTTP API](http-api.md)
- [Client](client.md)
- [Services](services.md)
- [`docs/manual/README.md`](../manual/README.md)
