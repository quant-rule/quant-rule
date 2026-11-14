# Execution Plan — #668 DIST-070 Shell HTTP Client

- Status: In Progress
- Claim type: Behaviour Claim
- Issue: [#668](https://github.com/quant-rule/quant-rule/issues/668)
- Parent plan: `DIST-000` (DIST-070)
- Date: 2026-11-14 (UTC)
- Worktree: `/home/zantgo/Proyectos/quant-rule--668-shell-http`
- Branch: `feat/668-shell-http`

## Intent

Make the command shell a thin `quantrule.api/v1` client with no engine
dependency. The shell must reach runs, research, and profiles; the browser and
the shell must share one HTTP contract. The operator gets stable printed output
and a `--json` machine view; an agent gets a command for every research-loop and
profile action.

## Current state

`crates/quantrule-shell` depended on `quantrule-app`, `-backtest`, `-data`, and
`-model`, executed every stage locally, fetched Dukascopy data, and held a local
SQLite research memory. The gateway routes that exist are `/health`, `/version`,
`/hypotheses`, `/catalog/datasets`, `/catalog/configs`, `/model/compute`,
`/runs`, `/runs/{id}`, `/runs/{id}/cancel`, `/profiles`, `/inspect/candles`,
`/inspect/timeframes`, `/datasets/{version}/candles`, and
`/datasets/{version}/ladder/{timeframe}`. `/runs/{id}/artifacts/{name}`,
`/runs/{id}/digest`, and `/inspect/events` are specified in `http-api.md` and
arrive with the gateway stream; the simulation routes arrive with #680.

## Scope

Included:

- One HTTP client module over `ureq`; one call per command.
- `health`, `dataset list|show|upload`, `config list|show`,
  `run submit|status|list|fetch|cancel`, `inspect candles|timeframes|events`,
  `verify`, `help`.
- `profile list|create|use` and the global `--profile`.
- `simulation list|create|windows|mark|marks|oracle|check|decide`.
- A global `--json` view for parity and agent use.
- Removal of local stage execution, source fetch, and local memory.
- Printed-output tests against an in-process fixture server.
- A parity test against the browser's run-detail fixture values.

Excluded:

- Any server route, contract type, browser, desktop, image, or deployment file.
- A local fallback when a route is missing.
- New simulation response types in `quantrule-api`; the missing types are
  reported for #680.
- Merging, release tagging, and the board transition to `Done`.

## Assumptions and uncertainty

- The gateway, profile middleware, digest, artifact, and event routes are
  specified but not all merged. The shell implements the documented paths and
  tests them against the fixture server.
- The simulation responses for `windows`, `marks`, `oracle`, `check`, and
  `decide` have no contract types in `quantrule-api`. The shell prints the
  gateway JSON canonicalised; #680 owns the types and the final shapes.
- `config show` reads the collection and selects the row because no single-config
  route is documented. `verify` takes a run id because the contract has no
  fixture-run registry.
- The browser E2E suite owns the DOM parity. This change tests the shell side
  against the same fixture values; #669 owns the automated cross-client test.

## Decisions

| Decision | Consequence |
|---|---|
| Use `ureq` directly, blocking | No runtime, no new dependency, one transport module |
| `--json` prints the gateway value | Parity is comparable; agents avoid text parsing |
| `run fetch` verifies size and SHA-256 before writing | A corrupted artifact is refused, not stored |
| Profile precedence is flag, stored, `default` | A refusal is never hidden by a fallback |
| Simulation responses print canonical JSON | No invented contract types in the client |
| Historical labs are marked, not deleted | The retired surface stays traceable |

## Acceptance criteria

- The shell manifest's product dependencies are exactly `quantrule-api`,
  `serde_json`, and `ureq`; `cargo tree` has no engine edge.
- Every listed command parses, calls the documented route once, and prints a
  deterministic result.
- `--profile` sets the header; `profile use` stores only a confirmed profile.
- The fixture-server tests cover every command and assert the request method,
  path, header, and body.
- The parity test prints the browser run-detail fixture's values for the same
  profile.
- The discovery checker passes with 27 commands.

## Work breakdown

1. Replace the dependency set and module graph. (Implementation)
2. Add the transport, session, profile store, and output modules.
3. Replace the parser, dispatcher, and help registry.
4. Add simulation verbs against the documented request fields.
5. Add the fixture server, printed-output tests, and parity test.
6. Update the command-surface registry, README, tutorials, changelog, and AI log.
7. Run the gate and record the observed result. (Test, Reviewer)

## Verification

Commands are run from the worktree with `CARGO_TARGET_DIR` inside it. The
canonical comparison is printed output and the dependency tree, not a summary.

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V1 | Dependency set | No engine or app crate in the shell tree | `cargo tree -p quantrule-shell --edges normal` | Only `quantrule-api`, `serde_json`, `ureq` and their transitive crates | No `quantrule-data`, `-model`, `-backtest`, or `-app` edge | Pass | agent | 2026-11-14 | architecture guard also asserts the manifest set |
| V2 | Formatting | The workspace is rustfmt-clean | `cargo fmt --all -- --check` | exit 0 | exit 0 | Pass | agent | 2026-11-14 | |
| V3 | Lints | No clippy warning in the shell | `cargo clippy -p quantrule-shell --all-targets -- -D warnings` | exit 0 | exit 0 | Pass | agent | 2026-11-14 | |
| V4 | Shell tests | Unit, architecture, printed-output, and parity tests pass | `cargo test -p quantrule-shell` | all pass | 66 tests pass | Pass | agent | 2026-11-14 | 22 unit, 8 architecture, 35 output, 1 parity |
| V5 | Discovery | Help, README, registry, and tutorials agree | `python3 tools/check_command_surface.py` | 27 implemented, 0 planned | 27 implemented, 0 planned; 17 tutorial files | Pass | agent | 2026-11-14 | |
| V6 | Parity tier | The port-parity registry passes | `tools/test_tiers.sh parity` | exit 0 | exit 0; 0 registered ports | Pass | agent | 2026-11-14 | |
| V7 | Browser parity | The shell prints the browser run-detail fixture | `cargo test -p quantrule-shell --test parity` | the fixture's values for the default profile | exact printed match; `X-QuantRule-Profile: default` | Pass | agent | 2026-11-14 | DOM comparison remains #669 |
| V8 | Full gate | The workspace gate passes | `tools/gate.sh` | exit 0 | exit 0; all checkers pass | Pass | agent | 2026-11-14 | R1 resolved before the final run |
| V9 | Server boundary | No server, engine, API, or browser file changed | `git status --short` and `git diff --name-only` | only shell, the moved app test, the registry, and documentation | shell, `crates/quantrule-app/tests/report_spec_isolation.rs`, `docs/registry/verification.json`, `tools/build_verification.py`, and docs | Pass | agent | 2026-11-14 | narrow R1 exception |

## Risks and recovery

- **R1 — verification registry (resolved).** The
  `report_spec_isolation::two_model_specs_with_disjoint_active_events_stay_disjoint`
  property moved from the retired `report model` shell test to
  `crates/quantrule-app/tests/report_spec_isolation.rs`. The assertion is
  unchanged: the two specs must produce event codes 8 and 9, and the two code
  sets must be disjoint. `docs/registry/verification.json` and
  `tools/build_verification.py` name the new path. The fixture bytes are read
  from the shared report fixture rather than duplicated. No fake or ignored test
  was added.
- **R2 — missing routes.** `/runs/{id}/digest`, `/runs/{id}/artifacts/{name}`,
  and `/inspect/events` are implemented against the documented contract and
  tested against the fixture. If the gateway changes the shape, the shell's
  decode test fails loudly.
- **R3 — simulation shapes.** The response shapes are undocumented until #680.
  The shell prints canonical JSON; a shape change does not require a client
  change, but a field-specific renderer would.
- **R4 — artifact traversal.** `run fetch` refuses absolute, `..`, empty, and
  backslash names before writing. A regression test covers the cases.

## Stop conditions

- A contract type is missing and cannot be worked around. (Simulation response
  types are worked around with canonical JSON; reported for #680.)
- A required file is owned by another stream. (The human authorized a narrow
  exception for the moved verification test and its registry entry.)
- A command cannot drop an engine dependency without a contract change. (None
  found.)
