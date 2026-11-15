# AI transparency log — DIST-203 desktop wiring and parity

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan + Requirements & Design; issue #695 (DIST-203, P2c,
  P2d, and P3d of #692); claim type Behaviour Claim.
- Intent: wire the embedded backends into the six services through one
  construction path and prove the desktop profile completes the verify-mini
  fixture with no Docker, then compare its canonical digest, artifacts, and run
  projection with the server profile. The server profile is the oracle and the
  digest is never adjusted.
- Boundary: the six service `main.rs` binaries, `quantrule-runtime` backend
  selection and two liveness probes, `tools/verify.sh` (one additive evidence
  flag), `tools/parity_desktop.sh`, the desktop-profile documentation,
  `CHANGELOG.md`, the plan's observed column, and this log. No route, use case,
  engine, model, strategy, execution, portfolio, migration SQL, Compose,
  Dockerfile, release-workflow, dependency, or P1e guard change.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--695-wiring`, branch
  `feat/695-wiring`, base `3d66cbb6`; `origin/main` merged at `34fbba2a` (#818
  P1e conformance) before verification.

## What was read

- `AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 12, 13, and 14.
- The v1.1 plan P2c/P2d/P3d, decisions D5, D6, D8, D10, and verification rows
  V27, V28, V29, and V37–V39.
- All six service `src/main.rs`/binary files; the runtime `Database`,
  `ObjectStore`, `Queue`, and `QueueConsumer` traits; `sqlite.rs`,
  `filesystem.rs`, `inprocess.rs`, `rabbitmq.rs`, `s3.rs`, `config.rs`,
  `health.rs`, and `queue.rs`.
- The supervisor `run.rs` and `cli.rs` (the environment a child receives), the
  staging script, `tools/verify.sh`, `tools/gate.sh`, and the P1e guard and
  contract (`tools/check_schema_prefixes.py`, `docs/architecture/desktop.md`).

## Implemented

- `quantrule_runtime::backends` is the one construction path. `Objects` selects
  `S3Store` or `FilesystemStore` and implements `ObjectStore`; `ControlQueue`
  selects `RabbitQueue` or `InProcessQueue` and implements both `Queue` and
  `QueueConsumer`. `objects_path` and `database_path` derive
  `<QR_DATA_DIR>/objects` and `<QR_DATA_DIR>/quantrule.sqlite` from the one
  configured data directory, and `open_database` returns `None` on the server
  profile.
- `data`, `model`, `evidence`, `simulation`, and `backtest` select the object
  store from the profile. `evidence` and `backtest` select the queue from the
  profile; evidence passes the handle its declared desktop branch opened, so the
  run store and the queue share one connection in one process. The in-process
  queue is one `queue__messages`/`queue__dead_letter` pair on the shared file,
  not one queue per process.
- The gateway needed no queue change: its cancel route proxies to evidence, and
  evidence publishes `run.cancelled` through the selected queue. The profile
  store branch is unchanged.
- `FilesystemStore::check` and `InProcessQueue::check` make the existing
  `/health` probes honest on the embedded backends without widening a trait.
- `tools/verify.sh --evidence-dir` retains the downloaded canonical artifacts,
  each run status, the assigned config and run ids, and the digest, so a parity
  harness can compare bytes rather than two printed lines.
- `tools/parity_desktop.sh` starts the desktop profile (six direct children or
  `qr-desktop`), runs the fixture, and compares the canonical digest, every
  canonical artifact byte, and the full run status projection (25-entry manifest
  and stage states). Its server oracle is `--server-url` or an isolated
  `quantrule-parity` Compose project running the six binaries in the server
  profile; it verifies the project label before it tears anything down. It
  never needs Docker for the desktop side.
- The server profile remains the default, every HTTP contract is unchanged, and
  the six sidecars start with `QR_PROFILE=desktop`, one absolute `QR_DATA_DIR`,
  and the URL environment the supervisor sets.

## Verification

| Command | Observed |
|---|---|
| `python3 tools/check_schema_prefixes.py` | `schema prefixes ok: 6 ledger rows, 13 tables, 4 baselines` |
| `cargo test -p quantrule-runtime --lib backends` | 9 passed, 0 failed |
| `tools/parity_desktop.sh --server-compose --repeat 2` | pass; both rounds `sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa`; 24 canonical artifact files byte-identical per round; run status identical |
| Desktop-only parity with `--via-supervisor` | pass; six children, five health endpoints, two runs, same digest; no sidecar survived the stop |
| Seeded mismatch (`--expected sha256:000…`) | exit 1; printed expected `sha256:000…` and observed `sha256:453c90b4…` |
| `cargo fmt --all -- --check` | passed; unchanged after the `origin/main` merge |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed; no warning in the gate output |
| `cargo test --workspace` | passed; 2,131 passed, 0 failed, 14 ignored across 104 targets |
| `tools/gate.sh` | passed; `workspace gate passed`, 22 checker steps, including `schema prefixes ok: 6 ledger rows, 13 tables, 4 baselines` |

The fixture produced 12 canonical artifacts and 3,559,171 bytes per run. The
two profiles derived the same run ids (`sha256:1a0b7cf1…` and
`sha256:f78973d0…`) because the harness pins `QR_IMAGE_DIGEST`; the canonical
digest is independent of it. The digest also equals the value recorded for the
same fixture in the v1.0.0 release note.

## Limitations

- The server oracle in this run is the six binaries in the server profile
  against Compose-provided Postgres, MinIO, and RabbitMQ, not the released image
  container. The canonical digest is identical to the v1.0.0 image's recorded
  digest for the same fixture.
- The in-process queue polls the shared file every 50 ms per consumer and takes
  a short immediate transaction per claim. The fixture and both profiles pass,
  but two consumers add write-lock traffic; a slow disk is the residual risk.
- The supervisor stop is a process kill, not a drain; no sidecar survived the
  observed stop, and the queue redelivers an unacknowledged message.
- Windows, the packaged AppImage, and the NSIS installer were not exercised on
  this Linux host. P3c's jobs and the Windows smoke remain release-tier.
- The parity harness's default `QR_IMAGE_DIGEST` is the v1.0.0 image digest as a
  pinned run-identity fixture, not a claim that this tree is that image.

## Learning and rejected alternatives

- Constructing one `BackendSet` with the database, store, and queue for every
  service was rejected: the model service uses neither a database nor a queue,
  and constructing one would add a dependency the service does not have. Three
  narrowly selected constructors keep each service's dependency set honest.
- Moving the SQLite open into `backends` for every service broke the merged P1e
  guard, which requires each ledger owner's binary to name its desktop branch,
  `QR_DATA_DIR`, `DATABASE_FILE`, and its own ledger. The correction restores
  each service's declared branch and keeps `open_database` for the backtest
  worker, which owns no schema; the guard's intent is met, not worked around.
- A queue-owned retry policy was rejected again: the worker and the run row own
  the 5 s/25 s policy, and the shared adapter must stay identical to RabbitMQ's
  immediate requeue.
- Starting the server dependencies on the default Compose project was rejected:
  `docker compose down -v` would delete the operator's volumes. The harness uses
  the isolated `quantrule-parity` project and refuses teardown when the project
  label does not match.
- Using a checked-in golden as the only oracle was rejected by the plan; the
  full parity run compares against the server profile, and `--expected` is
  documented as the pre-package pre-check.
- Changing the digest, the fixture, or the expected value on a mismatch was not
  considered; the compare is the oracle.

## Handoff

- `Refs #695`, not `Closes`: the six-sidecar run and the digest parity pass, but
  the full done-when also names the packaged AppImage/NSIS smoke and Windows
  evidence, which this run did not exercise.
- The release note still needs the desktop-profile sentence and the unsigned
  artifact path for P4 (#692).
