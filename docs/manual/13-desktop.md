# 13. Desktop Install

- Status: Current
- Audience: operator installing the QuantRule window on Fedora Linux or Windows

## Goal

Install the desktop app so that GNOME Activities search opens a QuantRule
window, and know the Windows path. The window loads the same gateway and the
same browser client as every other client; it is a third way to reach one
client, not a second client.

## Before you start

| Need | Value |
|---|---|
| Linux | Fedora with a GNOME session; another AppImage-capable desktop accepts the same files |
| Windows | Windows 10 or 11 with the WebView2 runtime (present by default) |
| Docker | Not required; the packaged app starts its own six services on loopback |
| Bun and Rust | Only when building the artifact from source |
| Artifact | `QuantRule_1.3.0_amd64.AppImage` (Linux) or `QuantRule_1.3.0_x64-setup.exe` (Windows) |
| Free ports | 8080–8089 and the 18081-and-up internal range, on loopback |
| Access | Per-user install; no administrator, account, license key, or cloud service |

The release artifact is attached to the v1.3.0 release; building it from source
produces the same name. A source bundle built without the staged sidecars is a
shell without a stack: its resources directory is empty and it falls back to the
external profile.

## Linux Fedora AppImage

### Build from the repository

The package carries the window, the supervisor, and the six service binaries.
Build and stage them before the Tauri bundle, from the repository root:

```bash
cargo build --release --locked \
  --bin qr-gateway --bin qr-data --bin qr-model \
  --bin qr-backtest --bin qr-evidence --bin qr-simulation
cargo build --release --manifest-path apps/desktop-shell/supervisor/Cargo.toml \
  --features embedded-profile

cd apps/desktop-shell
bun scripts/stage-package.mjs \
  --services-dir ../../target/release \
  --supervisor-bin supervisor/target/release
bunx @tauri-apps/cli@2 build --bundles appimage
```

Output: `apps/desktop-shell/target/release/bundle/appimage/QuantRule_1.3.0_amd64.AppImage`.

Building needs the Tauri 2 Linux libraries (WebKitGTK 4.1, librsvg, OpenSSL,
`patchelf`); on Fedora the development packages are `webkit2gtk4.1-devel`,
`librsvg2-devel`, `openssl-devel`, and `patchelf`.

### Verify the package

From `apps/desktop-shell`, the package check extracts the AppImage and requires
the shell, the supervisor, the six services, the built client, and no Python,
Bun, or Node runtime:

```bash
bash tests/check-package.sh target/release/bundle/appimage/*.AppImage
# package ok: shell, supervisor, six sidecars, client, no interpreter
```

### Install

From `apps/desktop-shell`:

```bash
mkdir -p ~/Applications ~/.local/share/applications \
  ~/.local/share/icons/hicolor/512x512/apps
cp target/release/bundle/appimage/QuantRule_1.3.0_amd64.AppImage ~/Applications/
chmod +x ~/Applications/QuantRule_1.3.0_amd64.AppImage
cp icons/icon.png ~/.local/share/icons/hicolor/512x512/apps/quantrule.png
```

The AppImage carries no signature and no repository metadata. It runs because
you made it executable; verify the artifact with `check-package.sh` before you
install it.

Write the menu entry. The heredoc expands `$HOME`, so `Exec` and `Icon` are
absolute paths in the installed file:

```bash
cat > ~/.local/share/applications/quantrule.desktop <<EOF
[Desktop Entry]
Type=Application
Name=QuantRule
Comment=A window over the QuantRule gateway
Exec=$HOME/Applications/QuantRule_1.3.0_amd64.AppImage
Icon=$HOME/.local/share/icons/hicolor/512x512/apps/quantrule.png
Terminal=false
Categories=Office;Finance;
EOF
update-desktop-database ~/.local/share/applications
```

`update-desktop-database` comes from `desktop-file-utils`. The three fields the
entry must get right are `Exec` (the installed AppImage), `Icon` (the installed
PNG), and `Categories` (`Office;Finance;`).

### FUSE

An AppImage type 2 mounts itself through `libfuse.so.2`. Fedora carries the
library in the `fuse-libs` package:

```bash
sudo dnf install fuse-libs
```

Without it the artifact still runs when it extracts itself instead:

```bash
~/Applications/QuantRule_1.3.0_amd64.AppImage --appimage-extract-and-run
```

The flag extracts to a temporary directory on every launch. To make that
permanent, append `--appimage-extract-and-run` to the `Exec` line.

### Launch and verify

Press Super, type `QuantRule`, and choose the result; `gtk-launch quantrule`
does the same from a terminal. The window starts its own stack, waits for
health, and opens the gateway. Verify the stack and the screen:

```bash
curl -fsS http://127.0.0.1:8080/ready   # 200, "state":"healthy"
```

The Connection screen shows the same report under `health-report` with the
`connected` badge. If the gateway bound another port, `ports.json` names it;
see [Ports and one-stack rule](#ports-and-one-stack-rule).

## Windows NSIS

The release artifact `QuantRule_1.3.0_x64-setup.exe` is built on `windows-latest`
and attached to the release. It is unsigned, so SmartScreen warns.

### Build on Windows

The build host needs the MSVC Rust toolchain, Visual Studio Build Tools with the
C++ workload, Bun, and WebView2. From the repository root in PowerShell or
Git Bash, run the same staging step with Windows paths, then the NSIS bundle:

```powershell
cargo build --release --locked --bin qr-gateway --bin qr-data --bin qr-model --bin qr-backtest --bin qr-evidence --bin qr-simulation
cargo build --release --manifest-path apps/desktop-shell/supervisor/Cargo.toml --features embedded-profile

cd apps/desktop-shell
bun scripts/stage-package.mjs --services-dir ../../target/release --supervisor-bin supervisor/target/release
bunx @tauri-apps/cli@2 build --bundles nsis
```

Output: `apps\desktop-shell\target\release\bundle\nsis\QuantRule_1.3.0_x64-setup.exe`.
The bundler downloads NSIS; no separate installer is needed.

### Install and verify

1. Run `QuantRule_1.3.0_x64-setup.exe`. It installs for the current user and
   needs no elevation.
2. SmartScreen shows “Windows protected your PC”. Choose **More info**, then
   **Run anyway**; the artifact carries no signature.
3. Launch **QuantRule** from Start menu search. Apps & Features (Installed apps
   on Windows 11) lists it as `QuantRule`.
4. Verify the stack with `curl.exe http://127.0.0.1:8080/ready`, and the
   Connection screen with its `health-report` badge.

## Ports and one-stack rule

| Item | Value |
|---|---|
| Gateway | Prefers `8080`, walks forward through `8081`–`8089` |
| Internal services | `data`, `model`, `evidence`, and `simulation` prefer `18081`–`18084`, walking forward the same way |
| Bind address | Loopback only; a desktop stack is never reachable from another machine |
| Lifetime | Children stop after a grace period when the window closes, and a supervisor crash closes the control socket |
| Port record | `~/.local/share/quantrule/supervisor/ports.json`, or `%LOCALAPPDATA%\QuantRule\supervisor\ports.json`; diagnostic only |

Never run `make up` and the desktop app together. If the Compose stack owns
`8080` first, the desktop supervisor moves to `8081`, but the shell probes
`8080`–`8089` in order and can attach to the Docker gateway instead of its own.
The app's own stack then runs unused behind a window that shows the Docker
stack. Use one stack at a time: stop the other one before launching the app.

The packaged Tauri app always starts its own local sidecars. To reach a remote
gateway, use the browser or the shell, not the packaged app.

## Uninstall

Linux, from the same user account:

```bash
rm -f ~/.local/share/applications/quantrule.desktop
rm -f ~/.local/share/icons/hicolor/512x512/apps/quantrule.png
rm -f ~/Applications/QuantRule_1.3.0_amd64.AppImage
update-desktop-database ~/.local/share/applications
```

`~/.local/share/quantrule` holds the desktop SQLite database and object store.
Removing it deletes every desktop dataset, run, and research record and cannot
be undone; remove it only when that history is not wanted.

Windows: **Settings → Apps → Installed apps → QuantRule → Uninstall**. The
uninstaller removes the application; `%LOCALAPPDATA%\QuantRule` is runtime data
and stays until it is removed deliberately, with the same warning.

## Failure

| Symptom | Cause | Next action |
|---|---|---|
| `dlopen(): error loading libfuse.so.2` or “AppImages require FUSE to run” | The host lacks the FUSE 2 library | `sudo dnf install fuse-libs`, or launch with `--appimage-extract-and-run` |
| No window after launch | The shell exited before the window mapped, usually a missing shared library or no graphical session | Run the artifact from a terminal and read stderr; install the missing GTK/WebKit runtime package; confirm the session has a display |
| Connection badge `unreachable` | A sidecar is missing, the first probe ran before the stack was ready, or every gateway port is taken | Wait for the second probe pass, then **Retry connection**; read `ports.json`; run `check-package.sh` on the artifact |
| Connection badge `token required` | The window attached to a gateway that enforces a bearer token, not its own loopback stack | Enter the token to inspect that stack, or stop it and relaunch the app to reach its own gateway |
| Port conflict, or `state: failed` in `ports.json` | `8080`–`8089` or the internal range is occupied, or a previous stack is still exiting | Close the other stack; wait for the grace period; relaunch |
| SmartScreen “Windows protected your PC” | The NSIS installer is unsigned | **More info** → **Run anyway** |
| “Permission denied” on launch | The copied AppImage lost its executable bit | `chmod +x ~/Applications/QuantRule_1.3.0_amd64.AppImage` |
| `check-package.sh` fails | The bundle was built without staged sidecars, or the test stub was packaged | Re-run the staging step from the repository root, then rebuild |

## Related

- Agent runbook — [issue #969](https://github.com/quant-rule/quant-rule/issues/969) adds `docs/manual/agent-runbook.md` with prompts, guardrails, and evidence discipline; link the chapter here when it lands
- [Operations](08-operations.md) — health, profiles, retention, remote access, backup and restore
- [Deployment](../architecture/deployment.md) — server and desktop shapes, image identity, reserved ports
- [Desktop profile](../architecture/desktop.md) — embedded backends, `QR_DATA_DIR`, and parity
- [First run](01-first-run.md) — upload, submit, and verify on a running stack
