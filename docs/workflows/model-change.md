# Model Change Workflow

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: adding, changing, or retiring an indicator, event, state, reading, or model family
- Owner: Quant Developer (implementation), Test Engineer (evidence), Quant Validator (behavior change)
- Contract: [`docs/model/2026-11-05/model-m1.0.md`](../model/2026-11-05/model-m1.0.md), [`docs/registry/README.md`](../registry/README.md)
- Claim type: model implementation and registry process; no strategy or edge claim.

## 1. Purpose

The Model Change Workflow answers one question: **how is a model change
implemented without letting the code, the registry, the validation claims, the
verification evidence, and the model family identity drift apart?**

It is the implementation counterpart to
[`model-research.md`](model-research.md). Research decides whether a measurement
deserves promotion; Model Change decides how that measurement is built,
registered, tested, and versioned.

## 2. When to use

Use it when:

- adding an indicator, reading, state, event, level, divergence, or progression
  rule;
- changing a formula, parameter default, availability rule, precedence rule, or
  event identity;
- retiring an indicator or event;
- changing a model family or version.

Do not use it when:

- the change is a strategy rule: use Strategy Research;
- the change is an execution or portfolio mechanic: use Portfolio Research;
- the change is a defect in an existing implementation: use the Audit Workflow;
- the change is a data source or partition: use Data Lifecycle.

## 3. Stages

```text
Requirement -> Contract -> Registry -> Implementation -> Tests -> Validation -> Verification -> Changelog
```

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Requirement | What measurement or behavior is needed, and why? | Quant Researcher | Requirement note | The research or audit route is named |
| 2. Contract | What is the exact formula, input, availability, and output? | Quant Developer + Architect | Contract section | Human approves a behavior change |
| 3. Registry | What is the indicator/event ID, code, role, claim, and parameters? | Quant Developer | `docs/registry/indicators.json` | `check_registry.py` passes |
| 4. Implementation | How is the pure engine implemented? | Quant Developer | Rust code | Engine purity and module docs pass |
| 5. Tests | What proves the behavior and its boundaries? | Test Engineer | Unit, causality, determinism, golden, edge, property tests | `cargo test` passes |
| 6. Validation | What claim does the measurement make? | Quant Researcher + Quant Developer | `validation.json` claim | `check_validation.py` passes |
| 7. Verification | What evidence class covers each row? | Test Engineer | `verification.json` and matrices | `check_verification.py`, `check_verification_matrix.py` pass |
| 8. Changelog | What changed and why? | Implementer | Changelog entry and AI log | `check_changelog.py` passes |

## 4. Adding an indicator or event

1. Add the contract section and the registry entry together.
2. Add the numeric codes and the source.
3. Add the event definitions and their claims.
4. Add golden, causality, determinism, and edge-case tests.
5. Add the validation claim and the pre-registered outcome.
6. Add the construct tests and the verification evidence.
7. Regenerate `indicators.md`.
8. Run `check_registry.py`, `check_validation.py`,
   `check_verification.py`, and `check_verification_matrix.py`.
9. Add the changelog release entry when the change is meaningful, and the AI log entry.

A signal or context entry without a claim is not complete. A diagnostic entry
has `claim: "none"` and cannot enter a strategy.

## 5. Model family and version

- A model identity uses `<type><family>.<version>`, for example `M1.0`.
- A model and a strategy never share an ID.
- A family number increases; a version increases within a family.
- A retired ID is never reused.
- A failed cell keeps its ID and its failure record.
- A new family is a new premise; a version is a change within an accepted
  family.
- The model family registry is
  [`docs/model/2026-11-05/model-families.md`](../model/2026-11-05/model-families.md).

## 6. Roles and boundaries

| Role | Responsibility | Boundary |
|---|---|---|
| Quant Researcher | States the measurement need and the claim | No production code; no post-hoc threshold |
| Quant Developer | Implements the formula, registry, and tests | No hypothesis change; no validation claim |
| Test Engineer | Designs the evidence classes and edge cases | No weakening a test to pass |
| Quant Validator | Independently checks a behavior change | Read-only; no repair |
| Architect | Checks the stage boundary and purity | No approval of a behavior change |
| Human | Approves a model behavior change | Sole final authority |

## 7. Human gates

| Gate | Human action |
|---|---|
| New measurement or formula | Approve the behavior change |
| New dependency | Approve the dependency |
| Model family or version | Approve the identity |
| Retirement | Approve the retirement and the history record |
| Behavior-change merge | Require the Quant Validator record |

## 8. Green criteria

A model change is green when:

- the contract, registry, code, tests, validation claim, and verification
  evidence agree;
- every new indicator and event has a causality test, a determinism test, a
  golden fixture, and the required edge cases;
- the engine remains pure: no I/O, clock, randomness, or global mutable state;
- the registry and generated Markdown are current;
- the validation claim is honest: `pending` is allowed, but it is not a pass;
- the verification matrix covers every active row;
- the full gate passes;
- the changelog release entry (or the reason none is needed) and AI log are present;
- a behavior change has the Quant Validator record.

## 9. Artifacts and records

| Artifact | Path |
|---|---|
| Model contract | `docs/model/<date>/<file>.md` |
| Registry | `docs/registry/indicators.json` |
| Generated table | `docs/registry/indicators.md` |
| Validation claims | `docs/registry/validation.json` |
| Verification evidence | `docs/registry/verification.json` |
| Verification matrices | `docs/model/<date>/indicator-verification.md`, `event-verification.md` |
| Model family registry | `docs/model/2026-11-05/model-families.md` |
| Changelog | `CHANGELOG.md` |
| AI log | `docs/ai-transparency-log/<date>/<issue>-<slug>.md` |

## 10. Stop conditions

Stop and request human direction when:

- the change alters a strategy rule or a policy threshold;
- a new dependency is required;
- the formula, availability, or precedence cannot be stated exactly;
- a test would need to be weakened or a golden fixture regenerated casually;
- the registry and code cannot be made to agree;
- the model family or version is ambiguous;
- the change is actually a defect fix or a refactor.

## 11. Handoff

| To | Packet |
|---|---|
| Model Research | The implemented measurement and the evidence it enables |
| Strategy Research | The published signal/context identity and its claim |
| Registry | The new IDs, codes, roles, claims, and parameters |
| Recordkeeping | The model version, contract, and changelog entry |

## 12. Related documents

- [`model-research.md`](model-research.md)
- [`strategy-research.md`](strategy-research.md)
- [`../model/2026-11-05/model-m1.0.md`](../model/2026-11-05/model-m1.0.md)
- [`../model/2026-11-05/model-families.md`](../model/2026-11-05/model-families.md)
- [`../registry/README.md`](../registry/README.md)
- [`../../AGENTS.md`](../../AGENTS.md)
