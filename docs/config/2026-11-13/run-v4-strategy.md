# Run Config v4 — The Strategy Is Part of the Run

- Date: 2026-11-13 (UTC)
- Status: Current
- Claim type: Behaviour Claim

A run config now names its human-owned strategy. The strategy is one of the
files the run config composes, so a `config_id` reproduces the whole run rather
than only its data, model, execution, and portfolio inputs.

## Schema

`quantrule.run/v4` adds one required field to v3:

```json
{
  "schema": "quantrule.run/v4",
  "name": "btcusd-2024-strategy",
  "model_id": "M1.0",
  "data": "data.json",
  "model": "model.json",
  "execution": "execution.json",
  "portfolio": "portfolio.json",
  "strategy": "strategy.json",
  "output_root": "../../../data/labs/btcusd-2024-strategy"
}
```

| Field | Rule |
|---|---|
| `strategy` | Relative path to a `quantrule.strategy/v1` file; required in v4, absent in v3 |
| Other fields | Unchanged from v3 |
| Cross-check | The strategy's execution and context timeframes must be in the data ladder, and the strategy execution timeframe must equal `execution.fill_timeframe` |

## Compatibility

| Schema | Strategy | Use |
|---|---|---|
| `quantrule.run/v4` | Required | A full run, including strategy, execution, and portfolio |
| `quantrule.run/v3` | Absent; a `strategy` field is rejected | A model-only run or a historical evidence config |

The catalog and the model route accept both. A config that is missing a
referenced file, including `strategy.json`, is catalogued as invalid with the
loader's message.

## Identity

The config hash and the catalog `config_id` cover every file the run names. For
v4 that is six files: the run file, data, model, execution, portfolio, and
strategy. A change to the strategy changes the id, so a run cannot silently
reuse a different rule set.

## Verification

- `config/examples/btcusd-2024-strategy/` loads and validates.
- The config catalog lists it as a valid v4 entry with six stored files.
- A v3 config with a `strategy` field is rejected with `schema`.
- Changing one byte of `strategy.json` changes the config hash.
