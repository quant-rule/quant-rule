# ADR-STR-003: Keep Signal Computation in `quantrule-app::signal`

- Date: 2026-11-26 (UTC)
- Status: Accepted (2026-11-26 UTC)
- Issue: [#1229](https://github.com/quant-rule/quant-rule/issues/1229)
- Decision owner: Human
- Approval: human, D4 in [REM-000](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md), 2026-11-26 (UTC)
- Related: [RA-008 strategy service](../../execution-plans/2026-11-22/1096-strategy-service.md),
  [ADR-RA-2](../2026-11-26/1204-run-v5-resolution.md),
  [research artifacts](../../architecture/research-artifacts.md),
  [backtest boundaries](../2026-10-27/analysis-and-backtest-boundaries.md)
- Claim type: ADR (Architecture Decision)

## Context

The signal artifact and the deterministic signal-mode reference position live in
`quantrule-app::signal`. The module is pure: it reads a published
`ModelBundle` and a frozen `StrategyPack`, calls the existing strategy engine,
and returns content-addressed bytes. It performs no I/O, reads no clock, and
uses no randomness. `quantrule-strategy-service` is a stateless adapter over
that API.

The module is not an isolated leaf. It uses `quantrule-app` pack parsing,
`config::StrategyConfig`, `stages::strategy::{BarRow, ModelBundle, validate_context}`,
and the `quantrule-backtest` intent, side, and strategy types. The application
crate already depends on `quantrule-api`, `quantrule-data`, `quantrule-model`,
and `quantrule-backtest`.

The original 65-ID change map anticipated a separate strategy crate. The
approved remediation decision D4 keeps the current module instead, and this ADR
records the deviation: the original change map is not reconstructed, because it
is not the plan of record; the deviation is the dropped extraction.

There is one consumer, the strategy service, and no independent library user. An
extraction would move pack, config, and strategy-bundle code with it, or make the
new crate depend on `quantrule-app` and invert the boundary. Either adds
manifests and edges without removing a real coupling.

## Decision

1. **Keep `quantrule-app::signal` as the single signal-computation home.** Do not
   create `crates/quantrule-strategy` in v1.6.1.
2. **Keep the strategy service a stateless adapter.** It resolves one frozen
   model bundle and one frozen signal-mode pack per request, calls `compute`
   once, writes canonical bytes, and registers metadata. It owns no database,
   queue, run, research, or execution state.
3. **Keep the dependency direction.** The signal path reads published model
   values and backtest decision types; it never reads strategy-service state,
   the filesystem, the clock, or a service.
4. **Record the deviation.** `REM-STR-004` amends the plan and architecture to
   state that no `quantrule-strategy` crate exists and that the original 65-ID
   extraction is dropped. No command, schema, artifact hash, or byte changes.
5. **Revisit only on a real second consumer.** A later extraction needs its own
   ADR, an equivalence plan, and the same golden, causality, and determinism
   tests. Convenience is not a reason to split.

Dependency graph:

```text
quantrule-api  quantrule-data  quantrule-model  quantrule-backtest
        ^              ^              ^                 ^
        └──────────────┴──────────────┴─────────────────┘
                       quantrule-app (pack, config, stages, signal)
                              ^
                 quantrule-strategy-service (stateless HTTP adapter)
```

## Alternatives considered

| Alternative | Consequence | Rejected because |
|---|---|---|
| Extract `crates/quantrule-strategy` | A named boundary and independent tests | It must move pack, config, and strategy-bundle types or depend back on `quantrule-app`; no independent consumer exists |
| Move the signal module into `quantrule-backtest` | One strategy engine home | It conflates alpha output with fills and P&L; the signal artifact is not an execution result |
| Duplicate the computation in the service | Service owns its bytes | Two definitions drift; the artifact and the tests would no longer have one home |
| Keep `quantrule-app::signal` | No new crate or edge; existing tests and hashes stay | Chosen: it matches the current module map and the one-consumer reality |

## Consequences

- **Positive:** No new crate, manifest, dependency edge, or release surface;
  the signal artifact, reference position, and their tests stay together.
- **Positive:** The service remains thin, and no command or schema changes.
- **Negative:** `quantrule-app` remains a larger crate; the module's purity is
  enforced by tests and module docs rather than by a crate boundary.
- **Negative:** A future extraction is a real refactor, not a rename; it must
  preserve bytes and prove equivalence.

## Invariants

- The same model artifact, pack, and schema produce the same signal bytes.
- A later bar cannot change an earlier row; the module reads no future value.
- Fold-mode packs are refused; the signal path never reads execution or
  portfolio policy.
- The service makes one pack and one model resolution per request, never one per
  bar.
- No database, queue, run, research, clock, or random source enters the signal
  path.

## Verification

`cargo test -p quantrule-app --lib signal::` covers byte determinism, causality,
golden rows, fold rejection, mismatched sources, short series, and edge cases.
`cargo test -p quantrule-app --test signal_artifact_public` covers the public
artifact surface. `cargo test -p quantrule-strategy-service` covers the adapter,
duplicates, concurrency, boundary imports, and the one-resolution rule.
`cargo metadata` shows no `quantrule-strategy` package, and
`qr-governance check --all` enforces the caps. No live run is required; this ADR
changes no behavior and claims no edge.
