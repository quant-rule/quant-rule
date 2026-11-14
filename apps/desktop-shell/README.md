# QuantRule desktop shell

A Tauri 2 window over the gateway URL. It is a second way to reach the one
client, not a second client: the gateway computes, the Svelte client renders,
and the shell supplies the window.

**Status:** the shell is implemented and its release matrix is documented below.
The Linux AppImage and Windows NSIS artifacts are not published yet; that
requires the release workflow (#673) and an authorized release. Issue #691 stays
open until both assets exist on a release.

## 1. Boundary

| It does | It does not |
|---|---|
| Load the gateway URL | Embed an engine |
| Remember the URL and the active profile | Read a local data file |
| Check GitHub Releases for a newer shell | Own an IPC command |
| Show the bundled client when the gateway is unreachable | Compute a measurement |
| Send `X-QuantRule-Profile` through the web client | Implement a use case |

- **No IPC domain.** There is no `#[tauri::command]`, no `generate_handler!`, no
  capability file, and no Tauri plugin. `tests/boundary.sh` fails on any of them.
- **No local file read.** The Rust process opens no file at run time. The
  bundled HTML, CSS, and JavaScript are embedded in the executable at build
  time; Tauri's production asset resolver serves them from memory. The app's
  `[workspace]` table keeps it outside the Rust workspace, and it imports no
  `quantrule-*` crate.
- **One persisted setting.** `localStorage` holds `quantrule.shell.gatewayUrl`.
  That is browser storage managed by the webview, not a file the Rust process
  reads.
- **The profile is the web client's.** The client already stores
  `quantrule.profile` and sends `X-QuantRule-Profile` on every call. The shell
  hosts that client with a persistent webview data directory, so the profile
  survives a restart; it adds no profile storage and no header of its own.
- **The fallback is not a compute path.** `apps/web/dist` is bundled only so an
  unreachable gateway shows the client's disconnected state instead of a webview
  error page. The client still calls `/api/v1`; with no gateway, every call
  fails and it reports `disconnected`.

## 2. A launch

1. The window loads the bundled `shell.html`.
2. In parallel, it probes `<gateway>/health` with `no-cors` (the gateway needs
   no CORS header) and asks GitHub for the newest release.
3. Reachable and no newer release: the same window navigates to the gateway.
4. A newer release: the update card stays visible with the release URL; **Open**
   or **Continue to QuantRule** still opens the gateway. Artifacts are unsigned,
   so there is no in-app install.
5. Unreachable: the shell stays on the settings page and shows the bundled
   client in an iframe, in its disconnected state. **Retry connection** re-probes;
   **Use default** restores `http://localhost:8080`.
6. A remote plain-HTTP gateway can be blocked by the webview's mixed-content
   policy even though a top-level navigation is allowed, so a failed probe there
   still attempts the navigation. HTTPS gateways and localhost are probed
   normally.

To change the gateway after a successful launch, make the current gateway
unreachable and restart the shell; the settings page is the launch page. The
gateway is an origin only: a path, query, fragment, or credentials are rejected.

## 3. Layout

| Path | Role |
|---|---|
| `src/main.rs` | The Tauri builder; no command, no file access |
| `build.rs` | `tauri_build::build()` |
| `tauri.conf.json` | Window, CSP, bundle targets, icons |
| `ui/shell.html` | The bootstrap page and its accessible status regions |
| `ui/shell.js` | DOM wiring for the probe, navigation, and update card |
| `ui/shell-lib.js` | Pure URL, version, probe, and launch-plan helpers |
| `scripts/stage-frontend.mjs` | Copies `apps/web/dist` and the bootstrap into `dist/` |
| `tests/shell.test.js` | Bun tests for the helpers and the static boundary |
| `tests/boundary.sh` | Fails on IPC, file access, plugin, or engine imports |
| `tests/gateway-stub.js` | Answers `/health` and `/` for the Linux smoke test |
| `tests/smoke-linux.sh` | Launches the release binary under Xvfb and proves the probe |
| `icons/` | Source SVG and the generated PNG/ICO set |

## 4. Build

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
cd apps/desktop-shell
bun run stage              # builds apps/web and stages dist/ (Tauri also runs this)
cargo check                # the app crate
cargo build --release      # binary at target/release/quantrule-desktop-shell
```

The app is deliberately outside the Rust workspace: `Cargo.toml` carries an
empty `[workspace]` table. The root `tools/gate.sh` never sees it and does not
need desktop libraries.

Installers are built with the Tauri CLI (it is build tooling, not a committed
dependency):

```bash
# Linux
bunx @tauri-apps/cli@2 build --bundles appimage
# Output: target/release/bundle/appimage/QuantRule_0.3.0_amd64.AppImage

# Windows (run on Windows)
bunx @tauri-apps/cli@2 build --bundles nsis
# Output: target\release\bundle\nsis\QuantRule_0.3.0_x64-setup.exe
```

## 5. Verify

```bash
cd apps/desktop-shell
bun test                       # 24 helper and boundary tests
bash tests/boundary.sh         # no IPC, no file read, no plugin, no engine
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo build --release
bash tests/smoke-linux.sh      # needs :8080 free, xvfb, and dbus
```

The smoke test starts a stub on the shell's default gateway port, launches the
release binary under Xvfb, and waits for `GET /health`. If a gateway already
owns `:8080`, run the test in a private network namespace instead of stopping
it:

```bash
unshare --user --map-root-user --net \
  bash -c 'ip link set lo up; exec bash tests/smoke-linux.sh'
```

### Windows smoke procedure

1. Install `QuantRule_0.3.0_x64-setup.exe`. It installs for the current user.
2. SmartScreen shows “Windows protected your PC”; choose **More info**, then
   **Run anyway**. The artifact is unsigned.
3. With the stack running at `http://localhost:8080`, launch **QuantRule**. The
   window opens the gateway and the connection pill is green.
4. Stop the gateway (`make down`) and restart the shell. It shows **Gateway
   unreachable** and the bundled client's disconnected state; no dataset, run,
   or result is visible.
5. Start the gateway, click **Retry connection**, and confirm the client opens.
6. Select a profile, quit, and relaunch. The same profile is active: the web
   client persisted it and still sends `X-QuantRule-Profile`.
7. Publish a newer tag, or point the check at an existing newer release, and
   relaunch. The update card shows the release URL; **Continue to QuantRule**
   still opens the gateway.

The Windows build and the NSIS bundle must be produced on `windows-latest`; they
were not built on this Linux host.

## 6. Unsigned artifacts

The AppImage and the NSIS installer carry no signature. No code-signing
certificate, no notarization, and no Tauri updater signing key is configured.

- **Windows.** SmartScreen shows “Windows protected your PC”. The user chooses
  **More info** → **Run anyway**. This is expected until signing becomes part of
  a release.
- **Linux.** The AppImage has no signature and no repository metadata. Make it
  executable (`chmod +x`) and run it. A distributor that adds a package
  signature is out of scope for v1.0.0.

## 7. CI matrix (for the release workflow owner)

`.github/workflows/**` is owned by the release work (#673); this branch adds no
workflow file. This is the job to wire into it. It produces both artifacts and
uploads them for the release job to attach.

```yaml
  desktop:
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            bundle: appimage
            name: quantrule-desktop-linux
            path: apps/desktop-shell/target/release/bundle/appimage/*.AppImage
          - os: windows-latest
            bundle: nsis
            name: quantrule-desktop-windows
            path: apps/desktop-shell/target/release/bundle/nsis/*-setup.exe
    runs-on: ${{ matrix.os }}
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4.2.2
      - name: Install the pinned toolchain
        run: rustup show
      - name: Install Bun
        uses: oven-sh/setup-bun@v2
        with:
          bun-version: "1.4.2"
      - name: Install the Linux desktop libraries
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            build-essential curl wget file patchelf \
            libwebkit2gtk-4.1-dev librsvg2-dev libssl-dev xvfb dbus-x11
      - name: Test the shell helpers
        working-directory: apps/desktop-shell
        run: bun test
      - name: Check the shell boundary
        working-directory: apps/desktop-shell
        run: bash tests/boundary.sh
      - name: Build the installer
        uses: tauri-apps/tauri-action@v0 # pin by SHA in the repository workflow
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          projectPath: apps/desktop-shell
          args: --bundles ${{ matrix.bundle }}
      - name: Smoke test the Linux shell
        if: runner.os == 'Linux'
        working-directory: apps/desktop-shell
        run: |
          bin="$(find target/release -maxdepth 1 -type f -name quantrule-desktop-shell -print -quit)"
          bash tests/smoke-linux.sh "$bin"
      - name: Smoke test the Windows shell
        if: runner.os == 'Windows'
        working-directory: apps/desktop-shell
        shell: pwsh
        run: |
          $exe = "target\release\quantrule-desktop-shell.exe"
          if (-not (Test-Path $exe)) { throw "missing $exe" }
          $proc = Start-Process -FilePath (Resolve-Path $exe) -PassThru
          Start-Sleep -Seconds 8
          if ($proc.HasExited) { throw "the shell exited with code $($proc.ExitCode)" }
          Stop-Process -Id $proc.Id -Force
      - name: Upload the installer
        uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.name }}
          path: ${{ matrix.path }}
          if-no-files-found: error
```

`tauri-action` runs `beforeBuildCommand` (`bun scripts/stage-frontend.mjs`), so
the web client is built with the pinned Bun before the Rust build.

The release job attaches the two artifacts after this matrix succeeds. The
version comes from `tauri.conf.json`; the globs are stable, the exact names are
not:

```yaml
      - uses: actions/download-artifact@v4
        with:
          pattern: quantrule-desktop-*
          path: desktop
          merge-multiple: true
      - name: Attach the desktop assets
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh release upload "$GITHUB_REF_NAME" \
            desktop/*.AppImage desktop/*-setup.exe --clobber
```

## 8. Release note text (unsigned)

Paste this into the release note when the assets are attached:

> **Desktop artifacts are unsigned.** `QuantRule_<version>_amd64.AppImage` and
> `QuantRule_<version>_x64-setup.exe` carry no signature. On Windows, SmartScreen
> shows “Windows protected your PC”; choose **More info** → **Run anyway**. On
> Linux, mark the AppImage executable and run it. The shell is a window over a
> gateway: it needs a reachable gateway and is not a self-contained server.

## 9. Remaining gate

- The release workflow (#673) must wire the matrix above and attach both assets.
- A release must be authorized and published. Until then, the shell is built
  and smoke-tested locally but no artifact is public.
- Windows NSIS was not built on this Linux host; the matrix is the first
  Windows build.
