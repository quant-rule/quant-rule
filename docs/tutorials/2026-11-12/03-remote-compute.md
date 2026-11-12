# Tutorial — Remote Compute

- Date: 2026-11-12
- Status: Target v1.0.0
- Audience: operator
- Related: [`deployment.md`](../../architecture/deployment.md), [`security.md`](../../architecture/security.md)

Client on the laptop, workers on a server, laptop switched off mid-run.

## 1. On the server

```bash
make remote
```

Expect the stack to start with only `:8080` published and a bearer token
required. Put the server on a private network (WireGuard or Tailscale) so the
port never faces the internet.

## 2. On the laptop

Open the server URL in a browser. The connection pill must read connected and
show the server version.

No install, no account, no copy of the data.

## 3. Upload once

Upload candles. The `data_version` is computed by the server. This is the only
time the bytes cross the network; every later run references the hash.

## 4. Submit and disconnect

Submit a run. When it shows `queued`, close the browser.

The message is durable, the inputs are immutable, and the worker owns the run.

## 5. From any machine

Reopen the URL from any machine with access. The runs list is server state, not
browser state. Download the bundle or read the digest.

## 6. Adding capacity

```bash
make scale N=4
```

Four `model` containers and four `backtest` containers, each with its own address
on the container network. Output must not change:

```bash
make verify
```

## Rules

| Rule | Reason |
|---|---|
| One dataset crosses the network once | Runs reference a hash |
| The client never sends a path | The server owns its filesystem |
| No result is cached in the browser | Stale evidence is worse than none |
| The token is a network gate, not an account | One principal, by decision |

## What can go wrong

| Symptom | Cause |
|---|---|
| `disconnected` | The port is not reachable, or the private network is down |
| `401` | Token missing or rotated |
| Upload fails | Size limit, or the CSV schema |
| Run never starts | No worker capacity |
