# AI Transparency Log — Model v4

- Date: 2026-11-04 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-04/369-model-v4.md`](../../execution-plans/2026-11-04/369-model-v4.md)

## Intent and boundary

Implement the approved v4 model contract and the pre-strategy validation
program. The work is model-only: no strategy, portfolio, execution, or cost
claim is made. No edge is claimed until the model path validation passes both
partitions.

## Work performed

### Implementation

- Added `quantrule.model/v4` parsing for level references and ATR thresholds,
  divergence scale and window, RSI levels, EMA pairs, MACD families, DI cross,
  trend-continuation thresholds, and progression targets, stability,
  invalidation, and minimum target.
- Added the 1h ceiling to the model and run validation; `4h` and above are
  rejected.
- Added the `quantrule.model-result/v4` result with `effective_config`,
  `timeframes`, and `charts`; the result carries no axes, cross-timeframe
  aggregate, prediction, or confidence.
- Made the result's `registry_version` read the model crate constant instead of
  a literal, and added a test that the constant matches
  `docs/registry/indicators.json`.

### Validation program

- Wrote the pre-registered path protocol
  [`docs/model/2026-11-04/path-validation-v4.md`](../../model/2026-11-04/path-validation-v4.md)
  before the development matrix was run.
- Rebuilt `tools/improvement/path_validation.py` to add one-vote-per-bar
  preparation, bundle-separated clusters, state-conditioned bases, random,
  back-lag, and shuffled controls, Bonferroni lift bounds, replication, and a
  single-candidate selection rule.
- Added the 30-run development matrix (3 instruments × 3 years × 2 ADX settings)
  and the 12-run validation matrix under `config/model-v4/`.
- Added unit tests for the barrier, selection, cluster, state, control, and
  matrix-expansion paths.

## Process note

Two smoke tests were run to check that the generated run configs resolve their
sources and engine files. The ETHUSD 2018 development run is part of the
development matrix. One XAUUSD 2021 validation bundle was produced by a config
smoke test during setup, before the development matrix. Its output was not
inspected, no path result was computed from it, and the bundle was deleted
before the development matrix finished. The validation decision is still treated as
one-shot: the harness is opened on validation only after the candidate is
frozen. This is recorded because the protocol says the validation partition is
opened once.

## Result

The pre-registered development gate (BTCUSD, ETHUSD, XAUUSD; 2018–2020;
5m/15m/1h) produced 12 primary family-timeframe cells, all grade C. No family
passed the matched-base lift, the state-conditioned incremental lift, the three
controls, and two-instrument replication together. The strongest cell, H1 1h,
had a +2.80pp unconditional lift but a −0.0214 corrected lower bound, a −0.0136
state-conditioned lift, and a +0.1448 back-lag placebo. The validation partition
was not opened, and no baseline was accepted.

Two intermediate checks matter for the record. An earlier one-month BTCUSD
2018-01 pilot exercised the harness before the protocol was written; it is
exploratory and is not a decision input. The 2018-only three-instrument subset
was run after the protocol and showed positive 5m point lifts for H2, H3, and
H4; the full 2018–2020 matrix reversed them. The subset was not used to select a
family, but it shows why a single year is not the decision unit. The full result
is the one in the run record.

## Verification

- `cargo test --workspace` passed after the v4 implementation.
- `cargo fmt --all -- --check` passed.
- `cargo clippy --all-targets -- -D warnings` passed after the implementation.
- `cargo doc --workspace --no-deps` passed without warnings.
- `python3 -m unittest discover -s tools/improvement` passed: 85 tests,
  including 21 path-validation tests.
- The development matrix and the path result are recorded in
  [`docs/improvement/2026-11-04/`](../../improvement/2026-11-04/); the final
  numbers are written after they were observed.
- `python3 tools/improvement/run.py ... --command path-validation` exited 0 and
  wrote the run folder; the index shows `no edge`.
- A BTCUSD 2018 `adx25` bundle was regenerated after the parser split and
  compared byte for byte with the matrix bundle: `events.csv`, `readings.csv`,
  and `result.json` are identical.
- The committed `docs/improvement/index.md` was updated by appending the v4 row
  to the retained v3 history, because the local run store no longer contains the
  pre-reset v3 runs. The generated `data/improvement/index.md` is the working
  view for the current store.

## Learning

- The earlier path harness computed a lower bound on the hit rate but not on the
  lift over the matched base. The v4 harness reports both, and the protocol
  gates on the lift bound and the state-conditioned lift.
- The first v4 harness aggregated event clusters across one-year bundles. The
  corrected version forms clusters inside each bundle, because bar indices
  restart at zero each year.
- The ADX setting is a sensitivity check on the trend-continuation family, not
  a second hypothesis. For the other three families it must leave the event set
  unchanged; the harness checks that as an isolation condition.
