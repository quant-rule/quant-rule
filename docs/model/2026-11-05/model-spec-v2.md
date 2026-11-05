# Model spec v2

- Date: 2026-11-05 (UTC)
- Status: **Implemented** (P1); the `Sign` column remains a human/Validator action
- Contract: `quantrule.model-spec/v2`
- Implementation: `crates/quantrule-model/src/spec/v2.rs`,
  `crates/quantrule-shell/src/spec/v2.rs`,
  `crates/quantrule-shell/src/command/report_run.rs`
- Plan: [`docs/execution-plans/2026-11-05/410-m2-model-programme.md`](../../execution-plans/2026-11-05/410-m2-model-programme.md)
- Claim type: model configuration and measurement contract; no edge claim

## 1. Purpose and boundary

A v2 spec declares one evaluation timeframe, the reading overrides that produced
the run, the registry events the model version uses, and a causal directional
vote. It may also declare panel sources and a session calendar. It configures a
measurement run; it is not a strategy. It carries no entry, exit, position,
cash, or P&L field, and it cannot read a forward outcome or a raw candle.

The v1 contract remains valid and unchanged. A document with an unknown schema
is rejected; the parser never guesses at an extension.

## 2. Document

```json
{
  "schema": "quantrule.model-spec/v2",
  "id": "M2.1",
  "family": "M2",
  "instrument": "XAU",
  "timeframes": [3600],
  "readings": {
    "ema": {"fast": 20, "mid": 50, "slow": 200},
    "rsi": {"period": 14},
    "atr": {"period": 14},
    "rvol": {"period": 20},
    "macd": {"fast": 12, "slow": 26, "signal": 9},
    "adx": {"period": 14, "threshold": 25.0, "slope_bars": 3},
    "structure_scale": "major",
    "extended": {"vwap_window": 24, "profile_window": 96}
  },
  "active_events": [
    "signal.vwap_reclaim",
    "signal.vwap_rejection",
    "signal.rvol_expansion"
  ],
  "prediction": {
    "kind": "vote",
    "long": ["signal.vwap_reclaim", "signal.rvol_expansion"],
    "short": ["signal.vwap_rejection"],
    "min_votes": 1
  },
  "conditions": [
    {
      "id": "session_open",
      "when": {
        "event": {"id": "signal.session_open", "timeframe": 3600}
      }
    }
  ],
  "panel": {
    "leader": "BTC",
    "sources": {
      "BTC": "../../data/sources/BTCUSD_M1_2018-01-01_2024-12-31.csv",
      "SP500": "../../data/sources/ESANDP500_M1_2018-01-01_2024-12-31.csv",
      "EURUSD": "../../data/sources/EURUSD_M1_2018-01-01_2024-12-31.csv"
    }
  },
  "session": {
    "zone": "london",
    "start_hour": 8,
    "end_hour": 16,
    "opening_range_bars": 2
  }
}
```

| Field | Type | Required | Rule |
|---|---|---|---|
| `schema` | string | yes | Exactly `quantrule.model-spec/v2` |
| `id` | string | yes | Stable model identifier, such as `M2.1` |
| `family` | string | yes | Model family, such as `M2` |
| `instrument` | string | yes | The evaluated instrument label |
| `timeframes` | array | yes | Exactly one positive timeframe in seconds |
| `evaluation_timeframe` | positive integer | no | Must equal the single timeframe; defaults to it |
| `readings` | object | no | Reading overrides; absent means the contract defaults |
| `active_events` | array of event IDs | yes | Unique registry events the model version uses |
| `prediction` | object | yes | The causal vote |
| `conditions` | array | no | Optional v1-style conditions over the combined event set |
| `panel` | object | no | Optional cross-asset sources |
| `session` | object | no | Optional session calendar |

Unknown fields are rejected. The v2 spec is single-timeframe; cross-timeframe is
a later schema.

## 3. Readings

`readings` is optional. Every field is optional and inherits the contract
default. Event thresholds are **not** configurable in v2; only the reading
periods, windows, and active flags are.

| Object | Fields | Defaults |
|---|---|---|
| `ema` | `fast`, `mid`, `slow`, `active` | 20, 50, 200, true |
| `rsi` | `period`, `active` | 14, true |
| `atr` | `period`, `active` | 14, true |
| `rvol` | `period`, `active` | 20, true |
| `macd` | `fast`, `slow`, `signal`, `active` | 12, 26, 9, true |
| `adx` | `period`, `threshold`, `slope_bars`, `active` | 14, 25.0, 3, true |
| `structure_scale` | `major` or `minor` | `major` |
| `extended` | the `ExtendedConfig` fields listed below | `ExtendedConfig::DEFAULT` |

The `extended` object accepts:
`vwap_window`, `vwap_band_mad`, `profile_window`, `profile_bins`,
`profile_value_area`, `rvol_fast`, `rvol_slow`, `volume_oscillator`,
`climax_volume`, `climax_range_atr`, `absorption_volume`,
`absorption_range_atr`, `absorption_level_atr`, `flow_window`, `mfi_window`,
`cmf_threshold`, `atr_slow_window`, `atr_slope_window`, `range_window`,
`mar_fast`, `mar_slow`, `keltner_window`, `keltner_atr`, `donchian_window`,
`level_lookback`, `level_atr`, `aroon_window`, `path_window`,
`candle_body`, `candle_wick`, `candle_close`, and `gap_atr`.

The resolved config is recorded in `provenance.json`, so a later run can be
shown to have used the same values.

## 4. Active events and roles

`active_events` is the set of registry event IDs the model version uses. Every
ID must exist in the registry, and the set is unique. The set may include
`signal`, `context`, and `diagnostic` events; a condition may read any of them.

Only a `signal` event may appear in `prediction.long` or `prediction.short`. A
`context` or `diagnostic` event in a vote list is rejected. This is the
registry-role rule: a descriptor is not a directional vote.

The `events.csv` artifact of a v2 run is filtered to the active set. The full
model batch still exists internally; the published artifact is the model
version's own event surface.

## 5. Prediction

Two prediction kinds are supported:

```json
"prediction": {
  "kind": "vote",
  "long": ["signal.vwap_reclaim"],
  "short": ["signal.vwap_rejection"],
  "min_votes": 1
}
```

```json
"prediction": {
  "kind": "direction",
  "events": ["divergence.vwap_regular_bullish", "divergence.vwap_regular_bearish"],
  "min_votes": 2
}
```

- `kind = vote`: `long` and `short` are disjoint lists of active `signal`
  events. At bar `t`, every listed event whose `available_bar_index` equals `t`
  adds one vote to its declared side.
- `kind = direction`: `events` is a non-empty list of active `signal` events.
  At bar `t`, every listed event whose `available_bar_index` equals `t` adds its
  own `direction_code` (`+1`, `-1`, or `0`). A neutral event is active but does
  not vote.
- An event is counted once per bar even if several records share its code.
- `total = long - short`. If `abs(total) >= min_votes`, the prediction is
  `sign(total)`; otherwise it is `0`.
- `min_votes` is 1–64.
- A vote prediction must not carry `events`; a direction prediction must not
  carry `long` or `short`. The parser rejects the wrong shape.

The vote is transient: it does not persist a previous event into a later bar.
The prediction at `t` uses only data available at `t`. It reads no forward
outcome, no position, no cash, and no P&L.

`prediction.csv`:

```text
timeframe_secs,bar,timestamp,prediction,long_votes,short_votes
```

## 6. Conditions

`conditions` is an optional array of the v1 condition shape. The v2 evaluator
reads the **combined** event set: the base v4 batch, the extended batch, the
session batch, and the evaluated instrument's panel batch. States come from the
base timeframe.

Conditions are diagnostics. They do not vote, and they do not change the
prediction. A condition may reference a session or panel event; the v1 parser's
ladder, expression-depth, node, and state-code limits still apply.

## 7. Panel and session

`panel` is optional. It declares the cross-asset sources:

- `leader` is the lead-lag leader. It must be the evaluated instrument or one of
  the panel sources.
- `sources` maps an instrument name to an M1 CSV path. The evaluated instrument
  is **not** listed; it comes from the run config's `data.json`. A relative path
  resolves against the spec file's directory.
- Every source is loaded to the single evaluation timeframe using the run
  config's period and refusal bound. No forward fill is used: the panel aligns
  on common timestamps.
- The panel instrument order is the leader, then the evaluated instrument, then
  the remaining source names in ascending order. The order is deterministic and
  is recorded in provenance.
- The panel events for the evaluated instrument are merged into its event view.
  The other instruments' panel events are not.

`session` is optional:

```json
{"zone": "london", "start_hour": 8, "end_hour": 16, "opening_range_bars": 2}
```

`zone` is `utc`, `london`, or `new_york`. The session readings and events are
computed for the evaluated instrument. A session close occurs on the last
in-session bar and becomes available on the next bar, as in the session-event
contract.

## 8. Command and artifacts

```text
report model --config <run.json> --spec <spec.json> [--out <dir>] [--json]
```

The run config supplies the evaluated instrument's source, period, data version,
output root, and available timeframes. The v2 spec supplies the model reading
overrides, the active events, the prediction, and the optional panel/session
context.

The output is the standard model bundle plus:

| Artifact | Content |
|---|---|
| `prediction.csv` | The causal vote and its raw long/short counts |
| `conditions.csv` | The optional condition values, when conditions are declared |
| `events.csv` | The active-event projection of the base, extended, session, and panel batches |
| `provenance.json` | The spec schema, id, family, instrument, file hash, readings, extended readings, active events, prediction definition, panel order, session, event counts, and conditions |

`readings.csv`, `levels.csv`, `legs.csv`, `result.json`, and the charts remain
the standard model artifacts.

## 9. Determinism and causality

The spec run is deterministic: the same candles, spec, panel sources, session
config, and run config produce byte-identical artifacts. The tests cover:

| Test | File |
|---|---|
| v2 parse, reading overrides, active events, prediction, panel/session | `crates/quantrule-model/src/spec/v2.rs` |
| JSON parsing, panel resolution, role rejection, condition validation | `crates/quantrule-shell/src/spec/v2.rs` |
| CLI run, `prediction.csv`, filtered events, provenance, byte-identical rerun | `crates/quantrule-shell/tests/report_command.rs` |

The model tests prove that a prefix run keeps the same prediction values and
that repeated runs are identical. The panel and session tests prove that their
events reach the condition evaluator for the evaluated instrument.

## 10. Limits and deferred work

| Limit | Reason |
|---|---|
| One timeframe | The M2 development programme is single-timeframe; cross-timeframe is a later schema |
| Vote only | The primary prediction is directional; a score is exploratory and not part of v2 |
| Fixed event thresholds | Only activation and reading windows vary; turning every threshold into a parameter would make the swarm an optimizer |
| No strategy fields | Entry, exit, position, cash, and P&L belong to the strategy/execution/portfolio tiers |
| No forward outcome | The evaluator, not the spec, owns the horizon and costs |
| Panel sources are M1 CSVs | The data engine constructs the one timeframe; the spec does not parse candles |
| v2 preflight | `tools/research/spec_preflight.py` still checks v1; the M2 charter/batch phase adds v2 preflight |

## 11. Verification

```bash
cargo test -p quantrule-model --lib spec::v2
cargo test -p quantrule-shell --bin quantrule spec::v2
cargo test -p quantrule-shell --test report_command a_v2_spec
cargo clippy -p quantrule-model -p quantrule-shell --all-targets -- -D warnings
cargo fmt --check
```

Observed on 2026-11-05 in the `feat/454-model-spec-v2` worktree:

| Command | Result |
|---|---|
| `cargo test -p quantrule-model --lib spec::v2` | 9 passed |
| `cargo test -p quantrule-shell --bin quantrule spec::v2` | 7 passed |
| `cargo test -p quantrule-shell --test report_command a_v2_spec` | 1 passed |
| `cargo clippy -p quantrule-model -p quantrule-shell --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |

The `Sign` column of the indicator checklist is still open; this contract does
not claim independent verification.
