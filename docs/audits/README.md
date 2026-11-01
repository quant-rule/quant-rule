# Audits

An audit is a read-only record of what is wrong, why it matters, and the
remediation options. It is not a plan and it is not an issue. The Planner turns
accepted findings into an execution plan and a fix issue/sub-issue tree. See the
[Audit Workflow](../workflows/audit-workflow.md).

## Location

```text
docs/audits/<YYYY-MM-DD>/<scope>-audit.md
```

One audit run is one document. Findings inside it have stable IDs:

```text
AUD-<YYYY-MM-DD>-<NN>
```

## Severity

| Code | Meaning |
|---|---|
| Critical | Silent wrong results, data corruption, irreproducible/unsafe behaviour, or a blocker with no acceptable workaround |
| Major | A supported workflow is wrong or materially misleading, or the risk is unacceptable for MVP |
| Minor | Edge case, documentation, usability, or limited impact |
| Info | Observation or improvement, not a defect |

Critical and Major findings must have at least two remediation options. Minor
may have one.

## Findings table

| Column | Content |
|---|---|
| ID | Stable `AUD-...` identifier |
| Severity | Critical / Major / Minor / Info |
| Area | data / model / backtest / shell / store / analytics / docs / workflow / security / performance |
| Finding | Short title |
| Evidence | Command and `file:line` |
| Impact | Why it matters |
| Options | A / B / C one-line descriptions |
| Recommended | A / B / C / defer |
| Verification | Test or command that proves closure |
| Status | `Open`, `Triaged`, `Planned`, `In progress`, `Fixed`, `Verified`, `Won't fix`, `Duplicate`, `Stale` |
| Links | Issue / plan / PR / ADR |

Long evidence and option trade-offs belong in a detail block below the table:

```text
### AUD-YYYY-MM-DD-01 — Title

- Expected invariant:
- Observed:
- Root cause:
- Impact:
- Affected contracts/docs:
- Dependencies:
- Effort:

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

## Rules

- Evidence must be reproducible; no claim without a command or code reference.
- Do not modify code, tests, issues, or PRs while auditing.
- Distinguish confirmed defects from hypotheses.
- Record limitations and unexamined areas.
- Do not rewrite evidence after publication; correct it with a dated note.
- Status and links may be updated by the Planner, Execution, and Improvement
  Verifier according to the role rules.
