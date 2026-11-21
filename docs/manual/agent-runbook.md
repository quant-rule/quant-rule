# Agent Runbook

- Date: 2026-11-21 (UTC)
- Status: Current
- Audience: a coding agent handed a client, remote-access, or operations task by the human
- Related: [AGENTS.md](../../AGENTS.md), [First run](01-first-run.md), [Operations](08-operations.md), [Client access](13-client-access.md), [Command reference](09-command-reference.md), [Deployment](../architecture/deployment.md), [Security](../architecture/security.md)

## 1. Read first

| Topic | Read |
|---|---|
| Product and boundaries | [AGENTS.md](../../AGENTS.md) |
| Client access | [Client access](13-client-access.md) |
| Operations | [Operations](08-operations.md) |
| Deployment | [Deployment](../architecture/deployment.md) |
| Security | [Security](../architecture/security.md) |

QuantRule has a browser GUI and a headless CLI. There is no desktop app, installer,
Tauri window, or macOS support in v1.5.0.

## 2. Guardrails

| Do not | Why |
|---|---|
| Install or search for an AppImage, NSIS, EXE, MSI, deb, rpm, or dmg | The desktop distribution is retired |
| Run a desktop supervisor or `qr-desktop` | It no longer exists |
| Assume macOS support | Unsupported in v1.5.0 |
| Delete a data directory without asking | It may hold the only local copy |
| Change a port or `QR_BIND` without asking | It changes the deployment contract |
| Run `make up` twice against the same host | It competes for `8080` |

## 3. Browser client

Open the gateway URL and enter the bearer token on the Connection screen.

```text
https://one-name/
```

The browser is the same SPA on Windows and Linux. It holds no engine and no local
data.

## 4. CLI client

The CLI is a headless client. Build it from source on the machine where the agent
runs:

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

Do not install it on the gateway host unless the agent runs there. It is a client,
not a service.

## 5. Server

| Host | Runtime |
|---|---|
| Linux | Docker or Podman Compose |
| Windows | Docker Desktop with WSL2 |
| macOS | Unsupported in v1.5.0 |

```bash
make up
```

A remote gateway needs no local Docker.

## 6. Evidence to report

| Item | Evidence |
|---|---|
| Browser | URL, version, image digest, health |
| CLI | Command, exit code, JSON or text output |
| Server | `make ps`, `/health`, `/version` |
| Run | Run id, canonical digest, artifact list |
| Failure | Stable error code and the next action |

## 7. Troubleshooting

| Symptom | Action |
|---|---|
| Browser `unreachable` | Check the URL, token, and TLS |
| CLI `QUANTRULE_API_URL` invalid | Use an absolute HTTP(S) URL ending at `/api/v1` |
| CLI has no default profile | Pass `--profile` or run `profile use` |
| Port already in use | Stop the other stack; do not run two |
| macOS package requested | Unsupported in v1.5.0 |

## 8. Related

- [Client access](13-client-access.md)
- [Operations](08-operations.md)
- [Command reference](09-command-reference.md)
- [Deployment](../architecture/deployment.md)
