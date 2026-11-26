# Signal Artifact and Reference Position

- Date: 2026-11-22 (UTC)
- Status: Signal artifact contract, pure reference computation and the stateless strategy service implemented (RA-007, RA-008); the additive public signal surface and clients implemented (RA-009); run-v5 catalog/worker reachability implemented (RA-004/RA-005); accepted by [ADR-RA](../../adr/2026-11-22/1082-research-artifact-layer.md)
- Claim type: Behaviour Claim
- Related: [Research packs and run v5](research-artifact-packs.md), [Research artifacts](../../architecture/research-artifacts.md)

The signal artifact is the reusable alpha output of one frozen model artifact and
one signal-mode strategy pack. It records what the alpha would do, not what an
account did. Execution and portfolio policies are not inputs: the same signal
bytes are consumed by every execution-simulation and portfolio policy.

## 1. Inputs and rejection

| Input | Rule |
|---|---|
| Frozen model artifact | The published execution and context timeframes, the rows, the events, and the data version |
| Strategy pack | `kind: strategy`, payload `mode: signal`, validated and frozen |
| Model artifact id | The caller resolves it from the frozen manifest and passes it in |
| Execution pack | Not an input |
| Portfolio pack | Not an input |

A fold-mode pack is refused with `FoldMode`. A signal-mode pack that carries a
non-null `giveback_atr` or a non-zero `giveback_arm_atr` is already refused by
the pack parser. The computation also returns `GivebackUnsupported` if the
engine ever produces a giveback exit, rather than silently dropping it. Giveback
stays fold-mode until candles and MFE are an explicit signal-service input.

The model must publish the pack's execution timeframe and every context
timeframe, and every condition timeframe must be the execution timeframe or a
declared context timeframe. A missing execution timeframe or an empty execution
series is an error, not a zero-row artifact.

## 2. Document

```json
{
  "schema": "quantrule.signal-artifact/v1",
  "strategy_pack_id": "sha256:…",
  "model_artifact_id": "sha256:…",
  "data_version": "fixture-data-v1",
  "timeframe": "5m",
  "period": { "from": "2024-01-01", "to": "2024-12-31" },
  "rows": [
    {
      "bar_index": 0,
      "open_time": 1704067200,
      "available_bar_index": 0,
      "action": "hold",
      "side": null,
      "intent": null,
      "reason": null,
      "evidence": []
    }
  ]
}
```

The artifact carries exactly those fields. It carries no fill, quantity, fee,
cash, equity, P&L, return, edge or other evaluation metric. The closed parser
rejects an unknown field, so a later addition is a schema change, not a silent
extension.

`period` is the model artifact's requested period when it recorded one, and the
first and last execution-bar UTC dates otherwise. `available_bar_index` is the
first bar on which the row was knowable. The writer publishes every row on its
own bar, so the value equals `bar_index`; the reader accepts a later value and
never an earlier one.

## 3. Row fields

| Field | Meaning |
|---|---|
| `bar_index` | Zero-based execution bar in the published timeframe |
| `open_time` | UTC open time as Unix seconds |
| `available_bar_index` | First bar on which the row was knowable |
| `action` | `hold`, `enter`, `exit` or `bracket` |
| `side` | `long`, `short` or null |
| `intent` | Market entry, two-sided bracket, flat exit, or null |
| `reason` | Stable action reason, or null |
| `evidence` | Event and state references |

### Actions, sides and reasons

| Action | Side | Intent | Reasons |
|---|---|---|---|
| `hold` | null | null | null |
| `enter` | required | market or bracket | `entry`, `bracket_fill` |
| `exit` | required for a closed position | flat | `event_condition`, `time_stop`, `stop_loss`, `take_profit` |
| `exit` | null | flat | `bracket_expired`, `bracket_rejected`, `entry_rejected` |
| `bracket` | null | bracket | `bracket_entry` |

An entry or bracket row asks for the next bar. An exit row asks for a flat target.
A reference stop or target exit and a reference bracket fill are published only
when the strategy itself holds; a strategy decision always owns the row. A
pending market fill or exit announced by the previous row is not repeated.

### Intent shapes

```json
{"kind":"market_next_open","stop":{"kind":"atr_multiple","multiple":2.0},
 "target":{"kind":"reward_to_risk","multiple":1.5},
 "valid_until":null,"max_slippage_bps":null}
```

```json
{"kind":"bracket","upper_price":105.0,"lower_price":95.0,
 "stop":{"kind":"atr_multiple","multiple":2.0},
 "target":{"kind":"reward_to_risk","multiple":1.5},
 "valid_for_bars":1,"max_slippage_bps":null}
```

```json
{"kind":"exit"}
```

The current strategy engine produces a market-next-open entry and a bracket with
positive ATR stop and reward-to-risk target. A parsed price stop or limit entry
is accepted by the contract but the reference computation rejects it as
`entry_rejected` or `bracket_rejected`, because it cannot compute a level the
signal cannot express. Adding that behaviour is a future schema and engine
change.

### Evidence

Evidence uses the strategy decision record's event and state shape:

```json
{"kind":"event","event_code":1,"event_type":3,"subtype":1,"source":4,
 "occurred_bar":1,"available_bar":1,"direction":1}
```

```json
{"kind":"state","timeframe_secs":300,"field":"ema_stack","value":-1,"bar_index":3}
```

A hold, stop, target, time-stop or rejection row carries no condition evidence.
An event-condition exit carries the exit conditions; an entry carries the entry
conditions; a bracket decision carries the compression-range event.

## 4. Reference position

The state machine is deterministic and independent of execution and portfolio
policies.

1. A market entry decided on bar `N` opens at bar `N + 1`'s open.
2. A two-sided bracket is live for the next bar. An open at or beyond a boundary
   fills at the open on that side; otherwise the first boundary reached fills
   there. If both boundaries are touched while the open is between them, the
   bracket is `bracket_rejected` because OHLC cannot order the two triggers.
3. An open position carries the pack's ATR stop and reward target, resolved from
   the reference entry and the decision bar's ATR(14):
   `stop = entry ∓ stop_atr_multiple × ATR` and
   `target = entry ± stop_distance × reward_to_risk`.
4. The bracket is evaluated from the entry bar onward. The entry bar checks the
   stop before the target. A later bar resolves a gap at the open before the
   intrabar stop-first rule.
5. A bar with zero volume cannot open a market entry or fill or trigger a
   bracket. A position's bracket stays active across a zero-volume bar. A
   non-contiguous bar rejects a pending entry or bracket; an open position's
   bracket is still evaluated.
6. A market exit decided on bar `N` closes the reference position at bar
   `N + 1`'s open. A reference exit is a target state, not a fill, so it is
   applied even on an untradable bar.
7. The time stop closes a position when
   `bar position − entry position >= time_stop_bars`. It counts published
   execution bars, matching the fold. Zero disables the event-composite time
   stop; the bracket engine requires a positive value.

`entry_rejected` means the promised market entry could not open (zero volume,
gap, missing or invalid ATR, or an unsupported entry style). `bracket_rejected`
means the bracket could not fill or resolve (zero volume, gap, invalid ATR,
ambiguous two-sided trigger, or unsupported risk specification).
`bracket_expired` means the bracket's entry window ended without a trigger.

## 5. Causality and availability

A row reads only the strategy snapshot and the reference position at or before
its own bar. An event enters only when its `available_bar_index` is at or before
the bar. Appending a bar, truncating later bars, or changing a later bar cannot
change an earlier row. This is tested by comparing a full artifact with a
truncated one and with a changed final bar.

## 6. Identity and determinism

- JSON keys are sorted, whitespace is insignificant, and the bytes end with one
  newline.
- The artifact id is the SHA-256 of the stored canonical bytes.
- The reader refuses bytes that parse but are not the canonical form, so one
  document cannot acquire two identities.
- The same model artifact, pack and schema produce the same bytes and id.
- The computation uses no clock, no randomness, and no unordered iteration.
- Execution and portfolio packs do not change the signal bytes.

The model artifact id is the canonical digest over the manifest's
`(name, sha256)` entries. The computation records the id the caller supplies;
`SignalArtifact::verify_sources` refuses a pack, model artifact or timeframe
other than the one the artifact was computed from. The strategy service verifies
the id against the frozen manifest before it writes the artifact.

## 7. Strategy service

`POST /internal/v1/signals` accepts `model_artifact_id`, `data_version`,
`config_id` and `strategy_pack_id` plus the profile header. The service:

1. Reads the pack record and canonical bytes from the data service and requires
   `kind: strategy`, `state: frozen`, a matching id and `mode: signal`. A fold
   pack is refused before any model call or write.
2. Resolves the model with `data_version` + `config_id`, verifies the manifest
   schema, dataset, config and canonical artifact id, then hash- and size-checks
   every listed object and assembles the bundle in memory.
3. Runs the pure computation, re-checks the pack, model id, timeframe, data
   version and content id, and writes the canonical bytes directly under
   `artifacts/<sha256 hex>`.
4. Calls `POST /internal/v1/artifacts/signal/{id}` with `{"bytes": N}`. The data
   service re-reads the object, checks its hash and size, and inserts the
   `catalog.artifacts` row idempotently. A duplicate returns the stored row with
   `created: false`.

The signal bytes never travel in the create request, so the 64 MiB PUT body cap is
unchanged. Clients read the stored document from `GET /signals/{id}/bytes`; the
data service verifies its hash and serves metadata from `GET /signals` and
`/signals/{id}`. The service is stateless: no database, queue, migration or
config root, and one pack plus one model resolution per request rather than per
bar. The catalog row is the pre-run record. Evidence `runs.run_lineage` is
recorded at run submission, not here, because the signal must exist first; that
path adds the `run:<id>` reference. A new signal may stay unreferenced for the
30-day retention window.

## 8. Non-goals

- No fill, quantity, fee, cash, equity or P&L record.
- No edge, return, confidence or statistical metric.
- No fold-mode execution, position-aware giveback, or per-bar network call.
- No signal bytes in the create request and no new reference kind.
- No `runs.run_lineage` write by the strategy service.
