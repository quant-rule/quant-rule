# AI transparency log — packaged desktop parity (#695)

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: prove the packaged desktop profile for DIST-203 (#695, P3c/P3d) — the
  AppImage with the real six sidecars, a completed verify-mini run, the server
  canonical digest, and a durable Linux and Windows package smoke on the release
  verification path.
- Boundary: verification and packaging tests only. Files written:
  `apps/desktop-shell/tests/smoke-package-parity-linux.sh`,
  `apps/desktop-shell/tests/smoke-package-parity.ps1`,
  `apps/desktop-shell/README.md`, the desktop smoke steps in
  `.github/workflows/release.yml`, a two-line `QR_PYTHON` seam in
  `tools/verify.sh` so a Windows host can name its interpreter without changing
  the default, `CHANGELOG.md`, and this log. No service or runtime source,
  migration, SQLite or queue code, existing package check, or v1.1 plan was
  edited. No tag, release, or merge; the only board transition is #695 to
  In Review after the PR checks passed.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--695-package-final`,
  branch `test/695-package-parity`, base `bd310aa4`. Linux host, Rust 1.99.1,
  Bun 1.4.2.

## What was read

- `AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 12, 13, and 14.
- Issue #695 and the P3c/P3d rows of
  `docs/execution-plans/2026-11-15/692-desktop-v1.1.md`.
- `apps/desktop-shell/{supervisor,scripts,tests,tauri.conf.json,README.md}`,
  `tools/parity_desktop.sh`, `tools/verify.sh`, `tools/gate.sh`, and
  `.github/workflows/release.yml`.

## What changed

- Added `tests/smoke-package-parity-linux.sh`. It extracts the AppImage, reuses
  `check-package.sh --tree`, checks the staged client and the two verify-mini
  configs, starts the packaged `qr-desktop` with `QR_PROFILE=desktop`, a fresh
  `QR_DATA_DIR`, the staged config, and a deliberately invalid `DOCKER_HOST`,
  proves six sidecars and five healthy endpoints, runs `tools/verify.sh` for the
  fixture, requires the server digest, stops the stack through the supervisor's
  `--parent-stdin` EOF path, asserts no child survives, then repeats in a second
  fresh data directory with an all-zero expected digest and requires that
  comparison to fail.
- Added `tests/smoke-package-parity.ps1`, the NSIS equivalent: silent
  `/S /D=` install, payload checks, packaged supervisor, six sidecars, fixture,
  digest, stop, and an optional seeded-mismatch control. It runs the shared
  `tools/verify.sh` through Git Bash. `tools/verify.sh` now reads `QR_PYTHON`
  (default unchanged, `python3`) so the Windows step can name the unversioned
  interpreter; the desktop test files stay interpreter-free for
  `tools/check_python.py`.
- Added the Linux and Windows packaged-profile steps to the desktop job in
  `.github/workflows/release.yml`, plus a `quantrule` client build for the
  fixture driver. The existing `check-package.sh`, `check-package.ps1`, and
  `smoke-package-linux.sh` steps were left in place and still run.
- Updated the desktop README layout, verify section, CI matrix, and remaining
  gate; added one `CHANGELOG.md` entry under `[Unreleased] / Added`.

## Verification and evidence

Executed on this host unless stated otherwise.

- `cargo build --release --locked --bins` for the six services and
  `quantrule-shell`: success.
- `cargo build --release --manifest-path apps/desktop-shell/supervisor/Cargo.toml
  --features embedded-profile`: success.
- `bun scripts/stage-package.mjs --services-dir ../../target/release
  --supervisor-bin supervisor/target/release`: seven executables and five
  example configs staged.
- `bunx @tauri-apps/cli@2 build --bundles appimage`: built
  `apps/desktop-shell/target/release/bundle/appimage/QuantRule_1.0.0_amd64.AppImage`,
  174,856,696 bytes, `sha256:910c1467544ded14a05ae22ffef71da4fff946e7544584b57853898fb92f195f`.
- `bash tests/check-package.sh <AppImage>`: `package ok: shell, supervisor, six
  sidecars, client, no interpreter`.
- `bash tests/smoke-package-linux.sh <AppImage>`: `package-smoke ok`.
- `bash tests/smoke-package-parity-linux.sh --keep --shell-bin
  target/release/quantrule <AppImage>`: package contents ok; the packaged
  supervisor reported `state: ready` with `profile: desktop` and six sidecars;
  five health endpoints healthy; the fixture completed through the packaged
  gateway with two runs, 12 canonical artifacts and 3,559,171 bytes each; the
  canonical digest was
  `sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa`,
  equal to the server profile oracle; the seeded all-zero control exited
  nonzero with `expected sha256:000…, observed sha256:453c90b4…`; both stops left
  no packaged process.
- `bun test` in `apps/desktop-shell`: 32 passed. `bash tests/boundary.sh`: pass.
- `cargo test`, `cargo fmt --all -- --check`, and
  `cargo clippy --all-targets -- -D warnings` in the supervisor: 23 unit and 7
  lifecycle tests passed; fmt and clippy clean.
- `cargo fmt --all -- --check` and `cargo clippy --all-targets -- -D warnings`
  in `apps/desktop-shell`: clean.
- `bash tools/verify.sh --self-test`: printed the fixed self-test digest.
- `python3 tools/check_python.py`: 777 product files carry no interpreter
  reference.
- `bash -n` on the Linux script and the PowerShell parser on the Windows script:
  clean. The workflow YAML parses with 20 desktop steps.
- `tools/gate.sh`: passed (`CARGO_TARGET_DIR=target/gate`), including CLI,
  terms, changelog, document length, Python, and port-parity checks.
- PR #820 opened with `Refs #695`; its `sync`, `verify`, `web`, and `web-e2e`
  checks passed.
- Windows execution is **not** verified: no Windows host or runner was
  dispatched here. The PowerShell path is syntax-checked only. It remains
  pending a `windows-latest` run after the branch reaches the repository.

## Learning

- Tauri maps `resources/config` to `config/`, so the staged examples are
  `config/verify-mini/...`, not `config/examples/verify-mini/...`. The first
  smoke run failed on that path and the scripts were corrected; the repo path is
  not the package path.
- A shell that holds the write end of the supervisor's stdin FIFO is inherited
  by the supervisor and prevents EOF. The harness holds the write end in a
  separate `sleep` holder and kills it, so the supervisor's own read end sees
  EOF. The first draft held the fd in the harness and would not have stopped.
- The Windows package smoke cannot assume the versioned interpreter alias that
  `tools/verify.sh` defaults to. The `QR_PYTHON` seam keeps the shared digest
  logic in one place; the alternative, copying the fixture logic into
  PowerShell, would have created a second oracle.
- `tools/check_python.py` treats the whole `apps/` tree as product, including
  tests, so the Linux smoke parses `ports.json` with `jq` rather than adding a
  second interpreter reference. The first draft used `python3` and the gate
  caught it.
- The Linux evidence is reproduced; the Windows evidence is an implemented,
  parsed path awaiting execution. The CHANGELOG and README say that in those
  terms.
