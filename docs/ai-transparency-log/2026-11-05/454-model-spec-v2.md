# AI Transparency Log — Model Spec v2 (#454)

- Date: 2026-11-05 (UTC)
- Tool: OpenCode agent session; model DeepSeek V4.1 Flash
- Work item: #454 P1 model spec v2
- Parent plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Contract: [`docs/model/2026-11-05/model-spec-v2.md`](../../model/2026-11-05/model-spec-v2.md)
- Status: P1 implemented and tested in the `feat/454-model-spec-v2` worktree;
  commit and pull request follow this record

## 1. Intent and boundary

Implement `quantrule.model-spec/v2` so an M2.x cell can configure reading
windows, activate a finite event set, define a causal directional vote, and use
optional session and panel context. The work stays in the model and shell:
no strategy, execution, portfolio, analytics, or live behavior changes.

The model owns the resolved contract, the combined event view, the condition
evaluator, and the vote. The shell owns JSON parsing, panel source loading, and
artifact writing. Event thresholds remain fixed; only reading windows and event
activation are configurable.

## 2. Implemented work

| Item | Detail |
|---|---|
| `crates/quantrule-model/src/spec/v2.rs` | v2 types, validation, combined event view, condition evaluator, vote, `SpecRun`, edge tests |
| `crates/quantrule-model/src/input.rs` | `MarketStateConfig` carries the `ExtendedConfig`; default-preserving |
| `crates/quantrule-model/src/timeframe.rs` | `TimeframeModel` reads the config extended settings and exposes `candles()` |
| `crates/quantrule-shell/src/spec.rs` | Shared registry role resolution and `schema_of`; v1 helpers exposed to v2 |
| `crates/quantrule-shell/src/spec/v2.rs` | v2 JSON parser, reading overrides, prediction, panel sources, session, conditions |
| `crates/quantrule-shell/src/command/report_run.rs` | v1/v2 dispatch, panel loading, v2 provenance, spec file hash |
| `crates/quantrule-shell/src/command/report_run/model_artifacts.rs` | `prediction.csv`, active-event `events.csv` |
| `docs/model/2026-11-05/model-spec-v2.md` | Contract and verification |
| `crates/quantrule-shell/tests/fixtures/config-run/spec-v2.json` | End-to-end fixture |

The v1 contract and parser are unchanged. The v2 parser rejects unknown fields,
two timeframes, a context/diagnostic vote, an inactive prediction event, and a
panel that repeats the evaluated instrument.

## 3. Verification and evidence

Observed on 2026-11-05 in the `feat/454-model-spec-v2` worktree:

| Command | Observed result |
|---|---|
| `cargo test -p quantrule-model --lib spec::v2` | 7 passed |
| `cargo test -p quantrule-shell --bin quantrule spec::v2` | 5 passed |
| `cargo test -p quantrule-shell --test report_command` | 13 passed |
| `cargo test --workspace --quiet` | exit 0; every reported suite passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `cargo doc --workspace --no-deps` | exit 0 |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_links.py` | 532 files, no broken links |
| `python3 tools/check_terms.py` | 338 files, no banned terms |

The end-to-end test runs `report model --spec` twice and compares
`prediction.csv`, `events.csv`, `conditions.csv`, and `provenance.json` byte for
byte. It checks the prediction values are `-1`, `0`, or `1`, the provenance
names the v2 schema/family/panel/session, and the vote lists contain only signal
events.

## 4. Learning

- `TimeframeModel::compute` hardcoded `ExtendedConfig::DEFAULT`. The override
  could not reach the reading until the config was carried on
  `MarketStateConfig`; the default path is unchanged.
- The v1 condition evaluator reads only the base and extended batches. A v2
  condition on a session or panel event would silently fail, so v2 has its own
  evaluator over the combined event set.
- Panel sources need an explicit owner. The evaluated instrument's candles come
  from the run config; repeating its path in `panel.sources` is rejected. The
  leader must be named so the lead-lag order is deterministic.
- The spec file hash uses the same FNV-1a fingerprint as the run config, not a
  new dependency.
- A context event in a vote list is a role error, not a weight of zero. The
  parser rejects it instead of silently ignoring it.

## 5. Rejected or deferred

| Suggestion | Decision | Reason |
|---|---|---|
| Refactor the v1 evaluator into a shared generic evaluator | Rejected for P1 | It would widen the change and risk v1 behavior; v2 has a small single-timeframe evaluator |
| Allow a score prediction | Deferred | The approved primary prediction is a directional vote; a score is exploratory |
| Add cross-timeframe support | Deferred | The M2 programme is single-timeframe first |
| Add a new hashing dependency | Rejected | The existing FNV-1a fingerprint is sufficient for provenance |
| Put panel/session logic in the shell | Rejected | It is model computation; the shell only loads sources |
| Update `spec_preflight.py` to v2 in P1 | Deferred | The M2 charter/batch phase owns the v2 preflight; the v2 contract records the limit |
| Treat `calendar_bucket` as a spec event | Rejected | It is a reading with no event; the area cell exercises the session readings |

## 6. Boundary

This log does not claim:

- that the M2 development swarm has run;
- that `model_validation` was opened;
- that any model has an edge;
- that the `Sign` column is filled;
- that the v2 preflight exists.

The next gate is the human's authorization to commit, push, and open the P1 pull
request.
