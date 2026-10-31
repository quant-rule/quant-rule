# Migration — Strategy v2 to v3

- Date: 2026-10-30 (UTC)
- Status: Current
- Issues: [#228](https://github.com/quant-rule/quant-rule/issues/228), [#235](https://github.com/quant-rule/quant-rule/issues/235), [#236](https://github.com/quant-rule/quant-rule/issues/236)
- Plan: [`docs/execution-plans/2026-10-30/228-unified-timeframe-ladder.md`](../../execution-plans/2026-10-30/228-unified-timeframe-ladder.md)
- Current contract: [`docs/config/2026-10-30/strategy-v3.md`](strategy-v3.md)

## 1. What changed

| v2 | v3 |
|---|---|
| `schema: quantrule.strategy/v2` | `schema: quantrule.strategy/v3` |
| `name: axis-agreement` | `rule: ladder-context` |
| `timeframe: 15m` | `ladder: ["5m", "15m", ...]`; `ladder[0]` is execution |
| `long_threshold`, `short_threshold` | `entry.execution` and `entry.context` thresholds |
| No context weights | `weights` per axis and context timeframe |
| `stop_atr_multiple`, `reward_to_risk` | `risk.stop` and `risk.target` |
| Implicit `agreement_lost` exit | `exit.on` with `context_lost` and/or `structure_lost` |

A v2 file is rejected with:

```text
quantrule.strategy/v2 is retired; use quantrule.strategy/v3 with rule "ladder-context"
```

There is no automatic migration. The old flat fields cannot express an active
ladder, context weights, or the new exit reasons.

## 2. Minimal v3 config

```json
{
  "schema": "quantrule.strategy/v3",
  "rule": "ladder-context",
  "thesis": "Trade with the higher-timeframe regime after a lower-timeframe structure shift.",
  "ladder": ["5m", "15m"],
  "weights": {
    "direction": { "15m": 1.0 },
    "momentum": { "15m": 1.0 },
    "structure": { "15m": 1.0 },
    "progression": { "15m": 1.0 }
  },
  "entry": {
    "execution": {
      "direction_min": 0.1,
      "momentum_min": 0.1,
      "structure_min": 0.0,
      "structure_shift": { "from_max": 0, "to_min": 1 }
    },
    "context": {
      "direction": { "kind": "bullish", "share_min": 0.5 },
      "momentum": { "kind": "not_bearish", "bearish_share_max": 0.5 },
      "structure": { "conflict_count_max": 0 },
      "progression": {
        "direction": "bullish",
        "late_or_terminal_count_max": 0
      }
    }
  },
  "exit": { "on": ["context_lost", "structure_lost"] },
  "risk": {
    "stop": { "kind": "atr", "multiple": 2.0 },
    "target": { "kind": "reward_to_risk", "multiple": 1.5 }
  },
  "no_trade": {
    "missing_context": "reject",
    "require_contiguous_execution": true
  }
}
```

## 3. Field mapping

| v2 field | v3 field | Rule |
|---|---|---|
| `timeframe` | `ladder[0]` | Must equal `execution.fill_timeframe`. |
| — | `ladder[1..]` | Ascending, unique, slower than execution; in `data.available_timeframes` and the policy universe. |
| — | `weights` | Optional; missing block means equal weights. A present block must cover every context timeframe. |
| `long_threshold` | `entry.execution.direction_min`, `.momentum_min` | Use the old threshold for both, or state each separately. |
| — | `entry.execution.structure_min` | The Structure score floor. |
| — | `entry.execution.structure_shift` | Native Structure state codes. |
| — | `entry.context` | Direction share, Momentum opposing-share cap, Structure conflict cap, Progression late/terminal cap. |
| `stop_atr_multiple` | `risk.stop.multiple` | Same ATR distance. |
| `reward_to_risk` | `risk.target.multiple` | Same reward-to-risk. |
| — | `exit.on` | Non-empty, unique subset of `context_lost` and `structure_lost`. |
| — | `no_trade` | `missing_context` is `reject`; `require_contiguous_execution` is `true`. |

## 4. Ladder membership

The active ladder must be a subset of both:

- `data.json.available_timeframes`
- `policy.json.universe.timeframes`

The execution fill timeframe must equal `ladder[0]`. A mismatch is a config
error naming the file and JSON path.

## 5. Weights

- Keys are context timeframes only.
- Values are finite and `> 0`.
- Omit a block for equal weights on that axis.
- If a block is present, include every context timeframe.
- Weights are normalized in active-ladder order when the context is computed.
- Weights express importance, not calibration.

## 6. Verification

```bash
cargo run -q -p quantrule-shell -- policy check --config <run.json>
cargo run -q -p quantrule-shell -- strategy explain --config <run.json>
```

`policy check` validates the v3 strategy against the data ladder, policy
universe, execution fill timeframe, model, portfolio, and analytics.
`strategy explain` prints the active ladder, context ladder, entry, exit, and
risk in trader words.

## 7. Historical rule

The old `axis-agreement` behavior is preserved by a dated ADR and golden
fixture, not by a live config path:

- [`docs/adr/2026-10-30/axis-agreement-retirement.md`](../../adr/2026-10-30/axis-agreement-retirement.md)
- [`docs/adr/2026-10-30/axis-agreement-golden.csv`](../../adr/2026-10-30/axis-agreement-golden.csv)
