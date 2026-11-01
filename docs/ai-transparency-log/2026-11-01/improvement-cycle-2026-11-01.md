# AI Transparency Log — Continuous Improvement Cycle (High-Timeframe Trend)

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential Orchestrator,
  Quant Researcher, Quant Developer, Result Analyst, Triage Officer, and
  verification passes
- Work item: `improvement-cycle-2026-11-01`
- Plan: [`docs/execution-plans/2026-11-01/improvement-cycle-2026-11-01.md`](../../execution-plans/2026-11-01/improvement-cycle-2026-11-01.md)
- Experiment: [`docs/experiments/2026-11-01/EXP-2026-11-01-03-high-timeframe-trend.md`](../../experiments/2026-11-01/EXP-2026-11-01-03-high-timeframe-trend.md)
- Claim type: experiment result; no edge or profitability claim

## Intent and boundary

The human asked for the full trading continuous improvement cycle with a green
result. The approved protocol tested whether a higher-timeframe trend ladder
reduces the cost drag that dominated the 5m/15m control. The AI's boundary was:

- read the repository and prior experiments;
- reproduce the published baseline;
- write the pre-registration and deterministic configs;
- run the approved matrix;
- analyze and triage the result;
- record the outcome and stop at the pre-registered cap.

The human approved the protocol, the two treatment variants, the three
instruments, the 2021–2022 / 2023–2024 split, the 0.1% cost assumption, and the
green criteria before the matrix ran. The AI did not change a threshold, cost,
split, or criterion after seeing a result. No commit, push, pull request, merge,
release, or product-code change was made.

## What the AI did

| Step | Action | Evidence |
|---|---|---|
| Preflight | Inspected `main` at `3ed2bef`, worktrees, audits, prior experiment, and the published BTCUSD 2024 baseline | Baseline reproduced byte-for-byte: `-19.64%`, halted, 117 trades, gross `+125.46`, commission `-2085.46` |
| Protocol | Proposed the 1h/4h hypothesis, two brackets, three instruments, the split, costs, and green criteria; asked for approval | Human selected the full protocol and kept 0.1% commission |
| Worktree | Created `feat/improvement-cycle-2026-11-01` in a dedicated worktree | `git status` clean before work |
| Pre-registration | Wrote the plan, experiment record, shared configs, generator, and analyzer before the matrix | `docs/experiments/2026-11-01/` |
| Config validation | Ran `policy check` on five cells across all variants | All printed `policy ok` |
| Matrix | Ran 36 cells into isolated roots | `matrix 36 cells`; all five bundles plus `backtest.json` |
| Analysis | Computed evaluation-year, IS, OOS, drawdown, cost, profit-factor, CI, and green checks | `results.json`, `results.txt` |
| Determinism | Reran one control and one `t2` cell into separate roots | All five bundles byte-identical for both |
| Data check | Ran `lab diagnose` on six treatment OOS cells | BTCUSD/ETHUSD 2023 4h coverage 46%/28%; BTCUSD 2023 zero trades |
| Data audit | Counted 2023 gaps and fetched a missing BTCUSD range from the provider into `/tmp/opencode` | 36,709/48,110 missing 2023 M1 minutes; the fresh fetch reproduces the 1,260-minute Christmas gap; [audit](../../audits/2026-11-01/crypto-2023-data-audit.md) |
| Repository gates | Ran `cargo test --workspace`, `python3 tools/check_links.py`, and `git diff --check` | 1,596 passed, 0 failed, 7 ignored, 47 suites; 314 Markdown files with no broken relative links; diff check clean |

## Verification and evidence

- The matrix is the pre-registered 3 instruments × 3 variants × 4 years.
- The only positive OOS cell is `t2` BTCUSD `+1.39%` over two years, 11 trades;
  its net-expectancy CI lower bound is `-56.02`, so it is not statistically
  distinguishable from zero.
- The control loses 15–35%; BTCUSD and ETHUSD halt, XAUUSD does not halt but
  still loses 31%. The 5m/15m cost drag is not fixed by the higher timeframe.
- The treatments produce 9–11 OOS trades per instrument, below the 30-trade
  floor. The strict context filters are the binding constraint.
- BTCUSD/ETHUSD 2023 4h coverage is a data blocker; the 2023 treatment cells are
  not a clean `no edge` test. The zero-trade BTCUSD 2023 cell is consistent with
  the configured filters and the gaps, not a crash or nondeterminism.
- No Rust code, test, fixture, schema, or product contract was changed.

## Learning and rejected suggestions

- **Rejected: lower the commission after seeing the control lose.** The 0.1%
  assumption was pre-registered and conservative; changing it would be tuning
  to green.
- **Rejected: relax `conflict_count_max` or `late_or_terminal_count_max` in this
  experiment.** That is a new hypothesis and must be a new pre-registration.
  It is recorded as the next candidate, not implemented here.
- **Rejected: report `no edge` across the board.** The 2023 crypto data is
  incomplete; the honest result is `not green` with a data blocker and an
  insufficient sample, not a clean no-edge conclusion.
- **Rejected: keep the 2024 `t2` BTCUSD cell as a green result.** One instrument,
  11 trades, and a CI straddling zero do not meet the pre-registered stability
  criteria.
- **Rejected: re-fetch the same provider to fill the 2023 gap.** A fresh
  three-day BTCUSD fetch reproduced the 1,260-minute Christmas gap and the
  one-minute gaps; the source does not have the missing bars.
- **Rejected: interpolate or carry forward missing M1 bars.** It would fabricate
  candles and violate the candles-only, no-synthetic-data invariant.
- **Rejected: publish partial 4h buckets to rescue the trading result.** The
  complete-bucket rule is a data/model contract; changing it needs an ADR,
  golden review, and a separate audit, not a quiet experiment fix.
- **Rejected: call the control a strategy failure without noting the cost
  assumption.** The 0.1% commission is part of the protocol; the control's
  failure is a cost-drag result at that assumption, not proof about all venues.
- **Learning:** the higher timeframe reduced the number of trades and the cost
  per trade, but the rule's context filters made the sample too small. The next
  experiment must fix the data and increase the sample without changing this
  experiment's criteria.

## Limitations of this log

- The AI performed the Governing, Quant, and verification roles sequentially.
  This is not an independent model judgment or a separate-session Quant
  Validator.
- The results are from local historical data and a deterministic backtest; they
  are not live-trading evidence.
- No commit or PR was created, so the branch has no review history yet.
