# QuantRule Documentation

The documentation tree is `docs/<kind>/<YYYY-MM-DD>/<file>.md`: the kind comes first, then the UTC date of the work item, then the file. A kind directory appears when it is first used.

## Kinds

| Kind | Holds | Path |
|---|---|---|
| Execution plans | The plan for a substantial work item, per `AGENTS.md` section 11. | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` |
| AI transparency log | The record of material AI contributions for a work item, per `AGENTS.md` section 6. One file per work item, named to match its execution plan. | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` |
| Model | The analytical model's contract: the four axes, the numeric output fields, the event model, and the aggregation rules. The implementation follows it. | `docs/model/<YYYY-MM-DD>/<file>.md` |
| Architecture decision records | A decision, the alternatives considered, and the consequences. | `docs/adr/<YYYY-MM-DD>/<file>.md` |

## Reading a work item

A work item's plan and its transparency log have the same base name and sit in two sibling folders:

```
docs/execution-plans/2026-10-21/14-model-engine.md
docs/ai-transparency-log/2026-10-21/14-model-engine.md
```

The plan says what was decided and why. The log entry says what the AI actually did, what was verified, and what was rejected or corrected. Each log entry links to its plan, and each plan names its log entry in its acceptance criteria.

A work item may also have documents in the other kinds. The model contract and the architecture decision record are referenced from the plan and from each other, and the contract is also referenced from `AGENTS.md` section 2. They carry their own date and name rather than the issue's.

## Dates

Every date segment is a UTC date, per `AGENTS.md` section 12. A work item is filed under the UTC date on which it was recorded, which is not necessarily the local date of the machine that produced it.
