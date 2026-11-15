# AI transparency log — DIST-203 desktop packaging

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: implement P3a/P3b/P3c of the desktop v1.1 plan for #695 — the
  sidecar supervisor, the shell lifecycle, the installer resources, the desktop
  release matrix steps, and the package checks. P3d parity is owned jointly
  with #693/#694 and is not claimed here.
- Boundary: only `apps/desktop-shell/**`, the desktop job in
  `.github/workflows/release.yml`, the desktop README, the root README desktop
  section, `docs/architecture/client.md`, `CHANGELOG.md`, and this log. No
  service crate, `crates/quantrule-runtime`, Dockerfile, Compose file, Makefile,
  `docs/port-parity`, or the v1.1 plan was edited. No issue was closed or
  merged.
- Environment: worktree `/home/zantgo/Proyectos/quant-rule--695-package`, branch
  `feat/695-package`, base `bb7b131f` (main after the v1.1 plan merge).

## What was read

- `AGENTS.md` sections 2, 3, 4, 7, 8, 9, 11, 12, 13, and 14; issue #695; the
  approved plan `docs/execution-plans/2026-11-15/692-desktop-v1.1.md` (P3a-P3c
  and section 11); ADR-12.
- The desktop shell, its boundary and smoke tests, `tauri.conf.json`, the
  `apps/web` staging script, and the v1.0 release workflow desktop job.
- The six service manifests and binaries, `quantrule-runtime` configuration,
  the `Service::url_variable` contract, the health report schema, and the data
  service's config scanner.
- The repository gate scripts that touch this change: `check_changelog`,
  `check_doc_length`, `check_links`, `check_terms`, `check_python`,
  `check_file_size`, and `tools/gate.sh`.

## Decisions

- **Supervisor location.** The plan names `crates/quantrule-desktop-supervisor`.
  The issue ownership for #695 allows only `apps/desktop-shell/**`, so the
  supervisor is a dependency-free package at `apps/desktop-shell/supervisor/`
  with the same binary name (`qr-desktop`) and no crate dependency at all.
  This is stronger than "no engine dependency": there is no dependency block to
  link an engine through. The deviation is reported for the plan record.
- **Orphan recovery without service changes.** The plan gives each service a
  control-socket watchdog. That is service code and is out of scope here. The
  supervisor instead wraps each service in `qr-desktop --guard`, which connects
  to the control socket before starting the service and kills it when the
  connection closes. The package still carries exactly one supervisor
  executable. #693/#694 may later move the watch in-process; the guard is
  protocol-compatible with that.
- **No IPC, no plugin.** The shell spawns the supervisor with
  `std::process::Command` from `setup()`, holds its stdin open, and kills it on
  `RunEvent::Exit`. It reads no file and adds no `#[tauri::command]`.
  `tests/boundary.sh` now allows exactly one spawn and still fails on an IPC
  command, a file read, a plugin, or an engine import.
- **Port allocation.** Gateway 8080 with a 8081–8089 fallback; data, model,
  evidence, and simulation from 18081–18084 with a ten-port window. The shell
  probes the configured URL and then `http://127.0.0.1:8080`–`8089` across two
  passes. A fallback is not persisted.
- **Profile selection.** `--profile desktop` or `QR_DESKTOP_PROFILE=desktop`
  starts the sidecars with the documented `QR_PROFILE=desktop` and
  `QR_DATA_DIR` contract. The packaged build enables the `embedded-profile`
  cargo feature so the default is desktop; a plain development build defaults
  to external so it neither starts a server-backed stack nor claims success.
- **Resources, not `externalBin`.** The seven executables and the built client
  are bundled with `bundle.resources` (`sidecars/`, `client/`, `config/`) and
  resolved from the resource directory the shell passes. The shell never uses
  `tauri-plugin-shell`.
- **Package checks stay in the owned tree.** The plan names
  `tools/check_desktop_package.sh`; ownership for #695 excludes `tools/`, so the
  checks are `apps/desktop-shell/tests/check-package.sh` and
  `check-package.ps1`. The workflow calls them in the desktop job.
- **No new dependency.** The supervisor uses `std` only; the shell gains no
  crate. `cargo build` of the supervisor produces no third-party `.crate`.
- **Windows console.** The supervisor and guard set `CREATE_NO_WINDOW` through
  the safe `std::os::windows::process::CommandExt` API so a packaged app does
  not flash a console per process. This is not `unsafe` and needs no crate.

## Rejected options

- Linking the supervisor into the shell: it would make the shell a process
  manager and complicate its exit path; a separate binary keeps the boundary.
- Using `tauri-plugin-shell` and one IPC command per service: explicitly
  rejected by plan D7 and the boundary test.
- PID hunting or a process-inspection crate for orphans: rejected by plan D9;
  the guard uses the kernel's socket close instead.
- Auto-detecting the sidecars and starting them without a profile: rejected
  because a development tree with staged sidecars could start a Postgres stack
  and look like a desktop profile.
- A checked-in golden digest for parity: the oracle is the server profile
  (plan D10); nothing is claimed until #693/#694 land.

## Verification and evidence

| Check | Command | Observed |
|---|---|---|
| Supervisor unit tests | `cargo test` in `apps/desktop-shell/supervisor` | 23 passed |
| Supervisor lifecycle tests | same | 7 passed (start/health/stop, crash/restart, orphan recovery, port fallback, missing sidecar, blank data directory, external profile) |
| Supervisor clippy | `cargo clippy --all-targets -- -D warnings` | clean |
| Shell helpers, candidates, boundary | `bun test` in `apps/desktop-shell` | 32 passed |
| Shell boundary | `bash tests/boundary.sh` | one spawn; no IPC, file read, engine, plugin |
| Shell clippy | `cargo clippy --all-targets -- -D warnings` | clean |
| Changelog | `python3 tools/check_changelog.py` | 6 releases, 111 entries |
| Doc length | `python3 tools/check_doc_length.py` | 34 files within cap |
| Links | `python3 tools/check_links.py` | 956 Markdown files, no broken links |
| Terms | `python3 tools/check_terms.py` | 594 files, no banned terms |
| Interpreter gate | `python3 tools/check_python.py` | 741 product files, no interpreter reference |
| Workflow YAML | `python3 -c "import yaml; yaml.safe_load(...)"` | parsed |
| Linux AppImage | `bunx @tauri-apps/cli@2 build --bundles appimage` | built `QuantRule_1.0.0_amd64.AppImage`, 160.86 MiB |
| Package contents | `bash tests/check-package.sh <AppImage>` | pass: shell, supervisor, six sidecars, client, no interpreter |
| Packaged supervisor smoke | `bash tests/smoke-package-linux.sh <AppImage>` | pass: the packaged `qr-desktop` started six stub sidecars, reached ready, and stopped them |
| Packaged data-directory diagnostic | extract the AppImage; run `qr-desktop --profile desktop --data-dir ""` | refused with the message naming `--data-dir` and `QR_DATA_DIR`; exit 2 |
| Packaged shell spawn (manual) | extract the AppImage, replace the six services with the stub, run `AppRun` under Xvfb | the shell spawned the packaged supervisor, six guards started, `/health` answered on 8080, and killing the shell left no survivor |
| Workspace gate | `tools/gate.sh` | pass: every workspace test and checker completed with `CARGO_TARGET_DIR=target/gate` |

## Learning and limits

- The first supervisor design let a guard connect to a listener that was never
  accepted; on stop it would close accepted streams but leave the backlog
  alive. The fix drops the listener too, and a connection accepted in the gap
  between the close and the list clear is dropped inside the lock, so a guard
  that has not been tracked still sees the close. The orphan test covers the
  killed-supervisor path; the accepted-gap case is covered by inspection, not
  by a timed test.
- The lifecycle tests initially raced the supervisor's `ports.json` write and
  shared free-port blocks between parallel tests. Both were test defects, not
  supervisor defects: readiness now waits for the record, and the allocator
  serializes blocks. The tests were run three times after the fix.
- The plan's parity criterion cannot be met before #693/#694. The supervisor's
  health wait requires an available aggregate state, so a Postgres-backed
  service reports `down` and the start fails honestly instead of faking a
  desktop profile.
- Windows evidence is not produced here. The NSIS build, the silent install
  check, and the `CREATE_NO_WINDOW` path run only on `windows-latest`.
- The plan's seven-executable count is preserved; the stub sidecar is a test
  binary that is never staged, and the package checks fail if it appears.

## #693 environment contract

The #693 worktree at `2e3f8721` adds `QR_PROFILE=desktop|server` and a
non-empty `QR_DATA_DIR` to `quantrule-runtime::Config`; the desktop profile is
selected once in each service `main.rs`. The supervisor already passes
`QR_PROFILE=desktop`; it now resolves, creates, and canonicalizes the data
directory, refuses a blank `--data-dir` or `QR_DATA_DIR` with a message naming
both, and passes the absolute non-empty path to every sidecar. A blank value is
tested at the CLI and with the stub environment. The runtime config was read
read-only and not edited.

## Handoff

- P3d parity is blocked on #693/#694 and the server oracle. The exact next
  command is `tools/parity_desktop.sh` once the backends and the storage/queue
  contracts land; it must compare the canonical digest, every artifact, the
  manifest, and the API projection, and it is not created here.
- `QR_IMAGE_DIGEST` is passed through when set, but a packaged run without one
  records no bundle digest yet. The parity harness can hold it constant; the
  bundle-digest fallback belongs to P3d.
- #695 stays open and the PR uses `Refs #695`; no server digest is claimed.
