# AI Transparency Log — MVP v1 Final Contract

- Date: 2026-10-28 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#168](https://github.com/quant-rule/quant-rule/issues/168)
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md)
- Branch/worktree: `docs/168-mvp-v1-contract` in `quant-rule--168-mvp-v1-contract`
- Status: In progress.

## Intent and boundary

C0 freezes the MVP v1 contract before any code changes: seven JSON config files,
five per-engine report bundles, the `backtest` and `report` command surface, the
accounting rules, and the release scope. It also commits the approved execution
plan and its AI log.

The boundary is documentation and example configuration only. C0 changes no
`.rs` file, no `Cargo.toml`, and no test. The report and config documents are the
specification the later work items implement; they are not evidence that the
implementation exists.

## Work performed

1. Wrote the execution plan in a planning session, then committed it as the plan
   of record at `docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`.
2. Resolved three approval gates with the human: the commission example uses
   10,000 cash and 10,000 notional, `report execution` runs the pipeline
   internally, and the release tag is `v0.2.0` with the title MVP v1.
3. Wrote `docs/config/2026-10-28/per-engine-config.md`: the seven files, every
   field, validation, cross-config rules, and error format.
4. Wrote `docs/reports/2026-10-28/per-engine-reports.md`: the five bundles, the
   artifact order, the JSON and CSV shapes, the boundaries, and availability.
5. Wrote `docs/adr/2026-10-28/config-driven-engines-and-reports.md`: the decision,
   the alternatives, and the consequences.
6. Added the seven example files under `config/examples/solusd-2025/`.
7. Updated `docs/README.md` and added superseded notes to the 2026-10-27 program
   plan and boundaries ADR.

## Corrections during planning

- The first draft used a single `ExecutionEngine::step(candle, decision,
  quantity)` call. That cannot size at the decision-bar close, because the bar's
  own fills are only known after the call. The contract now splits the engine
  into `advance(candle)` and `submit(decision, quantity)`.
- The first draft included an order state `Cancelled`. With one position and
  next-open fills, no order can be withdrawn before it resolves, so the state was
  removed rather than left unreachable.
- The first commission example paired 100,000 equity with a 10,000 notional,
  which makes a 0.1% equity rate cost 2% of the position round trip. The example
  now aligns cash and notional at 10,000; the contract did not change.
- The tag discussion clarified that the repository has no git tags. `v0.2.0` is
  the first tag; `0.1.0` is only the workspace version marker.

## Verification and evidence

Commands were run in `quant-rule--168-mvp-v1-contract` on 2026-10-28.

| Command | Observed result |
|---|---|
| `jq empty config/examples/solusd-2025/*.json` | All seven files parse; no output, exit 0 |
| `git diff --stat -- '*.rs' Cargo.toml Cargo.lock` | Empty; no code or manifest change |
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; `Finished dev profile` with no warning |
| `cargo test` | 1,217 passed, 0 failed, 1 ignored (`store_100k`) |
| `cargo doc --no-deps` | Passed; `Generated .../target/doc/quantrule_analytics/index.html and 4 other files` |

A link scan over 149 Markdown files found 0 broken relative links. The scan
resolved every relative link in `docs/` against the filesystem; it did not check
external URLs.

The pending rows are filled before the pull request is opened. No result is
claimed before it is observed.

## Learning

- The commission base is a product decision, not an arithmetic detail. The same
  rate means very different costs depending on the ratio of equity to position
  notional, so the example must make that ratio explicit.
- The execution API shape is driven by causality: sizing needs the post-fill,
  post-mark equity of the same bar, so the engine must separate advancing the bar
  from submitting the next decision.
- A report bundle is easier to reason about when availability is a single file
  (`report.json`) rather than a manifest that can go stale.
