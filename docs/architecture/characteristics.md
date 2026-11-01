# Architecture Characteristics

- Status: Current; approved 2026-10-31
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
| "every axis and rule has ... evidence"; boundaries must be provable | Testability (enabling) |
| "pure filters"; one-way dependencies; engines separate from the shell | Modularity |
| "local, single-user, one process"; no server or GUI | Simplicity and deployability |
| "revise it through a pre-registered experiment"; prior evidence remains valid | Evolvability |
| "every capability as a discoverable shell command"; help is the interface | Discoverability |

The characteristics are not a generic list of "-ilities." Each one is tied to a
sentence in the kata.

## 3. Driving characteristics (top three)

| Characteristic | Type | What "good" means here | Fitness function | Current evidence |
|---|---|---|---|---|
| **Reproducibility (determinism)** | Custom / operational | Same candles + data version + configuration produce byte-identical artifacts, interactive or not. Pure engines; no clock, randomness, or I/O in the signal path; fixed accumulation order; ordered collections; atomic writes; provenance. | Determinism tests; byte-identical rerun tests; golden fixtures; lab identity checks. | Strong. Determinism and byte-identical rerun tests pass. The v0.2.0 release recorded a full-year run twice, with 29 files byte-identical across roots and on stdout. |
| **Temporal integrity (causality)** | Custom / cross-cutting | No value at bar *t* depends on a later bar; every event appears only at or after its availability bar. Availability bars; as-of joins; confirmed pivots; immutable event records. | Causality and prefix tests per axis, event, and strategy; availability tests. | Strong. Causality, prefix, and availability tests cover the model and strategy suites. |
| **Auditability (evidence traceability)** | Cross-cutting | Every result names its data version, config hash, model version, period, and artifacts; stale or mismatched labs are refused. Provenance; lab identity; store verification; diagnosis; ADR trail. | `provenance.json`; lab summary checks; `store verify`; `lab diagnose`. | Strong for the accepted surface. Diagnosis v1 is implemented and awaiting human acceptance. |

## 4. Enabling characteristic: Testability

Testability is not a fourth driver. It is the instrument that makes the three
drivers credible: a reproducibility claim is only as good as the test that reruns
the bytes, and a causality claim is only as good as the prefix test that appends
future bars and asserts the past does not move. It is nonetheless non-negotiable.

Every axis and rule carries unit, fixed-expected-value, causality, determinism, and
edge-case tests. The `StrategyInput` boundary has a test proving it exposes only
primary axis values, bar identity, and the as-of context. The crate graph and the
shell/engine boundary have source-level guards.

Fitness function: `cargo test`; golden fixtures; boundary tests; architecture
guards; `cargo clippy --all-targets`. Current evidence: 1,391 passed, 0 failed, 6
ignored at v0.3.0; a 2026-10-31 run passed 1,506, failed 0, and ignored 7. The count
is evidence of investment, not the goal: a passing test that does not reach the
behavior proves nothing.

## 5. Supporting characteristics

| Characteristic | Type | What "good" means here | Fitness function | Current evidence |
|---|---|---|---|---|
| **Modularity** | Structural | One crate per pipeline responsibility; one-way dependencies; pure engines; the shell owns I/O. The backtest crate holds strategy, execution, portfolio, and the runner; analytics is the research engine, not a pipeline stage. | Crate graph; source-level boundary guards; `cargo clippy --all-targets`; review. | Strong at crate level. The shell is 34,896 non-integration-test lines and is the watch item. |
| **Simplicity and deployability** | Operational / structural | One local binary; files and SQLite; no service, UI, or server database. Keeps the product a tool, not a platform. | Release build; single binary; no network in tests. | Strong operationally; the command and configuration surface is broad. |
| **Evolvability** | Structural | Contracts are versioned; a new capability arrives as a new command or configuration; model and strategy changes are pre-registered. Protects prior evidence. | Versioned schemas; migration tests; ADR and plan index; command contract tests. | Partial: strong process, expensive contract changes. |
| **Discoverability** | Cross-cutting | Every command, argument, and output shape is reachable from help; output is fixed-width and deterministic. A capability that cannot be typed does not exist. | Help tests; `help all`; CLI tests. | Strong. Help and `help all` render from one command table and are tested. |

## 6. Deliberately not optimized

| Characteristic | Decision | Consequence |
|---|---|---|
| **Availability / continuity** | A run is a command; a failure is an error, not an outage. | No high availability, failover, or uptime target. |
| **Scalability / elasticity** | One user, one process, vertical only; no concurrent multi-user workload. | No distributed complexity; bounded by local memory and time. |
| **Low latency** | Batch and offline; the README's full-year run is about 20 seconds. | Determinism and simplicity win over speed. |
| **Security / privacy** | Local trust boundary; no authentication, no multi-tenancy; the only network path is explicit source fetch in the shell. | Not a shared service without redesign. Security is bounded, not ignored: secrets and source endpoints still follow the repository rules. |
| **Interoperability** | File contracts only: CSV, NDJSON, JSON, SQLite. | Stable and portable; no API. |

## 7. Style and quantum

**Style.** Pipes-and-filters (pipeline) inside a modular monolith. Each filter is a
pure transformation with an explicit input and output; the shell is an adapter at
the edge, not a fifth stage. The style is chosen for modularity, independent
verification, and deterministic composition, not for throughput or fault
tolerance.

**Quantum.** An architecture quantum is the smallest independently deployable unit
with high functional cohesion and synchronous connascence. QuantRule has one: one
deployable binary, one process, one local SQLite store. There is no independently
deployable engine, no shared service, and no second surface. A change to any crate
is a rebuild of the same binary; that is acceptable for a single-user local tool.

```text
shell (adapter: parse, fetch, store, render)
 ├─ data → model → backtest (strategy → execution → portfolio)
 └─ analytics (reads data and model; research and diagnosis, read-only)
```

### Crate map

| Crate | Role | Depends on (workspace) |
|---|---|---|
| `quantrule-data` | Data engine: normalization, timeframe ladder, ordering, gaps. | — |
| `quantrule-model` | Model engine: readings, four axes, events. | `quantrule-data` |
| `quantrule-backtest` | Strategy, execution, portfolio, and the deterministic runner. | `quantrule-data`, `quantrule-model` |
| `quantrule-analytics` | Research engine: cross-timeframe research and diagnosis. Not a pipeline stage; the shell owns its I/O. | `quantrule-data`, `quantrule-model` |
| `quantrule-shell` | The binary and only surface: parse, call, render; configuration load, source fetch, store, file I/O. | all of the above |

### Structural constraints

- Dependencies point one way; no engine reaches upstream; no circular dependencies.
- Engines are pure: no I/O, clock, randomness, or global mutable state. The shell is
  the only impure layer.
- The shell holds no domain logic: it parses a command, calls into the pipeline, and
  renders the result.
- Contract changes are schema-versioned and human-gated; a new capability arrives as
  a new command or configuration, not as a silent change to an existing contract.
- `unsafe` is forbidden; library code does not panic on invalid input; errors are
  explicit.

**Why this style is the least worst.** A pipeline gives each stage a narrow,
testable contract and makes the data flow auditable, which directly serves the
three drivers. The modular monolith avoids distributed complexity the product does
not need: one user, one process, one local store. Microservices and event-driven
architecture add distribution and asynchrony that make byte-identical determinism
harder without a multi-user need. A layered monolith hides the data flow that
auditability depends on. A plugin architecture is speculative; a new capability
arrives as a new command. An embedded library contradicts the kata's single
surface. The pipeline's documented weaknesses in scalability and fault tolerance
are deliberate non-goals. The style is not a claim of universal bestness; it is the
least-worst fit for the drivers.

## 8. Trade-offs (First Law)

| Trade-off | Decision | Consequence |
|---|---|---|
| **Reproducibility vs change velocity** | Golden fixtures and byte-identical output make intentional changes expensive. | Accepted; trust is the product. Changes are deliberate and pre-registered, not casual. |
| **Causality vs signal richness** | Confirmed pivots lag; no centered indicators. | Accepted; look-ahead is a defect, not a simplification. |
| **Auditability vs storage and time** | Five bundles plus diagnosis duplicate some data. | Accepted; local storage is cheap, and identity checks keep the copies consistent. |
| **Simplicity vs capability** | The shell is 34,896 non-integration-test lines. | Watch adapter creep; extract store, fetch, or render only if the boundary stops being cohesive. Extraction is a human-gated architecture change. |
| **Testability vs test runtime** | 1,391 tests at v0.3.0. | Accepted; keep the fast suite fast and the extended benches ignored. The count is not the goal; characteristic coverage is. |
| **Evolvability vs contract stability** | Command and configuration changes are human-gated; a new capability arrives as a new command. | Accepted; prior evidence must remain valid. |
| **Priority if forced to shed one** | Shed Evolvability first; versioning and process can buy it back. Never shed Reproducibility, Temporal integrity, or Auditability. Testability is the instrument that keeps them honest. | A change that invalidates prior evidence is an auditability failure, not an evolvability trade-off. |

## 9. Current baseline

Snapshot on 2026-10-31, measured in the task worktree. This is evidence, not a
guarantee; release notes are the record for released runs.

| Item | Observed |
|---|---|
| Workspace | Five crates; version 0.3.0; edition 2024; Rust 1.99.1 pinned |
| Size | 98,240 Rust lines including integration tests; 83,729 outside `tests/`; the shell is 34,896 outside its integration tests (41,387 including them) |
| Tests | 1,391 passed at v0.3.0; 1,506 passed, 0 failed, 7 ignored on 2026-10-31 |
| Full-year run | Five bundles, 29 bundle artifacts, and a root summary in about 20 seconds on a release build |
| Diagnosis | v1 implemented; awaiting human acceptance |
| Unsafe | Forbidden at every crate root |

## 10. Watch items and review triggers

- **Shell size.** The shell is larger than any engine. If it keeps growing, the
  adapter is becoming the system. Trigger: a shell module that holds domain logic,
  or a second deployable. Action: an ADR to extract store, fetch, or render, with a
  human decision.
- **Diagnosis acceptance.** Auditability's current evidence includes diagnosis v1,
  which is implemented but not yet accepted. If acceptance changes the artifact
  contract, update this record.
- **Test runtime.** The fast suite must stay fast. If its runtime starts to
  discourage running it, split it into fast and extended tiers rather than weaken
  coverage.
- **Contract changes.** A change to a command, configuration schema, report schema,
  or model version triggers a review of Evolvability and Auditability.
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
- `docs/adr/2026-10-31/adopt-architecture-baseline.md`
- `AGENTS.md` sections 2, 3, 4, and 12
- `docs/releases/2026-10-30/v0.3.0.md`
- `docs/adr/2026-10-30/abstraction-levels-and-engine-boundaries.md`
- `docs/adr/2026-10-31/diagnosis-artifact.md`
