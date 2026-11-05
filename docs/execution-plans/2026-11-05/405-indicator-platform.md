# Indicator Completeness and Model-Spec Platform — Execution Plan

- Date: 2026-11-05 (UTC)
- Status: **Approved** (human, 2026-11-05); implementation in progress
- Epic: [#405](https://github.com/quant-rule/quant-rule/issues/405)
- Coverage: [`docs/model/2026-11-05/indicator-coverage.md`](../../model/2026-11-05/indicator-coverage.md)
- Verification checklist: [`docs/improvement/2026-11-05/INDICATOR-VERIFICATION.md`](../../improvement/2026-11-05/INDICATOR-VERIFICATION.md)
- Claim type: platform and indicator programme; no edge claim

## 1. Intent

Make the model information-complete for OHLCV and configurable enough to run a
v6–v10 model swarm. The current model is a fixed Rust event set whose readings
are mostly price-momentum and trend transforms. This programme adds the missing
OHLCV dimensions — volume-weighted price, volume by price, participation,
volume-price flow, volatility structure, structure, candle anatomy, path,
cross-asset, and calendar — with a math formula for every reading and every
event, a registry entry, and tests.

## 2. Current state

- Model v4: EMA, RSI, ATR, RVOL, MACD, ADX/DI, pivots, compression, progression.
- Event sources 1–18; signal subtypes 1–9.
- The path gate tested the v4 event layer at 5m/15m/1h and 1h/100 bars: no edge.
- The research platform and swarm runbook exist; the model is not spec-driven.
- No predictive model baseline; no strategy baseline.

## 3. Scope

Included:

- Registry schema extension: every event carries a `formula`.
- Indicator/source/subtype allocation for the 36 new indicators.
- Extended readings and events, additive to the v4 batch.
- The first batch: `vwap` and `volume_profile` with level, divergence, and
  signal events.
- Spec engine and swarm integration in later phases.
- A read-only feature audit before the full primitive backlog is committed.

Explicit non-goals:

- No strategy work before an accepted predictive model baseline.
- No source repair or dataset change as a prerequisite.
- No absolute-volume rules; ratio/profile features only.
- No new dependency without approval.
- No change to the v4 event domain or its accepted baselines.

## 4. Assumptions and uncertainty

- Volume ratio/profile features are admissible despite source unit changes;
  absolute-volume rules are not.
- The v4 event batch remains the legacy report/dashboard/analytics domain; the
  extended batch is additive and published in the event CSV.
- A bounded expression tree can express the v6–v10 logic without a general
  purpose language.
- The feature audit may conclude that a dimension has no edge; that is a valid
  result.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Spec schema | `quantrule.model-spec/v1` | Approved |
| Evaluator | Rust evaluator, shell parses JSON | Approved |
| Event formula | Required on every registry event | Approved |
| Legacy boundary | Extended events separate from `TimeframeModel::events()` | Approved |
| Volume | Ratio/profile features; no absolute volume | Approved |
| Correction | One Bonferroni over all primary cells | Approved |
| Strategy | Not opened in this epic | Approved |

## 6. Acceptance criteria

- Every indicator in the coverage table has a registry entry, a reading formula,
  event formulas, and a test plan.
- Every implemented reading passes formula, causality, determinism, golden, and
  edge-case tests.
- Every implemented event passes availability, no-double-count, and precedence
  tests.
- The event CSV publishes both the v4 and extended batches.
- The legacy report, dashboard, and analytics fixtures remain valid.
- The spec engine runs a model spec end-to-end.
- The v6–v10 swarm runs and reports every cell.
- A model baseline is accepted or `no edge` is recorded.

## 7. Work breakdown

| Phase | Issue | Work | Status |
|---|---|---|---|
| 0 | [#411](https://github.com/quant-rule/quant-rule/issues/411), [#412](https://github.com/quant-rule/quant-rule/issues/412) | Registry schema, coverage, codes | Done |
| 1 | [#413](https://github.com/quant-rule/quant-rule/issues/413) | `vwap` and `volume_profile` readings/events | Done |
| 2 | [#414](https://github.com/quant-rule/quant-rule/issues/414) | Participation readings | Done |
| 3 | [#415](https://github.com/quant-rule/quant-rule/issues/415) | Volume-price flow readings | Done |
| 4 | [#416](https://github.com/quant-rule/quant-rule/issues/416) | Volatility and structure readings | Done |
| 5 | [#417](https://github.com/quant-rule/quant-rule/issues/417) | Anatomy and path readings | Todo |
| 6 | [#418](https://github.com/quant-rule/quant-rule/issues/418) | Cross-asset and calendar readings | Todo |
| 7 | [#419](https://github.com/quant-rule/quant-rule/issues/419)–[#422](https://github.com/quant-rule/quant-rule/issues/422) | Event layer | Todo |
| 8 | [#423](https://github.com/quant-rule/quant-rule/issues/423)–[#425](https://github.com/quant-rule/quant-rule/issues/425) | Spec engine | Todo |
| 9 | [#426](https://github.com/quant-rule/quant-rule/issues/426)–[#429](https://github.com/quant-rule/quant-rule/issues/429) | v6–v10 charters, swarm, validation | Todo |
| 10 | [#430](https://github.com/quant-rule/quant-rule/issues/430) | Closeout | Todo |

## 8. First-batch implementation

| Item | Path |
|---|---|
| Extended readings | `crates/quantrule-model/src/readings/extended.rs` |
| Extended events | `crates/quantrule-model/src/events/extended.rs` |
| Model wiring | `crates/quantrule-model/src/timeframe.rs` |
| CSV publication | `crates/quantrule-shell/src/command/report_run/model_artifacts.rs` |
| Registry | `docs/registry/indicators.json`, `docs/registry/schema.json` |
| Registry check | `tools/check_registry.py` |

The extended batch is **additive**: `TimeframeModel::events()` remains the v4
domain, so the legacy report, dashboard, analytics, and store fixtures stay
stable. `TimeframeModel::extended_events()` carries the new events, and the
shell's `events.csv` publishes both.

## 9. Verification

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 tools/check_registry.py
python3 tools/check_links.py
python3 tools/check_terms.py
git diff --check
```

First-batch tests:

```bash
cargo test -p quantrule-model extended
cargo test -p quantrule-analytics
cargo test -p quantrule-shell
```

## 10. Risks and recovery

| Risk | Mitigation |
|---|---|
| Extended events pollute legacy reports | Separate `extended_events()` batch; CSV publishes both |
| Registry codes collide | Unique `(type, subtype, source)`; CI check |
| Volume unit shifts | Ratio/profile features; no absolute volume |
| Golden fixtures change | Only intentional array-length changes; reviewed |
| DSL creep | Bounded grammar; no statistics or fitted coefficients |
| Overfitting | Finite pre-registered matrix; one correction; one-shot validation |

Recovery: the extended batch is additive; a failed indicator can be reverted
without touching the v4 model or its accepted baselines.

## 11. Stop conditions

- A reading or event fails causality, determinism, golden, or edge tests.
- The feature audit finds no candidate above the screen.
- A v6–v10 premise is a variant of a failed hypothesis.
- No cell passes dev; validation fails; human stops.

## 12. Approval

- Approved by: human, 2026-11-05
- Scope: the registry, the 36-indicator coverage, the first batch, and the
  platform phases
- Not authorized: strategy work, live capital, a new dependency, or a change to
  the v4 event domain
