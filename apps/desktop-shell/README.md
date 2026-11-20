# QuantRule desktop package

A Tauri 2 window over the gateway URL, plus a supervisor that starts the six
service binaries the installer carries. It is a second way to reach the one
client, not a second client: the gateway computes, the Svelte client renders,
and the package supplies the window and the processes.

**Status:** the shell, the supervisor, the six-sidecar desktop wiring, and both
installer files are implemented and released as v1.0.0 (prepared as v1.1.0) and current in v1.4.1. `QR_PROFILE=desktop`
selects SQLite, the filesystem CAS, and the durable in-process queue;
`tools/parity_desktop.sh` runs the fixture and compares the desktop canonical
digest with the server profile. The installers are unsigned and the Windows
smoke is release-tier (section 10).

## 1. Boundary

| It does | It does not |
|---|---|
| Load the gateway URL | Embed an engine |
| Remember the URL and the active profile | Read a local data file |
| Check GitHub Releases for a newer shell | Own an IPC command |
| Start one packaged `qr-desktop` supervisor | Manage a service or a port |
| Probe 8080–8089 when the configured URL is down | Read `ports.json` |
| Show the bundled client when the gateway is unreachable | Compute a measurement |
| Send `X-QuantRule-Profile` through the web client | Implement a use case |

- **No IPC domain.** There is no `#[tauri::command]`, no `generate_handler!`, no
  capability file, and no Tauri plugin. `tests/boundary.sh` fails on any of them.
- **One process, from the resource directory.** The shell starts `qr-desktop`
  once from `<resource>/sidecars` (or beside the executable) and holds its
  standard input open. Closing it, or killing it, is the stop signal. The shell
  never starts a service, allocates a port, or reads a diagnostic file.
- **No local data file.** The Rust shell process opens no file at run time. The
  supervisor writes `ports.json` under the per-user data directory for a human
  debugging a failed start; the shell does not read it. The bundled HTML, CSS,
  and JavaScript are embedded in the executable at build time.
- **One persisted setting.** `localStorage` holds `quantrule.shell.gatewayUrl`.
  That is browser storage managed by the webview, not a file the Rust process
  reads. A supervisor fallback port is not persisted.
- **The profile is the web client's.** The client already stores
  `quantrule.profile` and sends `X-QuantRule-Profile` on every call.
- **The fallback is not a compute path.** `apps/web/dist` is bundled only so an
  unreachable gateway shows the client's disconnected state. The client still
  calls `/api/v1`; with no gateway, every call fails and it reports
  `disconnected`.

## 2. A launch

1. The shell starts `qr-desktop` once, if the package carries one.
2. The window loads the bundled `shell.html`.
3. In parallel, the page probes the configured gateway, then the loopback
   candidates `http://127.0.0.1:8080`–`8089` twice, and asks GitHub for the
   newest release.
4. Reachable and no newer release: the same window navigates to whichever
   address answered.
5. A newer release: the update card stays visible with the release URL; **Open**
   or **Continue to QuantRule** still opens the gateway. Artifacts are unsigned,
   so there is no in-app install.
6. Unreachable: the shell stays on the settings page and shows the bundled
   client in an iframe, in its disconnected state. **Retry connection** re-probes;
   **Use default** restores `http://localhost:8080`.

The configured gateway is what the page persists. A fallback is this session's
address, so a later launch probes again and an existing setting still wins.

## 3. The supervisor

`supervisor/` is a separate, dependency-free Rust package that builds
`qr-desktop`. It is packaging, not a service: it owns no schema, no HTTP route,
and no measurement.

| Step | Behaviour |
|---|---|
| Resolve | Uses `--resource-dir`, `--data-dir`, and `--sidecar-dir`; defaults to the per-user data directory and refuses an empty one |
| Validate | Checks all six `qr-*` binaries before starting one; a missing binary fails the start |
| Allocate | Gateway prefers 8080 and walks 8081–8089; data/model/evidence/simulation prefer 18081–18084 with a ten-port window; loopback only |
| Start | Runs `qr-desktop --guard --control <addr> -- <service>` for each sidecar |
| Watch | Each guard holds a control-socket connection and kills its service when the socket closes |
| Health | `/health` must answer 200 with `healthy` or `degraded`; `qr-backtest` is checked by process liveness |
| Restart | A crashed child is restarted up to four times with bounded backoff, then the profile fails |
| Stop | Closing the control socket ends the guards, which end the services; a grace period covers the exit |
| Recover | A supervisor crash closes the socket, so no orphan survives; the next start retries the ports while a previous stack exits |

The chosen ports, PIDs, and restart counts are written to
`<data>/supervisor/ports.json` after every state change. It is a diagnostic
record, not a control channel; the shell never reads it.

Profiles:

| Selection | Effect |
|---|---|
| `--profile desktop` / `QR_DESKTOP_PROFILE=desktop` | Start the six sidecars with `QR_PROFILE=desktop` and `QR_DATA_DIR` |
| `--profile external` / `QR_DESKTOP_PROFILE=external` | Start nothing; the shell uses a gateway that is already running |
| Build feature `embedded-profile` | Changes the default to desktop; used by the release build |
| No flag, no feature | External, so a development build never pretends to be a desktop profile |

`QR_IMAGE_DIGEST` is passed through when it is set, so a parity harness can hold
run identity constant. Otherwise the supervisor hashes the six installed
sidecars and exports that package digest, so an upgraded package is a distinct
execution and `/health` names the package that is running.

The sidecars receive `QR_PROFILE=desktop`, `QR_DATA_DIR`, `QR_BIND`, the four
inter-service URLs, `QR_STATIC_DIR` for the bundled client, and
`QR_CONFIG_ROOT` for the bundled examples. `QR_DATA_DIR` is made absolute and
is never blank: a blank `--data-dir` or `QR_DATA_DIR` is refused before any
child starts, and #693's desktop profile rejects a blank value a second time.
`QR_BEARER_TOKEN`,
`QR_DATABASE_URL`, `QR_QUEUE_URL`, and the object-store variables are removed:
a desktop start never falls back to a server database or broker.

## 4. Layout

| Path | Role |
|---|---|
| `src/main.rs` | The Tauri builder; one supervisor spawn, no command, no file access |
| `build.rs` | `tauri_build::build()` |
| `tauri.conf.json` | Window, CSP, bundle targets, resources, icons |
| `ui/shell.html` | The bootstrap page and its accessible status regions |
| `ui/shell.js` | DOM wiring for the probe, navigation, and update card |
| `ui/shell-lib.js` | Pure URL, version, candidate, probe, and launch-plan helpers |
| `scripts/stage-frontend.mjs` | Copies `apps/web/dist` and the bootstrap into `dist/` |
| `scripts/stage-package.mjs` | Copies the seven executables and the example configs into `resources/` |
| `supervisor/src/` | The supervisor: CLI, ports, health, status, guard, run loop |
| `supervisor/src/bin/stub_sidecar.rs` | Test-only stand-in for the six services |
| `supervisor/tests/lifecycle.rs` | Start, health, crash/restart, orphan recovery, profiles |
| `resources/sidecars/` | Staged executables; committed empty |
| `resources/config/` | Staged example configs; committed empty |
| `tests/shell.test.js` | Bun tests for the helpers and the static boundary |
| `tests/boundary.sh` | Fails on IPC, file access, plugin, engine, or a second spawn |
| `tests/gateway-stub.js` | Answers `/health` and `/` for the Linux shell smoke test |
| `tests/smoke-linux.sh` | Launches the release shell under Xvfb and proves the probe |
| `tests/check-package.sh` | Extracts an AppImage and checks every expected file |
| `tests/check-package.ps1` | Silently installs NSIS and checks every expected file |
| `tests/smoke-package-linux.sh` | Runs the packaged supervisor with stub sidecars in an extracted AppImage |
| `tests/smoke-package-parity-linux.sh` | Runs the packaged Linux profile through the fixture and checks the digest |
| `tests/smoke-package-parity.ps1` | Installs the NSIS artifact, runs the packaged Windows profile through the fixture, and checks the digest |
| `icons/` | Source SVG and the generated PNG/ICO set |

## 5. Build

Prerequisites: Rust 1.99.1 (the repository toolchain), Bun 1.4.2, and the Tauri
2 Linux libraries. The locally verified Linux set is:

```bash
sudo apt-get update
sudo apt-get install -y \
  build-essential curl wget file patchelf \
  libwebkit2gtk-4.1-dev librsvg2-dev libssl-dev
```

The smoke test also needs `xvfb` and, for a session bus, `dbus-x11`.

Windows needs the MSVC Rust toolchain, Visual Studio Build Tools with the C++
workload, and the WebView2 runtime (present on Windows 10/11 and on the
`windows-latest` runner). NSIS is downloaded by the bundler.

```bash
# From the repository root: the six services, from this commit.
cargo build --release --locked \
  --bin qr-gateway --bin qr-data --bin qr-model \
  --bin qr-backtest --bin qr-evidence --bin qr-simulation

# The supervisor, with the packaged default.
cargo build --release --manifest-path apps/desktop-shell/supervisor/Cargo.toml \
  --features embedded-profile

cd apps/desktop-shell
bun run stage                  # builds apps/web and stages dist/ (Tauri also runs this)
bun scripts/stage-package.mjs \
  --services-dir ../../target/release \
  --supervisor-bin supervisor/target/release
cargo check                    # the shell crate
cargo build --release          # target/release/quantrule-desktop-shell
```

A plain `cargo build` after `bun run stage` but without staging the sidecars
still produces a shell, but not a desktop package: the resources directory is
empty and the app falls back to the external profile. The package check is what
refuses to call that an installer.

Installers are built with the Tauri CLI (it is build tooling, not a committed
dependency):

```bash
# Linux
bunx @tauri-apps/cli@2 build --bundles appimage
# Output: target/release/bundle/appimage/QuantRule_1.4.1_amd64.AppImage

# Windows (run on Windows)
bunx @tauri-apps/cli@2 build --bundles nsis
# Output: target\release\bundle\nsis\QuantRule_1.4.1_x64-setup.exe
```

The package contains the shell, `sidecars/qr-desktop`, the six
`sidecars/qr-*` services (`.exe` on Windows), the built client under `client/`,
and the example configs under `config/`. It contains no Python, Bun, or Node
runtime; `tests/check-package.sh` and `tests/check-package.ps1` fail if one
appears.

## 6. Verify

```bash
cd apps/desktop-shell
bun test                                  # 32 helper, candidate, and boundary tests
bash tests/boundary.sh                    # one spawn; no IPC, file read, plugin, engine
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo build --release
bash tests/smoke-linux.sh                 # needs :8080 free, xvfb, and dbus

cd supervisor
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test                                # 23 unit and 7 lifecycle tests

# After a bundle:
bash tests/check-package.sh target/release/bundle/appimage/*.AppImage
bash tests/smoke-package-linux.sh target/release/bundle/appimage/*.AppImage
bash tests/smoke-package-parity-linux.sh \
  --shell-bin ../../target/release/quantrule target/release/bundle/appimage/*.AppImage
```

The package parity smoke is the packaged-profile evidence: it checks the
payload, starts the packaged supervisor with a fresh `QR_DATA_DIR`, runs the
staged verify-mini fixture, and requires the same canonical digest as the
server profile. Its seeded-mismatch negative control runs by default. It needs
`curl` and `jq`; `tools/verify.sh` owns the fixture and digest logic. On
Windows, run the same evidence path after the NSIS bundle:

```powershell
$artifact = Get-ChildItem target\release\bundle\nsis\*-setup.exe | Select-Object -First 1
pwsh tests/smoke-package-parity.ps1 `
  -Artifact $artifact.FullName -Shell ..\..\target\release\quantrule.exe
```

Set `QR_PYTHON` to the host's unversioned interpreter command first; the
release job does. The stock Windows image does not expose the versioned alias
that `tools/verify.sh` defaults to.

The six-sidecar run and the digest parity are one command from the repository
root. The desktop side needs no Docker; the server oracle is a Compose project
that the script creates and removes, or a gateway passed with `--server-url`:

```bash
tools/parity_desktop.sh --server-compose --repeat 2

# The packaged supervisor is the same six sidecars:
cargo build --manifest-path apps/desktop-shell/supervisor/Cargo.toml
tools/parity_desktop.sh --desktop-only \
  --via-supervisor apps/desktop-shell/supervisor/target/debug/qr-desktop \
  --expected "$(printf 'sha256:%s' <a server digest>)"
```

The shell smoke test starts a stub on the shell's default gateway port, launches
the release binary under Xvfb, and waits for `GET /health`. If a gateway already
owns `:8080`, run the test in a private network namespace instead of stopping it:

```bash
unshare --user --map-root-user --net \
  bash -c 'ip link set lo up; exec bash tests/smoke-linux.sh'
```

### Windows smoke procedure

1. Install `QuantRule_1.4.1_x64-setup.exe`. It installs for the current user.
2. SmartScreen shows “Windows protected your PC”; choose **More info**, then
   **Run anyway**. The artifact is unsigned.
3. With the stack running at `http://localhost:8080`, launch **QuantRule**. The
   window opens the gateway and the connection pill is green.
4. Stop the gateway (`make down`) and restart the shell. It shows **Gateway
   unreachable** and the bundled client's disconnected state; no dataset, run,
   or result is visible.
5. Start the gateway, click **Retry connection**, and confirm the client opens.
6. Select a profile, quit, and relaunch. The same profile is active.
7. Publish a newer tag, or point the check at an existing newer release, and
   relaunch. The update card shows the release URL; **Continue to QuantRule**
   still opens the gateway.

The Windows build and the NSIS bundle must be produced on `windows-latest`; they
were not built on this Linux host.

## 7. CI matrix

`.github/workflows/release.yml` owns the matrix. Its desktop job:

| Step | Linux | Windows |
|---|---|---|
| Shell helpers, boundary, supervisor tests | yes | yes |
| Six service binaries, release | yes | yes |
| Supervisor, release with `embedded-profile` | yes | yes |
| Stage the seven executables and the examples | yes | yes |
| Tauri bundle | AppImage | NSIS |
| Package contents and version | `check-package.sh` | `check-package.ps1` |
| Packaged supervisor with stub sidecars | `smoke-package-linux.sh` | — |
| Packaged profile, six sidecars, fixture, digest | `smoke-package-parity-linux.sh` | `smoke-package-parity.ps1` |
| Shell smoke | Xvfb, stub gateway | launch for 8 s |
| Artifact | `quantrule-desktop-linux` | `quantrule-desktop-windows` |

The release job attaches both artifacts after this matrix succeeds. The version
comes from `tauri.conf.json`; the globs are stable, the exact names are not.

## 8. Unsigned artifacts

The AppImage and the NSIS installer carry no signature. No code-signing
certificate, no notarization, and no Tauri updater signing key is configured.

- **Windows.** SmartScreen shows “Windows protected your PC”. The user chooses
  **More info** → **Run anyway**. This is expected until signing becomes part of
  a release.
- **Linux.** The AppImage has no signature and no repository metadata. Make it
  executable (`chmod +x`) and run it. A distributor that adds a package
  signature is out of scope for v1.1.

## 9. Diagnostics

On a failed start, read `<data>/supervisor/ports.json`. The data directory is
`%LOCALAPPDATA%\QuantRule` on Windows and `$XDG_DATA_HOME/quantrule` (or
`~/.local/share/quantrule`) on Linux; `QR_DATA_DIR` overrides it.

The record names the profile, the state (`starting`, `ready`, `stopped`,
`failed`), the control address, the gateway URL, and every service with its
port, guard PID, and restart count. A service that exited is restarted by the
supervisor; a service that exhausted its budget fails the profile and stops the
rest.

## 10. Remaining gate

- **Embedded backends.** The SQLite repositories (#693), the filesystem CAS and
  durable queue (#694), and the six-sidecar wiring are merged. A desktop start
  opens one `quantrule.sqlite` and one `objects/` CAS under `QR_DATA_DIR`; the
  fixture completes with no Docker and produces the same canonical digest as
  the server profile (section 6). The `embedded-profile` build feature only
  selects the supervisor's default profile; `--profile desktop` always works.
- **Parity (P3d).** `tools/parity_desktop.sh` compares the canonical digest,
  every canonical artifact byte, and the run status (manifest entry set and
  stage states). It is release-tier: the server oracle needs Docker or a
  running gateway. `smoke-package-parity-linux.sh` and
  `smoke-package-parity.ps1` run the same comparison from the packaged
  artifacts, not the in-tree binaries.
- **Windows evidence.** The installer, the silent install, and the packaged
  smoke are produced only on `windows-latest`; this branch does not claim
  Windows execution evidence from Linux. The Linux package smoke and its
  seeded-mismatch control are executed on this host.
