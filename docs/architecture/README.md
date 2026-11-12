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

## Status of each document

| Document | Status | Answers |
|---|---|---|
| [`kata.md`](kata.md) | Current | Why the product exists |
| [`characteristics.md`](characteristics.md) | Current | Which qualities are measurable and how |
| [`services.md`](services.md) | Target v1.0.0 | Who does what, and what each service owns |
| [`data-plane.md`](data-plane.md) | Target v1.0.0 | How one run stays deterministic |
| [`control-plane.md`](control-plane.md) | Target v1.0.0 | How a run is submitted, retried, and cancelled |
| [`http-api.md`](http-api.md) | Target v1.0.0 | The client contract |
| [`deployment.md`](deployment.md) | Target v1.0.0 | How to run it with one command |
| [`client.md`](client.md) | Target v1.0.0 | The browser client |
| [`user-journey.md`](user-journey.md) | Target v1.0.0 | How it is used and what it returns |
| [`security.md`](security.md) | Target v1.0.0 | The trust boundary |
| [`operations.md`](operations.md) | Target v1.0.0 | Running, backing up, recovering |
| [`module-map.md`](module-map.md) | Target v1.0.0 | Where code lives |
| [`command-surface.md`](command-surface.md) | Current | Implemented and planned commands |

`Target` means the decision is accepted and the document is the implementation
contract; the code does not exist yet. The status flips to `Current` at
`DIST-090`.

## Glossary

One term per concept. A synonym is a defect.

| Term | Meaning |
|---|---|
| `dataset` | An uploaded candle series with one `data_version` |
| `data version` | `sha256` of the normalized candle bytes |
| `config` | A run configuration document |
| `config hash` | `sha256` of the canonical config bytes |
| `run` | One execution of the full pipeline. A research record is always written **improvement run** |
| `submission` | The client request that creates or finds a run |
| `artifact` | One immutable output file of a run |
| `bundle` | The complete artifact set of one run |
| `canonical digest` | `sha256` over the sorted artifact hashes, excluding provenance |
| `service` | One of the five deployable responsibilities |
| `data plane` | The synchronous deterministic run path |
| `control plane` | The asynchronous lifecycle |
| `operator` | The single principal |
| `worker` | A `model` or `backtest` container instance |
| `stage` | One step of the data plane |
| `simulation run` | One chart-first research record, with an inputs digest and no output digest |
| `mark` | A timestamped good or bad entry or exit on a chart window |
| `oracle` | The deterministic hindsight labeler that marks the same window |
| `research plane` | The interactive, hindsight, isolated plane owned by `simulation` |

## History

Retired documents keep their path and carry a status line. They are never
forked.

| Document | Status |
|---|---|
| `adr/2026-11-11/adopt-local-first-tauri-platform.md` | Superseded by ADR-2026-11-12-01 |
| `adr/2026-10-31/adopt-architecture-baseline.md` | Amended by ADR-2026-11-12-01 |
| `execution-plans/2026-11-11/588-tauri-platform.md` | Superseded by the distributed plan |
