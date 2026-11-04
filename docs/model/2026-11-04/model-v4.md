# Model v4 — Flat Result, Configurable Events, and 1h Ceiling

- Date: 2026-11-04 (UTC)
- Status: **Implemented; path validation no edge**
- Epic: [#369](https://github.com/quant-rule/quant-rule/issues/369)
- Plan: [`docs/execution-plans/2026-11-04/369-model-v4.md`](../../execution-plans/2026-11-04/369-model-v4.md)
- Claim type: model contract; no strategy or edge claim.

## 1. Config

The config schema is `quantrule.model/v4`. It carries defaults plus optional
per-timeframe overrides for:

- EMA, RSI, ATR, RVOL, MACD, and ADX/DI;
- active flags and periods;
- level references and ATR thresholds;
- divergence scale and window;
- RSI levels, EMA pairs, MACD families, DI cross, and trend continuation;
- progression target, stability, invalidation, and minimum target.

The pivot detection windows are fixed by the model definition at 2 minor bars and
5 major bars; the structure scale selects which confirmed pivots an event may
reference. Pivot windows are not a v4 config field.

The allowed timeframes are **1m, 5m, 15m, and 1h**. `4h` and above are rejected
by the run config validation.

## 2. Result

The result schema is `quantrule.model-result/v4`. It carries:

- `data_version`, `model_version`, `registry_version`, `requested_period`;
- `effective_config`;
- `ladder`;
- per-timeframe coverage;
- `latest.readings` and `latest.states`;
- `latest.context`;
- `levels`;
- `events`;
- `progression`;
- `charts` as chart-ready data.

The result carries no axes, no cross-timeframe aggregate, no prediction, and no
confidence field. The shell renders chart images from `charts`; the JSON carries
numbers only.

## 3. Compatibility

- v3 config and historical v3 reports remain readable.
- v4 is the new result path.
- v8 reads events/states; v7 remains historical.
- `market-report` remains the legacy v3 view.

## 4. Verification

- v4 parser tests, including 4h rejection.
- v4 result envelope and chart-data tests.
- Causality, determinism, availability, and edge cases for every event.
- The path-validation harness and the pre-registered matrix.

## 5. Validation result

The pre-registered path gate ran on `model_dev` (BTCUSD, ETHUSD, XAUUSD,
2018–2020, 5m/15m/1h). All 12 primary family-timeframe cells are grade C. No
event family passes the matched-base lift, the state-conditioned incremental
lift, the random/back-lag/shuffled controls, and two-instrument replication
together. The validation partition was not opened.

- Decision: [`docs/improvement/2026-11-04/MODEL-V4-GATE.md`](../../improvement/2026-11-04/MODEL-V4-GATE.md)
- Run record: [`MODEL-2026-11-04-01`](../../improvement/2026-11-04/MODEL-2026-11-04-01-model-v4-dev.md)
- Protocol: [`path-validation-v4.md`](path-validation-v4.md)

The v4 contract and implementation remain the model surface. The result is model
evidence, not a strategy verdict or a profitability claim.
