# AI transparency log — DIST-023 Makefile and one-command workflow

- Date: 2026-11-14 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash (implementation agent)
- Work item: [#644](https://github.com/quant-rule/quant-rule/issues/644), `DIST-023`
- Plan: `DIST-000` (DIST-023)

## Intent and boundary

Make the operator interface one command. The agent owned `Makefile`, the README
start note and target table, the operator-command section of
`docs/architecture/deployment.md`, the changelog, and this log. `compose.yaml`,
`Dockerfile`, `docker/**`, `.github/workflows/**`, `crates/**`, `apps/**`, and the
scripts that #671 and #672 will add were read but not edited. A later ownership
correction reserved `docs/architecture/operations.md` for #671; the file was not
touched.

No dependency, `unsafe`, product code, or test was added or changed. The
six-binary profile and the gateway-only migration workaround from #772 are
unchanged.

## Implemented and proposed

- Implemented: `help` (the default goal), `up`, `down`, `reset`, `logs`, `ps`,
  `scale`, `gate`, `remote`, `token`, `backup`, `restore`, and `dev-web`, with
  the existing `verify`, `test`, `test-fast`, `test-composed`, and `test-parity`
  preserved. `.PHONY` covers every target and `make help` is generated from the
  target comments.
- `make up` runs `docker compose up -d --build --wait`, checks `/health`, reads
  the product version from `/version`, and prints the URL line. `make down`
  keeps volumes; `make reset` reads a confirmation before `down -v` and prints
  the scripted form in the deployment note.
- `make scale` forwards to `docker compose up --scale` for `model` and
  `backtest` only when the `six-binaries` profile is active. In the default
  stack it reports the profile gap and exits nonzero, because `qr-model` does
  not exist yet (#643).
- `make remote`, `make token`, `make backup`, and `make restore` check for the
  owning script and fail with the issue number when it is absent. They do not
  fake a result and do not implement the script.
- Documentation: the README start note and target table, the deployment
  quick-start and Make-target sections, the changelog entry, and this log.
- Deferred by ownership: `docs/architecture/operations.md`, `docker/README.md`,
  and `tools/verify.sh`. The docker note still says `make up` is #644 and not in
  the tree; that sentence is now stale and belongs to the image stream.
  `tools/verify.sh` still invokes the retired local `backtest run` command, so
  `make verify` fails after the HTTP-shell rewrite (#771). The target was
  preserved as instructed; fixing the script is a separate change.

## Verification and evidence

The commands were run in the worktree
`/home/zantgo/Proyectos/quant-rule--644-makefile` on this host. The Windows and
Podman paths are documented, not executed here.

| Check | Command | Observed |
|---|---|---|
| Help lists every target | `make help` | 18 targets, matching the `.PHONY` set |
| Default goal | `make` | the same help output, exit 0 |
| Dry run | `make -n up` | exit 0; compose up, health, version, URL line |
| Compose parse | `docker compose config -q` | exit 0 |
| First start | `make up` | exit 0; `QuantRule v0.3.0 ready → http://localhost:8080` |
| Health | `curl -fsS http://localhost:8080/health` | `state=healthy`; postgres, object-store, queue healthy |
| Idempotent start | `make up` a second time | exit 0 in 7.8s; no container restart |
| Stop | `make down` | exit 0; volumes retained |
| Reset cancelled | `printf 'n\n' \| make reset` | exit 2; stack still running |
| Reset confirmed | `yes \| make reset` | exit 0; containers, network, and volumes removed |
| Start after reset | `make up` | exit 0 in 14.7s on empty volumes; health healthy |
| Scale gap | `make scale N=4` | exit 2; names the missing `six-binaries` profile (#643) |
| Scale dry run | `COMPOSE_PROFILES=six-binaries make -n scale N=2` | scale command for `model` and `backtest` |
| Fast tier | `make test` | exit 0; 63 test binaries, 1788 tests, every checker passed |
| Gate target | `make gate` | exit 0 in 1m25s |
| Gate script | `tools/gate.sh` | exit 0 in 25s |
| Composed tier | `make test-composed` | exit 2 with the honest not-wired message (`tools/test_tiers.sh`) |
| Parity tier | `make test-parity` | exit 0; 1 registered port |
| Missing scripts | `make remote`, `make token`, `make backup`, `make restore` | each exits nonzero and names #672 or #671 |
| Dev server | `make dev-web` | Vite ready on `http://localhost:5173`, proxying `/api` to `:8080`; stopped by a 30s timeout, port released |
| Verify target | `make verify` | **fails**: `tools/verify.sh` still calls the retired local `backtest run` command; the target is unchanged and the script is outside #644 |
| Concurrent stack | a later `make up` after another worktree started `quantrule-backup-test` | exit 2; host `:8080` was held by that project, so this worktree's gateway could not bind. The earlier start, health, and idempotency rows were observed while `:8080` was free. |

## Learning

- Compose v5.7.1 reports services without a healthcheck as healthy under
  `up --wait`; `backtest` and `evidence` are reported that way, and only the
  gateway has a real probe. This is existing behavior, not a new claim.
- `docker compose up --wait` returns quickly on an unchanged healthy stack, so
  `make up` is naturally idempotent without a bespoke guard.
- `docker compose logs -f` exits on a stopped project instead of hanging, so
  `make logs` is safe after `make down`.
- The Makefile parses the Compact JSON from `/version` with `sed` rather than
  adding a JSON tool. The version is the workspace version baked into the image
  (`0.3.0` today); the `v1.0.0` line in the deployment note is the target.
- Rejected: a Node/Python JSON parser or `jq` dependency for one field; a
  `FORCE` bypass that would make `reset` destructive without confirmation; and
  fake `qr-model` placeholders to make `make scale` succeed. Each would widen
  the boundary or weaken the guard.
- The last `make up` re-run collided with another worktree's Compose project
  (`quantrule-backup-test`, owned by the #671 stream) on host `:8080`. The stack
  and the volumes belong to that project; this session did not stop or alter
  them. A single host port is part of the deployment contract, so concurrent
  stacks from different worktrees need separate hosts or a compose override,
  which is outside #644.
