# Model, Strategy, Portfolio, Policy, and Execution Families — Naming Registry

- Date: 2026-11-05 (UTC)
- Updated: 2026-11-06 for the current-surface cleanup
- Status: **Approved** (human, 2026-11-05)
- Scope: model, strategy, portfolio, policy, and execution identifiers
- Claim type: naming and traceability contract; no edge claim

## 1. Purpose

QuantRule uses dot notation with a type prefix. A number alone is ambiguous: a
model version and a strategy version would share it in configs, reports, and
issue titles. The prefix makes the type explicit.

The rule is `<type><family>.<version>`:

| Prefix | Kind | Example |
|---|---|---|
| `M` | Model | `M1.0`, `M3.1` |
| `S` | Strategy | `S3.1` |
| `E` | Execution | `E1.0`, `E3.1` |
| `P` | Portfolio | `P1.0`, `P3.1` |
| `POL` | Policy | `POL1.0`, `POL3.1` |

`M1.0` is the only current model identity. `M2.1–M2.5` is historical with no
accepted baseline; `M3.x` is historical and underpowered; `M4.x` is the latest
model research family, with no accepted baseline. `S3.x`, `P3.x`, `POL3.x`, and
`E3.x` are future and follow an accepted model baseline.

The **S3 command surface** is the stage-command delivery plan; it is not the
`S3.x` strategy family. The two uses of "S3" are distinct: the command surface is
current, the strategy family remains future. The workflow index records the
distinction in [`docs/workflows/README.md`](../../workflows/README.md).

## 2. Registry

| ID | Meaning | Status | Record or contract |
|---|---|---|---|
| `M1.0` | Current model: flat indicator/event model | Baseline; path gate no edge; compression baselines accepted | [`model-m1.0.md`](model-m1.0.md) |
| `M2.1` | M2 model: volume and participation cell | Historical; `model_dev` no edge; not accepted | [M2 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M2.2` | M2 model: structure and volatility cell | Historical; `model_dev` no edge; not accepted | [M2 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M2.3` | M2 model: cross-asset cell | Historical; `model_dev` no edge; not accepted | [M2 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M2.4` | M2 model: session and time cell | Historical; `model_dev` no edge; not accepted | [M2 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M2.5` | M2 model: all-area vote cell | Historical; `model_dev` no edge; not accepted | [M2 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M3.1`–`M3.5` | M3 model: state-conditioned event cells | Underpowered at the count gate; not run | [M3 preflight](../../improvement/2026-11-05/M3-PREFLIGHT.md) |
| `M4.1`–`M4.5` | M4 model: location-conditioned directional event cells | Historical; `model_dev` no edge; not accepted | [M4 result](../../improvement/2026-11-07/M4-DEV-SWARM.md) |
| `S3.x` | Strategies built on an accepted M3.x model | Future; S2.x remains closed | P8/S3.x |
| `E3.x`, `P3.x`, `POL3.x` | Execution, portfolio, and policy after an accepted S3 strategy | Future | S3.x |

`M2.1`–`M2.5` name the five **primary cells** before the run. P5 ran all five on
`model_dev`; none passed, so none is an accepted model version and validation
was not opened. The cell ID is stable either way; a rejected cell is not
renumbered.

## 3. Naming rules

1. **Type prefix is mandatory.** A model ID starts with `M`; a strategy ID
   starts with `S`; execution starts with `E`; portfolio starts with `P`; policy
   starts with `POL`.
2. **Dot notation is `<family>.<version>`.** `M2.1` means model family 2,
   version 1. `S3.1` means strategy family 3, version 1.
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
9. **Current artifacts use the prefixed IDs only.** Historical names are mapped
   once, in [`docs/registry/history.md`](../../registry/history.md); they are not
   repeated in current contracts, code, tests, or README tables.
10. **The registry history log is the source of truth for aliases.**

## 4. Why this scheme

| Alternative | Consequence | Decision |
|---|---|---|
| Pure `1.1`, `1.2` | A model and a strategy collide in configs and reports | Rejected |
| Reuse old strategy version numbers | Collides with the existing strategy history | Rejected |
| `model/2.1`, `strategy/2.1` paths | Unambiguous, but verbose in tables and run IDs | Not chosen; the prefix is shorter and survives a flat namespace |
| Type-prefixed dot notation | Unambiguous, short, stable, and readable in a flat namespace | **Chosen** |

## 5. Where the IDs appear

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

## 6. Family lifecycle

| Stage | Meaning | Workflow |
|---|---|---|
| Exploratory | A premise is tested for coverage and hypothesis generation; no ranking, no validation | [`model-research.md`](../../workflows/model-research.md) |
| Primary | A pre-registered confirmatory cell | [`model-research.md`](../../workflows/model-research.md) |
| Accepted baseline | The human accepts a grade-A/B family as the model baseline | [`improvement-workflow.md`](../../workflows/improvement-workflow.md) |
| Strategy input | A strategy may read the accepted signal/context families | [`strategy-research.md`](../../workflows/strategy-research.md) |
| Retired | The family is no longer used; its ID and history remain | [`deprecation-retirement.md`](../../workflows/deprecation-retirement.md) |

An implementation change to a family is a
[`model-change.md`](../../workflows/model-change.md) change, not a research run.

## 7. Legacy boundary

The naming registry does not authorize deletion. Historical records stay in
their existing history directories and are indexed by
[`docs/registry/history.md`](../../registry/history.md). Current docs, code,
tests, configs, and README tables use only the prefixed IDs in this document.

## 8. Approval

- Approved by: human, 2026-11-05
- Scope: the naming scheme and the current ID registry
- Not authorized: deleting historical records, opening `S2.x`, or renaming old
  artifact paths
