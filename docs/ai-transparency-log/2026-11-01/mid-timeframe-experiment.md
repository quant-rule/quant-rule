# AI Transparency Log — Mid-Timeframe Experiment

> **Historical record (pre-reset).** This log predates the 2026-11-01 workflow
> reset and uses the retired workflow names. The current workflows are
> [`docs/workflows/audit-workflow.md`](../../workflows/audit-workflow.md) and
> [`docs/workflows/improvement-workflow.md`](../../workflows/improvement-workflow.md).

- Date: 2026-11-01 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential Orchestrator,
  Quant Researcher, Quant Developer, Result Analyst, Triage Officer, and
  verification passes
- Work item: `mid-timeframe-experiment`
- Plan: [`docs/execution-plans/2026-11-01/mid-timeframe-experiment.md`](../../execution-plans/2026-11-01/mid-timeframe-experiment.md)
- Experiment: [`docs/experiments/2026-11-01/EXP-2026-11-01-04-mid-timeframe.md`](../../improvement/INVALIDATIONS.md)
- Claim type: experiment result; no edge or profitability claim

## Intent and boundary

The human approved the `15m` execution / `1h` context protocol after the 1h/4h
matrix was stopped for low sample size and the 2023 data blocker. The AI's
boundary was to pre-register the middle ladder, run the 9 treatment cells, reuse
the identical 5m/15m controls from EXP-03, analyze the result, and stop at the
pre-registered cap.

The AI did not change a threshold, cost, split, or green criterion after seeing
a result. No commit, push, pull request, merge, release, or product-code change
was made. The 2023 data finding remains `Open`; this experiment uses 2024 only.

## What the AI did

| Step | Action | Evidence |
|---|---|---|
| Pre-registration | Wrote the plan, experiment record, shared configs, generator, and analyzer before the matrix | `docs/experiments/2026-11-01/EXP-2026-11-01-04-mid-timeframe.md` |
| Config validation | Ran `policy check` on three cells across the instruments | All printed `policy ok` |
| Matrix | Ran 9 treatment cells into isolated roots | `matrix 9 cells`; five bundles plus `backtest.json` |
| Control reuse | Read the EXP-03 5m/15m control cells for 2021, 2022, 2024 | Same configs, data versions, and source windows |
| Analysis | Computed IS/OOS metrics, costs, drawdown, profit factor, CI, and the green check | `results-mid.json`, `results-mid.txt` |
| Data check | Ran `lab diagnose` on `btcusd-mid-2024` | 1m 98.55%, 15m 95.44%, 1h 91.76%; 48 entry fills; net expectancy -20.77 CI [-39.34, -2.20] |
| Determinism | Reran `btcusd-mid-2024` into a separate root | All five bundles byte-identical |

## Verification and evidence

- The 15m/1h ladder produced 46 BTCUSD and 56 ETHUSD OOS trades, enough to test
  the rule; both are negative. XAUUSD produced 20 trades, below the 30-trade
  floor. The mean OOS return is `-9.27%`.
- The in-sample 2021–2022 returns are also negative (`-20.45%`, `-16.60%`,
  `-11.78%`). The rule does not have a positive edge on the tested sample.
- BTCUSD 2024 gross expectancy is `-1.70` with a 95% CI straddling zero; net
  expectancy is `-20.77` with a CI below zero. Costs amplify a gross edge that
  is not positive.
- The control remains worse: 2024 returns `-19.24%`, `-18.56%`, `-15.39%`;
  BTCUSD and ETHUSD halt.
- The 2024 1h coverage is above the pre-registered 90% threshold for
  BTCUSD/ETHUSD; the 2023 data blocker is not used in this experiment.
- No Rust code, test, fixture, schema, or product contract was changed.

## Learning and rejected suggestions

- **Rejected: lower the 0.1% commission to make the net result positive.** The
  gross expectancy is already negative for BTCUSD; cheaper costs would not
  create an edge, and the cost assumption was pre-registered.
- **Rejected: relax the 30-trade floor for XAUUSD.** The criterion was
  pre-registered; XAUUSD also has a negative return, so a relaxed floor would
  not produce green.
- **Rejected: drop XAUUSD to claim 2/3 positive.** No instrument is positive in
  2024; dropping one would not change the sign.
- **Rejected: use the 2021 ETHUSD `+1.31%` cell as a green signal.** It is
  in-sample, one instrument, 30 trades, and the other in-sample cells are
  negative.
- **Rejected: try a long-only or short-only variant.** The side diagnostic shows
  both directions negative for every instrument in 2021, 2022, and 2024; the
  only positive side is ETHUSD short in 2021, one instrument-year.
- **Rejected: call the 2023 data blocker the reason for this result.** This
  experiment uses 2024 only; the 2023 finding is separate and remains Open.
- **Learning:** the middle ladder fixes the sample-size problem but not the
  edge. The `ladder-context` rule family has now been tested at 5m/15m, 15m/1h,
  and 1h/4h over the available clean sample; none is green. A further attempt
  on this rule would be threshold tuning, not a new hypothesis.

## Limitations of this log

- The AI performed the Governing, Quant, and verification roles sequentially.
  This is not an independent model judgment or a separate-session Quant
  Validator.
- The OOS is 2024 only, a bull regime for crypto and gold; there is no
  bear-market OOS. The 2021–2022 in-sample years are also negative.
- The results are local historical backtests, not live-trading evidence.
- No commit or PR was created, so the branch has no review history yet.
