# AI transparency log — DIST-114 desktop shell

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Requirements & Design + Execution Plan; issue #691, claim type
  Behaviour Claim.
- Intent: add `apps/desktop-shell`, a Tauri 2 window over the gateway URL, with a
  remembered gateway, a release update check, a disconnected fallback, a Linux
  and Windows smoke path, and the CI matrix snippet for the release owner.
- Boundary: `apps/desktop-shell/**`, its icons and config, the shell README,
  `docs/architecture/client.md` section 9, `CHANGELOG.md`, and this log. No
  engine, no `apps/web` change, no workflow file, and no release.

## Implemented

- A Tauri 2 application outside the Rust workspace (`Cargo.toml` has an empty
  `[workspace]` table). No `#[tauri::command]`, no capability file, no Tauri
  plugin, and no `quantrule-*` dependency.
- `ui/shell.html` is the window page. It probes `<gateway>/health` with
  `no-cors`, checks the project's latest GitHub release, remembers the gateway
  in `localStorage`, and navigates the same window to the gateway. A newer
  release keeps the bootstrap visible. An unreachable gateway shows
  `apps/web/dist` in an iframe, in its disconnected state.
- The active profile is deliberately not shell state: the web client already
  persists `quantrule.profile` and sends `X-QuantRule-Profile`. The shell only
  supplies a persistent webview data directory.
- `scripts/stage-frontend.mjs` builds and copies `apps/web/dist` plus the
  bootstrap into `dist/`. It is a Bun script because Tauri runs
  `beforeBuildCommand` through the platform shell, and a Bash script would not
  run on Windows.
- `tests/shell.test.js` covers URL validation, the launch plan, version
  comparison, the probe, the update check, and the static boundary. 24 tests
  pass.
- `tests/boundary.sh` fails on IPC, Rust file access, a Tauri plugin, an engine
  import, or a data file in `dist/`.
- `tests/smoke-linux.sh` starts a stub on `:8080`, launches the release binary
  under Xvfb, and waits for `GET /health`; it observed `GET /health` and `GET /`
  (navigation).
- `README.md` carries the Linux and Windows smoke procedures, the unsigned
  artifact and SmartScreen warning, the CI matrix, and the release-note text.

## Verification

| Command | Observed |
|---|---|
| `cargo check` in `apps/desktop-shell` | passed |
| `cargo clippy --all-targets -- -D warnings` | passed |
| `cargo build --release` | passed; 11 MiB binary |
| `bun test` | 24 pass, 0 fail, 70 expectations |
| `bash tests/boundary.sh` | `boundary ok: no IPC command, no Rust file read, no engine, no plugin` |
| `bun run stage` | 7 staged files |
| `bash tests/smoke-linux.sh` (private network namespace, `:8080` owned by a running stack) | passed; probe and navigation observed |
| `bunx @tauri-apps/cli@2 build --bundles appimage` | built `QuantRule_0.3.0_amd64.AppImage` (110.01 MiB) |
| Same smoke test against the AppImage (`APPIMAGE_EXTRACT_AND_RUN=1`) | passed |
| `cargo fmt --all -- --check` | passed |
| `RUSTDOCFLAGS=-D warnings cargo doc --no-deps` | passed |
| `tools/gate.sh` | passed; `workspace gate passed with CARGO_TARGET_DIR=.../target/gate` |
| Unreachable-gateway launch under Xvfb, captured screenshot | the launcher showed `Gateway unreachable` and the bundled client rendered its `unreachable` connection state |

Not verified here: the Windows NSIS build, the Windows smoke step, and the
release publication. They require `windows-latest`, the release workflow (#673),
and an authorized release. The report does not claim both artifacts are
published.

## Learning and rejected alternatives

- **Rejected — a settings plugin or store.** `tauri-plugin-store`,
  `tauri-plugin-updater`, and a Rust settings file would each add a dependency,
  and the store and updater plugins expose an IPC domain. The issue authorizes
  Tauri 2 only. The gateway URL lives in the webview's `localStorage`; the
  release check is one `fetch` in the bootstrap.
- **Rejected — the Tauri updater for auto-install.** The artifacts are
  unsigned, and the updater verifies a signing key. A documented release-page
  check matches the unsigned release decision; there is no silent install.
- **Rejected — an always-visible iframe around the gateway.** It would keep the
  settings page reachable, but the ADR says the window loads the gateway URL. A
  full-window navigation is the faithful shape; the settings page is the launch
  page when the gateway is unreachable.
- **Corrected — a CORS health probe.** The first probe used a normal `fetch`.
  The gateway does not send `Access-Control-Allow-Origin`, so every gateway
  would look down from the `tauri://localhost` origin. `mode: "no-cors"` is
  enough to distinguish answered from unreachable.
- **Corrected — a Bash staging script.** `beforeBuildCommand` runs through
  `cmd.exe` on Windows; the stage step is now a Bun script, which is already the
  client toolchain.
- **Deferred — a native menu to return to settings.** It would need a second
  window or a Rust navigation path, and the shell is meant to stay small. The
  documented procedure is to restart while the gateway is unreachable. A
  Windows build and a real user trial should decide whether that is enough.
