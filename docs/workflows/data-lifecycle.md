# Data Lifecycle Workflow

- Date: 2026-11-07 (UTC)
- Status: Current
- Scope: source onboarding, normalization, quality, versioning, partitions, refresh, and invalidation
- Owner: Data Steward (source and quality), Test Engineer (evidence), Quant Validator (affected baselines)
- Contract: [`docs/config/2026-10-30/data-v3.md`](../config/2026-10-30/data-v3.md), [`data/partitions.json`](../../data/partitions.json)
- Claim type: data lifecycle; no model, strategy, or edge claim.

## 1. Purpose

The Data Lifecycle Workflow answers one question: **is the input valid, versioned,
and admissible for the run that will consume it?**

Data is not a tier. It is the input that every tier depends on. A data defect is
fixed through the Audit Workflow and gets a new data version; every affected
baseline is then re-established. A data change is never used to rescue a failed
hypothesis.

## 2. When to use

Use it when:

- onboarding a new source or instrument;
- repairing gaps, duplicates, out-of-order rows, or zero-volume candles;
- changing a data version, coverage window, or source cadence;
- adding or changing a partition, instrument, or alias;
- declaring volume admissibility;
- refreshing a source for a new year;
- retiring a source or data version.

Do not use it when:

- the change is a model formula: use Model Change;
- the change is a strategy rule: use Strategy Research;
- the change is a policy envelope: use Policy Change;
- the change is a defect in the data engine: use the Audit Workflow.

## 3. Stages

```text
Onboard -> Validate -> Version -> Partition -> Admit -> Consume -> Refresh/Retire
```

| Stage | Question | Owner | Output | Gate |
|---|---|---|---|---|
| 1. Onboard | What is the source, instrument, cadence, and coverage? | Data Steward | Source record | The source is legal and reproducible |
| 2. Validate | Is the series complete, ordered, and well-formed? | Test Engineer | Quality report | No unexplained gap, duplicate, or malformed row |
| 3. Version | What is the data version identity? | Data Steward | Data version | The version is explicit and immutable |
| 4. Partition | Which partition and instruments does it serve? | Data Steward | Partition entry | `check_partitions.py` passes |
| 5. Admit | Is the series admissible for the family that will use it? | Quant Researcher + Data Steward | Admissibility record | The declared rule passes before any outcome |
| 6. Consume | Does the run read only its declared window and version? | Quant Developer | Run record | The manifest and config agree |
| 7. Refresh/Retire | Does the source change or retire? | Data Steward + Planner | Refresh or retirement record | Affected baselines are re-established |

## 4. Source and version identity

- A source is a provider, instrument, timeframe, and coverage window.
- A data version is the identity of the input series: source, timeframe, and
  coverage. It is set by the caller, not inferred from the filesystem.
- A source candle file is immutable. A correction is a new data version.
- A data version is never reused for a different series.
- The data version is recorded in every run manifest and stage provenance.

## 5. Quality gates

A series is valid when:

- the header is the expected OHLCV header;
- every row parses;
- the rows are ordered by time;
- duplicate timestamps are rejected;
- the source cadence is the declared cadence;
- a gap is recorded, not silently filled;
- a zero-volume candle is recorded and handled by the declared rule;
- the coverage window is complete for the declared period;
- the instrument is declared in the partition file.

The data engine owns normalization, timeframe construction, ordering, and gap
handling. The shell performs source access; the engine never reaches for a
source.

## 6. Volume admissibility

Before a volume, participation, or flow family:

- compute the ratio of the largest to the smallest monthly median volume;
- compute the largest month-to-month median step;
- compare both to the declared rule;
- evaluate on the development partition before any outcome;
- record the result per instrument-year;
- exclude an instrument-year that fails;
- do not re-admit an instrument by changing the threshold after a result.

The rule is a data-quality gate, not a signal. The M3 charter is the worked
example.

## 7. Partitions and instruments

`data/partitions.json` declares:

- the timezone and excluded windows;
- each partition's window, use, and instruments;
- the holdout flag;
- the canonical instrument IDs and aliases.

Canonical IDs use the source symbol (`BTCUSD`, `ETHUSD`, `XAUUSD`, `EURUSD`,
`ESANDP500`). Historical research configs may use aliases (`BTC`, `XAU`,
`SP500`); the alias map resolves them without rewriting the historical configs.

A run may not cite an undeclared partition or an undeclared instrument. The
partition checker enforces both.

## 8. Roles and boundaries

| Role | Responsibility | Boundary |
|---|---|---|
| Data Steward | Source, version, partition, quality record | No model or strategy decision |
| Test Engineer | Quality evidence and edge cases | No weakening a check |
| Quant Researcher | Admissibility rule for the family | No post-hoc threshold |
| Quant Validator | Checks affected baselines | Read-only; no repair |
| Human | Approves a new source, partition, or data version | Sole final authority |

## 9. Human gates

| Gate | Human action |
|---|---|
| New source | Approve the source and its legal use |
| New data version | Approve the version identity |
| Partition change | Approve the partition and instruments |
| Admissibility rule | Approve the rule before the first run |
| Affected baselines | Accept the re-baseline or retirement |
| Retirement | Approve the retirement and the history record |

## 10. Green criteria

A data change is green when:

- the source and version identity are explicit;
- the quality report has no unexplained gap, duplicate, or malformed row;
- the instrument and partition are declared;
- the admissibility rule passed before any outcome;
- the run config and manifest agree on the version and window;
- every affected baseline is re-established or retired;
- the change is recorded in the changelog and the AI log.

## 11. Artifacts and records

| Artifact | Path |
|---|---|
| Data contract | `docs/config/<date>/data-v3.md` |
| Partition file | `data/partitions.json` |
| Source record | `data/sources/README.md` and the source file |
| Quality record | `docs/reports/<date>/<scope>-data-validation.md` |
| Data change record | `docs/config/<date>/data-change-<id>.md` |
| Baseline registry | `docs/improvement/baselines.md` |
| Invalidation | `docs/improvement/INVALIDATIONS.md` |
| Changelog | `CHANGELOG.md` |
| AI log | `docs/ai-transparency-log/<date>/<issue>-<slug>.md` |

## 12. Stop conditions

Stop and request human direction when:

- the source is not reproducible or legally usable;
- the cadence or header is not the declared one;
- a gap cannot be explained or recorded;
- the instrument is not declared in the partition;
- the admissibility rule would be chosen after an outcome;
- the change would rescue a failed hypothesis;
- the affected baselines cannot be identified;
- the change requires a new data schema.

## 13. Handoff

| To | Packet |
|---|---|
| Model Research | The new data version and the admissible instrument-years |
| Model Change | The data shape and the edge cases the code must handle |
| Strategy Research | The data version and the partition window |
| Portfolio Research | The data version, costs, and holdout window |
| Recordkeeping | The data change ID and the affected baseline IDs |

## 14. Related documents

- [`model-research.md`](model-research.md)
- [`strategy-research.md`](strategy-research.md)
- [`portfolio-research.md`](portfolio-research.md)
- [`policy-change.md`](policy-change.md)
- [`../config/2026-10-30/data-v3.md`](../config/2026-10-30/data-v3.md)
- [`../config/2026-10-30/policy-and-strategy.md`](../config/2026-10-30/policy-and-strategy.md)
- [`../../data/README.md`](../../data/README.md)
- [`../../data/partitions.json`](../../data/partitions.json)
- [`../../AGENTS.md`](../../AGENTS.md)
