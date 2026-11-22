# AI Transparency Log — RA-009 Additive Pack and Signal Client Surface

- Date: 2026-11-22 (UTC)
- Work item: [#1097](https://github.com/quant-rule/quant-rule/issues/1097)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Plan: [`docs/execution-plans/2026-11-22/1097-signal-client.md`](../../execution-plans/2026-11-22/1097-signal-client.md)
- Branch: `feat/1097-signal-client`

## Intent and boundary

Implement RA-009: make the pack registry, signal artifact and run lineage
reachable from the command shell and the browser through the public gateway.
The change is additive. The 49 existing command contracts, their arguments,
output fields, errors and renderers stay unchanged; the surface grows to 61.

The work does not add a migration, a dependency, a service, a client-side
signal computation, or a change to run submission and the worker. It does not
write lineage; it reads the lineage evidence already records.

## Work performed

- Recorded the pre-change surface before editing: `help-49.json` holds the exact
  `help <command>` text for all 49 existing commands, and `view-49.json` holds
  the 49 command names and the 40 observed shell requests.
- Moved `SignalRequest`/`SignalResponse` and their schema tokens to
  `quantrule-api`; `quantrule-service-api` re-exports them with identical wire
  bytes.
- Added public data aliases for signal metadata and bytes, and a public evidence
  route for run lineage.
- Added gateway routes for signal create, signal reads and lineage, including the
  optional-strategy 503, the data-path allowlist, the long deadlines, and a
  signal proxy module.
- Added 12 shell commands, parsers, help entries, dispatch, renderers and
  printed-output tests, including hash-verified pack and signal downloads.
- Added a `packs` browser section with Packs and Signals tabs, their transport
  modules and stores, profile-scope reset, and a run-detail lineage block.
- Updated the command registry, HTTP contract, client architecture, research
  artifact notes, pack and signal contracts, README, manual, tutorial, changelog
  and governance count references.

## Design decisions for review

- **Additive, not a contract change.** The existing help text is a checked
  fixture; appending entries cannot alter an earlier block.
- **Gateway stays a forwarder.** The signal create proxy builds
  `/internal/v1/signals`; it never decodes a request or a response, and signal
  bytes only travel on the public data read.
- **Optional strategy.** A missing `QR_STRATEGY_URL` is a configured v2 profile,
  not a failed gateway. Create returns `503 service.unavailable`; reads still
  work.
- **Metadata versus document.** The browser list and detail render
  `ArtifactRecord` only. The signal document is a verified download.
- **Lineage is a read.** `run lineage` and the run-detail block only read the
  recorded reference set; no submission path is added.
- **Docs compressed, not appended.** `http-api.md` and
  `research-artifacts.md` were rewritten in place to stay under their caps.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy -p quantrule-api -p quantrule-service-api -p quantrule-data-service -p quantrule-evidence-service -p quantrule-gateway -p quantrule-shell --all-targets -- -D warnings` | Pass |
| `cargo test -p quantrule-api -p quantrule-service-api` | Pass |
| `cargo test -p quantrule-data-service --test artifacts` | 8 pass |
| `cargo test -p quantrule-evidence-service --test lineage` | 4 pass, 1 ignored (needs a disposable Postgres) |
| `cargo test -p quantrule-gateway --test signal_proxy` | 7 pass |
| `cargo test -p quantrule-shell` | 65 printed-output, parity and architecture tests pass; `command_surface` pins the 49 contracts |
| `cd apps/web && bun test` | 170 pass |
| `cd apps/web && bun run check` | 0 errors, 0 warnings |
| `cd apps/web && bunx playwright test --project=chromium` | 63 pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Pass |
| `cargo run -q -p quantrule-governance -- check --all` | `governance check ok` |
| `docker compose config -q` | Exit 0 |
| `tools/gate.sh` | `gate passed: full` |

## Learning and corrections

- The shell test support module was already 971 lines; it was split into wire
  and routing modules before new routes were added, so no file crosses 1,000
  lines.
- `SignalRequest` already carried a closed `deny_unknown_fields` parser; the move
  keeps the schema tokens and adds no field, so existing clients decode the same
  bytes.
- The gateway data allowlist is a prefix list; `/signals` was added rather than
  widening the proxy to every path.
- The browser run detail loads lineage separately, so a missing lineage is an
  empty state and not a run error.
- Reformatting `tests/fixtures/client-parity/api.json` and `view.json` through a
  JSON parser turned `1.0` into `1` and broke the existing shared view. Both
  files were restored and the new values were inserted textually, so the
  pre-change bytes and float representation are unchanged.
- The Playwright fixture helper keys routes by the URL pathname; a content id
  contains `%3A`. The Packs E2E test decodes the path before matching, as the
  shell fixture already does.
- A strict `getByText(/Created/)` matched the signal notice and the table header.
  The notice now carries `data-testid="signal-created"`.

## Residual risk

- The v2 multi-host profile does not run the strategy service, so signal create
  is unavailable there by design. Reads and pack lifecycle still work.
- Run submission still does not record lineage. A run submitted before that
  integration has "No recorded lineage".
- No composed or HA evidence is claimed by this entry.

No edge or profitability claim is made.
