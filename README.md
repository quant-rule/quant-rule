# QuantRule

QuantRule turns an OHLCV hypothesis into reproducible, portfolio-constrained
evidence — and keeps that evidence auditable after the client that submitted it
has closed. It is candles-only, rule-based, and deterministic. There is no
account, no cloud provider, and no live trading.

> **Status.** The current release tag is **v1.5.2** (2026-11-21). The desktop
> distribution is retired, Python is removed from the repository, and the browser
> and headless CLI are the only clients. One image and seven services serve the
> server profile. Windows and Linux are the supported browser and CLI platforms;
> macOS is unsupported. The browser client and command shell share one
> `quantrule.api/v1` contract. **v1.5.0** and **v1.5.1** are withdrawn because
> their Windows CLI gate failed; **v1.4.2** is the previous published release;
> **v1.3.0** is the stage execution surface; **v1.2.0** is the version-alignment
> release; **v1.1.0** is the 1.1.0 alignment release; **v1.0.0** is the historical
> desktop release. Current note:
> [`docs/releases/2026-11-21/v1.5.2.md`](docs/releases/2026-11-21/v1.5.2.md);
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

`make up` creates the local secrets and internal PKI, builds the image, starts
the stack, waits for health, and prints:

```text
QuantRule v1.5.2 ready → http://localhost:8080
```

`make` is the Linux path. Windows runs the same Compose file directly,
and make is optional there:

```powershell
docker compose build
docker compose create gateway
$env:QR_IMAGE_DIGEST = (docker compose images -q gateway | Select-Object -First 1)
docker compose up -d --wait
```

A direct Compose start must create the secret files and PKI and set
`QR_IMAGE_DIGEST` to the built image id; `make up` does all three. A server
without the digest refuses run and stage submissions instead of inventing a
build identity. PostgreSQL, RabbitMQ and MinIO require TLS; see the
[internal TLS contract](docs/config/2026-11-23/internal-tls.md).

```bash
tools/secrets.sh
tools/pki.sh
docker compose --env-file secrets/compose.env build
QR_IMAGE_DIGEST="$(docker compose --env-file secrets/compose.env images -q gateway | head -n1)" \
  docker compose --env-file secrets/compose.env up -d --wait
```

Fedora may use Docker Engine or Podman: `make up COMPOSE="podman compose"`. The
default stack starts ten long-running containers, a one-shot PKI staging job, a
one-shot database bootstrap, and four one-shot migration jobs; see
[`docs/architecture/deployment.md`](docs/architecture/deployment.md).

| Target | Effect |
|---|---|
| `make help` | List every target |
| `make up` / `make secrets` / `make pki` | Create missing secrets and PKI, build, start, wait, print the URL |
| `make down` | Stop, keep data |
| `make reset` | Stop and delete volumes, after confirmation |
| `make logs` / `make ps` | Observe |
| `make scale N=4` | Scale model and backtest when the multi-worker profile is enabled |
| `make verify` | Submit the two fixture configs through the gateway and print the canonical digest |
| `make test` / `make gate` | Fast tier / full workspace gate |
| `make test-composed` / `test-parity` / `test-ha` | Composed, port-parity, and opt-in Swarm HA lab tiers |
| `make remote` | Write `secrets/bearer_token`, publish only `:8080`, print the token and usage |
| `make token` | Print a new bearer token; `--out` writes a mode-600 file |
| `make compose-check` | Validate both Compose files with disposable secrets and PKI |
| `make backup` / `make restore FILE=…` | Backup and restore (DIST-080, #671) |
| `make dev-web` | Web client dev server against `localhost:8080` |

`make remote` reuses `QR_BEARER_TOKEN` or generates one, writes
`secrets/bearer_token` at mode 600, layers `compose.remote.yaml`, starts the
stack with only `:8080` published, and prints the token and a `curl` example.
The browser enters the token on the Connection screen and the shell reads
`--token`, `QUANTRULE_TOKEN` or `QUANTRULE_TOKEN_FILE`; both then send
`Authorization: Bearer` on every API call. Rotate by running `make remote` with
a new `QR_BEARER_TOKEN`; the script copies it into the gateway and signals a
reload. Keep the host on a private network, or terminate TLS at a reverse proxy;
internal PostgreSQL, RabbitMQ and MinIO traffic already uses TLS. See
[`docs/architecture/security.md`](docs/architecture/security.md) §2,
[file-backed secrets](docs/config/2026-11-23/secret-files.md), the
[internal TLS contract](docs/config/2026-11-23/internal-tls.md), and the
[remote tutorial](docs/tutorials/2026-11-12/03-remote-compute.md).
`make backup` and `make restore` call `tools/backup.sh` and `tools/restore.sh`.

To keep a token across restarts, generate one first:

```bash
export QR_BEARER_TOKEN="$(./tools/token.sh)"
make remote
export QUANTRULE_TOKEN="$QR_BEARER_TOKEN"
curl -fsS -H "Authorization: Bearer $QUANTRULE_TOKEN" http://localhost:8080/profiles
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

## Clients and platforms

QuantRule has two clients and no desktop application:

| Client | Windows | Linux | macOS |
|---|---|---|---|
| Browser GUI | Yes | Yes | **No** |
| `quantrule` CLI | Yes | Yes | **No** |

The browser is the human workstation. The CLI is the agent and automation
client. Both reach the same gateway and hold no engine, database, or queue. There
is no Tauri shell, AppImage, NSIS, EXE, MSI, deb, rpm, or dmg.

Build the CLI from source:

```bash
cargo build --release -p quantrule-shell
```

A local server uses Docker or Podman Compose on Linux, or Docker Desktop on
Windows. macOS is unsupported in the next release. The step-by-step procedure is
[user manual chapter 13](docs/manual/13-client-access.md).

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
- **Seven services.** `gateway`, `data`, `model`, `backtest`, `evidence`,
  `simulation`, `strategy`; the v2 multi-host profile places six and the v3
  Swarm HA profile places all seven behind one 443-only ingress and a
  keepalived VIP.
- **One image.** One digest pins the whole compute stack, so version skew cannot
  occur.
- **References, not payloads.** Stages exchange content hashes; bytes live in
  object storage.
- **The engines did not change.** Every pipeline service is a thin adapter over
  `quantrule-app`.
- **No Python.** The repository, image, and tools are Rust; the client is built
  with Bun.

Details: [`services.md`](docs/architecture/services.md),
[`data-plane.md`](docs/architecture/data-plane.md),
[`control-plane.md`](docs/architecture/control-plane.md),
[`http-api.md`](docs/architecture/http-api.md),
[`multi-host.md`](docs/architecture/multi-host.md).

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
| `history list` | History | List one runtime kind, active or including archived |
| `history archive` | History | Archive one run, research record, simulation, or dataset |
| `history erase` | History | Erase one archived record after typed confirmation |
| `help` | Shell | Show the command map and contracts |
| `pack create` | Research artifacts | Register a local pack document and print its record |
| `pack list` | Research artifacts | List packs with an optional kind and lifecycle filter |
| `pack show` | Research artifacts | Read one pack record and its lifecycle history |
| `pack validate` | Research artifacts | Re-validate one draft pack |
| `pack freeze` | Research artifacts | Freeze one validated pack |
| `pack retire` | Research artifacts | Retire one frozen pack |
| `pack fetch` | Research artifacts | Download one pack's verified bytes |
| `signal create` | Research artifacts | Compute a signal from a frozen model artifact and strategy pack |
| `signal list` | Research artifacts | List signal metadata in the active profile |
| `signal show` | Research artifacts | Read one signal artifact record |
| `signal fetch` | Research artifacts | Download one signal's verified document |
| `run lineage` | Research artifacts | Read the references a run recorded |

### Stage, research, and operations surface

The stage and research surface is implemented: diagnostic stage runs, the
coupled `run submit --model-artifact` path, and the research records, baselines,
and invalidations. The operations surface adds `GET /operations` for retention
and cleanup status and the browser Operations screen. The research artifact
surface adds pack lifecycle, signal metadata and verified downloads, and run
lineage in both clients; the browser reaches them on the Packs screen and the
run detail. The contract is
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
| [`docs/architecture/multi-host.md`](docs/architecture/multi-host.md) | How the six services of the v2 profile run across hosts; v3 adds the seventh |
| [Deployment v3](docs/config/2026-11-22/deploy-v3.md) | The Swarm HA descriptor, render targets, pinned stateful units, check, and remote/overlay inspector |
| [Ingress](docs/config/2026-11-23/ingress.md) | Traefik behind a keepalived VIP, DNS/TLS, cert rotation, headers, body limits, ingress check rows, private-CA client trust, and the VIP-loss lab |
| [Swarm control plane](docs/config/2026-11-23/swarm-control.md) | Managers, node labels, drift refusal, rolling update, remote check, `ha` tier, and `tools/swarm.sh` |
| [HA deployment manual](docs/manual/15-ha-deployment.md) | Render, bootstrap, check, upgrade, and rotate the v3 profile |
| [`docs/architecture/cross-database-consistency.md`](docs/architecture/cross-database-consistency.md) | Outbox, dataset references, reconciliation and tombstones across four databases |
| [`docs/architecture/observability.md`](docs/architecture/observability.md) | Metrics, JSON logs, request/trace ids, and their non-goals |
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
| `crates/quantrule-runtime` | Configuration, logging, metrics, health, storage, queue, migrations |
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

`tools/gate.sh --changed <range>` classifies docs, governance, web, and full
changes and runs only the work the change can observe. Engine/shared changes,
`main`, and releases always run the full gate. The contract is
[`docs/workflows/change-scope.md`](docs/workflows/change-scope.md).

## Non-goals

- Tick data, order books, machine learning, statistical signal generation.
- Accounts, multi-tenancy, billing, cloud provider deployment, high availability.
- Live trading, or a profitability claim from a single backtest.
- A third-party embedded library or a public API.
