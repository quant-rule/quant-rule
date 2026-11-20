# QuantRule

QuantRule turns an OHLCV hypothesis into reproducible, portfolio-constrained
evidence — and keeps that evidence auditable after the client that submitted it
has closed. It is candles-only, rule-based, and deterministic. There is no
account, no cloud provider, and no live trading.

> **Status.** The current release tag is **v1.3.0** (2026-11-17), the stage
> execution surface release: diagnostic model, strategy, and portfolio stages,
> verified stage artifacts and Workbench tabs, promoted
> `run submit --model-artifact`, the v3 research ledger and Research screen,
> the Operations screen and `GET /operations`, and the 12-chapter user manual.
> One image and six services serve the server profile; one installer starts the
> same six sidecars on embedded SQLite, a filesystem CAS, and a durable
> in-process queue for the desktop profile. The browser client, command shell,
> and desktop shell share one `quantrule.api/v1` contract. The previous tag,
> **v1.2.0**, is the version-alignment release; **v1.1.0** is the 1.1.0
> alignment release; **v1.0.0** is the desktop release. Current note:
> [`docs/releases/2026-11-17/v1.3.0.md`](docs/releases/2026-11-17/v1.3.0.md);
> previous note:
> [`docs/releases/2026-11-16/v1.2.0.md`](docs/releases/2026-11-16/v1.2.0.md);
> mapping: [`docs/releases/README.md`](docs/releases/README.md). The
> architecture is specified in
> [`docs/architecture/`](docs/architecture/README.md).

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

## Start

```bash
git clone git@github.com:quant-rule/quant-rule.git
cd quant-rule
make up
```

`make up` builds the image, starts the stack, waits for health, and prints:

```text
QuantRule v1.3.0 ready → http://localhost:8080
```

`make` is the Linux and macOS path. Windows runs the same Compose file directly,
and make is optional there:

```powershell
docker compose up -d --build --wait
```

A direct Compose start must also set `QR_IMAGE_DIGEST` to the built image id;
`make up` records it automatically. A server without the digest refuses run and
stage submissions instead of inventing a build identity.

Fedora may use Docker Engine or Podman: `make up COMPOSE="podman compose"`. The
default stack starts all nine containers; see
[`docs/architecture/deployment.md`](docs/architecture/deployment.md).

| Target | Effect |
|---|---|
| `make help` | List every target |
| `make up` | Build, start, wait for health, print the URL |
| `make down` | Stop, keep data |
| `make reset` | Stop and delete volumes, after confirmation |
| `make logs` / `make ps` | Observe |
| `make scale N=4` | Scale model and backtest when the six-binary profile is enabled |
| `make verify` | Submit the two fixture configs through the gateway and print the canonical digest |
| `make test` / `make gate` | Fast tier / full workspace gate |
| `make remote` | Start with only `:8080` published and a bearer token |
| `make token` | Print a new bearer token |
| `make backup` / `make restore FILE=…` | Backup and restore (DIST-080, #671) |
| `make dev-web` | Web client dev server against `localhost:8080` |

`make remote` generates a token when `QR_BEARER_TOKEN` is unset, starts the stack
with only `:8080` published, and prints the token and a `curl` example. The
browser enters the token on the Connection screen and the shell reads `--token`
or `QUANTRULE_TOKEN`; both then send `Authorization: Bearer` on every API call.
Keep the host on a private network, or terminate TLS at a reverse proxy. See
[`docs/architecture/security.md`](docs/architecture/security.md) §2 and the
[remote tutorial](docs/tutorials/2026-11-12/03-remote-compute.md).
`make backup` and `make restore` call `tools/backup.sh` and `tools/restore.sh`.

To keep a token across restarts, generate one first:

```bash
export QR_BEARER_TOKEN="$(./tools/token.sh)"
make remote
curl -fsS -H "Authorization: Bearer $QR_BEARER_TOKEN" http://localhost:8080/profiles
```

`make verify` needs a healthy stack (`make up`). It uploads the checked-in
`config/examples/verify-mini` data and two configs with the same effective
behavior, submits both runs through the gateway, and prints one canonical
digest. A reviewer compares the line from a second host with
`tools/verify.sh --expected "canonical digest sha256:…"`.

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

A Tauri package is published from GitHub Releases. The desktop release is
tagged `v1.0.0` (prepared as v1.1.0; the tag was renumbered on 2026-11-15) and
adds a supervisor that starts the six bundled services on loopback with the
embedded SQLite, filesystem CAS, and durable in-process queue. There is no
Docker requirement, no engine in the shell, and no local data read by the shell;
the server remains the same stack and the same HTTP contract. The packaged
desktop profile and the server profile both reproduce canonical digest
`sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa` for
the verification fixture. The current release is `v1.3.0`; `v1.2.0` is the
version-alignment release, `v1.0.0` is the desktop release, and `v0.9.0` the
distributed platform release.
Details: [`apps/desktop-shell/README.md`](apps/desktop-shell/README.md) and
[ADR-12](docs/adr/2026-11-12/12-desktop-distribution.md).

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

## Commands (current surface)

The shell is a thin client over `quantrule.api/v1`. Most commands make one
gateway call; `run fetch`, `stage fetch`, `stage explain`, and `stage events`
read more than one. It depends only on the contract crate and its HTTP
transport, and it holds no engine. Every command accepts `--token` and `--json`;
`--profile` is the global selection flag except on `profile use`, which names
the profile as its argument. Local source fetch and local stage execution were
removed in the distributed release (v0.9.0, prepared as v1.0.0).

The [user manual](docs/manual/README.md) teaches each journey and lists the
browser control for every command; the table below is the command index.

| Command | Family | Purpose |
|---|---|---|
| `health` | Gateway | Read the service states, queue depth, and worker count |
| `dataset list` | Catalog | List datasets with coverage |
| `dataset show` | Catalog | Read one dataset record |
| `dataset upload` | Catalog | Upload a CSV and print its `data_version` |
| `config list` | Catalog | List run configurations and validity |
| `config show` | Catalog | Read one configuration summary |
| `run submit` | Runs | Submit a run over a dataset and config, optionally reusing a frozen model artifact |
| `run status` | Runs | Read stages, provenance, artifacts, and digest |
| `run list` | Runs | List runs with an optional state filter |
| `run fetch` | Runs | Download a run's artifacts into a directory |
| `run cancel` | Runs | Cancel a queued or running run |
| `stage run model` | Stages | Submit a diagnostic model stage |
| `stage run strategy` | Stages | Submit a strategy stage over a frozen model artifact |
| `stage run portfolio` | Stages | Submit a portfolio stage over a frozen decisions artifact |
| `stage explain` | Stages | Read one published strategy decision |
| `stage list` | Stages | List diagnostic stage runs |
| `stage show` | Stages | Read one stage detail and artifact files |
| `stage events` | Stages | Read the model event batch and coverage |
| `stage fetch` | Stages | Download a stage's verified artifacts |
| `stage cancel` | Stages | Cancel a queued or running stage |
| `stage delete` | Stages | Delete one diagnostic stage |
| `research create` | Research | Pre-register a v3 improvement run |
| `research submit` | Research | Submit the coupled pipeline run |
| `research list` | Research | List improvement runs |
| `research show` | Research | Read one improvement run and lazy digest |
| `research compare` | Research | Compare published numbers with a baseline |
| `research decide` | Research | Record one human decision and registry row |
| `baseline list` | Research | List accepted baselines |
| `baseline show` | Research | Show the current tier baseline |
| `invalidation list` | Research | List invalidation tombstones |
| `inspect candles` | Inspection | Read candle counts and coverage |
| `inspect timeframes` | Inspection | Read bars and partial buckets per timeframe |
| `inspect events` | Inspection | Read the event batch for one timeframe |
| `verify` | Evidence | Print the recorded canonical digest |
| `profile list` | Profiles | List profiles with their record counts |
| `profile create` | Profiles | Create a profile, or return the existing one |
| `profile use` | Profiles | Store the local default profile |
| `simulation list` | Research | List simulation runs |
| `simulation create` | Research | Create a simulation run from a bundle |
| `simulation windows` | Research | Select the simulation windows |
| `simulation mark` | Research | Add one human mark |
| `simulation marks` | Research | List human and oracle marks |
| `simulation oracle` | Research | Read the deterministic oracle marks |
| `simulation check` | Research | Run the event study and read its verdict |
| `simulation decide` | Research | Record test, refine, or stop |
| `help` | Shell | Show the command map and contracts |

### Stage, research, and operations surface

The stage and research surface is implemented: diagnostic stage runs, the
coupled `run submit --model-artifact` path, and the research records, baselines,
and invalidations. The operations surface adds `GET /operations` for retention
and cleanup status and the browser Operations screen. The contract is
[`docs/architecture/stage-surface.md`](docs/architecture/stage-surface.md).

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
| [User manual](docs/manual/README.md) | Every journey, command, screen, term, and error |
| [Stage to backtest tutorial](docs/tutorials/2026-11-17/stage-to-backtest.md) | Diagnostic stages to a promoted evidence run |
| [End-to-end verification tutorial](docs/tutorials/2026-11-17/end-to-end-verification.md) | Clean-stack proof and the two-run digest |
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
| `crates/quantrule-api` | HTTP contract types |
| `crates/quantrule-service-api` | Internal service contract types |
| `crates/quantrule-runtime` | Configuration, health, storage, queue, migrations |
| `crates/quantrule-gateway` | Gateway: profiles, static client, proxy, health |
| `crates/quantrule-data-service` | Data service (`qr-data`) |
| `crates/quantrule-model-service` | Model service (`qr-model`) |
| `crates/quantrule-backtest-service` | Backtest worker (`qr-backtest`) |
| `crates/quantrule-evidence-service` | Evidence service (`qr-evidence`) |
| `crates/quantrule-simulation-service` | Simulation service (`qr-simulation`) |
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
