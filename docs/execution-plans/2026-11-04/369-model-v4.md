# Execution Plan: Model v4 and Pre-Strategy Validation

- Date: 2026-11-04 (UTC)
- Status: **Verified — no edge; validation gate not reached**
- Epic: [#369](https://github.com/quant-rule/quant-rule/issues/369)
- Contract: [`docs/model/2026-11-04/model-v4.md`](../../model/2026-11-04/model-v4.md)
- Protocol: [`docs/model/2026-11-04/path-validation-v4.md`](../../model/2026-11-04/path-validation-v4.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/369-model-v4.md`](../../ai-transparency-log/2026-11-04/369-model-v4.md)
- Claim type: model contract, implementation, and validation evidence; no
  strategy or edge claim.

## 1. Intent

Make the model a v4 contract with configurable events and progression, a 1h
timeframe ceiling, a flat result with chart-ready data, and a claim-specific
path-validation harness. Find a validated model event or record no edge before
any strategy work.

## 2. Scope

Included:

- v4 config and result contracts.
- Configurable level, divergence, signal, trend-continuation, and progression
  parameters.
- v4 result with charts, no axes, no prediction, and no confidence.
- Timeframe ceiling: 1m, 5m, 15m, 1h; 4h rejected.
- Path-validation harness with matched baselines, controls, and correction.
- Four hypotheses, two ADX settings (eight config-hypothesis pairs), 5m/15m/1h,
  three instruments.
- Development matrix and one-shot validation.

Excluded:

- Strategy and portfolio changes.
- 4h or higher timeframes.
- Prediction or confidence fields.
- Axis removal from historical v3 reports.
- Post-hoc threshold changes.

## 3. Acceptance criteria

| Criterion | Result |
|---|---|
| v4 config parses and rejects 4h, unknown fields, and pivot-window overrides | Met |
| v4 result has readings, states, levels, events, progression, context, coverage, and charts | Met |
| v4 result has no axes, cross-timeframe aggregate, prediction, or confidence | Met |
| Every active indicator/event has a validation claim | Met |
| The harness uses path outcomes, matched baselines, controls, and correction | Met |
| The eight-config development matrix is recorded | Met: 4 families × 2 ADX settings, 9 one-year cells |
| One candidate is frozen and validated once | Not applicable: no development candidate passed; validation not opened |
| A model baseline or no edge is recorded | Met: no edge |
| No strategy or portfolio run is performed | Met |

## 4. Work breakdown

| Phase | Work | Paths | Result |
|---|---|---|---|
| 0 | v4 contract and ADR | `docs/model/2026-11-04/` | Contract and protocol written |
| 1 | v4 config | `crates/quantrule-model/src/input.rs`, events, progression, shell config | Implemented |
| 2 | v4 result and charts | `crates/quantrule-model/src/result.rs`, shell report | Implemented |
| 3 | registry updates | `docs/registry/` | Updated; version `2026-11-04.1` |
| 4 | path harness | `tools/improvement/` | Implemented with controls and state bases |
| 5 | hypothesis matrix and runs | `config/model-v4/`, `data/model-v4/` | Dev matrix run; validation not opened |
| 6 | validation and decision | `docs/model/`, `docs/improvement/` | No edge recorded |

## 5. Verification

Commands and observed results:

- `cargo fmt --all -- --check`: passed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo test --workspace`: passed.
- `cargo doc --workspace --no-deps`: passed.
- `python3 -m unittest discover -s tools/improvement`: passed.
- `python3 tools/check_registry.py`: 10 indicators, 16 events, 61 parameters.
- `python3 tools/check_validation.py`: 25 claims for 10 indicators and 16 events.
- `python3 tools/improvement/validate.py` on the dev manifest: ok.
- `python3 tools/improvement/run.py ... --command path-validation`: exit 0;
  the decision is no edge.
- `python3 tools/check_links.py`, `check_terms.py`: passed.
- Repro check: a regenerated BTCUSD 2018 `adx25` bundle is byte-identical to the
  matrix bundle for `events.csv`, `readings.csv`, and `result.json`.

Evidence:

- [`docs/improvement/2026-11-04/MODEL-V4-GATE.md`](../../improvement/2026-11-04/MODEL-V4-GATE.md)
- [`MODEL-2026-11-04-01`](../../improvement/2026-11-04/MODEL-2026-11-04-01-model-v4-dev.md)
- Local: `data/improvement/2026-11-04/MODEL-2026-11-04-01/`

## 6. Stop conditions

- No edge: reached. The validation partition is not opened and no strategy or
  portfolio work follows.
- Insufficient sample: not reached; every primary cell has at least 916 events
  and 629 clusters.
- Partition breach: none found; one validation config smoke test is disclosed in
  the AI transparency log and produced no evaluation result.
- Open critical finding: none known.
- Decision outside this plan: none.
