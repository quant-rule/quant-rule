# ADR — Add a Simulation Service and a Research Plane

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`01`](01-adopt-distributed-service-based-architecture.md), [`10`](10-python-boundary.md)
- Plan: [`dist-000-distributed-platform.md`](../../execution-plans/2026-11-12/dist-000-distributed-platform.md)

## Context

Chart-first research marks good and bad entries and exits on a historical chart
with the future visible, in the same schema as a deterministic oracle, and checks
the implied rule on a later, unmarked window. It is not a backtest. It is an
upstream hypothesis generator.

The previous increment ran it as a Python harness plus a throwaway local web
page. The distributed platform had no owner for it, which was a defect in the
plan, not a property of the product.

## Decision

Add a **sixth service**, `simulation`, and a third architectural plane.

| Plane | Style | Determinism | Has a queue |
|---|---|---|---|
| Data | Synchronous, fixed order | Required | no |
| Control | Asynchronous | Not required | yes |
| **Research** | **Synchronous, interactive** | **Bounded** | **no** |

**Rules, each enforced by a test:**

1. The research plane never writes into a run's artifact set.
2. No strategy, execution, or portfolio code may read a simulation record.
3. `simulation` may read a model bundle; it may never compute a model.
4. A simulation run has no `run_id`, no canonical digest, and no control-plane
   message.

**Determinism boundary (D21).** A simulation run records an **inputs digest**
over the model bundle, window selection, seed, mark criterion, and check
thresholds. It records **no output digest**: the oracle is deterministic, the
human marks are judgment, and a digest over human judgment would claim
reproducibility the system does not have.

**Six services, one image.** `gateway`, `data`, `model`, `backtest`, `evidence`,
`simulation` — six binaries, one digest.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Fold into `evidence` | Two determinism classes in one service; the auditability claim becomes ambiguous |
| Leave it as a CLI | Loses the marking loop in the client; keeps two UIs for one task |
| Keep the Python service | Contradicts D18; a second runtime in the image |
| A separate image for simulation | Two digests; version skew between research and pipeline |

## Consequences

- The service catalog grows to six; the deployment to nine containers.
- The research plane is isolated by construction, so hindsight cannot leak into
  the signal path.
- `evidence` keeps its narrow remit: run records and bundles.
- The client gains two screens and a chart.
- A sixth service is a real operational cost: one more container, one more
  failure mode, one more schema.

## Revisit trigger

Revisit if research work becomes multi-user, or if oracle and check grow beyond
a few seconds and need the control plane.

## Verification

- `services.md` lists six services with owned data.
- A guard test proves `simulation` cannot reach a run artifact or a strategy.
- A simulation run carries an inputs digest and no output digest.
