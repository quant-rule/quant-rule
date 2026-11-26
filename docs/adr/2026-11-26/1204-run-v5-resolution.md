# ADR-RA-2: Run v5 Resolution, Lineage and Worker Consumption

- Date: 2026-11-26 (UTC)
- Status: Accepted (2026-11-26 UTC)
- Issue: [#1204](https://github.com/quant-rule/quant-rule/issues/1204)
- Decision owner: Human
- Approval: human, 2026-11-26 (UTC)
- Related: [ADR-RA](../2026-11-22/1082-research-artifact-layer.md), [research artifacts](../../architecture/research-artifacts.md), [v1.6.1 plan](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md)
- Claim type: ADR (Architecture Decision)

## Context

`quantrule.run/v5` is a contract and an identity. The loader can resolve its
five references through an in-memory resolver, but production does not:

- the data-service config bundle does not expose the v5 reference set;
- evidence does not validate or record the set at submission;
- the worker calls `RunConfig::load`, which rejects v5;
- `Workspace::create` requires a `strategy` file that v5 does not have.

The result is a signal artifact that cannot be cited by an executable run and a
`run lineage` read with no producer. The v5 config hash already covers the
references, so the run identity is correct; the missing piece is resolution and
recording, not a new identity formula.

Evidence must not depend on `quantrule-app`; the data service already does, and
the worker already does. A pack or signal may not travel in a lineage row, and a
run may not make a network call per bar.

## Decision

1. **Split ownership by content.**
   - `data` owns strategy, portfolio and execution pack bytes, the optional
     signal-artifact bytes, and their registry rows.
   - `evidence` owns model-stage artifacts, run rows, and run lineage.
   - `backtest` owns the per-bar fold.
2. **Carry references, not payloads.** Extend `quantrule.internal/v1`
   `ConfigBundle` with an ordered `references: Vec<RunArtifactReference>`.
   v3/v4 bundles carry an empty list; a v5 bundle carries the present references
   with pack provenance from the registry. The list is deterministic and holds
   no pack or signal body.
3. **Validate data-owned references at upload.** The data service parses the v5
   document, resolves the three packs and the optional signal artifact from its
   own registry and object store, and verifies each id and hash before it marks
   the bundle valid. It records the references with pack provenance.
4. **Validate the model reference at submission.** The data service checks the
   model reference's shape; evidence validates it against the succeeded
   model-stage artifact's canonical artifact-set id. It must equal both the v5
   reference and the optional `--model-artifact` id. A model artifact that is not
   a succeeded stage is refused.
5. **Resolve every reference before the run row.** On submission, evidence
   resolves every present reference through the data service (packs, signal) or
   the stage store (model), verifies the content hash, and writes the run row,
   its outbox intent, and its lineage rows in one evidence transaction. A
   missing or mismatched reference creates no run row and no queued message.
6. **Consume once in the worker.** After a successful claim, the worker builds a
   `MemoryReferences` from the data-service pack and signal bytes and the
   evidence model manifest, then calls `RunConfig::load_with` once. It uses the
   resolved `RunConfig` for the fold. No catalog call occurs per bar.
7. **Do not require legacy files for v5.** `Workspace` materializes the files the
   bundle owns. For a v5 run it does not require `strategy`, `execution` or
   `portfolio` files; the fold uses the resolved `RunConfig` and its strategy.
   v3/v4 keep the existing file path and `RunConfig::load` behavior.
8. **Keep identity and the database boundary.** The run id and config hash
   formulas do not change; no database is added. Lineage is evidence-local, pack
   and signal bytes stay in data, and model bytes stay in the evidence store.

## Alternatives considered

| Alternative | Consequence | Rejected because |
|---|---|---|
| Evidence depends on `quantrule-app` to load v5 | One loader call | Breaks the service boundary; duplicates the worker's engine dependency in evidence |
| Resolve only in the worker | Smaller submission change | A missing reference creates a doomed run and no early error; lineage still has no producer |
| Resolve only at submission, store the resolved bytes in evidence | Worker needs no client reads | Duplicates pack/signal payloads and makes evidence a second byte authority |
| Add a new artifact database or registry | One obvious owner | A fifth stateful system is not needed; the catalog and evidence already own the rows |
| Resolve per bar in the fold | Simpler call site | Breaks determinism and performance; the fold must not do network I/O |

## Consequences

Positive:

- `run submit` refuses a missing or mismatched v5 reference before a run exists;
  `run lineage` has a producer and an immutable, ordered set.
- The worker resolves once and the fold stays local and deterministic; v3/v4
  keep their existing loader and output.
- No new database, no per-bar network call, and no payload in lineage.

Negative: `ConfigBundle` gains a field, the data service a v5 parse path, and
`RunStore::submit` shares one transaction with lineage. The worker needs content
reads, and a v5 model artifact depends on a succeeded model stage.

## Invariants

- A lineage row carries a kind, id, hash and optional pack provenance; never a body.
- The same five references produce one run, one lineage set and one digest.
- A later bar cannot change an earlier signal or lineage row.
- A missing or mismatched reference produces no run row and no queue message; a
  succeeded run has all of its bytes, and a failed load publishes no artifact set.
- The signal path reads no statistic and no wall clock; v3/v4 keep their hashes,
  command meanings and output.

## Verification

Fast tests cover reference parsing, pack/signal/model hash mismatch, atomic
run/lineage submit, same-set idempotence, different-set refusal, v5 load with
`MemoryReferences`, v4 regression, and no partial publish. The composed tier
submits a v5 run twice and compares the canonical digest. A worker test proves
one resolution before the fold, not per bar; the plan's verification table decides
completion, and this ADR claims no implementation.
