# Execution Plan: Mid-Timeframe Cost/Sample Experiment

- Date: 2026-11-01 (UTC)
- Status: Verified — experiment stopped, not green; no edge on the tested sample
- Work item: `mid-timeframe-experiment`
- Owner: Orchestrator (sequential Quant and Verification passes)
- Experiment: [`docs/experiments/2026-11-01/EXP-2026-11-01-04-mid-timeframe.md`](../../experiments/2026-11-01/EXP-2026-11-01-04-mid-timeframe.md)
- Prior experiment: [`docs/experiments/2026-11-01/EXP-2026-11-01-03-high-timeframe-trend.md`](../../experiments/2026-11-01/EXP-2026-11-01-03-high-timeframe-trend.md)
- Data audit: [`docs/audits/2026-11-01/crypto-2023-data-audit.md`](../../audits/2026-11-01/crypto-2023-data-audit.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-01/mid-timeframe-experiment.md`](../../ai-transparency-log/2026-11-01/mid-timeframe-experiment.md)

## 1. Intent

The 1h/4h treatment was too selective (9–11 OOS trades per instrument against a
30-trade floor) and was exposed to the 4h complete-bucket gap. The 5m/15m
control is cost-dominated. The next test is the middle of the ladder: `15m`
execution with `1h` context, 2 ATR stop / 2R target, the same entry filters,
exits, and costs. The intended outcome is a reproducible answer to whether the
middle ladder produces enough trades to meet the policy objective out-of-sample
in 2024, without the 2023 data blocker.

Green is not guaranteed. A negative result is a valid completion.

## 2. Current state

| Item | Observed |
|---|---|
| Prior matrix | 36 cells; no green. Control loses 15–35%; 1h/4h treatments 9–11 OOS trades; only `t2` BTCUSD +1.39% with a CI straddling zero |
| Data audit | `AUD-2026-11-01-70` Open: 2023 BTCUSD/ETHUSD M1 gaps are provider-side; 4h coverage 46%/28%; a fresh fetch reproduces the 1,260-minute gap |
| 1h coverage | 2024 BTCUSD/ETHUSD 91.76%, XAUUSD 66.51% calendar (weekend-inclusive); 2021 BTCUSD 98.72%, ETHUSD 96.67%; 2022 BTCUSD 85.41%, ETHUSD 90.93% |
| Control | The 5m/15m control cells for 2021, 2022, and 2024 are reused from EXP-03 with identical configs and data versions |
| Toolchain | Release binary built from `3ed2bef`; `cargo test --workspace` 1,596 passed |

## 3. Scope

Included:

- Pre-register the `15m`/`1h` hypothesis, matrix, split, costs, green criteria,
  and stop conditions before any treatment cell runs.
- Generate 9 treatment configs: 3 instruments × 3 evaluation years (2021, 2022,
  2024). 2023 is excluded by the data audit.
- Run each treatment cell into an isolated root.
- Analyze in-sample 2021–2022 and out-of-sample 2024.
- Reuse the EXP-03 5m/15m control cells for the same years; record their config
  hash and data versions.
- Record the result, triage, decision, and limitations.

Excluded:

- 2023 crypto: the data audit finding is Open.
- Any code, model, schema, or test change.
- Any threshold, cost, or split change after the result.
- A 4h context, a partial-bucket policy change, or interpolated candles.
- A second provider or a new data source.

## 4. Assumptions and uncertainty

- The 1h context is less sensitive than the 4h context to the provider gaps; the
  2024 1h coverage is 91.76% for BTCUSD/ETHUSD. XAUUSD's 66.51% is calendar
  coverage including weekends, not a missing weekday session.
- 2024 is the only OOS year. It is a bull year for crypto and gold; there is no
  bear-market OOS. This is a regime limitation.
- The control cells from EXP-03 are deterministic and identical; reusing them is
  evidence, not a new run. A new control run would produce the same bytes.
- The 15m/1h ladder may still be too selective. The 30-trade floor is the
  binding risk.

## 5. Decisions

| Decision | Consequence | Approval |
|---|---|---|
| `15m` execution / `1h` context, 2 ATR/2R | Middle cost/sample point; avoids 4h buckets | Human approved 2026-11-01 |
| 2021–2022 IS, 2024 OOS | Avoids the 2023 data blocker; one OOS year | Human approved 2026-11-01 |
| Reuse EXP-03 5m/15m controls | Same configs and data versions; no duplicate run | Human approved 2026-11-01 |
| Keep 0.1% commission, 2 bps, funding | Consistent with the prior matrix | Human approved 2026-11-01 |
| Green = 10% OOS, ≤20% DD, ≥30 trades/instrument, 2/3 positive, PF ≥1.2, CI > 0, contribution ≤60% | Policy-aligned; strict | Human approved 2026-11-01 |
| One matrix, no post-hoc changes | If it fails, stop at `not green` | Human approved 2026-11-01 |

## 6. Acceptance criteria

- The pre-registration exists before the first treatment cell runs.
- All 9 treatment cells complete with five bundles plus `backtest.json`.
- The analysis reports IS and OOS metrics, costs, drawdown, profit factor,
  confidence interval, and the green check.
- One treatment cell is rerun into a separate root; five bundles are
  byte-identical.
- The experiment record and AI log contain the actual result and limitations.
- If the treatment is green, a Quant Validator reproduction is recorded. If not,
  the record says `not green` and no strategy change is proposed.

## 7. Work breakdown

| Step | Work | Role | Paths |
|---|---|---|---|
| W1 | Write the pre-registration and generator | Quant Researcher / Orchestrator | `docs/experiments/2026-11-01/` |
| W2 | Generate and validate the 9 treatment configs | Quant Developer | `docs/experiments/2026-11-01/config/` |
| W3 | Run the treatment matrix | Loop Governor | `target/experiment-EXP-2026-11-01-04/` |
| W4 | Analyze IS/OOS and the green criteria | Result Analyst | `analyze_mid.py`, `results-mid.json` |
| W5 | Triage and record | Triage Officer / Orchestrator | experiment record |
| W6 | Determinism rerun; if green, validate | Quant Validator / Orchestrator | `docs/reports/2026-11-01/` |
| W7 | Update the plan and AI log; prepare for review | Orchestrator | plan, AI log |

## 8. Verification

| Check | Command | Expected |
|---|---|---|
| Config validity | `quantrule policy check --config <cell>/run.json` | `policy ok` |
| Matrix completion | `tools/experiment/run-matrix.sh docs/experiments/2026-11-01/matrix-mid.txt <root>` | `matrix 9 cells`; five bundles per cell |
| Determinism | rerun one treatment; `diff -r` the five bundles | no differences |
| Analysis | `python3 docs/experiments/2026-11-01/analyze_mid.py <root>` | deterministic JSON/text; green check evaluated |
| Docs | `python3 tools/check_links.py` | no broken links |
| Repository | `git diff --check` | clean |

No Rust source or test change is expected. If a product defect is found, stop and
route it to an audit; do not patch the experiment around it.

## 9. Risks and recovery

- **Too few trades:** the 30-trade floor may fail. Record it; do not relax the
  floor.
- **One OOS year:** 2024 is a bull regime; a green result may not generalize.
  This is a recorded limitation, not a reason to change the split after the run.
- **XAU coverage:** the 66.51% calendar figure is weekend-inclusive; the
  diagnosis must be read with that context.
- **Reused control:** the EXP-03 control labs must match the new years and data
  versions; the record cites the exact cells.
- **No product rollback:** only docs/configs are added.

## 10. Stop conditions

- A config error, missing source, nondeterminism, or incomplete bundle that
  cannot be rerun within the harness-error allowance.
- 2024 1h coverage below 90% for BTCUSD/ETHUSD, or a new data blocker.
- The green criteria fail: record `not green` and stop.
- A result requires a threshold, cost, split, or sample change: stop and start a
  new pre-registration.
- Human stop.

## 11. Outcome

All 9 treatment cells completed. The `mid` ladder is less bad than the 5m/15m
control but not profitable: 2024 OOS returns are BTCUSD `-9.07%`, ETHUSD
`-14.88%`, XAUUSD `-3.86%`; the mean is `-9.27%`; only BTCUSD and ETHUSD reach
30 trades. The in-sample 2021–2022 returns are also negative. The treatment is
not green.

This is a clean **no-edge** result for the `ladder-context` rule on the tested
sample, not a data-blocked one. The experiment is stopped without a threshold,
cost, split, or sample change. The data audit `AUD-2026-11-01-70` remains Open
for any future 2023 work. Full result:
[`docs/experiments/2026-11-01/results-mid.txt`](../../experiments/2026-11-01/results-mid.txt).
