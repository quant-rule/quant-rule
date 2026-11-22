# Client Access

- Date: 2026-11-21 (UTC)
- Status: Current
- Audience: operator, reviewer, agent
- Related: [Client](../architecture/client.md), [Deployment](../architecture/deployment.md), [Security](../architecture/security.md)

## 1. Goal

Reach one QuantRule server from a supported client without installing a desktop
application.

## 2. Supported clients

| Client | Windows | Linux | macOS |
|---|---|---|---|
| Browser GUI | Yes | Yes | **No** |
| `quantrule` CLI | Yes | Yes | **No** |
| Desktop app | No | No | No |
| Installer | No | No | No |

macOS is unsupported in v1.5.0. It can return later as a separate decision.

## 3. Browser

Open the gateway URL:

```text
https://one-name/
```

Enter the bearer token on the Connection screen when the deployment requires
one. The browser client is the same SPA on Windows and Linux; it holds no engine
and no local data.

No AppImage, NSIS, EXE, MSI, deb, rpm, or dmg exists. There is no desktop window.

## 4. CLI

The CLI is a headless client. It runs on the machine where the agent or operator
works, not on the gateway host.

```bash
cargo build --release -p quantrule-shell
```

```bash
export QUANTRULE_API_URL=https://one-name/api/v1
export QUANTRULE_TOKEN=...
./target/release/quantrule health --json
```

Windows PowerShell:

```powershell
$env:QUANTRULE_API_URL = "https://one-name/api/v1"
$env:QUANTRULE_TOKEN = "..."
.\target\release\quantrule.exe health --json
```

The CLI has no installer. It stores only the API URL, token, and default profile.
It never opens a local database and never runs a pipeline stage.

## 5. Server on a local machine

| Host | Server runtime |
|---|---|
| Linux | Docker or Podman Compose |
| Windows | Docker Desktop with WSL2 |
| macOS | Unsupported in v1.5.0 |

```bash
make up
```

A remote gateway needs no local Docker.

## 6. What this proves

- The browser client reaches the gateway on Windows and Linux.
- The CLI reaches the same gateway with the same token and profile.
- No desktop package is required.
- The server remains the Linux image with seven services in the single-host
  stack; the v2 multi-host profile places six.

## 7. Failure

| Symptom | Action |
|---|---|
| Browser shows `unreachable` | Check the gateway URL, token, and TLS |
| CLI says `QUANTRULE_API_URL` is invalid | Use an absolute HTTP(S) URL ending at `/api/v1` |
| CLI has no default profile | Pass `--profile` or run `profile use` |
| macOS client expected | Unsupported in v1.5.0 |
