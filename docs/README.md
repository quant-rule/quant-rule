# QuantRule Documentation

The documentation tree is `docs/<kind>/<YYYY-MM-DD>/<file>.md`: the kind comes first, then the UTC date of the work item, then the file. A kind directory appears when it is first used.

## Kinds

| Kind | Holds | Path |
|---|---|---|
| Execution plans | The plan for a substantial work item, per `AGENTS.md` section 11. | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` |
| AI transparency log | The record of material AI contributions for a work item, per `AGENTS.md` section 6. One file per work item, named to match its execution plan. | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` |
| Architecture decision records | A decision, the alternatives considered, and the consequences. Not yet created; the first decision creates it. | `docs/adr/<YYYY-MM-DD>/<file>.md` |

## Reading a work item

A work item has two files in two sibling folders, with the same base name. To read one, start from its issue and take the slug from the issue title:

```
docs/execution-plans/2026-10-21/5-shell-application-surface.md
docs/ai-transparency-log/2026-10-21/5-shell-application-surface.md
```

The plan says what was decided and why. The log entry says what the AI actually did, what was verified, and what was rejected or corrected. Each log entry links to its plan, and each plan names its log entry in its acceptance criteria.

## Dates

Every date segment is a UTC date, per `AGENTS.md` section 12. A work item is filed under the UTC date on which it was recorded, which is not necessarily the local date of the machine that produced it.
