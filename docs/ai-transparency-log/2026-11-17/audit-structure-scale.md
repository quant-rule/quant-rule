# AI transparency log — `structure.scale` plumbing (P2.5, AUD-48)

- Date: 2026-11-17 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: [Audit Workflow](../../workflows/audit-workflow.md) stage 3
  (Execution) for `AUD-2026-11-17-48`.
- Claim type: Behaviour Claim.
- Intent: make `structure.scale` select the confirmed pivot scale used by the
  structure readings and the lower-high/lower-low sequence events. The field was
  parsed and serialized but every structure computation hard-coded
  `PivotScale::Major`.
- Boundary: branch `fix/audit-structure-scale`, based on current `main`
  `7ae3a984`. No push, PR, merge, or board change. The Quant Validator pass is
  sequential by the same agent, not independent.

## What changed

- `ExtendedReadings::compute` now delegates to a new
  `compute_with_scale(candles, readings, config, structure_scale)`, keeping
  `PivotScale::Major` as the default. `structure_series` reads
  `confirmed_at(structure_scale)` and filters touched levels by the same scale.
- `ExtendedEventConfig` carries a `structure_scale` field (default `Major`),
  with `with_structure_scale` and `structure_scale()`; the sequence events read
  the configured scale instead of `PivotScale::Major`.
- `TimeframeModel::compute` passes `MarketStateConfig::structure_scale` to the
  readings and the extended event config, so the v4 `structure.scale` value now
  reaches the computation.
- Default configs are unchanged: `MarketStateConfig::new(Major)` and every
  existing `ExtendedConfig::DEFAULT` path keep the major scale and the same
  golden output.

## Verification observed

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --lib structure_scale_selects` | 1 passed, 0 failed |
| `cargo test -p quantrule-model --test structure_scale` | 2 passed, 0 failed |
| `cargo test -p quantrule-app --lib config::tests::model_v4_reads_the_structure_scale` | 1 passed, 0 failed |
| `cargo test -p quantrule-model --lib --tests` | all suites pass |
| `cargo test -p quantrule-app --lib config::tests` | 27 passed, 0 failed |
| `cargo fmt --all -- --check` | pass |
| `tools/gate.sh` | pass: 2,325 passed, 0 failed, 16 ignored; 25 checkers |

## Quant validation (sequential, not independent)

- **Expected invariant:** a config field that selects a documented behavior
  changes that behavior (`docs/registry/README.md`). The model contract and the
  registry say the structure scale selects the confirmed pivots the structure
  output reads (`docs/model/2026-11-05/model-m1.0.md:32-34`;
  `docs/registry/indicators.md:228`).
- **Observed before:** `MarketStateConfig::structure_scale` was written by the
  parser but no computation read it; `structure.rs` and
  `events/extended/structure.rs` used `PivotScale::Major`.
- **Observed after:** the unit test sets a 10.0 major high and a 3.0 minor high
  in the same series. With the major scale, `last_pivot_high` is `None` at bar 7
  and `Some(10.0)` at bar 11; with the minor scale it is `Some(10.0)` at bar 7
  and `Some(3.0)` at bar 11. The minor event series emits lower-high sequence
  events at bars 14 and 17 (counts 2 and 3); the major series emits none.
- **Default output:** `ExtendedReadings::compute` and
  `ExtendedEventConfig::new` still default to `Major`, and no example config or
  accepted baseline sets `structure.scale: minor`. The model and event goldens
  are unchanged.
- **Registry:** the existing `model.structure.scale` row already names the
  behavior; no registry edit is needed, and the gate's registry checker runs.
- **Residual risk:** a `minor`-scale config now changes the published
  `last_pivot_*`, sequence counts, structure state, and level touches, and the
  lower-high/lower-low events. That is the intended contract; no accepted
  baseline uses the field.
- **Independence:** the Quant Researcher/Developer/Validator roles were
  performed sequentially by the same agent. This is not an independent
  validation and is not claimed as one.

## Learning

- The config field was wired to parsing and serialization but not to the
  computation. A test that only parses the field would have passed while the
  behavior stayed absent; the test now compares the published readings and
  events across scales.
- The pivot detector needs `index >= lookback` and confirms after `lookback`
  more bars. The fixture therefore places the first minor pivot at bar 2 or
  later; an earlier pivot cannot exist, which is why the first test shape had to
  be rebuilt.
- The sequence event emits on every extension of the count at or above two, not
  only on the first extension; the test asserts both the bar-14 and bar-17
  events rather than assuming one.
