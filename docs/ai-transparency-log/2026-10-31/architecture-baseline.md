# AI Transparency Log: Architecture Baseline

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: `architecture-baseline`
- Plan: `docs/execution-plans/2026-10-31/architecture-baseline.md`
- Docs: `docs/architecture/README.md`
- Decision: `docs/adr/2026-10-31/adopt-architecture-baseline.md`

## Intent and boundary

The human asked for a product kata and an architecture-characteristics document,
then approved a split into living system documentation under `docs/architecture/`
and a short ADR that adopts the baseline. The work is documentation only. The AI
did not change code, configuration, commands, model formulas, strategy, execution,
portfolio, or analytics. It did not add a dependency, change an existing command,
or make a profitability claim.

## Implemented

- `docs/architecture/kata.md`: the one-page, solution-agnostic product kata, with a
  note on its architecture-kata lineage and adaptation.
- `docs/architecture/characteristics.md`: the method, derivation, three driving
  characteristics, enabling Testability, supporting characteristics, deliberate
  non-goals, style and quantum, trade-offs, baseline, watch items, and references.
- `docs/architecture/README.md`: the system documentation entry point, pipeline,
  crate map, and contract links.
- `docs/adr/2026-10-31/adopt-architecture-baseline.md`: the short adoption decision,
  alternatives, consequences, and links. It does not embed the reference docs.
- `docs/execution-plans/2026-10-31/architecture-baseline.md`: the approved plan of
  record.
- `AGENTS.md`: section 5 gains the living `docs/architecture/<file>.md` kind; section
  2 points to the architecture baseline.
- `docs/README.md`: the Architecture kind, the current architecture table, and the
  new ADR row.

## Verification and evidence

Commands and observed results, run in the task worktree on 2026-10-31:

| Command | Observed result |
|---|---|
| `cargo fmt --all -- --check` | No output; clean |
| `cargo test -q` | Exit 0; 1,506 passed, 0 failed, 7 ignored across 47 test binaries |
| `cargo clippy --all-targets -- -D warnings` | Finished; no warnings |
| `cargo doc --no-deps` | Finished; no warnings |
| `git diff --check` | Clean |
| `rg -n -e Rust -e crate -e SQLite -e cargo -e clippy -e tokio -e serde docs/architecture/kata.md` | No matches |
| Python relative-link check over the new docs | All relative targets exist |
| `git status --short --branch` | Only `AGENTS.md`, `docs/README.md`, and the new docs; no code or configuration |

The test count is a snapshot; the v0.3.0 release note remains the record for that
released run. The docs change exercises no runtime behavior, so the test and lint
runs prove the tree still builds, not that the documentation is correct.

## Learning, rejected or corrected suggestions

- The first proposal put the kata and characteristics inside one ADR. The human
  challenged that, and the online ADR definition confirmed the objection: an ADR is
  a short record of one decision and should link supporting material. The work was
  split into living architecture docs plus a short adoption ADR.
- `katas` was rejected as the document name. The architecture-kata sources use "a
  kata" for one exercise and "Architectural Katas" or "katas" for the collection;
  the single product kata is `kata.md`.
- `docs/system/` was rejected as the folder name because it is broader than the
  content and overlaps with the existing configuration, report, and tutorial
  kinds. `docs/architecture/` is the missing kind.
- A dated `docs/architecture/<YYYY-MM-DD>/` folder was rejected because a living
  baseline is not a work-item snapshot. The ADR remains dated.
- The first ADR draft used relative link paths that resolved one directory too
  shallow. The link check found them, and they were corrected.
- The source draft's "do not drop ... Testability" line contradicted a top three
  that named Auditability. The baseline now says: never shed Reproducibility,
  Temporal integrity, or Auditability; Testability is the enabling instrument.
- The source draft said "one crate per pipeline stage." The crate map was corrected:
  `quantrule-backtest` holds strategy, execution, portfolio, and the runner;
  `quantrule-analytics` is the research engine, not a pipeline stage.
- The test and line-count claims were corrected against the tree. The v0.3.0 release
  note records 1,391 passed tests; the 2026-10-31 run passed 1,506. The line counts
  are a 2026-10-31 snapshot: 98,240 total, 83,729 outside `tests/`, and 34,896 for
  the shell outside its integration tests.
- The diagnosis status is recorded as implemented and awaiting human acceptance, not
  as accepted. Auditability is strong for the accepted surface.
- The kata attribution was corrected after checking the sources: Dave Thomas
  introduced the code kata; Ted Neward adapted it to architectural katas; Neal Ford
  and Mark Richards use the practice in their teaching.

## Limitations

- The architecture baseline is documentation, not a runtime change. No model,
  strategy, execution, portfolio, or analytics behavior is verified here.
- The evidence numbers are snapshots and will age; release notes and later test runs
  are the record.
- Link checking is static and relative-path only; it does not validate external
  URLs.
- A docs commit was created after this entry was written. Push and pull request
  remain gated; the human has not authorized those operations.
- The branch is based on `c8dd519`. `main` advanced to `265341f` with an unrelated
  diagnosis gap fix during the work. The test and lint runs are on the older base;
  the branch must be updated and re-verified before a pull request.
