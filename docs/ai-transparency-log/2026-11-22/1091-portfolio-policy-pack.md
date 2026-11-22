# AI Transparency Log — RA-003 Portfolio Policy Pack

- Date: 2026-11-22 (UTC)
- Work item: [#1091](https://github.com/quant-rule/quant-rule/issues/1091)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Plan: [`docs/execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md`](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)
- Branch: `feat/1091-portfolio-policy-pack`

## Intent and boundary

Implement the portfolio policy pack schema and validator on the RA-002 pack
envelope. This entry covers RA-003 only. The execution-simulation pack, signal
artifact, strategy service, run v5, persistent registry and client commands
remain separate issues.

The implementation stays in `quantrule-app::pack`, beside the strategy pack, so
one crate owns configuration parsing, canonical bytes, identity and lifecycle.
It reuses the engine's `SizingMode` and `MaxDrawdownConfig` so sizing and
drawdown have one definition.

## Work performed

- Added `pack::portfolio` with sizing, per-trade risk, exposure, maximum
  positions, drawdown, rebalance and declarative constraints.
- Absented initial cash, currency and leverage from the payload. The closed
  schema rejects them, along with correlation, covariance, optimisation and
  other statistical or fitted fields.
- Constrained the v1 constraint list to `max_positions_per_instrument` and
  `min_holding_bars`. The list must be strictly ascending by kind and unique;
  a per-instrument limit cannot exceed `max_open_positions`.
- Added `RebalanceRule::None` and a deterministic `bar_interval` rule with a
  positive bar count.
- Extracted the shared identity and lifecycle seam into `pack::core`. Strategy
  and portfolio packs now use the same canonical bytes, content hash and state
  machine.
- Sealed `PackIdentity` and made `PackIndex::register` generic over it. The
  index accepts both pack types without accepting an arbitrary type.
- Added 26 portfolio unit tests, 2 mixed-kind identity tests and 6 public-API
  tests, including a golden canonical hash and repeated-serialization checks.
- Updated the config contract, architecture status, module map, changelog and
  this log.

## Design decisions for review

- The `(name, version)` label remains global across pack kinds, matching
  RA-002. Two packs of different kinds with the same label conflict.
- `drawdown.action` accepts `halt` only, as the approved config contract
  records. A limit without a halt is not a policy.
- `rebalance` accepts `none` or
  `{"mode": "bar_interval", "bars": N}`. No statistic or fitted parameter is
  involved.
- A per-instrument limit greater than `max_open_positions` is rejected as
  inconsistent rather than silently ignored.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy -p quantrule-app --all-targets -- -D warnings` | Pass |
| `cargo test -p quantrule-app` | 254 unit, 6 portfolio identity, 5 strategy identity, 1 isolation, 1 doctest — all pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p quantrule-app --no-deps` | Pass |
| `cargo run -p quantrule-governance -- check --all` | `governance check ok` |
| `tools/gate.sh` | `gate passed: full`; `governance check ok` |

No Quant Validator record exists yet. The human freeze decision and independent
reproduction remain RA-010, after the signal artifact and service exist.

## Learning and corrections

- The RA-002 `PackIndex::register` accepted `&StrategyPack` directly. A generic
  sealed trait was the narrowest change that preserved type safety while adding
  the portfolio kind.
- Duplicating the pack lifecycle in the portfolio type would have created a
  second state machine. Extracting `PackCore` kept one identity and lifecycle
  implementation for both kinds without changing strategy behaviour.
- The portfolio payload cannot reuse `PortfolioConfig` because that type owns
  initial cash and currency, which this pack deliberately excludes.

## Residual risk

The execution-simulation and signal packs, run v5, the persistent registry and
the client surface are not implemented. No production claim is made by this
entry.
