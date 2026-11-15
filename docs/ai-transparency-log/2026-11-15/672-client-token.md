# AI transparency log — DIST-081 client token gap

- Date: 2026-11-15 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash (S4 client-token subagent)
- Work item: [#672](https://github.com/quant-rule/quant-rule/issues/672), `DIST-081`
- Branch: `feat/672-client-token`
- Worktree: `/home/zantgo/Proyectos/quant-rule--672-client` (base `642a764b`)
- Claim type: Behaviour Claim

## Intent and boundary

Close the client half of the remote exposure profile: the shell and the browser
must be able to send the token the gateway already requires, without storing it
in a URL, a build artifact, a committed file, or printed output. The agent owned
`crates/quantrule-shell/**`, `apps/web/src/lib/api.ts`, the connection
components and their tests and e2e, `docs/tutorials/2026-11-12/03-remote-compute.md`,
the accuracy of `docs/architecture/security.md`, `CHANGELOG.md`, `README.md`,
the global-flag paragraph in `docs/architecture/command-surface.md`, and this
log.

The gateway auth seam, `tools/remote.sh`, `tools/token.sh`, Compose, the
Dockerfile, Makefile, `docs/port-parity`, and the retirement stream were read
where relevant and not edited. No dependency was added, no `unsafe` was
introduced, and no test was weakened.

## Implemented and proposed

- Implemented: the shell has a global `--token <token>` and reads
  `QUANTRULE_TOKEN` when the flag is absent. The value is validated as visible
  ASCII, held in a `BearerToken` newtype whose `Debug` renders `redacted`, and
  attached as `Authorization: Bearer` by the one client helper every GET and
  POST uses. An unset or empty value keeps the previous behavior.
- Implemented: the shell prints the token in no output path. The short help
  names the flag and variable, and the token is not written beside the stored
  profile.
- Implemented: the shell fixture can require a token and mirrors the gateway's
  open `/health` and `/version` routes. Nine printed-output tests cover the 401
  refusal, explicit and environment tokens, flag precedence, a wrong token, the
  JSON view, an interactive line, the open health route, invalid input, and a
  token scan of stdout, stderr, and the state directory.
- Implemented: `api.ts` has one active token and sets the header only when one
  is present; the browser stores it under `quantrule.token` in `localStorage`
  through `token.ts`. The Connection screen has a password field, a `Use token`
  action, and `Forget token`, and states where the value lives.
- Implemented: a `401` becomes the `unauthorized` connection state with the
  stored identity cleared and a token-required message. Because the gateway
  leaves `/version` and `/health` open, the connection refresh also probes the
  gated `/profiles` route; only that answer proves the token was accepted. A
  non-401 failure of the probe is left to the screen that owns the route.
- Implemented: bun unit tests cover the header, clearing it, the URL and error
  text, storage round-trip, blank values, blocked storage, and the 401
  classification. Playwright tests cover the token-required state, the observed
  `Authorization` header, persistence across reload, forgetting, no URL, page
  text, or cookie leakage, and a wrong token that is not echoed.
- Implemented: the remote tutorial now prints the token from `make remote`,
  enters it in Connection, and shows `--token` / `QUANTRULE_TOKEN` for the shell,
  with the TLS/private-network warning. `security.md` §2 and §2.1 describe the
  client handling and its risks; the changelog and README carry the change.
- Observed but not edited: `tools/remote.sh` still prints the #802-era sentence
  that the browser and shell do not send `Authorization` and that remote browser
  use needs a proxy. That text is now false. The script is outside this work
  item's ownership, so it needs a follow-up edit to point at the Connection
  field and `--token`.
- Not implemented: proxy token injection and any account system. The token
  remains one shared secret and one principal, by decision.

## Verification and evidence

All commands ran in `/home/zantgo/Proyectos/quant-rule--672-client` on branch
`feat/672-client-token` at base `642a764b`, on 2026-11-15 UTC.

| Check | Command | Observed |
|---|---|---|
| Shell tests | `cargo test -p quantrule-shell` | 84 passed, 0 failed (29 unit, 8 architecture, 2 client parity, 35 HTTP, 1 parity, 9 token) |
| Shell lints | `cargo clippy -p quantrule-shell --all-targets -- -D warnings` | passed, no warnings |
| Web install | `bun install --frozen-lockfile` | 54 packages, lockfile unchanged |
| Web types | `bun run check` | 0 errors, 0 warnings |
| Web unit tests | `bun test` | 109 passed, 0 failed |
| Web build | `bun run build` | 153 modules, `dist/` written |
| Web e2e | `bun run e2e` | 33 passed, including six token tests |
| Workspace gate | `tools/gate.sh` | passed; fmt, clippy, workspace tests, rustdoc, and every checker |
| Live fixture, no token | `curl /api/v1/profiles` | `401 auth.unauthorized` |
| Live fixture, wrong token | `curl -H 'Authorization: Bearer wrong-token'` | `401 auth.unauthorized` |
| Live fixture, token | `curl -H "Authorization: Bearer $TOKEN"` | `200` |
| Live fixture, open routes | `curl /api/v1/health`, `/api/v1/version` | `200` without a token |
| Shell without token | `quantrule --json profile list` | exit 1, `error: auth.unauthorized: a bearer token is required` |
| Shell `--token` | `quantrule --json --token "$TOKEN" profile list` | exit 0, same bytes as the environment run |
| Shell `QUANTRULE_TOKEN` | environment token, no flag | exit 0, same bytes as the flag run |
| Shell wrong token | `--token wrong-token` | exit 1, `auth.unauthorized`, value not echoed |
| Fixture log | `auth=absent/present`, `match=false/true` | no token in the log |
| Leak scan | grep of stdout, stderr, state directory, fixture log | no token found |
| Browser header | Playwright request observer | `Bearer qr_browser_token…` present on API calls |

The live fixture is a temporary Python server outside the repository; it logs
only whether a header was present and whether it matched, so the token itself
never reaches a log. The browser evidence is the Playwright request boundary:
the test asserts the header and that the token is absent from the URL, page text,
and cookies while present in `localStorage`.

## Learning

- The gateway exempts `/health` and `/version`, so a connection screen that
  only asks those two can show `connected` while every gated call answers `401`.
  The `/profiles` probe is what makes the 401 state honest; the e2e fixture
  mirrors the open-path list for the same reason.
- A derived value named `state` shadows Svelte's `$state` rune and compiles as a
  store subscription. Renaming it to `view` fixed the only check failure.
- A derived `Debug` on the invocation would have printed the token through any
  future error path. The private newtype with a redacting `Debug` makes that
  mistake unavailable rather than relying on review to catch it.
- The stale `tools/remote.sh` trailer is a real documentation defect produced by
  the previous slice and is recorded above rather than silently edited outside
  this work item's boundary.
