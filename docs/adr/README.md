# Architecture Decision Records

- Status: Current
- Contract: a decision, the alternatives, and the consequences

A decision record is accepted once. A later decision supersedes it; the old
record keeps its path and gains a superseded status.

| Decision | Status | Path |
|---|---|---|
| Flat indicator/event model; composite axes removed from decisions | Accepted 2026-11-03 | `docs/adr/2026-11-03/flat-indicator-event-model.md` |
| History, retention, and display policy | Accepted | `docs/adr/2026-10-25/history-retention-and-display-policy.md` |
| Market analysis and full-execution backtest boundaries | Accepted; partly superseded by the 2026-11-05 cutover | `docs/adr/2026-10-27/analysis-and-backtest-boundaries.md` |
| Extended-range M1 window | Accepted | `docs/adr/2026-10-27/extended-range-window.md` |
| Abstraction levels and engine boundaries | Accepted 2026-10-30 | `docs/adr/2026-10-30/abstraction-levels-and-engine-boundaries.md` |
| Rule purity and statistics boundary | Accepted 2026-10-30 | `docs/adr/2026-10-30/rule-purity-and-statistics-boundary.md` |
| Stateless strategy and the position parameter | Accepted 2026-10-30 | `docs/adr/2026-10-30/stateless-strategy-position-parameter.md` |
| One-way timeframe ladder and trade-intent boundary | Accepted 2026-10-30 | `docs/adr/2026-10-30/one-way-timeframe-ladder.md` |
| Trustworthy backtest kata and architecture characteristics | Accepted 2026-10-31 | `docs/adr/2026-10-31/adopt-architecture-baseline.md` |
| One program, two first-class surfaces | Accepted 2026-11-11 | `docs/adr/2026-11-11/one-program-first-class-desktop.md` |
| Stage execution and evidence boundary | Accepted 2026-11-16 | `docs/adr/2026-11-16/01-stage-execution-and-evidence-boundary.md` |
| Retire the desktop distribution and embedded profile | Accepted 2026-11-21 | `docs/adr/2026-11-21/992-retire-desktop-distribution.md` |
| Browser GUI and terminal CLI boundary | Accepted 2026-11-21 | `docs/adr/2026-11-21/992-browser-cli-boundary.md` |
| Zero Python in the repository | Accepted 2026-11-21 | `docs/adr/2026-11-21/992-zero-python-governance.md` |
| Build once, release fast, and time out safely | Accepted 2026-11-21 | `docs/adr/2026-11-21/992-ci-build-once-release.md` |
| Research artifact layer and seventh strategy service | Accepted 2026-11-22 | `docs/adr/2026-11-22/1082-research-artifact-layer.md` |
| HA deployment profile, four databases and reliability contract | Accepted 2026-11-22 | `docs/adr/2026-11-22/1082-ha-deployment-profile.md` |
| Image distribution, signing, and supply-chain evidence | Accepted 2026-11-24 | `docs/adr/2026-11-24/1120-image-distribution.md` |
| Reference topologies, four rack domains, and the managed-stateful boundary | Accepted 2026-11-24 | `docs/adr/2026-11-24/1121-reference-topologies.md` |
| pgBackRest, the derived Patroni image, and the Compose archive | Accepted 2026-11-24 | `docs/adr/2026-11-24/1111-backup-restore.md` |
