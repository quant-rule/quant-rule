# QuantRule System Architecture

- Status: Current; approved 2026-11-12
- Global index: `docs/README.md`
- Decision: [`ADR-2026-11-12-01`](../adr/2026-11-12/01-adopt-distributed-service-based-architecture.md)

This folder is the entry point for the living system documentation. It holds the
product kata, the architecture characteristics, the service catalog, and the
runtime contracts. Decisions live in `docs/adr/`; model, configuration, and
report contracts live in their own folders.

## What QuantRule is

QuantRule is a distributed, single-operator research platform for rule-based
trading on OHLCV candles. It is not a prediction engine. It is an evidence
machine: a human can accept or reject a rule-based edge with reproducible,
causal, cost-inclusive evidence, including the answer "no edge."

Distribution exists because the work outlives the client and can exceed one
machine — not because there are many users.

## Pipeline and planes

```text
data plane (synchronous, deterministic)
  data → model → backtest → evidence

control plane (asynchronous)
  submit → queue → worker → finalize → completed
```

The gateway is the only client surface. The client is a browser. The shell is the
agent interface. Both speak `quantrule.api/v1`.

## Current status

This section is the single status index. Other documents link here instead of
repeating a status.

| Area | Status |
|---|---|
| Pipeline | Every stage is implemented: data, model, strategy, execution, portfolio, backtest, qualification, analytics. |
| Surfaces | Product 1.5.2 is a distributed platform with two clients: a Svelte browser client and a command shell, both on `quantrule.api/v1`. The command shell is a thin gateway client with no engine dependency; the six services are packaged in one image and nine-container stack. Windows and Linux are supported browser and CLI platforms; macOS is unsupported in v1.5.2. |
| Command surface | 49 implemented commands on the gateway-client surface in [command-surface.md](command-surface.md), including the model, strategy, and portfolio stage commands added in #852–#854, the optional `run submit --model-artifact` flag added in #855, and the research, registry, operations, and manual work through #859; the surface is released as v1.5.2, and the previous 27-command surface was released as v0.9.0 (prepared as v1.0.0). The discovery gate passes for the implemented set. |
| Stage surface | The stage contract, schema, worker, gateway, and the model, strategy, and portfolio slices are implemented (#850–#854); promotion is implemented (#855), the research ledger and manifest v3 are implemented (#856), the research commands and Research screen are implemented (#857), the operations surface and end-to-end verification are implemented (#858), and the 14-chapter user manual and checker are implemented (#859, #965, #970). [stage-surface.md](stage-surface.md). |
| Model | `M1.3` current; 49 indicators, 150 active events, 199 validation claims; accepted compression baselines; `M2.x` no edge; `M3.x` underpowered; `M4.x` no edge; `MR5.x` no edge; `MR6.x` no edge; `MR7.x` no edge. |
| Strategy | No accepted strategy baseline; the tier is gated by an accepted model baseline. |
| Portfolio | No accepted portfolio baseline; the tier is gated by an accepted strategy baseline. |
| Data partitions | `model_dev` 2018–2020, `model_validation` 2021–2022, `strategy_dev` 2018–2022, `strategy_validation` 2024, `portfolio_dev` 2024, `portfolio_holdout` 2025; 2023 excluded. A dense Binance BTCUSDT cross-venue source exists outside the partitions. |
| Improvement workflow | Current; the v3 research-ledger store, research commands, and Research screen are implemented; the SR1/SR2 strategy runs, the SR3 feature programme, the SR4 multi-instrument fade, the SR5 absorption count gate, the SR6 VWAP rejection run, the SR7 BTC+ETH H3 cross-section, the SR8 2023 buffer test, the SR8-A 2023 archive data rejection, the SR9 15m structure campaign, and the Binance BTCUSDT cross-venue check are recorded; accepted model baselines: 1m 4 ATR compression expansion, 1m compression-range, and 5m fresh-onset range; no accepted strategy or portfolio baseline. |
| Run contract | Current; `quantrule.improvement-run/v3` is current, v2 is historical. |
| Research swarm | Current contract; no active campaign. |
| Release tags | `v1.5.2` is the latest published release (2026-11-21), the Windows fixture recovery; `v1.5.0` and `v1.5.1` are withdrawn because their Windows CLI gate failed. `v1.4.2` is the second release recovery for the multi-host deployment; `v1.4.1` and `v1.4.0` are withdrawn because their workflows failed. `v1.3.0` is the stage execution surface (`sha256:d9b8daa1…`). `v1.2.0` is the version-alignment release (`sha256:26350916…`); `v1.1.0` is the 1.1.0 alignment release (`sha256:d5ce83da…`); `v1.0.0` is the desktop release, prepared as `v1.1.0` (`sha256:a133ff14…`); `v0.9.0` is the distributed platform (`sha256:1250438f…`). The tag mapping is in `docs/releases/README.md`. |
| Changelog | Grouped release history; `1.5.2` is the current section, and the historical headings are normalized to the current tags. |
| Tooling | The shell is the agent command path; it makes one gateway call per command. The improvement run store records evidence; the gateway `run submit` is the current execution path. `tools/gate.sh` runs the workspace gate. The distributed stack adds `make` targets and six services, specified in [deployment.md](deployment.md). The `simulation` service and the browser client own chart-first research; the Python simulation, chart, oracle, and trade tools are retired by DIST-105. The repository and the product image carry no Python ([zero-Python ADR](../adr/2026-11-21/992-zero-python-governance.md)). |
| Manual | Current; 14 chapters, the command and GUI references, the glossary, troubleshooting, client access, and multi-host (#859, #965, #970). [docs/manual/README.md](../manual/README.md) |

## Status of each document

| Document | Status | Answers |
|---|---|---|
| [`kata.md`](kata.md) | Current | Why the product exists |
| [`characteristics.md`](characteristics.md) | Current | Which qualities are measurable and how |
| [`services.md`](services.md) | Current; released as v0.9.0 (prepared as v1.0.0); current in v1.5.2 | Who does what, and what each service owns |
| [`research-artifacts.md`](research-artifacts.md) | Strategy and portfolio policy packs current (RA-002, RA-003); remaining artifacts target the next release; accepted by ADR-RA | The model artifact, packs, signal artifacts, run v5 and lineage |
| [`ha.md`](ha.md) | Target for the next release; accepted by ADR-HA | Swarm HA, four databases, replication, failure and evidence boundary |
| [`data-plane.md`](data-plane.md) | Current; released as v0.9.0 (prepared as v1.0.0); current in v1.5.2 | How one run stays deterministic |
| [`control-plane.md`](control-plane.md) | Current; released as v0.9.0 (prepared as v1.0.0); current in v1.5.2 | How a run is submitted, retried, and cancelled |
| [`http-api.md`](http-api.md) | Current; released as v0.9.0 (prepared as v1.0.0); current in v1.5.2 | The client contract |
| [`deployment.md`](deployment.md) | Current; released as v1.0.0 (prepared as v1.1.0); current in v1.5.2 | How to run it with one command |
| [`migrations.md`](migrations.md) | Current; accepted 2026-11-22 as HA-003 | Forward-only ledgers, expand/contract, job order, rollback |
| [`multi-host.md`](multi-host.md) | Current in v1.5.2; released as v1.4.2; hardening #980 | How the six services run across hosts |
| [`client.md`](client.md) | Current; product 1.5.2, current tag v1.5.2 | The browser client |
| [`user-journey.md`](user-journey.md) | Current; released as v1.0.0 (prepared as v1.1.0); current in v1.5.2 | How it is used and what it returns |
| [`security.md`](security.md) | Current; released as v0.9.0 (prepared as v1.0.0); current in v1.5.2 | The trust boundary |
| [`operations.md`](operations.md) | Current; released as v0.9.0 (prepared as v1.0.0); current in v1.5.2 | Running, backing up, recovering |
| [`module-map.md`](module-map.md) | Current; released as v0.9.0 (prepared as v1.0.0); current in v1.5.2 | Where code lives |
| [`command-surface.md`](command-surface.md) | Current; 49 commands, released as v1.5.2 | Implemented commands and the retirement record |
| [`stage-surface.md`](stage-surface.md) | Current; released as v1.3.0; current in v1.5.2 | The stage, research, and operations API, commands, services, and client screens |
| [`history-retention.md`](history-retention.md) | Current; implemented for #937 | Runtime history ownership, states, and safety |
| [S3 command-surface plan](../execution-plans/2026-11-06/s3-command-surface.md) | Approved 2026-11-06; all phases merged; human acceptance pending | The command-surface phases |

## Glossary

One term per concept. A synonym is a defect.

| Term | Meaning |
|---|---|
| `dataset` | An uploaded candle series with one `data_version` |
| `data version` | `sha256` of the normalized candle bytes |
| `config` | A run configuration document |
| `config hash` | `sha256` of the canonical config bytes |
| `run` | One execution of the full pipeline. An improvement run is the governed record around a coupled run |
| `research` | The client surface and activity: `research ...` commands, `/research` routes, and the Research screen |
| `improvement run` | The governed, pre-registered record created through the research surface; `quantrule.improvement-run/v3` |
| `submission` | The client request that creates or finds a run |
| `artifact` | One immutable output file of a run |
| `bundle` | The complete artifact set of one run |
| `canonical digest` | `sha256` over the sorted artifact hashes, excluding provenance |
| `service` | One of the six deployable responsibilities |
| `data plane` | The synchronous deterministic run path |
| `control plane` | The asynchronous lifecycle |
| `operator` | The single principal |
| `worker` | A `model` or `backtest` container instance |
| `stage` | One step of the data plane |
| `simulation run` | One chart-first research record, with an inputs digest and no output digest |
| `mark` | A timestamped good or bad entry or exit on a chart window |
| `oracle` | The deterministic hindsight labeler that marks the same window |
| `research plane` | The interactive, hindsight, isolated plane owned by `simulation` |
| `profile` | An organisational scope over records; not a security boundary |

## History

Retired documents keep their path and carry a status line. They are never
forked.

| Document | Status |
|---|---|
| `adr/2026-11-11/adopt-local-first-tauri-platform.md` | Superseded by ADR-2026-11-12-01 |
| `adr/2026-10-31/adopt-architecture-baseline.md` | Amended by ADR-2026-11-12-01 |
| `execution-plans/2026-11-11/588-tauri-platform.md` | Superseded by the distributed plan |
