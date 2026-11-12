# AI Transparency Log — Distributed Platform v1.0.0

- Date: 2026-11-12 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: `DIST-000`, plan [`dist-000-distributed-platform.md`](../../execution-plans/2026-11-12/dist-000-distributed-platform.md)
- Scope: documentation, architecture analysis, issue definition

## Intent and boundary

The human asked for a distributed architecture for an internal, single-operator
research tool, with Docker services over a network, a Svelte-only client, and a
complete documentation and issue set. The human explicitly excluded program
changes from this session.

AI contributions: architecture comparison, the two-plane model, service
decomposition, contract drafts, documentation rewrites, and issue text.

The AI did **not**: approve the plan, choose the product direction unilaterally,
run the distributed system, or verify any code behavior. No implementation
exists yet.

## Verification and evidence

| Claim | Evidence | Status |
|---|---|---|
| Documentation links resolve | `python3 tools/check_links.py` | verified |
| Retired terms absent | `python3 tools/check_terms.py`, `check_legacy.py` | verified |
| Plan structure valid | `python3 tools/check_execution_plans.py` | verified |
| Workflow index valid | `python3 tools/check_workflows.py` | verified |
| Command surface valid | `python3 tools/check_command_surface.py` | verified |
| Cross-machine determinism | not testable on one machine | **not verified**; claim narrowed in ADR-2026-11-12-07 |
| `make up` works | no code exists | **not verified** |

## Learning and rejected suggestions

- **Rejected: microservices.** Twenty services, sagas, and a mesh would add
  operational cost and determinism risk with no driving characteristic.
- **Rejected: event-driven data plane.** Choreographing data → model → backtest
  as events would make byte-identical output depend on queue timing.
- **Rejected: multi-tenancy now.** The human deferred it; the `owner` column and
  the authorization seam preserve the option at zero cost.
- **Rejected: Tauri retained.** A pure HTTP client needs no native shell; keeping
  it would preserve a second transport path.
- **Corrected: "no accounts" was read as "no protection".** Remote exposure still
  requires a network gate or a bearer token; recorded in the security document.
- **Corrected: cross-machine determinism was initially presented as a test.** It
  is an invariant plus a reviewer procedure, because no second machine is
  available to the operator.
- **Corrected: canonical digest must exclude `provenance.json`.** Including it
  would make every reviewer comparison fail for a correct system.
