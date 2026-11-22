# RA-009 Additive Pack and Signal Client Surface

- Date: 2026-11-22 (UTC)
- Status: In Review
- Issue: [#1097](https://github.com/quant-rule/quant-rule/issues/1097)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-22/1097-signal-client.md`](../../ai-transparency-log/2026-11-22/1097-signal-client.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md)
- Parent: [#1083](https://github.com/quant-rule/quant-rule/issues/1083), RA-009
- Target: `v1.5.7`

## 1. Intent

Make the research artifact layer reachable from both clients without changing any
existing command meaning. The change adds pack lifecycle, signal metadata and
download, and run lineage to the shell and the browser, and adds the public
gateway routes those two clients use.

## 2. Current state

- RA-002…RA-008 implement pack schemas, run v5, the pack registry, lineage,
  retention, the signal artifact and the seventh strategy service.
- Pack routes exist under `/catalog/packs`; signal and lineage routes are
  internal only.
- `SignalRequest`/`SignalResponse` live in `quantrule-service-api`, which the
  shell must not depend on.
- The shell has 49 commands; the browser reaches the same 49 use cases.
- `docs/architecture/http-api.md` is 229 lines (cap 230) and
  `docs/architecture/research-artifacts.md` is 170 lines (cap 170).

## 3. Scope

Included:

- Twelve additive commands: `pack create/list/show/validate/freeze/retire/fetch`,
  `signal create/list/show/fetch`, `run lineage`.
- Public gateway routes for signal create/list/show/bytes and run lineage.
- Public data aliases `GET /signals`, `/signals/{id}`, `/signals/{id}/bytes`.
- A `packs` browser section with Packs and Signals tabs, and a lineage block on
  run detail.
- Contract, manual, tutorial and changelog updates, including the 61-command
  count.

Excluded:

- Changing an existing command's name, arguments, output fields, errors, or
  renderer.
- Run submission, catalog scan, worker resolution of a v5 config, or writing
  lineage before submission.
- Any client-side signal computation or signal-row parsing.
- A migration (no new table or column is needed).
- A new dependency or a new service.

## 4. Assumptions and uncertainty

- The strategy service is optional; a v2 deployment has no `QR_STRATEGY_URL`.
- Existing pack and artifact routes keep their exact bytes and errors.
- `run lineage` can be a read-only command; `run status` keeps its current
  output and the browser run detail adds lineage as a separate block.
- The two contract documents can be net-compressed without dropping a rule.

## 5. Decisions

| # | Decision | Status |
|---|---|---|
| 1 | Add 12 commands for a 61-command surface; existing 49 contracts are unchanged; help grows additively | Approved |
| 2 | Command set is exactly the list in section 3 | Approved |
| 3 | Gateway signal create goes to strategy `/internal/v1/signals`; signal reads go to data; lineage goes to evidence; absent strategy is `503 service.unavailable` | Approved |
| 4 | `SignalRequest`/`SignalResponse` move to `quantrule-api`; service-api re-exports them with identical wire bytes | Approved |
| 5 | Data adds public signal aliases; internal artifact routes stay | Approved |
| 6 | One `packs` browser section with Packs/Signals tabs; signal list/detail carry metadata only; download carries the document | Approved |
| 7 | v2 returns 503 for signal create; pack and signal reads work; the browser disables create and states the gap | Approved |
| 8 | Amend the two contract documents whose policy forbids a public route/command/screen; the change is additive | Approved |
| 9 | Update every command-count reference, including `AGENTS.md`'s 49-command sentence | Approved |

## 6. Acceptance criteria

- `help` lists 61 commands; each existing command's contract is byte-identical.
- Every new command has a parser, help entry, printed-output test and tutorial
  invocation.
- `pack create` sends the local file bytes; `pack fetch` and `signal fetch`
  verify the content hash before writing.
- The gateway forwards the public routes with the validated profile, streams the
  create body, preserves bytes/ETag, refuses traversal, and keeps the bearer
  gate.
- Browser Packs and Signals controls reach the same use cases; run detail shows
  lineage or “No recorded lineage”.
- Docs, command registry, README, manual, changelog and AI log agree.

## 7. Work breakdown

| Step | Work | Paths |
|---:|---|---|
| 1 | Approved plan, AI log, pre-change help and parity fixtures | `docs/execution-plans`, `docs/ai-transparency-log`, `tests/fixtures` |
| 2 | Move signal types to `quantrule-api`; re-export from service-api | `crates/quantrule-api`, `crates/quantrule-service-api` |
| 3 | Public signal aliases and public lineage route | `crates/quantrule-data-service`, `crates/quantrule-evidence-service` |
| 4 | Gateway routes, optional-strategy 503, allowlist, deadlines, tests | `crates/quantrule-gateway` |
| 5 | Shell types, parser, help, dispatch, renderers, tests | `crates/quantrule-shell` |
| 6 | Browser transport, stores, panels, run lineage, tests | `apps/web` |
| 7 | Contract, architecture, manual, README, tutorial, changelog | `docs`, `README.md`, `CHANGELOG.md` |
| 8 | Format, clippy, tests, docs, governance, gate | workspace |

## 8. Verification

Commands:

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-api -p quantrule-service-api -p quantrule-data-service \
  -p quantrule-evidence-service -p quantrule-gateway -p quantrule-shell --all-targets -- -D warnings
cargo test -p quantrule-api -p quantrule-service-api -p quantrule-data-service \
  -p quantrule-evidence-service -p quantrule-gateway -p quantrule-shell
cd apps/web && bun test && bun run check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -q -p quantrule-governance -- check --all
docker compose config -q
tools/gate.sh
```

No migration is added, so no Postgres composed tier is required.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Plan | Plan and AI log exist before implementation | Files present | Present | Pass | Pass | AI (implementation) | 2026-11-22 | Plan and AI log created before implementation. |
| V02 | Fixtures | Pre-change 49 help contracts and 40 requests are pinned | `help-49.json`, `view-49.json` | 49/40 | Pass | Pass | AI (implementation) | 2026-11-22 | help-49.json has 49 entries; view-49.json has 49 commands and 40 requests. |
| V03 | Contracts | Signal types live in `quantrule-api`; service-api re-exports them | `cargo test -p quantrule-api -p quantrule-service-api` | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | Signal types moved; both contract crates pass. |
| V04 | Data | Public signal list/show/bytes work; unknown id 404; hash/ETag verified | `cargo test -p quantrule-data-service` | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | artifacts target: 8 pass, including public list/show/bytes, ETag, 404, hash mismatch. |
| V05 | Evidence | Public lineage route uses the existing visibility rule | `cargo test -p quantrule-evidence-service` | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | lineage target: 4 pass, 1 ignored (needs a disposable Postgres). |
| V06 | Gateway | Forward path/method/body/profile, 503 without strategy, token gate, traversal, bytes/ETag, deadlines, lineage | `cargo test -p quantrule-gateway` | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | signal_proxy target: 7 pass; gateway suite passes. |
| V07 | Shell | 61 commands; existing contracts byte-identical; 12 printed-output tests | `cargo test -p quantrule-shell` | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | 65 printed-output tests; 61 commands; command_surface pass. |
| V08 | Shell fetch | `pack fetch`/`signal fetch` verify the hash; parser refuses bad kind/state/digest before a call | Shell tests | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | write_verified unit tests and parser failure tests pass. |
| V09 | Browser | Packs/Signals paths, methods and bodies match the contract; profile reset covers both | `bun test` | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | bun test 170 pass; svelte-check 0 errors and 0 warnings. |
| V10 | Browser E2E | Pack lifecycle, signal create/503, run lineage | Playwright | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | Playwright chromium: 63 pass. |
| V11 | Parity | 61 commands and 12 appended requests; first 49/40 unchanged | Parity tests | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | shell parity 61 commands/52 requests; browser 61/50. |
| V12 | Docs | Registry, README, manual, contracts, tutorial, changelog agree; caps respected | `qr-governance check --all` | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | qr-governance check --all: ok. |
| V13 | Clippy | No warnings on touched crates | `cargo clippy … -- -D warnings` | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | clippy -D warnings on the six touched crates. |
| V14 | Docs build | Rustdoc and governance clean | `cargo doc`, governance | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps. |
| V15 | Compose | Compose file still validates | `docker compose config -q` | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | docker compose config -q exited 0. |
| V16 | Gate | Full gate passes | `tools/gate.sh` | Pass | Pass | Pass | AI (implementation) | 2026-11-22 | tools/gate.sh: gate passed: full. |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| An existing command contract drifts | Byte-for-byte help fixture and the 49-command/40-request snapshot |
| Gateway becomes a domain handler | Signal proxy builds only the internal path; it decodes nothing |
| Signal bytes over HTTP | Create carries a request; the browser downloads bytes from data only |
| v2 lacks the strategy service | Optional upstream and an explicit 503; the browser states the gap |
| Doc caps exceeded | Net-compress the two capped documents rather than append |
| Client computes a signal | Panel renders metadata and downloads the document; no row parsing |

Recovery: revert the additive surface; existing commands and routes are
untouched.

## 10. Stop conditions

Stop if an existing command contract must change, if a new dependency or
migration is required, if the gateway would need to decode a signal, or if a
governance cap cannot be met without dropping a rule.
