# Indicator and Parameter Registry

- Date: 2026-11-03 (UTC)
- Updated: 2026-11-06 for the current-surface cleanup
- Status: Populated 2026-11-03; CI check in `tools/check_registry.py`
- Contract: [`docs/model/2026-11-05/model-m1.0.md`](../model/2026-11-05/model-m1.0.md)
- ADR: [`docs/adr/2026-11-03/flat-indicator-event-model.md`](../adr/2026-11-03/flat-indicator-event-model.md)

## 1. Purpose

The registry is the single documented index of every indicator, event, source
code, and behaviour-affecting parameter in the model, policy, execution,
portfolio, and benchmark contracts.

If a number can change a model value, an event, a strategy decision, a fill, a
position, or a reported result, it belongs here. A number that is not in the
registry must not affect the model or an approved strategy.

The registry is documentation and validation metadata. It is not a
user-editable runtime configuration source. The spec parser embeds it at compile
time so event identifiers and roles cannot drift from the published registry.
The engines read their JSON configs; the registry checks that those configs and
the code defaults agree with the documented contract.

## 2. Files

```text
docs/registry/
  README.md          this guide
  schema.json        JSON Schema for the registry file
  indicators.json    indicators, events, parameters, and codes
  validation.json    one validation claim per active indicator and event
  verification.json  one evidence map per active indicator and event
  indicators.md      generated human-readable table
  history.md         current-vs-history boundary and deletion inventory
  deletion-manifest.md   file-by-file P0.1 deletion boundary
  deletion-manifest.tsv  exact path list for P0.1
```

`indicators.json` is the source of truth. `indicators.md` is generated from it
and is never edited by hand.

## 3. Entry types### 3.1 Indicator

| Field | Meaning |
|---|---|
| `id` | Stable snake_case identifier, e.g. `ema` |
| `code` | Stable numeric indicator code |
| `name` | Human name |
| `taxonomy` | Reporting label: `trend`, `momentum`, `structure`, `progression`, `volatility` |
| `role` | `signal`, `context`, or `diagnostic` |
| `claim` | One falsifiable sentence the validation harness tests |
| `formula` | The exact formula or a reference to the contract section |
| `inputs` | Candle fields and other readings the indicator consumes |
| `parameters` | Parameter entries, including fixed constants |
| `outputs` | Published readings and states |
| `availability` | First valid bar and any confirmation delay |
| `events` | Event IDs the indicator can produce |
| `tests` | Required test classes |
| `status` | `active`, `planned`, `retired`, or `experimental` |

### 3.2 Event

| Field | Meaning |
|---|---|
| `id` | Stable identifier, e.g. `level.break` |
| `code` | Stable positive event identity, unique across every event |
| `event_type` | 1 Level, 2 Divergence, 3 Signal |
| `subtype` | Numeric subtype |
| `source` | Numeric source code |
| `indicator` | Owning indicator ID |
| `role` | `signal`, `context`, or `diagnostic` |
| `claim` | One falsifiable sentence, or `none` for a descriptive event |
| `formula` | The exact arithmetic condition that emits the event; required and non-empty |
| `parameters` | Parameter IDs that shape the event |
| `precedence` | Events it suppresses on the same bar, if any |
| `tests` | Required test classes |
| `status` | `active`, `planned`, `retired`, or `experimental` |

`code` is the runtime identity. Several events may share a type, subtype, and
source, but no two events share a code. A retired code is never reused. The model
publishes the code in every event record, and the spec filters, counts, and votes
by it.

### 3.3 Parameter

| Field | Meaning |
|---|---|
| `id` | Stable identifier, e.g. `events.level.test_atr` |
| `path` | Config path, e.g. `defaults.events.level.test_atr` |
| `type` | `integer`, `number`, `boolean`, `string`, `array<...>`, or `object` |
| `default` | Default value; must match the code |
| `min`, `max`, `enum` | Allowed range or set, when applicable |
| `unit` | `bars`, `records`, `fraction`, `atr`, `seconds`, `bps`, `currency`, `period`, or `none` |
| `configurable` | `true` when a run config may change it; `false` for a fixed formula constant |
| `affects` | `model`, `policy`, `execution`, `portfolio`, or `benchmark` |
| `status` | `active`, `planned`, `retired`, or `experimental` |

A fixed formula constant is listed with `configurable: false`. It is documented
so a reader can find it, but it is not a tuning knob. A `configurable: true`
parameter must be accepted by a config parser and recorded in the run
provenance.

### 3.4 Codes

`codes` records the stable numeric registries: event types, event subtypes,
sources, roles, scales, target bases, lifecycle, stages, and unavailable reasons.
A code is never reused after retirement. The 2026-11-05 extension reserves source
codes 19–54 and signal subtypes 10–23 for the indicator-completeness programme.

## 4. Role and claim rules

- `signal`: claims a forward behavioural edge. It may be used in a strategy only
  after an A/B grade from the model validation gate. A C grade is allowed only as
  one explicit human-approved hypothesis.
- `context`: conditions or modulates another measurement. It may filter a
  strategy decision, but it cannot be the sole entry signal.
- `diagnostic`: descriptive only. It is never readable by a strategy.

Every `signal` and `context` entry has a claim. A `diagnostic` entry has
`claim: "none"`.

### 4.1 Validation claims

`validation.json` carries one claim per active indicator and event. A `signal`
event carries a predictive outcome, a horizon, a target, a stop, and a baseline.
A `context` or `diagnostic` event carries `primary_outcome: "none"` and
`predictive_status: "not_applicable"`; it is a construct claim, not an edge
claim. `construct_tests` names the test that exercises the condition, and
`construct_status` must be `covered` for the claim to pass the gate. The
construct coverage, including the session-range defect found and fixed during
P1b-2, is recorded in [`construct-gaps.md`](construct-gaps.md).

`predictive_status: pending` is honest and allowed. It means the construct is
covered but the forward claim has not been tested; it is never counted as a
pass.

## 5. CI checks

`tools/check_registry.py` enforces:

1. The registry root and the per-entry required keys agree with `schema.json`,
   and `schema.json` rejects additional properties.
2. Indicator, event, source, and parameter IDs are unique.
3. Event codes are unique, match `crates/quantrule-model/src/events/codes.rs`,
   and `REGISTRY_VERSION` matches `crates/quantrule-model/src/lib.rs`.
4. Every `configurable: true` parameter has a default config owner; a `fixed`
   parameter is documented but not configurable.
5. Every code default matches the registry default.
6. Every event source is a registered indicator or series.
7. Every `signal`/`context` event has a descriptive claim and at least one
   validation test kind; every `diagnostic` event has `claim: "none"`.
8. `indicators.md` is a current generation of `indicators.json`.

The check fails on drift; it is not a warning. `check_validation.py`
additionally resolves every `construct_tests` entry to a Rust function name.

### 5.1 Verification evidence

`verification.json` maps every active indicator and event to the eleven evidence
classes: formula, causality, determinism, golden, edge, availability,
no-double-count, precedence, math, spec, and emitter. Each class carries a
scope:

- `row` — the test names the row's indicator or event;
- `family` — the test exercises the family that contains the row;
- `tool` — a repository gate covers every row;
- `n/a` — the class does not apply.

`tools/check_verification.py` rejects a missing class, an unknown scope, a
missing Rust test, a row scope that does not name its row, or an unknown tool
gate. `family` means family-level evidence; it is not displayed as per-row
proof.

## 6. Scope

The registry covers **behaviour-affecting parameters**. It does not list syntactic
literals such as array indices, `0`/`1` comparisons, loop bounds, or JSON field
names. The test is: if changing the number changes an output or a decision, it is
in the registry.

## 7. Adding an indicator

The full implementation process is
[`docs/workflows/model-change.md`](../workflows/model-change.md). The research
process that justifies the indicator is
[`docs/workflows/model-research.md`](../workflows/model-research.md).

1. Add the contract section and the registry entry together.
2. Add the numeric codes and the source.
3. Add the event definitions and their claims.
4. Add golden, causality, determinism, and edge-case tests.
5. Add the validation plan and the pre-registered outcome.
6. Run `python3 tools/check_registry.py`.
7. Add the construct tests and the validation claim in `validation.json`.
8. Run `python3 tools/check_validation.py`.
9. Update the generated Markdown.

An indicator without a claim is `diagnostic` and cannot enter a strategy.
