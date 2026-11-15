# AI transparency log — DIST-202 durable in-process queue

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #694 (DIST-202, P2b of #692); claim type
  Behaviour Claim.
- Intent: implement the durable queue for the desktop profile over the SQLite
  runtime that P1a merged. The queue persists a message before returning,
  leases a claim, redelivers after a killed consumer's lease expires, applies a
  caller-supplied backoff, dead-letters with the original bytes, and cancels by
  routing key and payload digest, following the approved v1.1 plan (PR #810).
- Boundary: `crates/quantrule-runtime/src/adapters/inprocess.rs`,
  `crates/quantrule-runtime/src/queue.rs`,
  `crates/quantrule-runtime/tests/inprocess_queue.rs`,
  `crates/quantrule-runtime/src/adapters/mod.rs` (one module declaration and a
  comment), `CHANGELOG.md`, and this log. No `sqlite.rs`, `Database` trait,
  `config.rs`, schema migration, service crate, desktop shell, gateway, Compose,
  Dockerfile, Makefile, CI, or port-parity change. No new dependency.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--694-queue`, branch
  `feat/694-queue`, base `3772ecdf` (P1a SQLite runtime and P2a filesystem CAS
  merged).

## What was read

- `AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 12, 13, and 14.
- Issue #694, the v1.1 plan P2b and decision D6, and verification rows V23-V26.
- `docs/architecture/control-plane.md` sections 2-6, the #653 plan and retry
  contract, and the P2a AI log's handoff to P2b.
- `crates/quantrule-runtime/src/adapters/mod.rs`, `sqlite.rs`, `filesystem.rs`,
  `rabbitmq.rs`, `queue.rs`, `config.rs`, and the gateway SQLite adapter.
- The backtest worker's `retry.rs` and `main.rs`, and the evidence consumer.

## Implemented (P2b)

- `InProcessQueue` implements `Queue` and `QueueConsumer`, reports
  `Backend::InProcess`, and stores state in two tables in the shared SQLite
  file: `queue__messages` and `queue__dead_letter`. Both use the `queue__`
  prefix from `Dialect::Sqlite.table(Schema::Queue, ...)`.
- `queue__messages` carries `id`, `routing_key`, `payload` (BLOB),
  `payload_digest` (SHA-256, for cancellation), `attempts`, `lease_owner`,
  `lease_expires_at`, `visible_at`, `created_at`, and `updated_at`. A check
  keeps the two lease columns null together. An index covers the ready order
  `(visible_at, id)` and another covers `(routing_key, payload_digest)`.
- `queue__dead_letter` carries `id`, `routing_key` (`run.dead`),
  `source_routing_key`, the original `payload` and digest, `attempts`, `reason`,
  `created_at`, and `dead_lettered_at`.
- A publish commits one immediate transaction before it returns. A claim picks
  the oldest ready row for the requested keys, increments `attempts`, and takes
  a lease in one immediate transaction. Ack deletes the row; nack clears the
  lease and sets `visible_at` to the caller's delay. Each operation is one
  transaction, so two consumers cannot claim the same row and a crash leaves
  the previous committed state.
- The handler contract matches RabbitMQ: `Ok` acks, `Err` nacks with immediate
  requeue and returns the handler error. The worker keeps its own 5 s / 25 s
  backoff and third-failure `run.dead`; the queue does not invent a second
  policy (plan D6 and `control-plane.md` section 3).
- `dead_letter` inserts the dead-letter row and deletes the pending row in one
  transaction, copying the original bytes and source key. `publish("run.dead")`
  writes the dead-letter table directly because no consumer drains that queue.
- `cancel(routing_key, payload)` deletes ready rows whose key and payload digest
  match, in one transaction, and is idempotent. A leased row is left to its
  consumer because the run row is authoritative; an expired lease is treated as
  ready.
- The clock is injected as `Now = Arc<dyn Fn() -> u64 + Send + Sync>`. Production
  passes the system clock; tests pass a fixed, advanceable clock, so lease and
  backoff tests do not sleep. The idle poll interval is real time because it
  only decides when a consumer looks again.
- The queue relies on `SqliteDatabase::open` for WAL, the five-second busy
  timeout, foreign keys, and `synchronous=FULL`. It never opens a second
  connection or changes those pragmas.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed after two test-only lint fixes (a `while let` loop and a mutex guard held across an await) |
| `cargo test -p quantrule-runtime` | passed; 73 library tests, 5 binary-inventory tests, 1 boundary test, 18 queue tests, and 13 SQLite tests; the Postgres lock test ignored as designed |
| `cargo test -p quantrule-runtime --test inprocess_queue` | 18 passed, 0 failed |
| `cargo test --workspace` | passed; 2064 passed, 0 failed, 11 ignored across 108 targets |
| `tools/gate.sh` | passed; every checker reported ok and the script printed `workspace gate passed`. It ran with `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` because the machine was under sustained multi-worktree build load; the setting changes debug symbols only, and the exact post-edit tree also passed `cargo test --workspace` without it |

The integration harness covers: publish/claim/ack; a nack redelivering the same
bytes with digest equality; three deliveries with 5 s and 25 s backoff followed
by a dead letter carrying the original bytes under `run.dead`; lease expiry with
the old acknowledgement refused; cancellation by key and digest across a reopen,
including a wrong key and a repeated cancel; cancellation leaving an in-flight
delivery; two queues on two connections delivering 32 messages exactly once;
publish and ack durability across a reopen; an unacked delivery surviving a
reopen and being redelivered after its lease; consumer ack and immediate
requeue; the trait-object boundary; `run.dead` publishing; key filtering; the
empty-key case; WAL and the busy timeout; and an empty routing key refusal.

## Limitations

- A lease expiry is simulated by dropping the queue without an ack and advancing
  the injected clock. That proves the persisted lease and the redelivery path,
  not an operating-system process kill; the composed tier owns a service kill.
- The two-consumer test uses two SQLite handles in one process, not two
  operating-system processes. WAL and `busy_timeout` are exercised across two
  connections, which is the contention the plan names; a multi-process run
  belongs to P2c/P3.
- `dead_letter` and `cancel` are adapter operations; the service binaries do not
  call them yet. P2c wires the queue into the six services.
- The worker's in-place retry is unchanged. The three-delivery test applies the
  worker's policy through the explicit operations; the adapter's consumer keeps
  the RabbitMQ-compatible immediate requeue.
- The lease default is 300 s. A handler that runs longer than the lease can be
  delivered twice. The default is deliberately long, but a slow consumer is a
  residual risk; P2c may tune the lease or add a renewal.

## Main merge (2026-11-15, after PR review)

`origin/main` moved to `2e964f5b` (#816 evidence SQLite, #813 desktop package).
Merging it into `feat/694-queue` produced one conflict, in `CHANGELOG.md`: main
still carried the P2a sentence saying the durable queue was gated on #693, while
this branch had added the queue entry. The resolution keeps the evidence SQLite,
filesystem CAS, `InProcessQueue`, and desktop-package entries, and drops the
stale gating sentence. `Cargo.lock` and the code merged without a conflict.

The merged tree was re-verified:

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo test -p quantrule-runtime` | passed; 73 library, 5 inventory, 1 boundary, 18 queue, and 13 SQLite tests; the Postgres lock test ignored as designed |
| `cargo test --workspace` | passed; 2082 passed, 0 failed, 12 ignored |
| `tools/gate.sh` | passed; 2082 tests passed, every checker reported ok, and the script printed `workspace gate passed`. It ran with `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` under build load; the same tree passed `cargo test --workspace` without that setting |

No queue code changed in the merge; the branch is up to date with the current
main and the PR is #814.

## Main merge 2 (2026-11-15, after #815)

`origin/main` then moved to `e9b46e51` (#815 data SQLite). The merge into
`feat/694-queue` was automatic: `CHANGELOG.md` kept the evidence, gateway,
filesystem, queue, desktop-package, and new data entries, and `Cargo.lock` and
the code merged without a conflict. Merge commit `c3dd679b`.

The merged tree was re-verified:

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo test -p quantrule-runtime` | passed; 73 library, 5 inventory, 1 boundary, 18 queue, and 13 SQLite tests; the Postgres lock test ignored as designed |
| `cargo test --workspace` | passed; 2102 passed, 0 failed, 13 ignored |
| `tools/gate.sh` | passed; 2102 tests passed, every checker reported ok, and the script printed `workspace gate passed`. It ran with `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` under build load; the same tree passed `cargo test --workspace` without that setting |

No queue code changed in either merge; the PR remains #814 and keeps
`Closes #694` because the filesystem CAS and the durable queue are complete.

## Learning and rejected alternatives

- A queue-level retry policy was rejected: the approved plan and the #653
  contract put the attempt count and the 5 s/25 s policy in the worker and the
  run row. Duplicating it in the queue would make the desktop and server
  profiles disagree and could dead-letter before the worker marks a run failed.
- A separate cancellation table was rejected: the plan's queue inventory has
  two tables, and a durable delete plus the existing `run.cancelled` message is
  enough. The run row remains the source of truth.
- Auto-cancelling a request when `run.cancelled` is published was rejected: the
  RabbitMQ profile does not remove a pending request, and the worker checks the
  run row at its boundaries. The explicit `cancel` method is available for a
  caller that wants the optimization.
- `UPDATE ... RETURNING` was not used. An immediate transaction makes the
  select-then-update claim safe and easier to review against the schema.
- Using `sqlite.rs`'s migration ledger for the queue tables was rejected: the
  queue is runtime infrastructure, not a service schema, and the plan lists it
  alongside the shared ledger. Idempotent DDL at open is enough for the two
  fixed tables; a future shape change becomes a migration.
- A `QueueConsumer` that dead-letters after three handler errors was rejected
  for the same reason as the retry policy: the handler returns `Err` only for
  an unexpected failure, and the worker's own loop decides when the budget is
  spent.

## Handoff to P2c

- Construct one `InProcessQueue` per service that uses the queue, from the
  service's `SqliteDatabase` handle and `QR_DATA_DIR`.
- Wire `publish(ROUTING_DEAD, ...)` to land in `queue__dead_letter`, which this
  adapter does, and decide whether the evidence cancel route also calls
  `cancel(ROUTING_REQUESTED, payload)`.
- Pass the worker's lease and decide whether a long run needs a lease renewal;
  the default is 300 s.
- Run the full fixture on SQLite, the filesystem store, and this queue before
  packaging (V27).
