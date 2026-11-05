# Model spec v1

- Date: 2026-11-05 (UTC)
- Status: **Approved** — the schema name and the Rust-evaluator split are the
  programme decisions; this document fixes the v1 shape.
- Contract: `quantrule.model-spec/v1`
- Implementation: `crates/quantrule-model/src/spec.rs`,
  `crates/quantrule-shell/src/spec.rs`
- Claim type: model configuration and measurement contract; no edge claim

## 1. Purpose and boundary

A model spec declares a timeframe ladder, one evaluation timeframe, and named
conditions over the model's published events and discrete states. It configures
a measurement run; it is not a strategy. It carries no entry, exit, position,
cash, or P&L field, and it cannot read a raw candle, an indicator value, an axis
score, or a statistic.

The shell parses JSON. The model receives resolved numeric codes, so the model
crate has no JSON dependency. The registry is embedded in the shell at compile
time and resolves event identifiers to their type, subtype, and source codes.

## 2. Document

```json
{
  "schema": "quantrule.model-spec/v1",
  "id": "fixture-v1",
  "ladder": [300, 900],
  "evaluation_timeframe": 300,
  "conditions": [
    {
      "id": "rsi_up_15m",
      "when": {
        "all": [
          {
            "event": {
              "id": "signal.rsi_threshold",
              "timeframe": 900,
              "direction": 1
            }
          }
        ]
      }
    },
    {
      "id": "di_up_15m",
      "when": {
        "state": {
          "state": "di_state",
          "timeframe": 900,
          "equals": 1
        }
      }
    }
  ]
}
```

| Field | Type | Required | Rule |
|---|---|---|---|
| `schema` | string | yes | Exactly `quantrule.model-spec/v1` |
| `id` | string | yes | Non-empty, stable within a run |
| `ladder` | array of positive integers | yes | Strictly ascending seconds; each timeframe is a first-class series |
| `evaluation_timeframe` | positive integer | no | Must be in the ladder; defaults to the first entry |
| `conditions` | array | yes | 1–64 entries with unique non-empty `id`s |

Unknown fields are rejected. A document with an unknown field is a typo, not an
extension.

## 3. Expressions

An expression is an object with exactly one of the following keys.

| Key | Operand | Meaning |
|---|---|---|
| `event` | event object | A matching event is available in the window |
| `state` | state object | A discrete state equals the expected code |
| `all` | non-empty array | Every child is true |
| `any` | non-empty array | At least one child is true |
| `not` | expression object | The child is false |

### Event

| Field | Type | Required | Rule |
|---|---|---|---|
| `id` | string | yes | A registry event ID; resolved to type, subtype, and source |
| `timeframe` | positive integer | yes | Must be in the ladder |
| `direction` | -1, 0, or 1 | no | Absent means either direction |
| `within_bars` | integer 0–10,000 | no | Lookback in referenced-timeframe bars; default 0 |

An event matches when its `available_bar_index` is at or before the as-of bar and
no more than `within_bars` behind it. The occurrence bar is irrelevant to
availability: an event that occurred earlier but became knowable now counts at
the bar it became knowable.

### State

| Field | Type | Required | Rule |
|---|---|---|---|
| `state` | string | yes | `ema_stack`, `rsi_zone`, `macd_state`, `macd_histogram_state`, `adx_regime`, `adx_slope`, or `di_state` |
| `timeframe` | positive integer | yes | Must be in the ladder |
| `equals` | integer | yes | Within the state's code range |

| State | Codes |
|---|---|
| `ema_stack`, `rsi_zone`, `macd_state`, `macd_histogram_state`, `adx_slope`, `di_state` | -1, 0, 1 |
| `adx_regime` | 0, 1 |

A state is read from the as-of bar of its timeframe. A missing state is false;
it is never replaced by zero.

## 4. As-of rule

An evaluation bar is one bar of the evaluation timeframe. For every referenced
timeframe, the as-of bar is the latest bar whose **close time** is at or before
the evaluation bar's close time. This is the same rule the strategy context
uses; a higher-timeframe bar that closes after the evaluation bar is never
visible.

The rule is causal: appending, changing, or removing later bars never changes a
past condition value. The model tests prove this on a prefix run.

## 5. Limits

| Limit | Value |
|---|---|
| Expression depth | 16 |
| Nodes per condition | 256 |
| Conditions per spec | 64 |
| Event lookback | 10,000 referenced-timeframe bars |

The limits keep an evaluator pass bounded. Exceeding one is a validation error,
not a clamp.

## 6. Evaluation result

The evaluator returns one boolean per evaluation bar per condition, in document
order, plus a true count. It does not write files; the shell owns artifacts.

## 7. Deferred to v2

| Extension | Why deferred |
|---|---|
| Reading overrides | The v1 model run uses the default readings and event settings |
| Event-family switches | The registry selects conditions; the run computes every family |
| Panel and session conditions | Their event series are separate from the per-timeframe batch |
| Strategy fields | A strategy is human-owned and reads the snapshot, not the spec |

A v2 document uses a new `schema` value. The v1 parser never guesses at an
unknown extension.

## 8. Verification

| Test | File |
|---|---|
| Schema, ladder, identifier, and expression validation | `crates/quantrule-model/src/spec/tests.rs` |
| Event/state/combinator semantics and direction filter | `crates/quantrule-model/src/spec/tests.rs` |
| Extended event batch feeding | `crates/quantrule-model/src/spec/tests.rs` |
| Causality, determinism, and missing-timeframe rejection | `crates/quantrule-model/src/spec/tests.rs` |
| JSON parsing, registry resolution, and end-to-end evaluation | `crates/quantrule-shell/src/spec.rs` |
