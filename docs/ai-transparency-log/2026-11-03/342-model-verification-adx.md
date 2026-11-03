# AI Transparency Log — Model Verification, v3 Config, ADX/DI

- Date: 2026-11-03 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Plan: [`docs/execution-plans/2026-11-03/342-model-verification-adx.md`](../../execution-plans/2026-11-03/342-model-verification-adx.md)

## Intent and boundary

The human asked for a model verification system that separates calculation
correctness from predictive validity, adds only EMA, RSI, MACD, ATR, ADX, and
RVOL, and verifies every event before strategy work. Strategy and portfolio work
were explicitly excluded.

## Proposed work

- Registry-driven verification matrix.
- Model v3 config.
- ADX/DI reading and event.
- Claim-validation harness updates.
- Board reconciliation.

## Verification

Recorded after implementation. No test result is claimed before it is observed.

## Learning

The prior model gate tested fixed-horizon signs, not the claim each event names.
A test that checks `ADX >= 25` by checking `ADX >= 25` is self-confirming; the
valid tests use synthetic regimes for construct validity and forward paths for
predictive validity. Predictive failure is not a code defect and must not be
"fixed" by changing thresholds after seeing the result.

## Implemented

- Added the ADX/DI reading family with Wilder smoothing, `adx`, `di_plus`,
  `di_minus`, `adx_regime`, `adx_slope`, and `di_state`.
- Added `signal.di_cross` and `signal.trend_continuation` event definitions,
  registry entries, report/CSV fields, and shell labels.
- Added `quantrule.model/v3` defaults, per-timeframe overrides, ADX parameters,
  active flags, and a `structure.scale` compatibility field.
- Added `docs/registry/validation.json` and `tools/check_validation.py`.
- Made `claim_validation.py` enumerate active registry events, source-qualified
  for Signal events, and grade the DI state.
- Ran a BTCUSD 2018-01 pilot with ADX threshold 25 and 20. Both returned
  `no_edge`; the result is recorded in `docs/model/2026-11-03/adx-verification.md`.

## Verification observed

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: pass.
- `cargo doc --workspace --no-deps`: pass.
- `python3 tools/check_registry.py`: 10 indicators, 16 events, 55 parameters.
- `python3 tools/check_validation.py`: 25 claims for 10 indicators and 16 events.
- `python3 tools/check_links.py`: 390 Markdown files, no broken links.
- `python3 tools/check_terms.py`: 243 files, no banned terms.
- `python3 -m unittest discover -s tools/improvement`: 64 tests pass.
- Claim pilot: BTCUSD 2018-01, 5m/15m/1h, gate `no_edge` for both ADX 25 and
  ADX 20.

## Learning

The pilot confirms the calculation path and the registry-driven event
enumeration. It does not confirm a predictive edge. The ADX threshold changes
the `trend_continuation` sample and sign across timeframes, so a threshold must
not be selected from this result; the full development matrix must be
pre-registered.
