# Architecture Characteristics

- Status: Current; approved 2026-10-31; updated 2026-11-06 for the current-surface cleanup
- Companion: `docs/architecture/kata.md`
- Decision: `docs/adr/2026-10-31/adopt-architecture-baseline.md`
- Normative rules: `AGENTS.md` sections 2, 3, 4, and 12

## 1. Method

Mark Richards and Neal Ford define an architecture characteristic as a non-domain
design consideration that influences structure and is critical to success.
Characteristics are the properties that shape the architecture, not the
functionality the product delivers. They must be measurable; a characteristic that
cannot be tested is a preference, not a driver. The architect's job is not to find
the best architecture but the least worst one, because every characteristic trades
against another. That is the First Law: everything is a trade-off. The Second Law —
why is more important than how — is why the kata precedes this record.

Agile development does not mean no architecture; it means architecture is
discovered and revised in short loops rather than fixed up front. The kata is the
smallest artifact that aligns the human and the agents on the product's purpose.
The characteristics are the measurable guardrails that keep those loops honest. The
ADR records the decision and its trade-offs. None of these is a frozen
specification: the command and configuration contracts are the stable interfaces,
while the kata and the characteristics are reviewed when evidence changes.

The method has four steps:

1. Derive the characteristics from the kata's demands.
2. Keep the driving set short: three drivers and one enabling characteristic.
3. Give each characteristic a fitness function and current evidence.
4. Record the style, the quantum, the deliberate non-goals, and the trade-offs.

Reproducibility, temporal integrity, and auditability appear in the kata as
product promises and here as architecture characteristics. That is not double
counting: the kata states the promise; the characteristics state the structural
properties that keep it. The distinction is analytical, not exclusive.

## 2. Derivation

| Demand from the kata | Characteristic it forces |
|---|---|
| "the same candles and the same configuration produce the same bytes, every time" | Reproducibility (determinism) |
| "no look-ahead"; events appear only when knowable | Temporal integrity (causality) |
| "inspect engine by engine, and defend in a validation record"; refuse stale labs | Auditability (evidence traceability) |
| "every reading and event has ... evidence"; boundaries must be provable | Testability (enabling) |
| "pure filters"; one-way dependencies; engines separate from the application service and shell | Modularity |
| "local, single-user, one process"; no server, account, or cloud runtime | Simplicity and deployability |
| "revise it through a pre-registered experiment"; prior evidence remains valid | Evolvability |
| "every capability as a discoverable command"; help is the interface today | Discoverability |

The characteristics are not a generic list of "-ilities." Each one is tied to a
sentence in the kata.

## 3. Driving characteristics (top three)

| Characteristic | Type | What "good" means here | Fitness function | Current evidence |
|---|---|---|---|---|
| **Reproducibility (determinism)** | Custom / operational | Same candles + data version + configuration produce byte-identical artifacts, interactive or not. Pure engines; no clock, randomness, or I/O in the signal path; fixed accumulation order; ordered collections; atomic writes; provenance. | Determinism tests; byte-identical rerun tests; golden fixtures; lab identity checks. | Strong. Determinism and byte-identical rerun tests pass. The v0.2.0 release recorded a full-year run twice, with 29 files byte-identical across roots and on stdout. |
| **Temporal integrity (causality)** | Custom / cross-cutting | No value at bar *t* depends on a later bar; every event appears only at or after its availability bar. Availability bars; as-of joins; confirmed pivots; immutable event records. | Causality and prefix tests per reading, event, and model run; availability tests. | Strong. Causality, prefix, availability, and merged-event-order tests cover the model suite. |
| **Auditability (evidence traceability)** | Cross-cutting | Every result names its data version, config hash, model version, period, and artifacts; a bundle is refused when its target is not empty. Provenance; report bundle; ADR trail. | `provenance.json`; byte-identical rerun tests; report-bundle tests. | Strong for the current surface. |

## 4. Enabling characteristic: Testability

Testability is not a fourth driver. It is the instrument that makes the three
drivers credible: a reproducibility claim is only as good as the test that reruns
the bytes, and a causality claim is only as good as the prefix test that appends
future bars and asserts the past does not move. It is nonetheless non-negotiable.

Every reading and event carries unit, fixed-expected-value, causality,
determinism, and edge-case tests. The backtest crate's `StrategyInput` boundary
has a test proving it exposes only bar identity and the published progression
values; that boundary is current. The crate graph and the shell/engine boundary
have source-level guards.

Fitness function: `cargo test`; golden fixtures; boundary tests; architecture
guards; `cargo clippy --all-targets`; the structural checker. Current evidence:
the current suite passes at 1,333 tests; the execution plan records the observed
counts. The count is evidence of investment, not the goal: a passing test that
does not reach the behavior proves nothing.

The [Refactor Workflow](../workflows/refactor-workflow.md) owns the periodic
structural assessment. It measures testability and modularity before and after a
behavior-neutral change and keeps the structural checker honest.

## 5. Supporting characteristics

| Characteristic | Type | What "good" means here | Fitness function | Current evidence |
|---|---|---|---|---|
| **Modularity** | Structural | One crate per pipeline responsibility plus a shared application service; one-way dependencies; pure engines; the service owns workspace, dataset, configuration, and run I/O. The backtest crate holds the strategy, execution, portfolio, and benchmark engines. | Crate graph; source-level boundary guards; `cargo clippy --all-targets`; structural report; review. | Strong at crate level. The application service is the new seam; the structural report names its largest modules. |
| **Simplicity and deployability** | Operational / structural | One local application; local files and SQLite; no deployed service, account, or server database. Keeps the product a tool, not a platform. | Release build; local binary or installer; no network in tests. | Strong operationally; the command and configuration surface is broad. |
| **Evolvability** | Structural | Contracts are versioned; a new capability arrives as a new command or configuration; model and strategy changes are pre-registered. Protects prior evidence. | Versioned schemas; migration tests; ADR and plan index; command contract tests. | Partial: strong process, expensive contract changes. |
| **Discoverability** | Cross-cutting | Every command, argument, and output shape is reachable from help; output is fixed-width and deterministic. A capability that cannot be typed does not exist. | Help tests; `help all`; CLI tests. | Strong. Help and `help all` render from one command table and are tested. |

## 6. Deliberately not optimized

| Characteristic | Decision | Consequence |
|---|---|---|
| **Availability / continuity** | A run is a command; a failure is an error, not an outage. | No high availability, failover, or uptime target. |
| **Scalability / elasticity** | One user, one process, vertical only; no concurrent multi-user workload. | No distributed complexity; bounded by local memory and time. |
| **Low latency** | Batch and offline; the README's full-year run is about 20 seconds. | Determinism and simplicity win over speed. |
| **Security / privacy** | Local trust boundary; no authentication, no multi-tenancy; the only network path is explicit source fetch in the shell. | Not a shared service without redesign. Security is bounded, not ignored: secrets and source endpoints still follow the repository rules. |
| **Interoperability** | File contracts only: CSV, NDJSON, JSON. | Stable and portable; no API. |

## 7. Style and quantum

**Style.** Pipes-and-filters (pipeline) inside a modular monolith. Each filter is a
pure transformation with an explicit input and output; the application service and
the shell are adapters at the edge, not a fifth stage. The style is chosen for modularity, independent
verification, and deterministic composition, not for throughput or fault
tolerance.

**Quantum.** An architecture quantum is the smallest independently deployable unit
with high functional cohesion and synchronous connascence. QuantRule has one: one
local application and one process, with local files and SQLite as its only state.
The command shell and the desktop UI are adapters over the same application
service; there is no independently deployable engine and no shared cloud service.
A change to any crate is a rebuild of the same local application; that is acceptable
for a single-user local tool. A team control plane would be a second quantum and
requires a new ADR.

```text
shell / desktop ui (adapters: parse, fetch, render)
 └─ application service (workspace, dataset, config, run, export)
     ├─ data → model
     └─ backtest (strategy, execution, portfolio, benchmark engines)
```

### Crate map

| Crate | Role | Depends on (workspace) |
|---|---|---|
| `quantrule-data` | Data engine: normalization, timeframe ladder, ordering, gaps. | — |
| `quantrule-model` | Model engine: readings, states, confirmed levels, events, progression, context, and the flat report. | `quantrule-data` |
| `quantrule-backtest` | Strategy, execution, portfolio, and benchmark engines. | `quantrule-data`, `quantrule-model` |
| `quantrule-app` | Application service: workspace, dataset, configuration, model run, and export. | `quantrule-data`, `quantrule-model`, `quantrule-backtest` |
| `quantrule-shell` | The command shell: parse, call the application service, render, and explicit source fetch. | `quantrule-app`, `quantrule-data`, `quantrule-model`, `quantrule-backtest` |

### Structural constraints

- Dependencies point one way; no engine reaches upstream; no circular dependencies.
- Engines are pure: no I/O, clock, randomness, or global mutable state. The
  application service and the shell are the impure layers.
- The shell holds no domain logic: it parses a command, calls the application
  service or a pipeline operation, and renders the result.
- Contract changes are schema-versioned and human-gated; a new capability arrives as
  a new command or configuration, not as a silent change to an existing contract.
- `unsafe` is forbidden; library code does not panic on invalid input; errors are
  explicit.

**Why this style is the least worst.** A pipeline gives each stage a narrow,
testable contract and makes the data flow auditable, which directly serves the
three drivers. The modular monolith avoids distributed complexity the product does
not need: one user, one process, local files. Microservices and event-driven
architecture add distribution and asynchrony that make byte-identical determinism
harder without a multi-user need. A layered monolith hides the data flow that
auditability depends on. A plugin architecture is speculative; a new capability
arrives as a new command. An embedded third-party library contradicts the product
boundary; the application service is internal to the workspace. The pipeline's documented weaknesses in scalability and fault tolerance
are deliberate non-goals. The style is not a claim of universal bestness; it is the
least-worst fit for the drivers.

## 8. Trade-offs (First Law)

| Trade-off | Decision | Consequence |
|---|---|---|
| **Reproducibility vs change velocity** | Golden fixtures and byte-identical output make intentional changes expensive. | Accepted; trust is the product. Changes are deliberate and pre-registered, not casual. |
| **Causality vs signal richness** | Confirmed pivots lag; no centered indicators. | Accepted; look-ahead is a defect, not a simplification. |
| **Auditability vs storage and time** | The model bundle duplicates some readings and events across JSON and CSV. | Accepted; local storage is cheap, and provenance keeps the copies traceable. |
| **Simplicity vs capability** | The shell is about 12,000 non-integration-test lines. | Watch adapter creep; extract fetch or render only if the boundary stops being cohesive. Extraction is a human-gated architecture change. |
| **Testability vs test runtime** | The cutover suite is broad. | Accepted; keep the fast suite fast and the extended benches ignored. The count is not the goal; characteristic coverage is. |
| **Evolvability vs contract stability** | Command and configuration changes are human-gated; a new capability arrives as a new command. | Accepted; prior evidence must remain valid. |
| **Priority if forced to shed one** | Shed Evolvability first; versioning and process can buy it back. Never shed Reproducibility, Temporal integrity, or Auditability. Testability is the instrument that keeps them honest. | A change that invalidates prior evidence is an auditability failure, not an evolvability trade-off. |

## 9. Current baseline

Snapshot on 2026-11-11, measured in the task worktree. This is evidence, not a
guarantee; release notes are the record for released runs.

| Item | Observed |
|---|---|
| Workspace | Five crates; version 0.3.0; edition 2024; Rust 1.99.1 pinned |
| Size | The structural report scans `src`: 216 Rust source files and 68,883 lines. The complete tree, including integration and inline tests, is 257 files and 81,690 lines; the shell is 18,247 lines across 68 files and the application service is 7,370 lines across 24 files. |
| Largest file | 973 lines at the preflight; the file-size gate remains at 1,000 |
| Tests | The full workspace suite passes: 1,452 passed, 0 failed, 8 ignored; the file-size and structural gates pass |
| Model report | The `report model` bundle over a fixture run |
| Unsafe | Forbidden at every crate root |

The structural report is the reproducible source for the size snapshot:
`python3 tools/check_structure.py --report`. Hand-counted line totals are
historical evidence, not a contract.

## 10. Watch items and review triggers

- **Shell size.** The shell is larger than any engine. If it keeps growing, the
  adapter is becoming the system. Trigger: a shell module that holds domain logic,
  or a second deployable. Action: an ADR to extract fetch or render, with a human
  decision.
- **Test runtime.** The fast suite must stay fast. If its runtime starts to
  discourage running it, split it into fast and extended tiers rather than weaken
  coverage.
- **Contract changes.** A change to a command, configuration schema, report schema,
  or model version triggers a review of Evolvability and Auditability.
- **Structural drift.** A file over 600 lines, a module with more than one
  responsibility, a new cross-module import, or a structural checker finding
  triggers a refactor assessment. The assessment, not a line count alone, decides
  whether a seam is worth changing.
- **New characteristic.** Keep the list short. A new driver needs a fitness function
  and a trade-off against the existing three; it does not get added by default.
- **No edge.** If the loop concludes "no edge," the product's scope may shrink.
  Re-derive the characteristics rather than preserving structure for its own sake.

## 11. References

- Mark Richards and Neal Ford, *Fundamentals of Software Architecture: An
  Engineering Approach*, O'Reilly, 2020.
- The Agile Manifesto (2001), for the preference for working software and responding
  to change over comprehensive documentation and a fixed plan.

## 12. Links

- `docs/architecture/README.md`
- `docs/architecture/kata.md`
- `docs/workflows/refactor-workflow.md`
- `docs/adr/2026-10-31/adopt-architecture-baseline.md`
- `AGENTS.md` sections 2, 3, 4, and 12
- `docs/releases/2026-10-30/v0.3.0.md`
- `docs/adr/2026-10-30/abstraction-levels-and-engine-boundaries.md`
