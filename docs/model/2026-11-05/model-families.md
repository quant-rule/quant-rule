# Model, Strategy, Portfolio, Policy, and Execution Families — Naming Registry

- Date: 2026-11-05 (UTC)
- Status: **Approved** (human, 2026-11-05)
- Scope: model, strategy, portfolio, policy, and execution identifiers
- Contract: [`AGENTS.md`](../../../AGENTS.md) sections 2, 8, and 11
- Claim type: naming and traceability contract; no edge claim

## 1. Purpose

QuantRule uses dot notation with a type prefix. A number alone is ambiguous: a
model version and a strategy version would share it in configs, reports, and
issue titles. The prefix makes the type explicit and keeps the historical
v6–v10 strategy names from colliding with the new model family.

The rule is `<type><family>.<version>`:

| Prefix | Kind | Example |
|---|---|---|
| `M` | Model | `M1.0`, `M3.1` |
| `S` | Strategy | `S1.1`, `S3.1` |
| `P` | Portfolio | `P1.0`, `P3.1` |
| `POL` | Policy | `POL1.0`, `POL3.1` |
| `E` | Execution | `E1.0`, `E3.1` |

`M1.0` is the only current model identity. `M2.1–M2.5` is historical with no
accepted baseline; `M3.x` is the next family. `S3.x`, `P3.x`, `POL3.x`, and
`E3.x` are future and follow an accepted M3.x baseline.

## 2. Registry

| New ID | Meaning | Old name | Status | Record or contract |
|---|---|---|---|---|
| `M1.0` | Current model: flat indicator/event model | model v4 | Baseline; path gate no edge; compression baselines accepted | [`docs/model/2026-11-05/model-m1.0.md`](model-m1.0.md) |
| `S1.1` | Confirmed-level breakout strategy | strategy v6 | Historical; experimental, not validated | [`STRATEGY-002-comparison.md`](../../improvement/2026-11-01/STRATEGY-002-comparison.md) |
| `S1.2` | Failed-level-test fade strategy | strategy v7 | Historical; validation failed | [`STRATEGY-003-comparison.md`](../../improvement/2026-11-01/STRATEGY-003-comparison.md) |
| `S1.3` | Event-composite strategy | strategy v8 | Historical; validation pending | [`349-strategy-v8.md`](../../execution-plans/2026-11-03/349-strategy-v8.md) |
| `S1.4` | 1m compression breakout strategy | strategy v9 | Historical; `strategy_dev` no edge | [`STRATEGY-V9-GATE.md`](../../improvement/2026-11-04/STRATEGY-V9-GATE.md) |
| `S1.5` | Compression bracket strategy family, including 5m and fresh-onset variants | strategy v10 | Historical; `strategy_dev` no edge | [`STRATEGY-BRACKET-GATE.md`](../../improvement/2026-11-04/STRATEGY-BRACKET-GATE.md) and [`STRATEGY-ONSET-5M-GATE.md`](../../improvement/2026-11-04/STRATEGY-ONSET-5M-GATE.md) |
| `M2.1` | M2 model: volume and participation cell | new | Historical; `model_dev` no edge; not accepted | M2.x charter (P3); [P5 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M2.2` | M2 model: structure and volatility cell | new | Historical; `model_dev` no edge; not accepted | M2.x charter (P3); [P5 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M2.3` | M2 model: cross-asset cell | new | Historical; `model_dev` no edge; not accepted | M2.x charter (P3); [P5 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M2.4` | M2 model: session and time cell | new | Historical; `model_dev` no edge; not accepted | M2.x charter (P3); [P5 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M2.5` | M2 model: all-area vote cell | new | Historical; `model_dev` no edge; not accepted | M2.x charter (P3); [P5 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M3.1`–`M3.5` | M3 model: state-conditioned event cells | new | Underpowered at the count gate; not run | [M3 preflight](../../improvement/2026-11-05/M3-PREFLIGHT.md) |
| `S2.1+` | Strategies built on an accepted M2.x model | new | Closed until an M2.x baseline is accepted | P8 |

`M2.1`–`M2.5` name the five **primary cells** before the run. P5 ran all five on
`model_dev`; none passed, so none is an accepted model version and validation
was not opened. The cell ID is stable either way; a rejected cell is not
renumbered, and `S2.x` remains closed.

## 3. Historical aliases

| Old name | Alias | Kind | Note |
|---|---|---|---|
| model v4 | `M1.0` | Model | Current model engine; not legacy |
| model v3 | historical label: model v3 | Model | Retained under its original v3 name; pre-family, not part of M1.x or M2.x |
| strategy v6 | `S1.1` | Strategy | Confirmed-level breakout |
| strategy v7 | `S1.2` | Strategy | Failed-level-test fade |
| strategy v8 | `S1.3` | Strategy | Event composite |
| strategy v9 | `S1.4` | Strategy | 1m compression breakout |
| strategy v10 | `S1.5` | Strategy | Compression bracket family |

The alias column is a mapping, not a rewrite. Historical files, configs, run
records, AI transparency logs, audits, plans, and reports keep their original
names. A reader resolves an old name through this table. New artifacts use the
new ID. The `S1.x` strategy-specific configs, contract docs, parsers, and tools
were removed in [#463](https://github.com/quant-rule/quant-rule/issues/463); the
history records and shared engines remain.

## 4. Naming rules

1. **Type prefix is mandatory.** A model ID starts with `M`; a strategy ID
   starts with `S`. A bare `1.1` is not a valid identifier in a new artifact.
2. **Dot notation is `<family>.<version>`.** `M2.1` means model family 2,
   version 1. `S2.1` means strategy family 2, version 1.
3. **A model and a strategy never share an ID.** `M2.1` and `S2.1` are distinct.
4. **Family numbers increase; versions increase within a family.** A family is
   not renumbered after a version is published.
5. **A retired ID is never reused.** A failed cell keeps its ID and its failure
   record.
6. **The prefix is carried in artifacts.** Spec `id`, run manifest, report
   header, issue title, and branch slug use the prefixed ID. The current run
   manifest carries `M`, `POL`, `E`, and `P`; `S` returns with S3.x.
7. **`M1.0` is the baseline, not a candidate.** The current model engine and its
   tests remain. It is the reference against which M2.x is compared.
8. **`S2.x` stays closed** until the human accepts an M2.x model baseline. A
   strategy may not be used to rescue a failed model cell.
9. **Historical names remain in place.** A file named `strategy-v9-compression.md`
   stays that way; the registry maps it to `S1.4`. Renaming old artifacts is a
   separate decision with no evidence value.
10. **The registry is the source of truth for aliases.** A new alias is added
    here before it is used in an issue, config, or report.

## 5. Why this scheme

| Alternative | Consequence | Decision |
|---|---|---|
| Pure `1.1`, `1.2` | A model and a strategy collide in configs and reports | Rejected |
| Reuse v6–v10 for the new models | Collides with the existing strategy versions and their history | Rejected |
| Rename every historical artifact | Large diff, broken links, no evidence value | Rejected |
| `model/2.1`, `strategy/2.1` paths | Unambiguous, but verbose in tables and run IDs | Not chosen; the prefix is shorter and survives a flat namespace |
| Type-prefixed dot notation | Unambiguous, short, stable, and readable in a flat namespace | **Chosen** |

## 6. Where the IDs appear

| Artifact | Field or position | Example |
|---|---|---|
| Model spec | `id`, `family` | `"id": "M2.1"`, `"family": "M2"` |
| Model run record | `model_family`, `model_id` | `M2`, `M2.1` |
| Program manifest | `program_id` | `PROGRAM-2026-11-05-01`; cells name `M2.x` |
| Cell result | `cell_id`, `model_id` | `CELL-M2.1`, `M2.1` |
| Report header | `Model` | `M2.1` |
| Issue title | Prefix | `[M2.1] Volume and participation cell` |
| Branch | Prefix in slug | `feat/410-m2-1-volume-participation` |

The `program_id` keeps the existing `PROGRAM-YYYY-MM-DD-NN` form; the model ID
lives in the cell and in the spec. A run ID keeps the existing `MODEL-...`,
`STRATEGY-...`, or `PORTFOLIO-...` form and records the family in its manifest.

## 7. Legacy boundary

The naming registry does not authorize deletion. The removal list is in
[`410-legacy-inventory.md`](../../execution-plans/2026-11-05/410-legacy-inventory.md). In summary:

- **Keep:** history records (AI transparency logs, improvement run records,
  audits, execution plans, reports), the `M1.0` model engine and tests, and the
  shared data/model/backtest/shell libraries.
- **Remove later:** old strategy-specific configs, old strategy contract docs,
  old strategy-specific parsers, and old strategy-specific evaluation tools
  after a reference check.
- **Do not rename:** historical artifacts. Map them through this registry.

## 8. Approval

- Approved by: human, 2026-11-05
- Scope: the naming scheme and the alias table
- Not authorized: deleting historical records, opening `S2.x`, or renaming old
  artifact paths
