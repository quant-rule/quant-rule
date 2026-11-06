# Refactor Assessments

A refactor assessment is a read-only record of where the program's structure
costs more than it should: a module with more than one responsibility, a
dependency that points the wrong way, a file that has grown past its seam, a
test that does not prove the behavior it names, or a repository layout that no
longer reads clearly. It is not a plan, an issue, or a defect report.

The Planner turns accepted findings into an execution plan and a refactor
issue/sub-issue tree. See the [Refactor Workflow](../workflows/refactor-workflow.md).

## Location

```text
docs/refactors/<YYYY-MM-DD>/<scope>-refactor.md
```

One assessment run is one document. Findings inside it have stable IDs:

```text
REF-<YYYY-MM-DD>-<NN>
```

## Impact

Refactor findings are structural, not correctness defects. Use impact levels,
not audit severity.

| Level | Meaning |
|---|---|
| High | Violates a declared boundary or architecture characteristic; blocks testing or change; a module with mixed responsibilities that already causes defects |
| Medium | Measurable coupling or cohesion friction with a bounded fix and no behavior risk |
| Low | Naming, placement, duplication, documentation, or readability with limited impact |
| Info | Observation or opportunity, not a problem |

High and Medium findings carry at least two remediation options. Low may carry
one. Info requires no action.

## Findings table

| Column | Content |
|---|---|
| ID | Stable `REF-...` identifier |
| Impact | High / Medium / Low / Info |
| Area | data / model / backtest / shell / tests / docs / tools / layout / performance |
| Finding | Short title |
| Evidence | Command and `file:line` |
| Principle | coupling / cohesion / boundary / atomicity / testability / readability / duplication / performance |
| Options | A / B / C one-line descriptions |
| Recommended | A / B / C / defer |
| Verification | Test, check, or metric that proves closure |
| Status | `Open`, `Triaged`, `Planned`, `In progress`, `Refactored`, `Verified`, `Deferred`, `Won't refactor`, `Duplicate`, `Stale` |
| Links | Issue / plan / PR / ADR / audit finding |

Long evidence, before-and-after metrics, and option trade-offs belong in a
detail block below the table:

```text
### REF-YYYY-MM-DD-01 — Title

- Expected invariant:
- Observed:
- Structural cause:
- Impact:
- Affected contracts/docs:
- Behavior risk:
- Dependencies:
- Effort:
- Metric before:
- Metric target:

#### Options
A. ...
B. ...
C. ...

#### Recommendation
...

#### Verification
...

#### Links
...
```

## Statuses

| Status | Meaning | Moved by |
|---|---|---|
| `Open` | Recorded, not yet reviewed for action | Structure Auditor |
| `Triaged` | Verified against the current code; impact and route agreed | Refactor Planner |
| `Planned` | Covered by an approved execution plan and refactor issue | Refactor Planner after human approval |
| `In progress` | Implementation is underway | Refactor Implementer |
| `Refactored` | Refactor merged; equivalence evidence still to be checked | Refactor Implementer or PR |
| `Verified` | Equivalence and metric improvement independently checked | Refactor Verifier only |
| `Deferred` | Accepted, but not scheduled | Human only |
| `Won't refactor` | Accepted residual structural debt | Human only |
| `Duplicate` | Covered by another finding | Refactor Planner |
| `Stale` | No longer reproduces or no longer applies | Refactor Planner |

`Refactored` is not `Verified`. Only the Refactor Verifier moves a finding to
`Verified`; only the human decides `Deferred` or `Won't refactor`.

## Baseline

Every assessment records the baseline it was measured against:

| Item | Content |
|---|---|
| Commit | `git rev-parse HEAD` |
| Toolchain | `rustc --version`, `cargo --version` |
| Test inventory | `cargo test --workspace -- --list` count |
| Golden fixtures | SHA256 of every fixture file |
| Representative output | Command and SHA256 of the output or bundle |
| Gates | Observed result of every command in the workflow gate |
| Scorecard | Structural report before the change |

The verifier recomputes the same values after the refactor. A baseline is
evidence, not a guarantee.

## Rules

- Evidence must be reproducible; no claim without a command or code reference.
- Do not modify code, tests, issues, or PRs while assessing.
- Distinguish confirmed structural problems from hypotheses and unexamined
  areas.
- Record limitations and unexamined areas.
- Do not rewrite evidence after publication; correct it with a dated note.
- Status and links may be updated by the Planner, Implementer, and Verifier
  according to the role rules.
- A structural problem that is actually a correctness defect belongs in an
  audit, not here.
