# AI transparency log — DIST-202 filesystem content-addressed store

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Execution Plan; issue #694 (DIST-202, P2a of #692); claim type
  Behaviour Claim.
- Intent: implement the object-store adapter for the desktop profile over the
  filesystem with an atomic publish, content verification on write and read, and
  a traversal guard, following the approved v1.1 plan (PR #810), which splits
  #694 into P2a (filesystem CAS) and P2b (durable SQLite queue).
- Boundary: `crates/quantrule-runtime/src/adapters/filesystem.rs`,
  `crates/quantrule-runtime/src/adapters/mod.rs` (one module declaration), the
  filesystem tests, `CHANGELOG.md`, and this log. No `Cargo.toml`, `sqlite.rs`,
  schema, migration, service, desktop, Compose, Dockerfile, Makefile, CI, or
  port-parity change. No new dependency.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--694-storage`, branch
  `feat/694-storage`, base `6a714ba3` (v1.0.0 released).

## What was read

- `AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 12, 13, and 14.
- Issue #694, issue #693, and ADR-2026-11-12-12.
- `docs/architecture/services.md` section 7, `control-plane.md`, `data-plane.md`,
  ADR-2026-11-12-04, and the atomic-publish execution plan
  `docs/execution-plans/2026-11-14/654-atomic-publish.md`.
- The v1.1 plan in PR #810, P2a/P2b/P2c, decisions D5/D6, and verification rows
  V20–V26.
- `crates/quantrule-runtime/src/adapters/mod.rs`, `s3.rs`, `rabbitmq.rs`, the
  `ObjectStore`, `Queue`, and `QueueConsumer` traits, and the data-plane key
  layouts used by the data, model, backtest, and evidence services.

## Implemented (P2a)

- `FilesystemStore` implements `ObjectStore` over a root directory and reports
  `Backend::Filesystem`.
- `put` resolves and validates the key, verifies the bytes against a
  content-addressed key, then publishes through `spawn_blocking`: create the
  parent directory, write a unique temporary file in the same directory,
  `sync_all` it, rename it onto the key, and fsync the directory. A kill between
  the temporary write and the rename leaves a file that no key resolves, and a
  restart sees no object.
- `publish` makes an object immutable. An identical republish returns success
  without a second write; a different body under an existing key is refused with
  an `AlreadyExists` error. This is what makes a retry idempotent and keeps the
  publish atomic on Windows, where `rename` cannot replace an existing file.
- A key whose last segment is a bare lowercase SHA-256 (`artifacts/{sha256}`,
  `datasets/{sha256}`) is verified on `put` and on `get` with
  `quantrule_api::sha256_hex`. `exists` is a metadata check and reads no bytes.
  A `runs/{run_id}/manifest.json` key names a run, not the bytes, so it is not
  verified against its name.
- Key validation refuses an empty key, an absolute path, a backslash, a control
  character, the reserved characters `< > " | ? *`, an empty segment, `.` and
  `..` segments, a key over 1024 bytes, and a segment over 255 bytes. This mirrors
  the gateway's artifact-name rule and keeps an accepted key portable.
- The root directory belongs to the operator. A symlink planted inside the root
  is outside this adapter's threat model and is recorded in the module docs.

## Verification

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | passed |
| `cargo clippy -p quantrule-runtime --all-targets -- -D warnings` | passed |
| `cargo test -p quantrule-runtime --lib` | 59 passed, 0 failed |
| `cargo test -p quantrule-runtime --lib adapters::filesystem` | 13 passed, 0 failed |
| `cargo test --workspace` inside `tools/gate.sh` | passed; no failed target stopped the gate |
| `tools/gate.sh` | passed; every checker reported ok and the workspace gate printed `workspace gate passed` |

The 13 filesystem tests cover: a readable put and the `Filesystem` backend; nested
keys creating directories; a staged body invisible until commit; a real process
kill leaving no visible object; a fail-injected writer plus restart and a later
publish; a corrupt object failing the hash check; a put whose bytes do not match
its address writing nothing; traversal and non-portable keys refused on read,
write, and exists; digest equality across two puts of the same bytes; the store
used through `Arc<dyn ObjectStore>`; and an identical republish succeeding while
a different body is refused.

The killed-writer claim is covered twice. In
`a_killed_writer_process_leaves_no_visible_object`, the test spawns its own test
binary as a child with `--exact` and two environment variables; the child stages
a complete temporary file, records its path, and sleeps, and the parent waits for
that marker and calls `Child::kill()` (SIGKILL on Unix, TerminateProcess on
Windows). The final key is absent, the staged temporary file remains, and a new
store on the same root sees nothing. `a_killed_writer_leaves_no_visible_object`
is the fail-injection variant: it stages the temporary file in-process, stops
before the rename, and proves the restart path plus a later successful publish.
The test exercises the adapter's own process boundary; the composed tier's
service-level kill remains #665.

## Scope correction

The first pass also drafted an in-process queue over a JSON `QueueState` file, as
the #694 brief allowed while #693 was absent. The approved plan fixes P2b's
durable store as SQLite tables in the shared file (D6), and P2b is gated on P1a's
SQLite runtime. #693 has added
`rusqlite = { version = "0.42", features = ["bundled"] }` to
`crates/quantrule-runtime/Cargo.toml` in its own worktree, but
that change is not committed or merged. Shipping the file-backed queue would
have added a durable store the plan replaces and a second queue implementation
reviewed apart from P2b, and using `rusqlite` would have required duplicating
#693's dependency declaration. The queue files were therefore removed from this
PR. This log does not claim any queue behavior as delivered.

## Handoff to P2b (#694, blocked on #693/P1a)

- The dependency is already in #693's uncommitted
  `crates/quantrule-runtime/Cargo.toml`: `rusqlite` 0.42 with `bundled`. Do not
  add a second declaration; rebase onto #693 and use it through
  `spawn_blocking`.
- Planned owned paths: `crates/quantrule-runtime/src/adapters/inprocess.rs`,
  `queue.rs`, `config.rs`, and tests. The plan writes the file name without an
  underscore.
- Durable schema: one table for pending records (id, routing key, payload,
  attempts, in-flight, visible-at) plus a cancellation table, in the shared
  SQLite file with WAL and a busy timeout. One transaction per claim, ack, and
  nack.
- Delivery contract from `control-plane.md` §3: three deliveries, 5 s after the
  first failure and 25 s after the second, then dead-letter `run.dead` with the
  original payload bytes and attempt count. A lease makes a killed consumer's
  in-flight record visible again; a redelivery is flagged and carries the same
  bytes. Cancellation is durable by routing key and payload digest.
- `queue.rs` may need explicit claim/ack/nack operations if the service needs
  them; the current `QueueConsumer` shape acknowledges inside the adapter, which
  was enough for a process-local prototype but is not evidence for P2b.
- P2c wires the six services. This PR wires none and leaves `config.rs`,
  `QR_PROFILE`, and `QR_DATA_DIR` to P1a/P2c so it does not collide with #693.

## Learning and rejected alternatives

- `canonicalize` as the traversal guard was rejected: it needs the root to exist
  and turns a key check into filesystem I/O. Segment validation is local,
  deterministic, and testable without a directory.
- Writing directly to the final key was rejected: it violates the `put`
  atomicity contract in `adapters/mod.rs` and the data-plane failure table.
- Allowing a different overwrite on Unix and refusing it only on Windows was
  rejected: the same key would then have different behavior per profile. The
  stricter immutable rule matches every current key (content-addressed or unique
  per run) and is the plan's Windows assumption.
- Verifying only `artifacts/{sha256}` was rejected after reading the data
  service: the upload key is `datasets/{data_version}` and the worker verifies
  `Digest::sha256(bytes) == data_version`, so the bare-digest-last-segment rule
  covers both layouts.
- Wrapping the blocking file I/O in the async executor was rejected: `put` uses
  `spawn_blocking`, and `get` uses `tokio::fs`, so a large artifact does not
  block a runtime worker.
- A separate helper binary for the kill test was rejected: re-executing the test
  binary with `--exact` and two environment variables keeps the kill inside the
  crate with no new binary, dependency, or build target. A shell `kill -9` was
  rejected because `Child::kill()` already sends SIGKILL on Unix and terminates a
  process on Windows, so the test uses the standard library only.
- The kill child sleeps until the parent reaps it; a marker file, not a fixed
  sleep in the parent, is what makes the handshake reliable on a slow machine.
