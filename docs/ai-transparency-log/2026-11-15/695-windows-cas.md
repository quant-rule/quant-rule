# AI transparency log — Windows filesystem CAS key mapping (#695)

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: unblock the Windows packaged smoke, which reached six sidecars and a
  fixture run but failed when the filesystem CAS wrote the logical run key
  `runs/sha256:<hex>/manifest.json` as a Windows path containing `:`.
- Boundary: the runtime filesystem adapter, its unit tests, this log, and one
  changelog entry. No S3, Postgres, queue, service, migration, or strategy
  change. No tag, release, or merge.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--695-package-final`,
  branch `fix/695-filesystem-cas-windows`, base `8940cdba`.

## What changed

- `crates/quantrule-runtime/src/adapters/filesystem.rs` now maps every logical
  key segment to a portable filename before joining it under the store root.
  `%` and `:` are percent-encoded, trailing dots and spaces are encoded, and a
  Windows device name has its first character encoded. `get`, `put`, and
  `exists` resolve through the one mapping.
- The logical key and content verification are unchanged. A bare SHA-256 last
  segment is still the content address; a run manifest key still names the run.
- Tests cover a colon run key on Linux with a no-colon physical path, percent
  round-tripping and non-collision with a literal escape sequence, reserved
  names and trailing characters, traversal rejection, idempotent republish,
  corrupt bytes, and the existing killed-writer cases.

## Verification

- `cargo test -p quantrule-runtime`: passed, including 16 filesystem tests.
- `cargo test --workspace`: passed.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `tools/gate.sh`: passed.

## Learning

- Windows treats `:` as a stream separator and cannot store it in a filename;
  the content-addressed blob keys were already safe, but every run key carries
  `sha256:<hex>` in its run-id segment.
- Encoding `%` as well as `:` is what makes the mapping injective; otherwise a
  key containing the literal text `%3A` would collide with an encoded colon.
- The mapping is applied on every platform, not only Windows. That keeps the
  physical layout deterministic and lets the Linux tests prove the exact
  Windows-ready name. Existing desktop data written before this mapping would
  need a migration or a fallback, but the v1.1 desktop profile is not released
  and the parity and smoke runs use fresh data directories.
