# ADR-2026-11-16-01 — Stage Execution and the Evidence Boundary

- Date: 2026-11-16 (UTC)
- Status: Accepted
- Deciders: Human
- Claim type: ADR (Architecture Decision)
- Epic: #842
- Issue: #844

## Context

The trader needs to iterate the model, strategy, and portfolio engines without
running the full pipeline for every inspection. Today the data plane computes one
coupled run, and `docs/architecture/http-api.md` §8 explicitly prohibits an
endpoint that executes a single pipeline stage. The prohibition protected the
evidence path, but it also made engine iteration slow and opaque.

The current worker already consumes a content-addressed model bundle, so a model
artifact can be reused without changing the model contract. A standalone strategy
stage cannot be treated as evidence: it simulates its own position, while the
coupled runner supplies the position execution actually holds. The same applies
to portfolio fills.

## Decision

1. Add **server-side diagnostic stage runs** for the model, strategy, and
   portfolio engines. The data stage remains `dataset upload`, `dataset show`,
   and `inspect candles/timeframes`; there is no `data run`.
2. The **evidence service owns the stage lifecycle** in a `stage` schema
   (`runs`, `submissions`, `artifacts`). The backtest worker computes every stage
   kind; the gateway proxies `/stages/*`; the shell and browser remain thin
   clients.
3. A stage artifact is **content-addressed**, profile-scoped, immutable, and
   expires under `QR_STAGE_RETENTION_DAYS`. A killed worker leaves no visible
   artifact. The stage manifest is `quantrule.stage-manifest/v1` and carries
   `evidence: false`.
4. A stage artifact is **never evidence**: it cannot enter qualification, the
   improvement ledger, a baseline, or the canonical digest.
5. A backtest may **reuse a frozen model artifact** through
   `run submit --model-artifact`. The model config hash must match, the model
   artifact hash is recorded, and the model bytes must equal a full run.
   Strategy, execution, and portfolio are **recomputed**; their stage artifacts
   are never reused as evidence.
6. `run submit` remains the **only evidence command**. No `backtest run` alias is
   introduced. The coupled run is the only producer of a canonical digest.
7. End-to-end verification is a **Release & Operations** procedure, not a quant
   tier. It proves a clean deployment reproduces a digest; it is not evidence.
8. The `http-api.md` §8 prohibition is **superseded for diagnostic stage runs
   only**. The prohibition continues to bind every other single-stage shortcut.

## Alternatives

| Alternative | Why rejected |
|---|---|
| Keep full-run-only | Safe, but every model or portfolio inspection pays for the whole pipeline; iteration and diagnosis stay slow |
| Restore local stage commands | The shell would need an engine dependency; the distributed platform removed that surface on purpose |
| Treat stage runs as evidence | Confounds the evidence path; strategy and portfolio stages can differ from the coupled run, and a stage has no canonical digest |
| Add a seventh stage service | Six services, one image is the deployment boundary; the backtest worker and evidence store already have the needed seams |
| Reuse strategy and portfolio stage artifacts in a backtest | Decisions depend on the position execution actually holds; the backtest must recompute them |

## Consequences

Positive:
- The trader can inspect model events, strategy decisions, and portfolio fills
  without a full run, while the evidence path stays single and reproducible.
- Stage artifacts are content-addressed and deterministic, like every other
  engine artifact.
- The CLI and browser share one contract and one parity test.

Negative:
- A new lifecycle, schema, queue routes, retention policy, and test surface.
- Stage runs need a lower queue priority than evidence runs.
- The diagnostic/evidence distinction must be visible in the API, the CLI, and
  the GUI, or users will mistake one for the other.

Mitigations:
- `evidence: false`, explicit badges, and tests that reject a stage artifact
  where evidence is required.
- Additive migrations and a feature flag; the coupled run is unaffected if the
  stage surface is disabled.
- TTL deletes only stage artifacts; evidence copies its inputs and is permanent.

## Links

- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)
- Command surface: [`docs/architecture/command-surface.md`](../../architecture/command-surface.md)
- HTTP API: [`docs/architecture/http-api.md`](../../architecture/http-api.md)
- Data plane: [`docs/architecture/data-plane.md`](../../architecture/data-plane.md)
- Services: [`docs/architecture/services.md`](../../architecture/services.md)
- Run contract: [`docs/workflows/run-contract.md`](../../workflows/run-contract.md)
