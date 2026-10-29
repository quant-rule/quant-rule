# AI Transparency Log — Post-Release MVP v1 Verification

- Date: 2026-10-29 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#177](https://github.com/quant-rule/quant-rule/issues/177), post-release re-verification
- Program: [#141](https://github.com/quant-rule/quant-rule/issues/141)
- Plan: [`docs/execution-plans/2026-10-28/141-mvp-v1-final-contract.md`](../../execution-plans/2026-10-28/141-mvp-v1-final-contract.md) §9
- Branch/worktree: `test/177-post-fix-verification` in `quant-rule--177-mvp-v1-verification`
- PR: [#199](https://github.com/quant-rule/quant-rule/pull/199)
- Status: PR open; checks pass; merge pending.

## Intent and boundary

The 2026-10-28 verification record predates the #189 performance fix and the
#196 Progression change. Its peak-RSS criterion and its release-tag criterion
were still described as unresolved, while the current `main` had already
resolved both. This work re-runs the full-year SOLUSD 2025 matrix on the current
`main` and corrects the record. It changes no engine, command, configuration, or
report format; the only repository changes are the verification report, the
docs index, the plan's evidence line, and this log.

## Work performed

1. Built the release binary in the verification worktree and ran the full-year
   example config twice into separate roots, measuring wall time and peak RSS
   with `/usr/bin/time -v`.
2. Compared the two roots with `diff -rq` and with SHA-256 hashes over all 29
   files, then ran the same command twice to one root and compared stdout.
3. Checked the five bundle artifact counts and the six PNG dimensions.
4. Ran `report portfolio` into a fresh root, refused a non-empty run root, and
   ran a config with a missing source to confirm it left no root behind.
5. Ran `report model` and `market-report` with the same data version and period
   and compared the four legacy artifacts byte for byte.
6. Ran `strategy-run` over the full year and scanned the NDJSON keys for fill,
   position, cash, equity, P&L, quantity, commission, and funding fields.
7. Compared the strategy, execution, and portfolio bundles with the pre-#196
   baseline generated in the #196 session; confirmed the model and analytical
   differences are the intended Progression change.
8. Re-ran `fmt`, `clippy`, `test`, and `doc`.
9. Updated the verification report, the docs index, the plan's evidence line,
   and this log.

## Findings

- **The 2 GiB criterion is met.** The two runs peaked at 900,300 KiB and
  900,816 KiB, with wall times of 21.08 s and 21.37 s. The increase over the
  pre-#196 baseline (811,512 KiB) is the per-bar published Progression leg
  record from #196.
- **The tag criterion is met.** `v0.2.0` points to commit `42e0021` (annotated
  tag object `ef844f6`) and the GitHub release is titled “MVP v1”. The tag does
  not include #196; #196 is a post-release fix intended for the next minor
  release, as its plan states.
- **The strategy, execution, and portfolio bundles are unchanged.** They are
  byte-identical to the pre-#196 baseline. The model and analytical bundles
  differ only because #196 publishes the confirmed Progression leg and the
  continuous chart field.
- **The 2026-10-28 report was stale, not wrong.** Its measurements were accurate
  for the commit it verified. The correction labels them as historical and adds
  the current evidence instead of rewriting them.
- **The Progression goldens remain a known debt.** They are regenerated from the
  Rust implementation until #197 restores the independent replay; the fixture
  headers and the report say so.

## Verification and evidence

Commands were run in `quant-rule--177-mvp-v1-verification` on 2026-10-29.

| Command or check | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Passed; no diff |
| `cargo clippy --all-targets -- -D warnings` | Passed; no warning |
| `cargo test` | 1,350 passed, 0 failed, 6 ignored |
| `cargo doc --no-deps` | Passed; exit 0; no warning |
| Full-year run A | 21.08 s, 900,300 KiB peak RSS, exit 0 |
| Full-year run B | 21.37 s, 900,816 KiB peak RSS, exit 0 |
| `diff -rq` between A and B | No output |
| SHA-256 over 29 files | Identical |
| `cmp` of two `--json` runs, same root | Identical |
| `report model` vs `market-report` | Four artifacts byte-identical |
| `strategy-run` | 2,346 NDJSON lines; no fill, position, cash, equity, P&L, quantity, commission, or funding key |
| `report portfolio` in a fresh root | Created only `portfolio/`, seven files |
| Non-empty run root | Exit 1; existing file untouched |
| Missing source | Exit 1; no run root created |
| Strategy/execution/portfolio vs baseline | Byte-identical |
| Model/analytical vs baseline | Differ, as intended by #196 |
| H1 Progression stage availability | 3,911/8,535 (45.8%), up from 52/8,535 (0.6%) |

## Learning

- A verification record is only current for the commit it measured. When a
  later fix changes a criterion, the record needs an explicit re-verification
  section rather than a silent edit.
- The tag and `main` are different evidence surfaces after a post-release fix.
  The report names both so a reader does not infer that `v0.2.0` contains #196.
- The stdout comparison must use the same output root because the envelope
  includes bundle paths; comparing different roots compares paths, not output.
- The memory budget is sensitive to per-bar retained records. The #196
  published-leg record costs about 89 MiB on this input, which is acceptable
  but worth remembering for the next retention change.

## Limitations

The re-verification uses one machine and one input. It establishes the budget
for this run, not a general performance guarantee. The independent Progression
oracle is still missing; #197 tracks it.
