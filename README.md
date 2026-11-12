# QuantRule

QuantRule is a local-first desktop research workstation for a solo systematic
researcher. It turns an OHLCV hypothesis into reproducible,
portfolio-constrained evidence. Data, configs, runs, analytics, and research
memory stay on the machine. There is no account, no cloud runtime, and no live
trading.

**Current release:** [`v0.9.0` — The Tauri Platform](https://github.com/quant-rule/quant-rule/releases/tag/v0.9.0) (2026-11-11).
Linux `.deb` and `.AppImage` installers are attached to the release. Linux is the
current target; Windows and macOS are future work.

## Start

Desktop:

```bash
cd apps/desktop
npm ci
npm run tauri dev
```

CLI:

```bash
cargo build --release
./target/release/quantrule
```

Build the Linux bundles and static page with `./site/build.sh`. Native
prerequisites and desktop tests are in [`apps/desktop/README.md`](apps/desktop/README.md).

## What it does

One deterministic pipeline over local OHLCV candles:

`data → model → strategy → execution → portfolio → qualification → analytics`

Each stage can run on its own and hands off an explicit artifact; `backtest run`
orchestrates the same engines as one coupled run. The model publishes numbers and
codes, not labels or predictions. Strategy, execution, portfolio, and
qualification remain separate boundaries.

The current model is [`M1.3`](docs/model/2026-11-10/model-m1.3.md). Every stage is
implemented. No strategy or portfolio baseline is accepted, and no edge is
claimed. The status index is [`docs/README.md`](docs/README.md).

## Commands

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

Full usage: run `quantrule help <command>`, or see the
[command registry](docs/architecture/command-surface.md) and
[`docs/tutorials/`](docs/tutorials/).

## Principles

- **Local-first.** No account, cloud runtime, or remote data store.
- **Deterministic.** Same candles, data version, and configuration produce the
  same output. No wall-clock time, unseeded randomness, or unordered iteration
  in the signal path.
- **Candles-only and rule-based.** No tick data, order books, machine learning,
  or statistical signal generation.
- **Human acceptance.** The engine measures and evaluates; the human owns the
  criteria and the final decision. No command declares an edge.

## Documentation

| Document | What it answers |
|---|---|
| [`docs/README.md`](docs/README.md) | Documentation index and current status |
| [`docs/architecture/README.md`](docs/architecture/README.md) | Architecture baseline and pipeline |
| [`docs/architecture/command-surface.md`](docs/architecture/command-surface.md) | Command registry, stage contract, and discovery gate |
| [`docs/workflows/README.md`](docs/workflows/README.md) | Workflow index and selection guide |
| [`docs/model/2026-11-10/model-m1.3.md`](docs/model/2026-11-10/model-m1.3.md) | Current model contract |
| [`docs/config/2026-11-11/run-v3-and-portfolio-constraints.md`](docs/config/2026-11-11/run-v3-and-portfolio-constraints.md) | Run v3 and portfolio constraints |
| [`docs/registry/README.md`](docs/registry/README.md) | Indicator, event, parameter, and validation registry |
| [`docs/registry/history.md`](docs/registry/history.md) | Retired surfaces and history boundary |
| [`docs/releases/2026-11-11/v0.9.0.md`](docs/releases/2026-11-11/v0.9.0.md) | Current release note |
| [`CHANGELOG.md`](CHANGELOG.md) | Grouped release history |
| [`AGENTS.md`](AGENTS.md) | Repository rules, workflow, and approval gates |

## Repository

| Path | Role |
|---|---|
| `crates/quantrule-data` | Data engine |
| `crates/quantrule-model` | Model engine |
| `crates/quantrule-backtest` | Strategy, execution, portfolio, and benchmark engines |
| `crates/quantrule-app` | Shared application service |
| `crates/quantrule-shell` | Command shell |
| `apps/desktop` | Tauri 2 + Svelte 5 desktop UI |
| `site/` | Linux bundles, checksums, and static download page |
| `docs/` | Contracts, workflows, plans, releases, and course deliverables |
| `tools/` | Data, chart, trade, research, and checker tooling |

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The full gate, including documentation, registry, command-surface, and desktop
checks, is in [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

## Non-goals

- Tick data, order books, machine learning, and statistical signal generation.
- A server, cloud runtime, accounts, multi-tenancy, live trading, or an embedded
  library.
- A profitability claim from a single backtest, or treating a green backtest as
  an accepted edge.
