# QuantRule — Architecture Characteristics

## 1. Method

The characteristics are derived from the kata's demands and kept short: three
drivers, one enabling characteristic, and the supporting properties that protect
the product promise. Each characteristic has a measurable target and current
evidence. The style and quantum are recorded after the table.

## 2. Derivation

| Kata demand | Characteristic |
|---|---|
| Same input produces the same evidence | Reproducibility |
| No look-ahead; events only when knowable | Temporal integrity |
| Evidence can be inspected and defended | Auditability |
| Every reading and rule is testable | Testability (enabling) |
| One local pipeline with clear boundaries | Modularity |
| A Linux installer a researcher can run | Deployability |
| Contracts can evolve without rewriting the engine | Evolvability |
| No cloud bill and no data egress | Cost and local security |

## 3. Driving characteristics

| Characteristic | Target | Measurement | Current score (1–5) | Evidence |
|---|---|---|---|---|
| Reproducibility | A fixed fixture run is byte-identical across repeated runs | Rerun comparison, golden fixtures | 5 | Determinism and byte-identical rerun tests |
| Temporal integrity | No output at bar *t* changes when later bars are appended | Causality and prefix tests | 5 | Model, event, and strategy boundary tests |
| Auditability | Every evidence bundle names data version, config hash, code commit, artifacts | Provenance schema check and export review | 4 | Model and stage provenance; research memory is new |
| Testability (enabling) | Fast suite plus desktop E2E and installer smoke | `cargo test`, UI E2E, install test | 4 | 1,443 Rust tests; UI tests are new |

## 4. Supporting characteristics

| Characteristic | Target | Measurement | Current score (1–5) |
|---|---|---|---|
| Modularity | One crate per pipeline stage; no dependency cycles | Crate graph and structural checker | 5 |
| Deployability | Clean Linux install in under five minutes | Installer smoke test | 3 (new) |
| Evolvability | Versioned configs and research-memory schema | Contract and migration tests | 4 |
| Cost | No cloud bill; static download site | Hosting and infrastructure inventory | 5 |
| Local security | No engine network calls; data stays local | Offline test and network isolation | 4 |

## 5. Deliberately not optimized

| Characteristic | Decision | Consequence |
|---|---|---|
| Scalability | Single user, single machine | No horizontal scaling |
| Elasticity | Batch runs, no service | No autoscaling |
| Availability | Local app, no uptime target | A failed run is an error, not an outage |
| Multi-tenancy | No accounts or team workspace | Team cloud is future work |
| Interoperability | File contracts only | No API or plugin marketplace |

## 6. Style and quantum

- **Style:** pipes-and-filters inside a modular monolith.
- **Quantum:** one local deployable app with local files and SQLite; the CLI and
  the Tauri UI are adapters over the same application service.
- **Distribution:** a static download page for Linux installers.
- **Future:** a team control plane would be a second quantum; it is not part of
  this increment.

## 7. Trade-offs

| Trade-off | Decision | Consequence |
|---|---|---|
| Local-first vs collaboration | Keep data and compute local | No team sync in v1 |
| Reproducibility vs change velocity | Golden fixtures and versioned contracts | Intentional changes are deliberate |
| Simplicity vs future cloud | Defer the control plane | A future ADR and characteristics review are required |
| Linux-only vs reach | Ship Linux first | Windows and macOS are future work |

## 8. Future trigger

If a collaborative team cloud is introduced, re-derive the characteristics:
security, multi-tenancy, sync, availability, and cost will need new targets. They
are not current drivers.

## 9. Evidence

- `cargo test --workspace`: 1,443 passed, 0 failed, 8 ignored at `v0.8.0`.
- Repository checks: structure, file size, links, terms, command surface.
- The desktop UI, installer, and local research memory are Phase 3 evidence and
  are not yet implemented.
