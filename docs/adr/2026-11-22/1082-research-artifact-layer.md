# ADR-RA: Research Artifact Layer and Seventh Strategy Service

- Date: 2026-11-22 (UTC)
- Status: Accepted
- Issue: [#1082](https://github.com/quant-rule/quant-rule/issues/1082)
- Decision owner: Human
- Related: [ADR-HA](1082-ha-deployment-profile.md), [research artifacts](../../architecture/research-artifacts.md)

## Context

The strategy is human-owned and deterministic, but today it is embedded in a run
configuration. The same rule cannot be frozen, validated, reused or cited as a
first-class research object. Portfolio policy and execution assumptions are also
run-specific, so comparing the same alpha under several policies is opaque.

The model already produces a reusable content-addressed artifact. The platform
therefore needs a matching artifact layer for alpha and its simulation inputs,
without splitting the per-bar strategy/execution/portfolio fold.

QuantRule never trades. "Execution" and "portfolio" in this decision mean
historical simulation, not broker connectivity or live accounting.

## Decision

1. Add four RA objects, with the model artifact as the upstream fifth:
   - model artifact (existing, produced by `model`);
   - strategy pack;
   - signal artifact;
   - portfolio policy pack;
   - execution-simulation pack.
2. A pack is an immutable, versioned, content-addressed definition. A signal
   artifact is a computed, content-addressed output of a model artifact and a
   signal-mode strategy pack.
3. Add a stateless seventh `strategy` service that validates signal-mode packs,
   computes signal artifacts, writes them to the object store and records lineage.
   It has no database, no queue and no per-bar network call.
4. Keep the strategy/execution/portfolio fold inside `backtest`. Fold-mode packs
   are not executed by the strategy service.
5. Signal mode uses a deterministic reference position. Giveback remains
   fold-mode unless candles become an explicit signal-service input.
6. `quantrule.run/v5` references the model artifact, packs and signal by ID and
   hash. `RunId` keeps its current formula; hashes enter through `config_hash` and
   provenance. v3 and v4 configs remain readable.
7. Store pack definitions and lifecycle in the catalog database, pack bytes and
   signal artifacts in the object store, and lineage in evidence. No fifth
   database.
8. Freeze is a human decision supported by an independent Quant Validator record.
   A frozen pack never changes; a change is a new pack.
9. Keep the browser and CLI contract frozen. Pack and signal management is
   additive; the 49 existing commands and screens remain compatible.
10. Keep alpha and edge separate. Statistics live only in the research and
    qualification layer and never enter a pack, signal or strategy decision.

## Alternatives considered

| Alternative | Consequence | Rejected because |
|---|---|---|
| Keep strategy only in run config | Simple | No reusable alpha, no stable lineage, opaque comparisons |
| Split strategy, execution and portfolio into three services | Independent scaling | Creates a distributed transaction per bar and breaks determinism |
| Put the strategy service in front of the fold | One service for all modes | Position-aware decisions would require a network round trip per bar |
| Store packs in a new database | Clean ownership | Adds a fifth stateful system without a scaling need |
| Make signal mode identical to fold mode | No semantic split | Signal artifacts cannot be reused without the actual position and fills |
| Add live execution | Product expansion | Contradicts the research-only mission |

## Consequences

Positive:

- Alpha becomes a frozen, reusable, auditable object.
- The same signal can be evaluated under several portfolio and execution packs.
- The fold remains deterministic and local to one backtest worker.
- Run identity and provenance name every pack and artifact hash.
- The client contract remains compatible.

Negative:

- Run v5 and pack lifecycle add migration and documentation work.
- Signal mode and fold mode have different position semantics and must be
  described explicitly.
- The strategy service adds a seventh deployment unit and a new service boundary.
- Pack and signal storage needs retention and garbage collection.

## Invariants

- No statistical or fitted field enters the signal path.
- A later bar cannot change an earlier signal.
- Same model artifact plus pack plus schema produces the same signal artifact.
- A frozen pack is immutable.
- A run references content hashes, not payloads.
- A succeeded run has all of its bytes; no partial publish.
- The platform never trades.

## Verification

Schema, causality, determinism, golden, reuse, run-identity, lineage, freeze and
client-parity tests are defined in the RA verification matrix. No implementation
is claimed by this ADR.
