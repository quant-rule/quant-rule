# Strategy Configuration and Decisions

- Status: Current; implemented 2026-11-06; updated 2026-11-16 for the gateway run path; updated 2026-11-09 for the `vol_regime` and `structure_state` states
- Schema: `quantrule.strategy/v1`
- Commands: `run submit`, `run status`, `run fetch`
- Related: [`engine-artifacts.md`](engine-artifacts.md),
  [`command-surface.md`](../../architecture/command-surface.md),
  [`run-v4-strategy.md`](../2026-11-13/run-v4-strategy.md),
  [`strategy-research.md`](../../workflows/strategy-research.md)
- Claim type: strategy configuration and decision artifact; no edge or
  profitability claim.

## 1. Boundary

The strategy file is human-owned. It names the thesis as a deterministic rule over
published model measurements, and it never contains an indicator formula, a
statistic, a fill price, a fee, cash, equity, or P&L. The run's strategy stage
reads the published model bars and writes decisions; execution and portfolio run
in the same submitted pipeline. The research process is
[`strategy-research.md`](../../workflows/strategy-research.md).

A file-based full run uses `quantrule.run/v4` and names `strategy.json` alongside
the four engine files. A `quantrule.run/v5` run references a strategy pack whose
payload carries the same strategy fields; the pack is validated by the same
engine parser. `quantrule.run/v3` remains accepted for a model-only run and
rejects a `strategy` field. The strategy file is not a separate command.

## 2. Schema

```json
{
  "schema": "quantrule.strategy/v1",
  "id": "S1.0",
  "engine": "event_composite",
  "execution_timeframe": "5m",
  "context_timeframes": ["15m"],
  "event_window_bars": 1,
  "time_stop_bars": 0,
  "require_contiguous": true,
  "risk": {
    "stop_atr_multiple": 2.0,
    "reward_to_risk": 1.5
  },
  "entry": [
    {
      "kind": "event",
      "timeframe": "5m",
      "event_type": 3,
      "subtype": 1,
      "source": 4,
      "direction": 1
    }
  ],
  "exit": [
    {
      "kind": "state",
      "timeframe": "5m",
      "field": "macd_state",
      "value": -1
    }
  ]
}
```

| Field | Rule |
|---|---|
| `schema` | Exactly `quantrule.strategy/v1`. |
| `id` | Optional non-empty human label. It is recorded in the summary. |
| `engine` | `event_composite` or `compression_bracket`. |
| `execution_timeframe` | `1m`, `5m`, `15m`, or `1h`; the model ceiling is `1h`. In a submitted run, it must equal the run's `execution.fill_timeframe`; the loader rejects a mismatch. |
| `context_timeframes` | Optional unique timeframes, ascending, different from the execution timeframe, and published by the model bundle. |
| `event_window_bars` | Event-composite only; positive integer, default `1`. The window is in execution bars. |
| `time_stop_bars` | Maximum holding period in execution bars; `0` disables it for the event-composite engine. A bracket requires a positive value. |
| `giveback_atr` | Event-composite only; positive entry-ATR threshold. When set, the strategy exits after the position gives back at least this much of its favorable excursion. |
| `giveback_arm_atr` | Event-composite only; non-negative entry-ATR favorable excursion required before the giveback exit can fire; default `0`. |
| `require_contiguous` | Boolean, default `true`. A decision requires the execution bar to follow the previous one at the execution cadence. |
| `risk.stop_atr_multiple` | Positive finite ATR stop multiple. |
| `risk.reward_to_risk` | Positive finite target multiple of the stop distance. |
| `entry` | Event-composite only; one to six conditions; at least one must be an event. |
| `exit` | Event-composite only; zero to six conditions. |
| `event` | Compression-bracket only; the event identity that publishes the range. |
| `entry_valid_bars` | Compression-bracket only; exactly `1` in the current engine. |

For a context timeframe, the execution-bar window is converted to the smallest
context bar count that covers the same span, rounded up. A 20-bar 5m window on a
1h context reads 2 context bars (120 minutes); the latest completed context bar
may be up to one context bar old. The conversion is part of the window contract,
not a separate per-timeframe setting.

The parser rejects an unknown field, a duplicate condition, more than six
conditions, an empty entry list, a condition without an entry event, and entry
events with conflicting nonzero directions.

## 3. Conditions

The event-composite engine reads a conjunction of entry conditions and a
disjunction of exit conditions. Each condition names a published timeframe.

```json
{ "kind": "event", "timeframe": "5m", "event_type": 3, "subtype": 1, "source": 4, "direction": 1 }
```

```json
{ "kind": "state", "timeframe": "5m", "field": "macd_state", "value": -1 }
```

| Field | Rule |
|---|---|
| `kind` | `event` or `state`. |
| `timeframe` | A timeframe published by the model bundle. |
| `event_type` | `1` Level, `2` Divergence, or `3` Signal. |
| `subtype` | The registered subtype. The engine accepts any registered subtype, not only `1..=7`. |
| `source` | The registered source code, `1..=57`. |
| `direction` | `-1`, `0`, or `+1`. Zero matches either direction and takes the matched event's side. |
| `contrarian` | Optional boolean on an event condition, default `false`. When `true`, the entry side is the opposite of the matched event direction; the event identity and evidence are unchanged. |
| `field` | A published state field: `ema_stack`, `rsi_zone`, `macd_state`, `macd_histogram_state`, `adx_regime`, `adx_slope`, `di_state`, `rvol_regime`, `vol_regime`, `structure_state`, `progression_stage`, or `progression_leg_direction`. |
| `value` | The exact state code the condition requires. |

The parser rejects an event identity that is not in the registry, any
identity whose registered role is `diagnostic`, and a `contrarian` field on a
state condition or on the bracket engine.

The event-composite engine accepts any registered event identity whose role is
`signal` or `context`. It rejects an identity that is not in the registry and
any identity whose role is `diagnostic`. The source is `1..=57`; the event type
is `1..=3`. The engine is bounded by the registered identity and role, not by a
subtype range: the earlier `1..=7` limit was removed in SR6, and extended events
with a registered identity are part of the language.

## 4. Compression bracket

```json
{
  "schema": "quantrule.strategy/v1",
  "id": "S-BRACKET",
  "engine": "compression_bracket",
  "execution_timeframe": "5m",
  "time_stop_bars": 12,
  "require_contiguous": true,
  "risk": {
    "stop_atr_multiple": 2.0,
    "reward_to_risk": 1.5
  },
  "event": {
    "timeframe": "5m",
    "event_type": 3,
    "subtype": 8,
    "source": 5
  },
  "entry_valid_bars": 1
}
```

The bracket engine reads the neutral compression-range event and places a
two-sided stop entry at the published high and low. The event timeframe must
equal the execution timeframe; the bracket reads the execution view. Execution
chooses the side when a boundary fills. The strategy does not know the side in
advance. The bracket event must be the registered compression-range identity
`(3,8,5)`; an unregistered, diagnostic, or other event identity is rejected.

On the entry bar the bracket is live for the whole bar. OHLC cannot order a
pre-trigger low or high against the entry, so the conservative rule records a
stop-out when the bar range crosses the stop. The rule is stated in the
execution contract; it is not a claim that the intrabar path was observed.

The bracket engine is supported by the run's strategy stage. The run supplies
the executed position and execution chooses the side when a boundary fills; the
strategy does not know the side in advance.

## 5. Gateway path and decision artifact

The strategy is part of a submitted run. There is no standalone strategy
command.

```bash
quantrule run submit <data_version> <config_id>
quantrule run status <run_id>
quantrule run fetch <run_id> <dir>
```

`run submit` accepts a valid `quantrule.run/v4` config and returns the `run_id`.
A `quantrule.run/v5` config is validated through a supplied reference resolver
until the persistent registry lands; the gateway submission path accepts it in a
later release. The run's strategy stage writes `<out>/strategy/` into the
artifact set:

| File | Contents |
|---|---|
| `decisions.ndjson` | One decision record per execution bar, in ascending order. |
| `summary.txt` | Counts by decision, side, and reason; data version; config identities; period. |
| `provenance.json` | `quantrule.stage-provenance/v1` with the model source artifact and the strategy config identity. |

`run fetch` downloads that artifact set and verifies each artifact against the
run record. A rejected entry in execution does not re-run the strategy; the run
supplies the actual position on the next bar.

A decision record carries:

| Field | Meaning |
|---|---|
| `schema` | `quantrule.strategy-decision/v1`. |
| `timeframe_secs` | The execution timeframe. |
| `bar_index` | The zero-based execution bar. |
| `open_time` | The bar's UTC open time in Unix seconds. |
| `available_bar_index` | The first bar on which every input was knowable; equal to the decision bar. |
| `position_before` | The position the runner supplied: `flat`, `long`, or `short`. |
| `position_after` | The intended position after the decision. A bracket entry remains `flat` until execution fills it. |
| `decision` | `hold`, `enter`, `enter_bracket`, or `exit`. |
| `side` | `long`, `short`, or `null`. |
| `reason` | `entry`, `bracket_entry`, `event_condition`, `time_stop`, `giveback`, or `null`. |
| `intent` | The entry or exit request, or `null`. |
| `atr14` | The decision bar's ATR(14), used by execution to resolve an ATR stop. The strategy decision logic does not read it. |
| `evidence` | The as-of event and state references that justified the decision. |

The record carries no fill, quantity, fee, cash, equity, or P&L.

## 6. Determinism and causality

- The model bundle is read in bar order; every collection is a `BTreeMap` or a
  sorted vector.
- A context timeframe is read only through the latest bar whose close is at or
  before the execution bar's close.
- An event enters the snapshot only when its `available_bar_index` is at or
  before the decision bar.
- The same model bundle, strategy file, and bar produce the same decision bytes.
- The bundle provenance records the model data version and config identity and
  the strategy config identity; no clock value is written.

## 7. Verification

- Parser tests cover both engines, unknown fields, duplicate contexts, and
  engine-incompatible fields.
- Run tests load a v4 config with `strategy.json`, assert the strategy artifact
  is present in the fetched set, and compare the decision bytes across runs.
- A boundary test asserts a decision record carries no fill, quantity, fee,
  cash, equity, or P&L.
- The model bundle reader reads only the published model artifacts; it does not
  re-run the model or read a candle.
