# Execution Plan: Improvement Workflow Cycle — High-Timeframe Trend (Pre-Reset)

- Date: 2026-11-01 (UTC)
- Status: Verified — experiment stopped, not green (data and sample-size blockers); pre-reset record
- Work item: `improvement-cycle-2026-11-01`
- Owner: Orchestrator (sequential Improvement Workflow, Quant, and Verification passes)
- Experiment: [`docs/experiments/2026-11-01/EXP-2026-11-01-03-high-timeframe-trend.md`](../../improvement/INVALIDATIONS.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-01/improvement-cycle-2026-11-01.md`](../../ai-transparency-log/2026-11-01/improvement-cycle-2026-11-01.md)

## 1. Intent

Run the next pre-registered trading experiment after two negative results:
EXP-2026-10-30-02 found `no edge` out-of-sample on SOLUSD 2025, and the
published BTCUSD 2024 control lost 19.64% and halted at the 20% drawdown limit.
The diagnosis shows the control's gross P&L was positive but commission and
turnover dominated it: gross `+125.46`, commission `-2085.46`, cost drag
`16.66×` gross.

The intended outcome is a reproducible answer to one question: does a
higher-timeframe trend-continuation ladder capture larger moves and reduce the
cost drag enough to meet the policy objective out-of-sample? A negative answer
is a valid completion; the cycle then records `no edge` and stops.

## 2. Current state

| Item | Observed |
|---|---|
| Repository | `main` at `3ed2bef`, clean, equal to `origin/main` |
| Audit | All 29 findings AUD-2026-10-31-41..69 `Verified`; no open Critical finding |
| Prior experiment | EXP-2026-10-30-02: four SOLUSD 2025 cells, every OOS cell negative; triage `no edge`, stop at the cap |
| Published baseline | BTCUSD 2024 5m/15m control: `-19.64%`, `20.22%` max drawdown, halted; 117 trades; cost drag `16.66×` gross |
| Data | Local M1 sources BTCUSD, ETHUSD, XAUUSD 2018-01-01..2024-12-31; no network needed |
| Toolchain | Release binary built from `3ed2bef`; `cargo test` gate recorded at `fdf2580` in the audit closure |

## 3. Scope

Included:

- Pre-register the hypothesis, matrix, splits, costs, green criteria, and stop
  conditions before any matrix cell runs.
- Generate 36 deterministic run configurations: 3 instruments × 3 variants
  (control, 2 ATR/2R, 3 ATR/3R) × 4 evaluation years (2021–2024).
- Run each cell into an isolated output root.
- Analyze in-sample (2021–2022) and out-of-sample (2023–2024) metrics, costs,
  stability, and the pre-registered green criteria.
- Record the result, triage, decision, and limitations.
- If a treatment passes, run the Quant Validator reproduction and determinism
  checks; if not, record `no edge` and stop.

Excluded:

- Product code, model/strategy implementation, schema, or test changes.
- Any post-hoc threshold, cost, or split change.
- Policy account/objective/risk changes. The policy envelope is fixed; only the
  permitted timeframe ladder is aligned with each variant.
- A profitability claim from one cell, live trading, capacity, or market impact.

## 4. Assumptions and uncertainty

- The 0.1% commission per operation is conservative and is kept unchanged from
  the prior experiment so the result cannot be attributed to cheaper costs.
- Yearly runs reset the portfolio and the model. December is the warm-up month;
  the evaluation year starts on 1 January. A position opened during the warm-up
  and still open on 1 January is a boundary effect and is recorded.
- The 4h context needs 200 bars (about 33 days), so the first days of January
  may have no context. This is part of the causal warm-up.
- The source has weekend, holiday, and provider gaps. The diagnosis records the
  coverage; no gap is filled or removed.
- Three instruments are a stability check, not a cross-sectional proof. A green
  result remains sample-specific.
- The policy target is 10% per 365 days with a 20% maximum drawdown. The
  two-year OOS target is 21% cumulative, or 10% annualized.

## 5. Decisions

| Decision | Consequence | Approval |
|---|---|---|
| Test 1h execution / 4h context | Larger moves per trade, fewer trades; more warm-up | Human approved 2026-11-01 |
| Two treatments: 2 ATR/2R and 3 ATR/3R | One sensitivity check inside one engine; no post-hoc choice | Human approved 2026-11-01 |
| Keep 0.1% commission, 2 bps slippage, 0.0001/8h funding | Conservative; avoids lowering costs to reach green | Human approved 2026-11-01 |
| Instruments BTCUSD, ETHUSD, XAUUSD | Liquid, trending, available; not selected from results | Human approved 2026-11-01 |
| IS 2021–2022, OOS 2023–2024 | Two regimes; OOS is not used for any choice | Human approved 2026-11-01 |
| Green = 10% annualized OOS, ≤20% drawdown, ≥30 trades/instrument, positive on ≥2/3, no halt | Policy-aligned; strict | Human approved 2026-11-01 |
| One matrix, no post-hoc changes | If it fails, stop at `no edge` | Human approved 2026-11-01 |

## 6. Acceptance criteria

- The pre-registration exists before the first matrix cell runs.
- All 36 cells complete and write five bundles plus `backtest.json`.
- The analysis reports full-run, in-sample, and out-of-sample metrics per cell
  and per variant, including costs, trades, drawdown, profit factor, and the
  aggregate stability check.
- At least one control and one treatment cell are rerun into a separate root and
  the five bundles are byte-identical.
- The experiment record contains the results, triage, decision, and limitations;
  the AI transparency log records the actual AI use.
- If the treatment meets every green criterion, a Quant Validator record
  reproduces the result and states residual risk. If it does not, the record
  says `no edge` and no strategy change is proposed.

## 7. Work breakdown

| Step | Work | Role | Paths |
|---|---|---|---|
| W1 | Write the pre-registration and deterministic config generator | Quant Researcher / Orchestrator | `docs/experiments/2026-11-01/` |
| W2 | Generate the 36 configs and validate one cell per variant with `policy check` | Quant Developer | `docs/experiments/2026-11-01/config/` |
| W3 | Run the matrix into isolated roots | Loop Governor | `target/experiment-EXP-2026-11-01-03/` |
| W4 | Analyze full, IS, and OOS metrics and the green criteria | Result Analyst | `docs/experiments/2026-11-01/analyze.py`, `results.json` |
| W5 | Triage the result and write the decision | Triage Officer | experiment record |
| W6 | If green, reproduce and validate; if not, record `no edge` | Quant Validator / Orchestrator | `docs/reports/2026-11-01/` |
| W7 | Update the records and prepare the branch for review | Orchestrator | AI log, plan, experiment record |

## 8. Verification

| Check | Command | Expected |
|---|---|---|
| Config validity | `quantrule policy check --config <cell>/run.json` | `policy ok` for one cell per variant |
| Matrix completion | `tools/experiment/run-matrix.sh docs/experiments/2026-11-01/matrix.txt <root>` | `matrix 36 cells`; five bundles per cell |
| Determinism | rerun one control and one treatment; `diff -r` the bundles | no differences |
| Analysis | `python3 docs/experiments/2026-11-01/analyze.py <root>` | deterministic JSON/text; every criterion evaluated |
| Docs | `python3 tools/check_links.py` | no broken relative links |
| Repository | `git diff --check` | clean |

No Rust source, test, or fixture changes are expected. If a product defect is
found, the cycle stops and routes it to an audit; the experiment is not patched
around it.

## 9. Risks and recovery

- **Runtime:** 36 runs over 13-month windows; expected minutes, not hours. A
  failed cell is rerun only for a harness or config error and is recorded.
- **Data gaps:** the source has provider gaps; coverage is reported, not patched.
- **Warm-up boundary:** December trades and the first January days are causal
  warm-up; they are excluded from the yearly trade filter and the equity is
  rebased on 1 January.
- **No edge:** the most likely outcome. The cycle stops; no threshold is changed.
- **False green:** two variants and three instruments are a small matrix. A pass
  must also survive the Quant Validator's reproduction and the recorded
  limitations. No live or capacity claim is made.
- **Rollback:** no product code is changed. Reverting the branch removes only
  experiment records and configs.

## 10. Stop conditions

- A config error, missing source, nondeterminism, or incomplete bundle that
  cannot be rerun within the harness-error allowance.
- A critical audit finding appears.
- The green criteria fail: record `no edge` and stop.
- A result requires a threshold, cost, or split change: stop and start a new
  pre-registration; do not change this one.
- Human stop.

## 11. Outcome

The 36-cell matrix completed; the determinism rerun of one control and one
treatment is bundle-identical. No variant is green. The treatments produced
9–11 OOS trades per instrument against the 30-trade floor; the control loses
15–35% (BTCUSD and ETHUSD halt). BTCUSD/ETHUSD 2023 4h coverage is 46%/28%, so the 2023
treatment cells are a data blocker rather than a clean `no edge` test.

The experiment is stopped without a threshold, cost, or split change. The next
cycle requires a BTCUSD/ETHUSD 2023 data audit/fix and a new pre-registration.
The audit is [`docs/audits/2026-11-01/crypto-2023-data-audit.md`](../../audits/2026-11-01/crypto-2023-data-audit.md);
a fresh provider fetch reproduces the 1,260-minute gap, so the next choice is a
dense second source or a 2021/2024-only sample with the limitation recorded.
Full result: [`docs/experiments/2026-11-01/results.txt`](../../improvement/INVALIDATIONS.md).
