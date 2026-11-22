# AI Transparency Log — RA-004 Execution-Simulation Pack

- Date: 2026-11-22 (UTC)
- Work item: [#1092](https://github.com/quant-rule/quant-rule/issues/1092)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Plan: [`docs/execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md`](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)
- Branch: `feat/1092-execution-pack`

## Intent and boundary

Implement the execution-simulation pack schema and validator on the RA-002 pack
envelope, reusing the pack identity and lifecycle from RA-002/RA-003. This entry
covers RA-004 only. Run v5, signal artifacts, the strategy service, the persistent
registry and client commands remain separate issues. The execution engine and the
run v4 configuration semantics are unchanged.

The pack describes historical simulation only. It contains no broker, OMS, order
routing, venue, latency, participation or statistical field.

## Work performed

- Added `pack::execution` with the fill timeframe, market-next-open order,
  stop-first tie rule, fee rate, slippage, commission, funding, zero-volume and
  end-of-data policy.
- Reused the engine's `ExecutionConfig`, `CommissionConfig`, `FundingConfig` and
  `FundingInterval` so costs have one definition.
- Extracted the fee, slippage, commission and funding JSON parsers from
  `config::portfolio` and `config::execution` into `config::costs`, parameterised
  by JSON path. The run configs and the pack now share the same validation; the
  extracted code is behaviour-neutral for the existing files.
- Added closed single-variant enums for the order, tie, zero-volume and
  end-of-data codes, so an unsupported value is rejected instead of stored.
- Implemented the draft, validated, frozen and retired lifecycle and registered
  `ExecutionPack` in the sealed `PackIdentity`/`PackIndex` seam.
- Added 23 execution-pack unit tests, one mixed-kind identity update, and 6
  public-API tests including a golden canonical hash and repeated-serialization
  checks.
- Updated the config contract, architecture status, module map, configuration
  index, changelog and this log.

## Design decisions for review

- `fill_timeframe` uses the strategy pack's timeframe parser, which accepts
  `1m`, `5m`, `15m` and `1h`. The existing `execution.json` parser accepts the
  wider data ladder, but a run already requires the strategy execution timeframe
  to equal the fill timeframe, and the strategy pack has the one-hour ceiling. A
  pack that named `4h` could never pair with a valid strategy pack.
- The `.0` numeric representation is preserved by the shared canonical writer.
  The golden fixture therefore records `fee_rate: 0.0`, not `0`, and the hash is
  taken over those exact bytes.
- The `(name, version)` label remains global across pack kinds, matching RA-002
  and RA-003. Two packs of different kinds with the same label conflict.
- Commission and funding remain required fields even though they live in
  `portfolio.json` today. The approved pack contract assigns them to the
  execution-simulation pack; run v5 wiring is a later issue.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy -p quantrule-app --all-targets -- -D warnings` | Pass |
| `cargo test -p quantrule-app` | 277 unit, 6 execution identity, 6 portfolio identity, 1 isolation, 5 strategy identity, 1 doctest — all pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p quantrule-app --no-deps` | Pass |
| `cargo run -q -p quantrule-governance -- check --all` | `governance check ok` |
| `tools/gate.sh` | `gate passed: full`; `governance check ok` |

No Quant Validator record exists yet. The human freeze decision and independent
reproduction remain RA-010, after the signal artifact and service exist.

## Learning and corrections

- The fee, slippage, commission and funding parsers were private to the run
  config modules. Copying them into the pack would have created a second cost
  validation path; extracting them to `config::costs` kept one definition while
  letting each caller supply its own JSON path.
- `ExecutionConfig::new` validates finiteness and sign but not an upper slippage
  bound. The `< 10000` basis-point bound lived only in `config::execution`; the
  shared cost module now owns it so a pack and an execution file cannot disagree.
- The execution config parser uses the wider data timeframe parser, while the
  strategy pack uses the one-hour ceiling. Choosing the strategy parser for the
  pack avoids a valid pack that no run could use, and the difference is recorded
  for review rather than hidden.

## Residual risk

The execution pack is not yet referenced by run v5, the persistent registry or a
client surface. Run-level cross-checks (fill timeframe against the data ladder,
commission versus the portfolio account, and the strategy execution timeframe)
remain with the run v5 issue. No production claim is made by this entry.
