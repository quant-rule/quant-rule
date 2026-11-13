# QuantRule

QuantRule turns an OHLCV hypothesis into reproducible, portfolio-constrained
evidence — and keeps that evidence auditable after the client that submitted it
has closed. It is candles-only, rule-based, and deterministic. There is no
account, no cloud provider, and no live trading.

> **Status.** The architecture target is **v1.0.0**, a distributed
> single-operator platform. The current tree is the v0.9.0 local surface. The
> distributed implementation is tracked by `DIST-000` and specified in
> [`docs/architecture/`](docs/architecture/README.md). Nothing in this README
> claims that the distributed surface is implemented.

## Why it is distributed

Not because there are many users. Because the work outlives the client, can
exceed one machine, and must not collapse with the UI.

| Reason | Consequence |
|---|---|
| A run is longer than a session | Submit, close the client, return to the result |
| Compute can be larger than the laptop | Move workers to another host |
| A crash must not take the UI down | Services fail independently |
| The CPU-bound stage is shardable | Scale `model` and `backtest` workers |

A monolith is the better choice when runs are seconds long and stay on one
machine. That was true before; it is not true now.

## Start (target v1.0.0)

```bash
git clone git@github.com:quant-rule/quant-rule.git
cd quant-rule
make up
```

`make up` builds the image, starts the stack, waits for health, and prints:

```text
QuantRule v1.0.0 ready → http://localhost:8080
```

| Target | Effect |
|---|---|
| `make down` | Stop, keep data |
| `make reset` | Stop and delete volumes |
| `make logs` / `make ps` | Observe |
| `make scale N=4` | Scale model and backtest workers |
| `make verify` | Print the canonical digest |
| `make remote` | Publish only `:8080` behind a token |

Full deployment contract: [`docs/architecture/deployment.md`](docs/architecture/deployment.md).

## How it is used

1. `make up`, open the URL.
2. Upload candles once; the server returns a `data_version`.
3. Pick a config, submit. The response is immediate.
4. Close the laptop. The run continues.
5. Return: stages, provenance, artifacts, canonical digest.
6. Compare the digest, or hand it to a reviewer.

Full journey: [`docs/architecture/user-journey.md`](docs/architecture/user-journey.md).

## Desktop

A Tauri shell is published from GitHub Releases. It is a window over the gateway
URL: no Docker, no local data, no engine. The server remains the same stack.
Details: [`docs/adr/2026-11-12/12-desktop-distribution.md`](docs/adr/2026-11-12/12-desktop-distribution.md).

## The research loop

Before a run, the platform supports chart-first exploration: create a simulation
run from a recorded bundle, mark good and bad entries and exits on the chart with
the future visible, compare against a deterministic oracle, and check the implied
rule on a later unmarked window. It is hindsight by design, it is isolated by
test, and a strategy can never read it.

Details: [`docs/architecture/user-journey.md`](docs/architecture/user-journey.md).

## Architecture in one screen

```text
browser ── HTTP ── gateway ── postgres
                     │            ▲
                     │            └── evidence ── minio
                     ▼
                  rabbitmq ── backtest ── minio
                                │   └── model
                                └────── data
```

- **Two planes.** A synchronous, deterministic data plane computes one run. An
  asynchronous control plane decides which run happens when.
- **Six services.** `gateway`, `data`, `model`, `backtest`, `evidence`, `simulation`.
- **One image.** One digest pins the whole compute stack, so version skew cannot
  occur.
- **References, not payloads.** Stages exchange content hashes; bytes live in
  object storage.
- **The engines did not change.** Every pipeline service is a thin adapter over
  `quantrule-app`.
- **No Python in the product.** The image is Rust and static assets; the client is
  built with Bun.

Details: [`services.md`](docs/architecture/services.md),
[`data-plane.md`](docs/architecture/data-plane.md),
[`control-plane.md`](docs/architecture/control-plane.md),
[`http-api.md`](docs/architecture/http-api.md).

## Commands (v0.3.0 tree)

The table below is the implemented surface today, not the v1.0.0 target. The
v1.0.0 surface is listed as `Planned` in
[`docs/architecture/command-surface.md`](docs/architecture/command-surface.md)
section 9.

| Command | Stage | Purpose |
|---|---|---|
| `candles` | Data | Print normalized candles as NDJSON |
| `candles-tolerant` | Data | Print candles and a quality report |
| `timeframes` | Data | Print candles for each configured timeframe |
| `events` | Model | Print the event batch |
| `report model` | Model | Write the model bundle |
| `strategy explain` | Strategy | Explain one decision at a bar |
| `strategy run` | Strategy | Write the strategy decisions bundle |
| `execution run` | Execution | Write the execution fills bundle |
| `portfolio replay` | Portfolio | Write the portfolio accounting bundle |
| `backtest run` | Orchestration | Run the coupled backtest and write the manifest |
| `qualify` | Qualification | Evaluate evidence against human-owned criteria |
| `analytics evaluate` | Analytics | Write metrics, benchmarks, and evaluation tables |
| `run check` | Configuration | Validate the run configuration |
| `fetch` | Data | Fetch a Dukascopy range to CSV |
| `memory` | Memory | Read and write the local research memory |
| `help` | Shell | Show the command map and contracts |

## Principles

- **Deterministic.** Same data version, config, and image digest produce the same
  artifacts. No wall-clock time, unseeded randomness, or unordered iteration in
  the signal path.
- **Auditable.** Every result names its data version, config hash, model version,
  and image digest.
- **Candles-only and rule-based.** No ticks, order books, machine learning, or
  statistical signal generation.
- **Single principal.** No accounts. Remote exposure is a network decision.
- **Human acceptance.** The engine measures and evaluates; the human owns the
  criteria and the final decision.

## Documentation

| Document | Answers |
|---|---|
| [`docs/README.md`](docs/README.md) | Index and current status |
| [`docs/architecture/README.md`](docs/architecture/README.md) | Architecture entry point and glossary |
| [`docs/architecture/kata.md`](docs/architecture/kata.md) | Why the product exists |
| [`docs/architecture/characteristics.md`](docs/architecture/characteristics.md) | Measurable qualities and fitness functions |
| [`docs/architecture/deployment.md`](docs/architecture/deployment.md) | How to run it |
| [`docs/workflows/README.md`](docs/workflows/README.md) | Workflow index |
| [`docs/model/2026-11-10/model-m1.3.md`](docs/model/2026-11-10/model-m1.3.md) | Current model contract |
| [`CHANGELOG.md`](CHANGELOG.md) | Grouped release history |
| [`AGENTS.md`](AGENTS.md) | Repository rules and approval gates |

## Repository

| Path | Role |
|---|---|
| `crates/quantrule-data` | Data engine |
| `crates/quantrule-model` | Model engine |
| `crates/quantrule-backtest` | Strategy, execution, portfolio, benchmark |
| `crates/quantrule-app` | Shared application service |
| `crates/quantrule-api` | HTTP contract types (target) |
| `crates/quantrule-server` | Gateway and workers (target) |
| `crates/quantrule-shell` | Command shell |
| `apps/web` | Svelte 5 browser client, built with Bun |
| `docs/` | Contracts, workflows, plans, releases, course |
| `tools/` | Data, research, and checker tooling |

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
tools/gate.sh
```

## Non-goals

- Tick data, order books, machine learning, statistical signal generation.
- Accounts, multi-tenancy, billing, cloud provider deployment, high availability.
- Live trading, or a profitability claim from a single backtest.
- A third-party embedded library or a public API.
