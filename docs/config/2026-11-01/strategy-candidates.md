# Strategy Candidate Registry

- Date: 2026-11-01 (UTC)
- Status: Current for the first improvement cycle
- Scope: candidate rules only; no candidate is a validated baseline

| Rule | Schema | Ladder used by the first cycle | Entry | Status |
|---|---|---|---|---|
| `ladder-context` | `quantrule.strategy/v3` | 15m / 1h | Direction/Momentum thresholds, Structure shift, higher-timeframe context | Control; not green in prior experiments |
| `level-retest` | `quantrule.strategy/v4` | 15m / 1h | Retest of a confirmed level with a structural bracket | Experimental; not green |
| `three-timeframe` | `quantrule.strategy/v5` | 1m / 5m / 15m | 5m/15m priority agreement and a 1m level retest | Experimental; not green |
| `breakout` | `quantrule.strategy/v6` | 15m / 1h | Close-through of a confirmed level with a structural bracket | Experimental; not green |
| `level-fade` | `quantrule.strategy/v7` | 15m / 1h | Fade a failed test of a confirmed level with a structural bracket | Experimental; 2024 validation failed |

The candidate registry is not a baseline. The model gate must pass before any
strategy run is a valid tier result. The run contract is
[`docs/workflows/run-contract.md`](../../workflows/run-contract.md).

The v3 contract is [`docs/config/2026-10-30/strategy-v3.md`](../2026-10-30/strategy-v3.md);
the v6 contract is [`docs/config/2026-11-01/strategy-v6.md`](strategy-v6.md).
The v4 and v5 shapes reuse the v3 common entry, exit, risk, and no-trade blocks
with the `entry.level` and `entry.agreement` extensions.
