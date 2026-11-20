# QuantRule Documentation

Work-item documents use `docs/<kind>/<YYYY-MM-DD>/<file>.md`: the kind comes first, then the UTC date of the work item, then the file. Living system documents sit directly under their kind, as `docs/architecture/<file>.md`. A kind directory appears when it is first used.

## Kinds

| Kind | Holds | Path |
|---|---|---|
| Execution plans | The plan for a substantial work item, per `AGENTS.md` section 11. | `docs/execution-plans/<YYYY-MM-DD>/<issue>-<slug>.md` |
| AI transparency log | The record of material AI contributions for a work item, per `AGENTS.md` section 6. | `docs/ai-transparency-log/<YYYY-MM-DD>/<issue>-<slug>.md` |
| Agent roles | The quant and engineering roles, their handoffs, and when each is required. | `docs/roles/<YYYY-MM-DD>/<file>.md` |
| Architecture | The living system documentation: overview, product kata, architecture characteristics, style/quantum, and cross-cutting views. | `docs/architecture/<file>.md` |
| Course deliverables | Submission sources and generated PDFs for the Software Architecture and Development Methodologies courses. | `docs/course/{architecture,methodology}/{src,deliverables}` |
| Model | The analytical model's contract and evidence. The implementation follows it. | `docs/model/<YYYY-MM-DD>/<file>.md` |
| Configuration | The engine JSON configuration: files, fields, validation, and cross-config rules. | `docs/config/<YYYY-MM-DD>/<file>.md` |
| Reports | Historical report bundles and verification records. | `docs/reports/<YYYY-MM-DD>/<file>.md` |
| Releases | The release notes, the release index, and the tag rule. | `docs/releases/<YYYY-MM-DD>/<tag>.md`; `docs/releases/README.md` |
| Architecture decision records | A decision, the alternatives considered, and the consequences. | `docs/adr/<YYYY-MM-DD>/<file>.md` |
| Audits | Read-only findings, evidence, and remediation options. | `docs/audits/<YYYY-MM-DD>/<scope>-audit.md` |
| Refactors | Read-only structural findings, evidence, and refactor options. | `docs/refactors/<YYYY-MM-DD>/<scope>-refactor.md` |
| Governance | The improvement charter, green criteria, autonomy, and stop conditions. | `docs/governance/<YYYY-MM-DD>/<file>.md` |
| Workflows | The living workflow contracts, indexed by `docs/workflows/README.md`. | `docs/workflows/<file>.md` |
| Manual | The living user manual: one chapter per journey, command and GUI reference, glossary, troubleshooting. | `docs/manual/<file>.md` |
| Workflow templates | The skeleton for a new workflow and the data, release, deprecation, and requirements records. | `docs/workflows/templates/<file>.md` |
| Improvement registry | The committed run index, baselines, invalidation tombstones, and run record template; seeded into the evidence research ledger. | `docs/improvement/<file>.md` |
| Port parity | Golden fixtures captured from a retired implementation, and the registry the gate checks. | `docs/port-parity/README.md`; `docs/port-parity/registry.json`; `docs/port-parity/<id>.golden.json` |
| Changelog | The grouped release history: meaningful user-facing or developer-facing changes by release and type. | `CHANGELOG.md` |

The tutorial kind is defined in `AGENTS.md`; current tutorials are under
`docs/tutorials/`.

## Where status lives

| Status area | Owner |
|---|---|
| Product, pipeline, and service status | [`docs/architecture/README.md`](architecture/README.md) |
| Model contracts and evidence | [`docs/model/README.md`](model/README.md) |
| Run contracts | [`docs/config/README.md`](config/README.md) |
| Programme evidence and programme status | [`docs/improvement/programmes.md`](improvement/programmes.md) |
| Roles | [`docs/roles/README.md`](roles/README.md) |
| Releases | [`docs/releases/README.md`](releases/README.md) |
| Architecture decisions | [`docs/adr/README.md`](adr/README.md) |
| Workflows | [`docs/workflows/README.md`](workflows/README.md) |
| Developer tools | [`tools/README.md`](../tools/README.md) |
| User manual | [`docs/manual/README.md`](manual/README.md) |

## Current workflows

The workflow index, selection guide, status vocabulary, record map, and the
per-workflow paths are in [`docs/workflows/README.md`](workflows/README.md).
The index is the single source of workflow status; this page does not repeat it.

## Local workspace

`data/` is a local workspace. Only its README files and `partitions.json` are
committed; candles, reports, and run artifacts stay local. The shell stores only
its default profile under `QUANTRULE_HOME`; research memory lives in the
evidence service.

## Reading a work item

A work item's plan and its transparency log have the same base name and sit in two
sibling folders:

```
docs/execution-plans/2026-10-21/14-model-engine.md
docs/ai-transparency-log/2026-10-21/14-model-engine.md
```

The plan says what was decided and why. The log entry says what the AI actually did,
what was verified, and what was rejected or corrected.

AI transparency logs are point-in-time records. Status lines such as "pull request
is open" describe the moment the entry was written; the current state is in Git,
GitHub, and the execution plan.

## Dates

Every date segment is a UTC date, per `AGENTS.md` section 12. A work item is filed
under the UTC date on which it was recorded, which is not necessarily the local date
of the machine that produced it.
