# AI Transparency Log — Local data layout

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential implementation and verification passes
- Work item: none; human-approved local workspace change
- Plan: [`docs/execution-plans/2026-10-31/local-data-layout.md`](../../execution-plans/2026-10-31/local-data-layout.md)
- Status: Committed as `509f6ed` and `43a5e29`; pull request [#271](https://github.com/quant-rule/quant-rule/pull/271) open

## Intent and boundary

The human asked for a documented local workspace: raw candle CSVs in
`data/sources/`, one directory per artifact kind, a short committed README in
each directory, and tool defaults that keep the tree clean. They also asked to
delete the stale local artifacts rather than archive them, because the stored
labs predate the current strategy and execution configuration.

The change is documentation, local file layout, command defaults, and example
paths. It does not touch the model, strategy, execution, portfolio, or analytics
behavior, any signal path, or any historical record. Historical plans, audits,
transparency logs, reports, and releases keep their original paths and numbers;
only current documentation was updated. No dependency or `unsafe` code was
added.

## Implemented

- Replaced the blanket `/data/` ignore with a rule that re-includes directories
  and commits only `README.md` at any depth.
- Added nine short READMEs: `data/README.md` plus one in each of `charts`, `db`,
  `labs`, `notes`, `reports`, `research`, `scratch`, and `sources`.
- Changed defaults to `data/sources` for `fetch`, `data/research` for
  `research export`, and `data/db/quantrule.db` for every store command. The
  default now lives in one constant rather than four duplicated literals.
- Updated the `help` descriptions, the glossary entry for a lab folder, and the
  regenerated golden fixtures. The short `help` output did not change.
- Updated `config/examples/solusd-2025/{data,run}.json` and the current
  documentation that names those paths.
- Re-ran the SOLUSD 2025 example against the current config and replaced the
  stale `--json` envelope with the observed one, labelled with its data version
  and capture date. Added a field table and a provenance paragraph.
- Updated the three quant role manuals to the current example numbers and the
  new workspace paths.
- Split the crate-local test-artifact ignore into a separate branch
  `chore/test-artifact-ignore`, because it has no dependency on this layout.
- Cleaned the local checkout: 23 raw CSVs moved to `data/sources/`, the legacy
  `first-run`, `first-run-2`, `verification`, `raw`, `processed`,
  `tutorial-role-notes`, lab, and chart artifacts deleted. `data/` now holds
  eight directories and the 23 sources.

## Verification and evidence

| Check | Observed |
|---|---|
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --all-targets -- -D warnings` | Passed, no warnings |
| `cargo test` | 1,472 passed, 0 failed, across 45 suites including doctests |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | Passed |
| `git diff --check` | Clean |
| `git add -n .` under `data/` | Exactly the nine READMEs listed |
| Example run | `backtest` completed; envelope matches the README |
| Default `store init` | Wrote `data/db/quantrule.db`, schema 2 |
| Default `research export` | Wrote nine files under `data/research/` |
| Default `fetch` | Wrote `data/sources/EURUSD_M1_2026-01-05_2026-01-05.csv`, 1,438 rows |
| Lab summary | `wrote data/labs/lab-result-2026-10-31T03-45-00Z/summary.txt` |
| Envelope values | 100 trades, final equity 8010.411285, max drawdown 0.200542, halted |
| Determinism | A second `backtest --json` printed a byte-identical envelope |
| Local cleanup | 23 CSVs in `data/sources/`; eight directories at the root; 1.6 GB |

## Learning and rejected suggestions

- Rejected keeping `/data/` and committing READMEs with `git add -f`. It leaves
  the tree undocumented in a clone and makes every addition a special case. The
  re-inclusion rule keeps the same guarantee without force.
- Rejected leaving the four duplicated `data/quantrule.db` literals in place.
  Moving the default into `DEFAULT_DATABASE` makes the next path change one
  edit, and `store init`, `store ingest`, `store remove`, and `store verify`
  cannot drift apart.
- Rejected editing historical `docs/{audits,execution-plans,reports,releases}`
  and older transparency logs. They record what was true at the time; rewriting
  their paths would falsify the record.
- Rejected hand-editing the example envelope. The old one was stale because the
  configuration changed, so the run was repeated and the observed bytes were
  pasted unchanged.
- Rejected moving the test-artifact ignore into this change. Mixing an
  unrelated hygiene fix in the same PR would make the diff harder to review.
- Noted, not changed: the local `.git/info/exclude` carried two duplicate
  `data` lines that hid the new rules. They were removed from that local-only
  file; no repository file was affected.
- The role manuals' example numbers are observations of one in-sample run, not
  a profitability claim. The run halts at the configured drawdown limit, and
  the manuals keep that limitation explicit.
