# AI transparency log — DIST-081 remote exposure profile

- Date: 2026-11-15 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash (S4 implementation subagent)
- Work item: [#672](https://github.com/quant-rule/quant-rule/issues/672), `DIST-081`
- Branch: `feat/672-remote`
- Worktree: `/home/zantgo/Proyectos/quant-rule--672-remote` (base `bb60d394`)
- Claim type: Behaviour Claim

## Intent and boundary

Make the documented remote procedure real without weakening the single-operator
boundary: one published gateway port, a bearer token when configured, and
current TLS/private-network guidance. The agent owned `tools/token.sh`,
`tools/remote.sh`, the `remote` and `token` targets in `Makefile`, the gateway
token environment in `compose.yaml`, `docs/architecture/security.md`,
`docs/architecture/deployment.md`, `README.md`, `CHANGELOG.md`, and this log.

The gateway auth code from #646 was read and not edited. The Dockerfile, CI,
shell, web client, service crates, `docs/port-parity`, and the pinned MinIO image
were not touched. No dependency was added, no `unsafe` was introduced, and no
test was weakened.

## Implemented and proposed

- Implemented: `tools/token.sh` prints 32 random bytes as 64 lowercase hex
  characters (256 bits) from `openssl rand` or `/dev/urandom`. It writes no file,
  so the token never enters the repository or an archive.
- Implemented: `tools/remote.sh` reads `QR_BEARER_TOKEN` or generates one with
  `tools/token.sh`, exports it to Compose, starts the stack with
  `up -d --build --wait`, and refuses to continue if any container other than
  the gateway publishes a host port. It then checks the live matrix — no token
  `401`, wrong token `401`, configured token `200` on `/profiles` — and prints
  the token, a `curl` example, the TLS/private-network rule, and the browser and
  shell limitation. The token is printed to the terminal; the script writes it
  to no file, though Docker holds it in the gateway container's environment.
- Implemented: `compose.yaml` passes `QR_BEARER_TOKEN` to the gateway with an
  empty default. The runtime filters an empty value to no token, so `make up`
  keeps the unauthenticated local behavior; `make remote` recreates the gateway
  with the check enabled.
- Implemented: the `remote` and `token` Makefile targets call the scripts and
  forward `COMPOSE`, so `make remote COMPOSE="podman compose"` has the same shape
  as `make up`.
- Implemented: `security.md` §2 records the procedure, the 401/401/200/200
  matrix, rotation, TLS/private-network rule, and §2.1 records that the browser
  and shell do not send `Authorization`. `deployment.md` §3.1 records the same
  procedure, and §6 no longer calls the remote client a browser.
- Implemented: the README target table and remote example, the CHANGELOG entry,
  and this log.
- Proposed but not implemented: a client token feature. The browser and shell do
  not send `Authorization`; a token-protected gateway answers their API calls
  `401`. Remote browser access needs a reverse proxy that injects the token or a
  private network without a token. The static client is token-exempt so the page
  loads, but that is not browser support. No client code was changed.
- Observed but not edited: `docs/tutorials/2026-11-12/03-remote-compute.md` §2
  still says to open the remote URL in a browser and expect `connected`. That is
  stale for a token-protected gateway. The tutorial is outside this work item's
  ownership; it needs the same browser limitation in a follow-up.

## Verification and evidence

All commands ran in `/home/zantgo/Proyectos/quant-rule--672-remote` on this host,
on branch `feat/672-remote` at base `bb60d394`.

| Check | Command | Observed |
|---|---|---|
| Script syntax | `bash -n tools/token.sh tools/remote.sh` | passed |
| Token shape | `./tools/token.sh` | 64 lowercase hex characters; no file written |
| Compose parse, unset | `docker compose config -q` | passed |
| Compose parse, token set | `QR_BEARER_TOKEN=… docker compose config -q` | passed |
| Remote start | `QR_BEARER_TOKEN=… make remote` | nine containers healthy; script matrix passed; only the gateway published `:8080` |
| Generated token | `./tools/remote.sh` with no `QR_BEARER_TOKEN` | exit 0; generated token, matrix passed, only `:8080` published |
| No token | `curl -i http://localhost:8080/profiles` | `401`, `WWW-Authenticate: Bearer`, `auth.unauthorized` |
| Wrong token | `curl -H 'Authorization: Bearer wrong-token' …/profiles` | `401` |
| Correct token | `curl -H "Authorization: Bearer $QR_BEARER_TOKEN" …/profiles` | `200` |
| Record route | `GET /catalog/datasets` with token and `X-QuantRule-Profile: default` | `200`; without token `401` |
| Exempt routes | `/health`, `/version`, `/` | `200` without a token |
| Published ports | `docker ps --filter label=com.docker.compose.project=quantrule` | only `quantrule-gateway-1` publishes `0.0.0.0:8080->8080/tcp`; every other container shows container-internal ports only |
| Unexposed ports | TCP connect to `127.0.0.1:5432`, `:9000`, `:5672` | all refused |
| Local unchanged | `docker compose up -d --wait` with no token, then `GET /profiles` | `200` without a token |
| Help | `make help \| grep -E 'remote\|token'` | both targets listed |
| Workspace gate | `./tools/gate.sh` | passed; 102 test targets, 1979 tests, 0 failed; every checker ok |
| Token scan | working tree and Git index search for the test token | no match |

Published-port evidence from the remote start:

| Container | Host-published |
|---|---|
| `quantrule-gateway-1` | `0.0.0.0:8080->8080/tcp`, `[::]:8080->8080/tcp` |
| `data`, `model`, `backtest`, `evidence`, `simulation` | none |
| `postgres`, `minio`, `rabbitmq` | none |

The other containers still show their service ports in `docker ps` (for example
`8080/tcp` or `5432/tcp`), which is the container network address, not a host
binding. `tools/remote.sh` reads `HostConfig.PortBindings`, so a host publish
outside the remote contract fails the start before the token checks run.

## Learning

- The first port-guard pattern was `*/gateway-1`; the real name is
  `/quantrule-gateway-1`, so the guard rejected the correct gateway. Matching
  `*-gateway-*` fixed it. The guard is worth keeping because a future compose
  edit that publishes a second port should fail loudly.
- The empty Compose default is what keeps local mode unchanged. It depends on the
  runtime's existing filter that turns an empty `QR_BEARER_TOKEN` into no token;
  the local `200` check proves the composition, not an assumption.
- A proxy that injects the token makes every visitor the same principal, so it
  does not create accounts. It is a private-network tool, and the documentation
  says so.
- The browser gap is real and was not papered over: the static client loads
  because it is exempt, but it has no token to send. The issue remains `Refs`,
  not `Closes`, while that gap exists.
