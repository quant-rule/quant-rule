# AI transparency log — DIST-030 gateway finalization

- Date: 2026-11-15 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash (implementation subagent)
- Work item: [#646](https://github.com/quant-rule/quant-rule/issues/646), `DIST-030`
- Plan: [`776-split-service-binaries.md`](../../execution-plans/2026-11-14/776-split-service-binaries.md) P8 and §5.8
- Branch: `feat/646-final`
- Worktree: `/home/zantgo/Proyectos/quant-rule--646-final` (base `ff0fa96d`)

## Intent and boundary

Close the remaining DIST-030 contract gaps after P8 retired `quantrule-server`:
the bearer-token seam, the profile middleware and real `/profiles` counts, and
`GET /inspect/events` without a model call in the data service. The agent owned
`crates/quantrule-gateway/**`, `crates/quantrule-model-service/**` only for the
internal event-batch route, `crates/quantrule-runtime/**` client extension
points, the two contract additions in `crates/quantrule-api/**`, gateway and
model tests, `CHANGELOG.md`, and this log. The data, evidence, simulation,
backtest, shell, and simulation-engine crates were read but not edited;
`apps/**`, Dockerfile, compose, Makefile, CI, and `docs/port-parity` were not
touched. The parked #646 worktree was read only; its auth logic and its
`EventBatch`/`EventRecord` contract hunk were reused, and its server-bound
handlers were not.

No dependency was added, no `unsafe` was introduced, and no test was weakened.
The gateway still links no app, engine, or domain service crate.

## Implemented and proposed

- Implemented: a bearer middleware on the API routes. When `QR_BEARER_TOKEN` is
  set, every API request except `/health`, `/version`, and the static client
  needs `Authorization: Bearer`; a refusal is `401 auth.unauthorized` with
  `WWW-Authenticate: Bearer`. The comparison is length-safe and has no early
  exit, the token is never logged, and an unset token keeps the documented
  private-network behavior. `/internal/*` also never falls back to the SPA, so
  an internal path is not even served as HTML.
- Implemented: `/profiles` now reads the run count from evidence and the
  simulation count from simulation, alongside the existing dataset count from
  data. The runtime gained `RunCounter` and `SimulationCounter` seams, their
  HTTP implementations, and `MemoryCounts` for tests.
- Bounded count, stated honestly: evidence and simulation expose no count route,
  and this work item may not edit those services, so each count is the first
  page of the service's list route (200 records, the largest page those routes
  accept). A profile with more records reports the page, not an invented total.
  The limitation is recorded in [`http-api.md`](../../architecture/http-api.md)
  and in the changelog.
- Implemented: `GET /inspect/events?data_version=…&timeframe=…` is proxied to a
  new model-owned internal route, `/internal/v1/events`. The model service reads
  the dataset reference through `DataClient`, verifies the object hash, computes
  the requested timeframe alone with the registry's default
  `MarketStateConfig`, and returns the full `all_events()` batch in the numeric
  `EventBatch`/`EventRecord` contract. Inspection writes no object. The gateway
  validates the profile, preserves the query, and builds the internal path; a
  client cannot choose it. The data service never computes a model.
- Implemented: a gateway route-table test over all 34 documented endpoints at
  both `/` and `/api/v1`, the record-route profile matrix (missing, unknown, and
  known), the token matrix, the SPA fallback and internal-path refusal, the
  upload 413 passthrough, the model-compute route, the inspection rewrite, and
  per-profile counts. A model-service test covers the batch, determinism, the
  unknown timeframe, a timeframe without a complete bar, an unknown dataset, and
  the required profile.
- Proposed but not implemented: an exact run/simulation count route. It needs
  evidence and simulation edits, which this work item's ownership excludes; the
  bounded list count is the honest stopgap.

## Verification and evidence

All commands were run in `/home/zantgo/Proyectos/quant-rule--646-final` on this
host, on branch `feat/646-final` at base `ff0fa96d`.

| Check | Command | Observed |
|---|---|---|
| Format | `cargo fmt --all -- --check` | passed |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| Workspace tests | `cargo test --workspace` | 102 test targets, 0 failed |
| Gateway tests | `cargo test -p quantrule-gateway` | 90 passed, 0 failed |
| Model-service tests | `cargo test -p quantrule-model-service` | 25 passed, 0 failed |
| Event batch test | `cargo test -p quantrule-model-service --test events` | 5 passed, 0 failed |
| Runtime count test | `cargo test -p quantrule-runtime counts` | 2 passed, 0 failed |
| Workspace gate | `tools/gate.sh` | passed; 102 test targets, 0 failed, and every Python checker ok |
| Command surface | `python3 tools/check_command_surface.py` | passed |

The route-table coverage is the `DOCUMENTED_ENDPOINTS` constant in
`crates/quantrule-gateway/tests/surface.rs`: 34 endpoints, 23 from
`http-api.md` §2 and 11 from §6, each asserted at both prefixes. The record set
is 30 of them; the four unprofiled routes are `/health`, `/version`, and
`GET`/`POST /profiles`.

## Learning

- The P3 refusal to compute a model in the data service is what makes
  `/inspect/events` a gateway rewrite rather than a data passthrough. Keeping the
  model call in the model service also keeps the data service's dependency set
  unchanged.
- The parked auth file was directly reusable; the parked event contract was
  reusable as a shape, but its data-service handler was not, because ownership
  moved with the P3 decision.
- A `ProfileSummary` count cannot be exact while the owning service exposes only
  a capped list page. Reporting the bound in the contract is preferable to
  inventing a number or silently keeping zero.
- The token is opt-in, so the unauthenticated shell contract is unchanged when
  `QR_BEARER_TOKEN` is unset. The shell has no token flag; a remote deployment
  that enables the token still needs a shell/client follow-up owned outside this
  work item.
