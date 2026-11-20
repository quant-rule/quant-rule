# Agent Runbook

- Status: Current
- Audience: a coding agent handed an install, desktop, or remote-access task by the human
- Related: [AGENTS.md](../../AGENTS.md), [First run](01-first-run.md), [Operations](08-operations.md), [Desktop install](13-desktop.md), [Command reference](09-command-reference.md), [Deployment](../architecture/deployment.md), [Security](../architecture/security.md), [Docker stack](../../docker/README.md)

This is the procedure an agent follows when the human asks it to install
QuantRule, start the desktop package, or point a client at another host. It does
not replace the gates in [`AGENTS.md`](../../AGENTS.md); it names the local
checks and the evidence to return. The manual chapters stay authoritative for
what each command does, and this page does not repeat their tables.

## 1. Read first

| Document | Read for |
|---|---|
| [`AGENTS.md`](../../AGENTS.md) §7, §9, §14 | Worktree discipline, Git gates, and decisions reserved to the human |
| [First run](01-first-run.md) | `health`, upload, `run submit`, `run fetch`, and `verify` |
| [Operations](08-operations.md) | Remote profile, bearer token, retention, backup, and restore |
| [Desktop install](13-desktop.md) | AppImage, desktop entry, Windows NSIS, and the app-menu check |
| [Command reference](09-command-reference.md) | The exact arguments and output of every command |
| [Deployment](../architecture/deployment.md) §1, §6, §8 | Quick start, split host, and what deployment does not provide |
| [Security](../architecture/security.md) §2 | Private network or token plus TLS; never public `:8080` |
| [Docker stack](../../docker/README.md) | Compose services, `QR_IMAGE_DIGEST`, and the Windows host notes |

Read the worktree and branch state before the first command. The session works in
the task's worktree, not in the project root or another task's directory.

## 2. Guardrails

| Action | Rule |
|---|---|
| `sudo`, `dnf`, or any package install | Ask first. State the package, the reason, and what fails without it. |
| Delete `~/.local/share/quantrule`, `%LOCALAPPDATA%\QuantRule`, or any data directory | Ask first. It holds the desktop SQLite file and content-addressed store. |
| Kill a process, `pkill`, or `kill -9` | Ask first. Close the window or run `make down`; a killed worker can leave a hidden partial run. |
| Change a port, bind address, or `QR_BIND` | Ask first. The desktop supervisor probes `8080`–`8089`; Compose publishes `8080`. |
| Open a firewall port | Ask first. Reach the gateway over a private network, or put TLS in front of it; never expose `:8080` to the public internet. |
| Any Git write: add, commit, push, PR, merge, tag | A separate authorization. An install task never commits, pushes, opens a PR, merges, or tags. |
| Run `make up` and the desktop app together | Never. They compete for `8080` and create two stacks over two data sets. Pick one profile per machine. |
| `make reset` or `restore --force` | Ask first. It deletes volumes or replaces a non-empty stack. |
| Token or secret handling | Never write a token to a file, a commit, or the report. Paste it only where the client asks. |

The desktop app is the server and the workstation on one machine. The Compose
stack is the server. A laptop is a client and runs neither.

## 3. Evidence to report

Return these five items. A missing item is a limitation, not a pass.

| Evidence | How to collect | Pass condition |
|---|---|---|
| Commands run | Copy every command exactly as executed, with its exit status | A reader can rerun the sequence |
| Health JSON | `curl -fsS http://127.0.0.1:8080/health` and `/ready`; on the shell, `quantrule health --json` | `state` is `healthy` or a named `degraded`; `/ready` is `200` |
| Installed paths | AppImage, `.desktop`, Start Menu shortcut, and the gateway chosen by the supervisor | The paths exist and belong to this install |
| App-menu entry | `gtk-launch quantrule` on Linux; Start Menu shortcut on Windows | The window opens from the entry, not only from a terminal |
| Unresolved limitations | The exact failure, its command, and what was not tried | Nothing is hidden or guessed |

A compact report block:

```text
Task:
Commands run:
Health JSON:
Installed paths:
App-menu entry verified: yes/no, by which command
Unresolved:
```

## 4. Recipe 1 — Linux Fedora AppImage and desktop entry

Use this on the Fedora workstation. It is a user-level install; no root is
needed for the AppImage, the desktop file, or the menu database. These are the
commands in [chapter 13](13-desktop.md), repeated so the agent does not have to
interpret them.

**Get the artifact**

From a checkout, build and stage it as chapter 13 describes. From the release,
download the attached asset:

```bash
mkdir -p ~/Applications
curl -fL --retry 3 -o ~/Applications/QuantRule_1.4.1_amd64.AppImage \
  https://github.com/quant-rule/quant-rule/releases/download/v1.4.1/QuantRule_1.4.1_amd64.AppImage
```

A downloaded release asset has no checkout-local `icons/icon.png`; use the
release icon URL below in place of the icon `cp` line.

**Install (chapter 13 commands)**

From `apps/desktop-shell`:

```bash
mkdir -p ~/Applications ~/.local/share/applications \
  ~/.local/share/icons/hicolor/512x512/apps
cp target/release/bundle/appimage/QuantRule_1.4.1_amd64.AppImage ~/Applications/
chmod +x ~/Applications/QuantRule_1.4.1_amd64.AppImage
cp icons/icon.png ~/.local/share/icons/hicolor/512x512/apps/quantrule.png
```

For a downloaded release asset, the `mkdir` and `chmod` still apply; replace the
two `cp` lines with the icon download. The asset is already at `~/Applications/`:

```bash
curl -fL --retry 3 -o ~/.local/share/icons/hicolor/512x512/apps/quantrule.png \
  https://raw.githubusercontent.com/quant-rule/quant-rule/v1.4.1/apps/desktop-shell/icons/icon.png
chmod +x ~/Applications/QuantRule_1.4.1_amd64.AppImage
```

**Write the desktop entry**

```bash
cat > ~/.local/share/applications/quantrule.desktop <<EOF
[Desktop Entry]
Type=Application
Name=QuantRule
Comment=A window over the QuantRule gateway
Exec=$HOME/Applications/QuantRule_1.4.1_amd64.AppImage
Icon=$HOME/.local/share/icons/hicolor/512x512/apps/quantrule.png
Terminal=false
Categories=Office;Finance;
EOF
update-desktop-database ~/.local/share/applications
desktop-file-validate ~/.local/share/applications/quantrule.desktop
```

**Launch from the menu**

```bash
gtk-launch quantrule
```

Press **Super** and type `QuantRule`; the entry must appear and open the same
window. If `gtk-launch` prints `No such application`, the desktop file is not in
`~/.local/share/applications` or the database was not refreshed.

**Verify**

```bash
cat "${XDG_DATA_HOME:-$HOME/.local/share}/quantrule/supervisor/ports.json"
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/ready
```

The supervisor prefers `8080` and walks to `8089` when it is busy. Read the
`gateway` field from `ports.json` and use that URL when `8080` is taken.

**If the AppImage will not start.** A missing `libfuse.so.2` is a package
decision, not an AppImage defect. Ask before `sudo dnf install fuse-libs`, or
use the no-install fallback for one launch:

```bash
~/Applications/QuantRule_1.4.1_amd64.AppImage --appimage-extract-and-run
```

For a permanent fallback, append `--appimage-extract-and-run` to the `Exec=` line
and run `update-desktop-database` again.

**Paste this to your agent**

```text
Install QuantRule on this Fedora workstation as a user-level AppImage and add a
desktop entry so it appears in GNOME search. Read AGENTS.md §7, §9, and §14 and
docs/manual/agent-runbook.md first. Work in the task worktree and make no Git
writes. Ask me before sudo, dnf, deleting ~/.local/share/quantrule, killing a
process, or changing a port. Do not run make up while the desktop app runs.
Use the chapter 13 install block: build or download the v1.4.1 AppImage, copy it
and icons/icon.png to the chapter's paths, chmod the AppImage, write
~/.local/share/applications/quantrule.desktop with the absolute Exec and Icon
paths, run update-desktop-database, validate the file, and launch it with
gtk-launch quantrule. Report the commands, /health and /ready JSON, the
installed paths, whether the GNOME entry opened the window, and any limitation.
```

## 5. Recipe 2 — Windows NSIS install

Install the NSIS package on Windows 10 or 11 with the WebView2 runtime present.
It installs for the current user and needs no administrator.

**Download**

```powershell
$version = "1.4.1"
$asset = "QuantRule_${version}_x64-setup.exe"
$url = "https://github.com/quant-rule/quant-rule/releases/download/v$version/$asset"
$out = Join-Path $env:USERPROFILE "Downloads\$asset"
Invoke-WebRequest -Uri $url -OutFile $out
Start-Process -FilePath $out
```

**Install**

1. SmartScreen shows “Windows protected your PC”; choose **More info**, then
   **Run anyway**. The artifact is unsigned.
2. Finish the installer and let it launch **QuantRule**.
3. Close the window, open **Start**, type `QuantRule`, and launch it from the
   entry. That is the app-menu check.

**Verify**

```powershell
Start-Sleep -Seconds 5
$ports = Join-Path $env:LOCALAPPDATA "QuantRule\supervisor\ports.json"
$gateway = (Get-Content $ports -Raw | ConvertFrom-Json).gateway
curl.exe -fsS "$gateway/health"
curl.exe -fsS "$gateway/ready"
$shortcut = Get-ChildItem "$env:APPDATA\Microsoft\Windows\Start Menu\Programs" `
  -Recurse -Filter "QuantRule*.lnk" | Select-Object -First 1
$shortcut.FullName
(New-Object -ComObject WScript.Shell).CreateShortcut($shortcut.FullName).TargetPath
Get-Process quantrule-desktop-shell | Select-Object Id, Path
```

The shortcut target is the installed shell. The package digest is computed from
the six installed sidecars, so a desktop run needs no `QR_IMAGE_DIGEST`.

**Paste this to your agent**

```text
Install the QuantRule v1.4.1 Windows NSIS package on this Windows 10/11 machine.
Read AGENTS.md §7, §9, and §14 and docs/manual/agent-runbook.md first. Make no
Git writes, and ask me before deleting %LOCALAPPDATA%\QuantRule, killing a
process, changing a port, or opening a firewall port. Download
QuantRule_1.4.1_x64-setup.exe, run it, and choose More info -> Run anyway at
SmartScreen because the artifact is unsigned. Let it launch once, then launch it
from the Start Menu. Report the commands, the gateway URL from
%LOCALAPPDATA%\QuantRule\supervisor\ports.json, /health and /ready JSON, the
shortcut and installed paths, whether the Start Menu entry opened the window,
and any limitation.
```

## 6. Recipe 3 — Browser or shell on a LAN laptop

The server runs the Compose stack. The laptop runs a browser or the `quantrule`
shell and no containers. Reach the gateway over a private network (WireGuard or
Tailscale); a bare LAN with a bearer token is the fallback, and plain HTTP
exposes the header.

**On the server: start the remote profile**

The three store passwords must be non-default and must stay the same across
restarts. A changed password against existing volumes prevents startup; `make
reset` would reinitialize and delete them, so ask first.

```bash
cd quant-rule
export QR_POSTGRES_PASSWORD='<stable-secret>'
export QR_RABBITMQ_PASSWORD='<stable-secret>'
export QR_OBJECT_STORE_PASSWORD='<stable-secret>'
make remote
# Copy the token printed by make remote, then pin the running image identity:
export QR_BEARER_TOKEN='<token printed by make remote>'
export QR_IMAGE_DIGEST="$(docker compose images -q gateway | head -n1)"
docker compose up -d --wait gateway
```

`make remote` prints the URL, the published-port check, the token matrix, and the
token once; the script writes the token to no file. Copy it to the laptop. The
gateway is recreated with `QR_IMAGE_DIGEST` set to the image it is actually
running, because a run or stage submission is refused when the server has no
image digest. The token is exported first so the recreation keeps authentication
on.

**Firewall**

Open `8080` only on the private interface, and only with approval:

```bash
sudo firewall-cmd --get-active-zones
sudo firewall-cmd --zone=<private-zone> --add-port=8080/tcp --permanent
sudo firewall-cmd --reload
```

Never place `8080` in the public zone. If the private network already carries
the traffic, add no rule.

**On the laptop: browser**

Open `http://<server>:8080`, choose **Connection**, enter the token in **Gateway
bearer token**, and press **Use token**. The badge must read `connected` and show
the server version. **Forget token** removes it.

**On the laptop: shell**

The shell default is `http://127.0.0.1:8080/api/v1`, so the laptop must name the
server:

```bash
export QUANTRULE_API_URL='http://<server>:8080/api/v1'
export QUANTRULE_TOKEN='<token printed by make remote>'
quantrule health
quantrule profile list
```

Without an installed shell, build it from a checkout and use the release binary:

```bash
cargo build --release -p quantrule-shell
./target/release/quantrule health
```

**Verify the two-machine path**

```bash
curl -fsS http://<server>:8080/health
quantrule health --json
quantrule dataset list
```

The run commands are in [First run](01-first-run.md); the full list is in the
[command reference](09-command-reference.md).

**Paste this to your agent**

```text
Connect this laptop to a QuantRule server on the LAN. Read AGENTS.md §7, §9, and
§14, docs/manual/08-operations.md, docs/manual/agent-runbook.md, and
docs/architecture/security.md §2 first. Make no Git writes. Ask me before sudo,
opening a firewall port, changing a bind, killing a process, or deleting data.
On the server, require the three non-default store passwords, run make remote,
then export the printed token and set QR_IMAGE_DIGEST to the running gateway
image before recreating the gateway; never expose :8080 to the public internet.
On the laptop, open http://<server>:8080 in the
browser, enter the token on Connection, and export
QUANTRULE_API_URL=http://<server>:8080/api/v1 and QUANTRULE_TOKEN for the shell.
Report the commands, health JSON from /health and /ready, both client URLs, the
token-matrix result printed by make remote, and any limitation.
```

## 7. Recipe 4 — Stop, verify, and troubleshoot

**Stop**

| Profile | Stop | Verify |
|---|---|---|
| Desktop | Close the window; the supervisor stops the six sidecars | `pgrep -af 'qr-(desktop|gateway|data|model|backtest|evidence|simulation)'` prints nothing |
| Compose | `make down` keeps volumes | `make ps` shows no running service |
| Compose, delete volumes | `make reset`, only with approval | The stack is gone and the volumes are deleted |

Do not kill a process to stop a profile. If the window will not close, report it
and ask.

**Verify**

```bash
curl -fsS http://127.0.0.1:8080/health
curl -fsS http://127.0.0.1:8080/ready
quantrule health --json
quantrule verify <run_id>
```

`/health` is the gateway's own liveness; `/ready` is the dependency authority
and returns `503` when a required service is down. A recorded digest is
verification of the artifact set, not reproducibility and not an edge claim.

**Troubleshoot**

| Symptom | Cause | Next action |
|---|---|---|
| `curl: (7) Failed to connect` | The stack is not running, or the URL is wrong | Start one profile; re-read `QUANTRULE_API_URL` or `ports.json` |
| Browser badge `token required` or `401 auth.unauthorized` | Missing, wrong, or rotated token | Enter the token printed by `make remote`; restarting rotates it |
| `internal` with “the server has no image digest” | Compose started without `QR_IMAGE_DIGEST` | Set it from the built gateway image and restart |
| App opens the wrong stack, or `address already in use` | `make up` and the desktop app run together | Stop one; they compete for `8080` |
| GNOME search has no QuantRule entry | Desktop file in the wrong directory or no database refresh | Move it to `~/.local/share/applications`, run `update-desktop-database` |
| AppImage reports `libfuse.so.2` | FUSE 2 is absent on Fedora | Ask for `fuse-libs`, or launch with `--appimage-extract-and-run` |
| Windows SmartScreen blocks the installer | The artifact is unsigned | **More info** → **Run anyway** |
| Windows app shows `Gateway unreachable` | The sidecar start failed or the chosen port is different | Read `%LOCALAPPDATA%\QuantRule\supervisor\ports.json` |
| `service.unavailable` or `queue.unavailable` | A dependency is down | Read `health` and `make logs`; stop |
| Remote run is refused | The server is healthy but has no digest, or the token is wrong | Check `QR_IMAGE_DIGEST` and the 401/401/200 matrix |

**Paste this to your agent**

```text
Diagnose this QuantRule install without changing it. Read
docs/manual/agent-runbook.md and docs/manual/12-troubleshooting.md first. Make
no Git writes. Do not kill processes, delete ~/.local/share/quantrule or
%LOCALAPPDATA%\QuantRule, change ports, or open firewall ports without my
approval. Collect the exact failing command, /health and /ready JSON, the
supervisor ports.json gateway field, the installed paths, and the desktop-entry
or Start Menu result. Report the cause, the next action, and what remains
unverified.
```

## 8. Related

- [Command reference](09-command-reference.md)
- [Desktop install](13-desktop.md)
- [Operations](08-operations.md)
- [Troubleshooting](12-troubleshooting.md)
- [Remote compute tutorial](../tutorials/2026-11-12/03-remote-compute.md)
- [Deployment](../architecture/deployment.md)
- [Security](../architecture/security.md)
- [Docker stack](../../docker/README.md)
- [`AGENTS.md`](../../AGENTS.md)
