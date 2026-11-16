# Tutorial — Remote Compute

- Date: 2026-11-12
- Status: Current; released as v0.9.0 (prepared as v1.0.0); unchanged in v1.0.0
- Audience: operator
- Related: [`deployment.md`](../../architecture/deployment.md), [`security.md`](../../architecture/security.md)

Client on the laptop, workers on a server, laptop switched off mid-run.

## 1. On the server

```bash
make remote
```

Expect the stack to start with only `:8080` published and a bearer token
required. The script checks the token matrix, then prints the token once. Copy
it to the laptop; it is written to no file.

Never expose `:8080` to the public internet. Keep the host on a private network
(WireGuard or Tailscale), or terminate TLS at a reverse proxy in front of the
gateway. The gateway serves plain HTTP, so a token sent over an untrusted
network without TLS is readable on the path.

## 2. Connect the browser

Open the server URL and choose **Connection**. Enter the printed token in the
**Gateway bearer token** field and press **Use token**. The badge must read
`connected` and show the server version.

The token is stored in this browser's `localStorage` and sent as
`Authorization: Bearer` on every API call. It is not part of the URL, the build,
or any page text. **Forget token** removes it.

On a gateway started by `make up` without a token, leave the field empty.

No install, no account, no copy of the data.

## 3. Connect the shell

```bash
export QUANTRULE_TOKEN='<token printed by make remote>'
quantrule dataset list
quantrule health
```

`QUANTRULE_TOKEN` is read once per invocation. `--token <token>` selects a token
for one call and wins over the environment:

```bash
quantrule dataset list --token "$QUANTRULE_TOKEN"
```

The shell sends the header on every gateway request, never stores the token
beside the profile, and never prints it. `quantrule help` lists the flag.

## 4. Upload once

Upload candles. The `data_version` is computed by the server. This is the only
time the bytes cross the network; every later run references the hash.

## 5. Submit and disconnect

Submit a run. When it shows `queued`, close the browser.

The message is durable, the inputs are immutable, and the worker owns the run.

## 6. From any machine

Reopen the URL from any machine with access; on a new machine, enter the token
in Connection. The runs list is server state, not browser state. Download the
bundle or read the digest.

## 7. Adding capacity

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
| The token stays out of the URL and the build | A shared secret is not page state |
| Use TLS or a private network with a token | Plain HTTP exposes the header |

## What can go wrong

| Symptom | Cause |
|---|---|
| Browser badge `token required` | The token is missing, wrong, or rotated |
| Shell `error: auth.unauthorized` | `--token` / `QUANTRULE_TOKEN` is missing or wrong |
| `disconnected` | The port is not reachable, or the private network is down |
| Upload fails | Size limit, or the CSV schema |
| Run never starts | No worker capacity |
