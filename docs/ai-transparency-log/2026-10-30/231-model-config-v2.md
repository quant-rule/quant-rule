# AI Transparency Log — Per-Timeframe Model Configuration

- Date: 2026-10-30 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential implementation and validation passes
- Work item: [#231](https://github.com/quant-rule/quant-rule/issues/231)
- Parent plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Status: Implemented; PR [#240](https://github.com/quant-rule/quant-rule/pull/240) open; CI passed

## Intent and boundary

Phase 3 makes the model's scalar reading periods configurable per timeframe.
Defaults apply everywhere; a timeframe can override the fast/middle/slow EMA,
RSI, ATR, or RVOL period. The structure scale remains configurable.

This first delivery does not expose percentile windows, progression, event, or
context thresholds. Those knobs remain fixed and are reserved for a follow-up
because they need separate event/progression golden review. The parser rejects
them rather than accepting fields that do not affect the computation.

The default configuration must preserve the current model output. A configured
override changes only the selected timeframe's readings; no model formula is
changed.

## Implemented

- Added `ReadingsConfig` with EMA fast/middle/slow, RSI, ATR, and RVOL periods.
- Added `Readings::compute_with_config`; `Readings::compute` uses the defaults.
- Extended `MarketStateConfig` with a per-timeframe readings array and override
  markers; `readings_for` and `set_readings` are explicit.
- Made `TimeframeModel::compute` use the configured readings for its timeframe.
- Added `model.json` v2 parsing:
  - `defaults` with EMA/RSI/ATR/RVOL periods;
  - optional `timeframes` overrides;
  - `structure.scale`.
- Kept `quantrule.model/v1` accepted for the historical shape.
- Added cross-config validation that an override timeframe is available in
  `data.json`.
- Updated the example config and config fixture to model v2.
- Updated the model and config contracts, README, and plan.
- Added a model integration test that proves an M5 override changes M5 readings
  while M15 remains on the default readings.

## Verification and evidence

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `cargo clippy --all-targets -- -D warnings` | Passed |
| `cargo test --workspace` | Passed; 45 suites, 0 failures |
| `cargo doc --no-deps` | Passed |
| Default golden tests | Passed; default outputs unchanged |
| M5 override vs M15 default | M5 readings changed; M15 equal to default |
| Invalid EMA order / zero period | Rejected |
| Unknown timeframe / unknown field | Rejected |
| Override outside available ladder | Rejected by cross-config validation |
| Worktree | `quant-rule--231-model-config-v2` |
| PR | #240 opened |
| CI `verify` and `sync` | Passed |

## Quant validation (sequential pass)

The model behavior changes only when an override is supplied. The sequential
validation checked:

- the default configuration reproduces the existing golden outputs;
- an M5 override changes M5's EMA/RSI availability and leaves M15 unchanged;
- the override is recorded in the model file covered by the run config
  fingerprint;
- no strategy or execution behavior changed in this phase.

This is a sequential same-agent pass, not an independent agent or model
judgment. An independent Quant Validator pass remains desirable before an
experiment uses per-timeframe overrides to support a performance claim.

## Deviations and decisions

- The first delivery exposes readings and the structure scale only. Percentile,
  progression, event, and context knobs remain fixed and are documented as
  reserved.
- `Readings` field names remain stable role labels (`ema20`, `ema50`, `ema200`,
  `rsi14`, `atr14`, `rvol20`); the configured period may differ. The model config
  and config fingerprint identify the actual periods.
- No resolved per-timeframe readings block was added to report provenance in
  this phase; the model file is part of the run config fingerprint and file set.
- `AGENTS.md` normative edits remain deferred to the behavior-changing model/
  strategy phases.

## Learning and rejected suggestions

- Rejected accepting progression/event/context fields before their calculations
  are parameterized. A config field that does not affect output is a lie.
- Rejected renaming every published reading field and store column in this
  phase. The role labels are stable; renaming the full output surface is a
  separate schema change.
- Rejected forcing every path command to use a configured model. Path commands
  have no run config; they keep the default model.
- Rejected a global model override. The requirement is per timeframe, and the
  override must not leak into another timeframe.

## Residual risk

- Per-timeframe periods change the meaning of the role-labeled readings; an
  experiment must record the periods and must not compare cells as if they used
  identical measurements.
- The store/research columns still carry role labels. The config fingerprint and
  data version record the actual periods, but a stored dataset does not carry a
  per-column period label.
- A full per-timeframe override matrix is not tested; the integration test uses
  one M5 override.
