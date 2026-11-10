# Model, Strategy, Portfolio, Policy, and Execution Families — Naming Registry

- Date: 2026-11-05 (UTC)
- Updated: 2026-11-08 for the research-family prefix (`MR`/`SR`/`PR`) and the
  `-Cnn` configuration suffix; 2026-11-10 for the `M1.3` audit correction
- Status: **Approved** (human, 2026-11-05; research-prefix amendment 2026-11-08)
- Scope: model, strategy, portfolio, policy, and execution identifiers
- Claim type: naming and traceability contract; no edge claim

## 1. Purpose

QuantRule uses dot notation with a type prefix. A number alone is ambiguous: a
model version and a strategy version would share it in configs, reports, and
issue titles. The prefix makes the type explicit.

The rule is `<type><family>.<version>`:

| Prefix | Kind | Example |
|---|---|---|
| `M` | Model | `M1.0` |
| `MR` | Model research family | `MR5.1` |
| `SR` | Strategy research family | `SR1.1` |
| `PR` | Portfolio research family | `PR1.1` |
| `S` | Strategy | `S1.0` |
| `E` | Execution | `E1.0` |
| `P` | Portfolio | `P1.0` |
| `POL` | Policy | `POL1.0` |

`M1.3` is the current model identity. `M2.1–M2.5`, `M3.x`, and `M4.x` are
historical **model research families** run on `M1.0`; they are not model
versions. From `MR5.x` onward, model research families carry the `MR` prefix so
they cannot be confused with a model version. A research cell may carry a
configuration suffix: `MR5.1-C01` is hypothesis 1, configuration variant 01.

The registry history log records the alias mapping in
[`docs/registry/history.md`](../../registry/history.md). `S3.x`, `P3.x`,
`POL3.x`, and `E3.x` are future and follow an accepted model baseline.

The **S3 command surface** is the stage-command delivery plan; it is not the
`S3.x` strategy family. The two uses of "S3" are distinct: the command surface is
current, the strategy family remains future. The workflow index records the
distinction in [`docs/workflows/README.md`](../../workflows/README.md).

## 2. Registry

| ID | Meaning | Status | Record or contract |
|---|---|---|---|
| `M1.0` | Historical model: flat indicator/event model | Superseded by `M1.1`; path gate no edge; compression baselines accepted | [`model-m1.0.md`](model-m1.0.md) |
| `M1.1` | `M1.0` plus opening-range sweep/control events | Model change; MR7 development `no_edge` | [`model-m1.1.md`](../2026-11-08/model-m1.1.md) |
| `M1.2` | `M1.1` plus pivot sequence, Fibonacci, and horizontal-band structure | Historical; corrected by `M1.3` after `AUDIT-2026-11-10-model-structures` | [`model-m1.2.md`](../2026-11-10/model-m1.2.md) |
| `M1.3` | Current model: `M1.2` plus the audit corrections to sequence occurrence, Fibonacci tests, and fixed tolerances | Model behavior correction; validation pending | [`model-m1.3.md`](../2026-11-10/model-m1.3.md) |
| `M2.1`–`M2.5` | Historical model research family 2: volume, structure, cross-asset, session, and all-area cells | `model_dev` no edge; not accepted | [M2 result](../../improvement/2026-11-05/M2-DEV-SWARM.md) |
| `M3.1`–`M3.5` | Historical model research family 3: state-conditioned event cells | Underpowered at the count gate; not run | [M3 preflight](../../improvement/2026-11-05/M3-PREFLIGHT.md) |
| `M4.1`–`M4.5` | Historical model research family 4: location-conditioned directional event cells | `model_dev` no edge; not accepted | [M4 result](../../improvement/2026-11-07/M4-DEV-SWARM.md) |
| `MR5.1`–`MR5.10` | Model research family 5 on `M1.0`: mechanism and path cells | `model_dev` no edge; not accepted | [MR5 result](../../improvement/2026-11-08/MR5-DEV-SWARM.md) |
| `MR6.1`–`MR6.5` | Model research family 6 on `M1.0`: ordered trend/momentum → participation confirmation, reverse-order controls, and a level arm | `model_dev` no edge; not accepted | [MR6 result](../../improvement/2026-11-08/MR6-DEV-SWARM.md) |
| `MR7.1`–`MR7.5` | Model research family 7 on `M1.1`: opening-range sweep/reclaim, no-reclaim control, and cost-aware path gate | `model_dev` no edge; not accepted | [MR7 result](../../improvement/2026-11-08/MR7-DEV-SWARM.md) |
| `S3.x` | Strategies built on an accepted model baseline | Future; S2.x remains closed | P8/S3.x |
| `E3.x`, `P3.x`, `POL3.x` | Execution, portfolio, and policy after an accepted S3 strategy | Future | S3.x |

`M2.1`–`M2.5` name the five historical primary cells. They ran on `model_dev`;
none passed, so none is an accepted model version and validation was not opened.
The cell ID is stable either way; a rejected cell is not renumbered. The same
rule applies to the M3 and M4 research families.

## 3. Naming rules

1. **Type prefix is mandatory.** A model ID starts with `M`; a model research
   family starts with `MR`; a strategy research family starts with `SR`; a
   portfolio research family starts with `PR`; a strategy ID starts with `S`;
   execution starts with `E`; portfolio starts with `P`; policy starts with
   `POL`.
2. **Dot notation is `<family>.<version>` for model/strategy/execution/portfolio/policy.** `M1.0` is model family 1, version 0. `MR5.1` is model research family 5, hypothesis 1.
3. **A configuration suffix is `-Cnn`.** `MR5.1-C01` is hypothesis `MR5.1`,
   configuration variant `01`. The suffix is not a new hypothesis; it is a
   pre-registered parameter variant of the same hypothesis.
4. **A model, a research family, and a strategy never share an ID.** `M1.0`,
   `MR5.1`, and `S1.0` are distinct.
5. **Family numbers increase; versions increase within a family.** A family is
   not renumbered after a version is published.
6. **A retired ID is never reused.** A failed cell keeps its ID and its failure
   record.
7. **The prefix is carried in artifacts.** Spec `id`, run manifest, report
   header, issue title, and branch slug use the prefixed ID. The current run
   manifest carries `M`, `POL`, `E`, and `P`; `MR` returns with the model
   research family; `S` returns with S3.x.
8. **`M1.0` is the baseline, not a candidate.** The current model engine and its
   tests remain. It is the reference against which every research family is
   compared.
9. **`S2.x` stays closed** until the human accepts a model baseline. A strategy
   may not be used to rescue a failed model cell.
10. **Current artifacts use the prefixed IDs only.** Historical names are mapped
    once, in [`docs/registry/history.md`](../../registry/history.md); they are
    not repeated in current contracts, code, tests, or README tables.
11. **The registry history log is the source of truth for aliases.**

## 4. Why this scheme

| Alternative | Consequence | Decision |
|---|---|---|
| Pure `1.1`, `1.2` | A model and a strategy collide in configs and reports | Rejected |
| Reuse old strategy version numbers | Collides with the existing strategy history | Rejected |
| `model/2.1`, `strategy/2.1` paths | Unambiguous, but verbose in tables and run IDs | Not chosen; the prefix is shorter and survives a flat namespace |
| Type-prefixed dot notation | Unambiguous, short, stable, and readable in a flat namespace | **Chosen** |
| Research families under the model prefix (`M5.x`) | Reads as a model version and collides with a future `M5.0` | Rejected |
| `M1.5.x` nested research | Still reads as a model version; needs a new grammar | Rejected |
| Separate research prefix (`MR`, `SR`, `PR`) | Distinguishes research from the model/strategy/portfolio identities | **Chosen** |

## 4.1 Research family grammar

```text
M1.0                  model contract
MR5.1                 model research family 5, hypothesis 1
MR5.1-C01             hypothesis 1, configuration variant 01
PROGRAM-YYYY-MM-DD-NN programme identity
```

The model is always named explicitly in the programme and run records. A
research family is not a model version, and a configuration variant is not a
new hypothesis.

## 5. Where the IDs appear

| Artifact | Field or position | Example |
|---|---|---|
| Model spec | `id`, `family` | `"id": "MR5.1-C01"`, `"family": "MR5"` |
| Model run record | `model_family`, `model_id` | `MR5`, `MR5.1-C01` |
| Program manifest | `program_id` | `PROGRAM-<date>-NN`; cells name `MR5.x-Cnn` |
| Cell result | `cell_id`, `model_id` | `MR5.1-C01`, `MR5.1-C01` |
| Report header | `Model` | `M1.0`; `Research family` `MR5.1-C01` |
| Issue title | Prefix | `[MR5.1] Failed breakout at a confirmed level` |
| Branch | Prefix in slug | `feat/mr5-failed-breakout` |

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
