# SR7 H3 Crypto Cross-Section — BTC+ETH

- Date: 2026-11-09 (UTC)
- Status: **Verified — no candidate; 9 bps and 2024 not opened**
- Charter: [`SR7-CHARTER.md`](../../governance/2026-11-09/SR7-CHARTER.md)
- Summary: [`SR7-H3-CRYPTO.md`](../../improvement/2026-11-09/SR7-H3-CRYPTO.md)
- Predecessor: [`SR6-VWAP.md`](../../improvement/2026-11-09/SR6-VWAP.md)
- Claim type: strategy research; no edge or profitability claim

## 1. Intent

Test whether the SR3 H3 1h fade is a crypto-major behavior by adding ETHUSD to
the BTCUSD control. The count gate runs first; the 20 cells run only if the
pooled H3 sample is large enough.

## 2. Current state

- SR3 H3 BTC: long 18 trades +$1,620 PF 2.23 max DD 4.41%; short 21 trades
  +$723 PF 1.73 max DD 4.17%; both positive at 9 bps; underpowered.
- SR4 showed the stack fade is not a EURUSD/XAUUSD behavior.
- SR6 VWAP rejection produced no candidate.
- 2024 remains closed; no strategy baseline exists.
- The accepted model baseline is `MODEL-2026-11-04-04..05`.

## 3. Scope

Included:

- A read-only H3 count gate on BTCUSD+ETHUSD, 2018–2022.
- Two rules × two instruments × two sides = 20 base cells.
- 20 feasibility preflights at 6 bps.
- 9 bps sensitivity for a side that passes the base gate.
- One development record and decision.

Excluded:

- 2024 validation; not opened.
- Any 15m or 1m model cell.
- Any post-hoc filter, threshold, target, stop, timeframe, or cost change.
- A new instrument beyond BTC/ETH.
- A portfolio or live/prop-firm claim.

## 4. Assumptions and uncertainty

- ETH has enough H3 events to add a meaningful sample; the count gate decides.
- ETH is correlated with BTC but is a separate instrument; a positive ETH result
  is cross-crypto evidence, not independent evidence.
- BTC H3 is the control; it should reproduce the SR3 sign.
- The 5% halt is bar-close and can overshoot; any observed cell above 5% fails.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Primary rule | H3 stack + weak ADX fade | Best DD-compliant BTC variant |
| Instruments | BTCUSD + ETHUSD | Crypto cross-section |
| Timeframe | 1h only | 15m entries have failed twice |
| Primary cells | ETH long/short | BTC is the control |
| Costs | 6 bps base, 9 bps passing side | Cost-aware gate |
| Count gate | ≥30 H3 events per side pooled | Stops an underpowered test |
| Validation | Closed | No 2024 without a freeze and approval |

## 6. Acceptance criteria

- The count gate runs and records H3 counts by instrument, side, and year.
- If pooled H3 has fewer than 30 events on either side, the programme stops as
  underpowered with no backtest.
- If the count gate passes, 20/20 preflight reports pass and 20/20 base cells
  complete with qualification records.
- Each side is evaluated against the charter's gate; the verdict is recorded
  even when it is `no candidate`.
- 9 bps cells run only for a side that passes the base.
- 2024 is not read.
- The run record, committed summary, AI log, and registry are updated.

## 7. Work breakdown

| Step | Work | Owner | Paths |
|---|---|---|---|
| 1 | Write charter, plan, count tool, 1h model bundles | Quant Researcher / Developer | `docs/governance/2026-11-09/`, `tools/improvement/h3_count.py` |
| 2 | Run the H3 count gate | Quant Validator | `data/improvement/2026-11-09/SR7-H3-CRYPTO/count-gate/` |
| 3 | If the gate fails, stop and record underpowered | Result Analyst | run record and summary |
| 4 | Build two strategy configs and 20 runs | Quant Developer | `config/sr7/`, `config/strategy/sr7-*.json` |
| 5 | Run 20 preflights, 20 base cells, qualification | Quant Developer | `data/improvement/2026-11-09/SR7-H3-CRYPTO/` |
| 6 | If a side passes, run its 9 bps cells | Quant Developer | `artifacts-9bps/` |
| 7 | Analyze and write `results.json`, `review.md`, `decision.md` | Result Analyst | local run record |
| 8 | Write the committed summary and update the registry | Quant Researcher / Orchestrator | `docs/improvement/2026-11-09/SR7-H3-CRYPTO.md` |
| 9 | Verify, commit, PR, merge, clean up | Engineering / Orchestrator | branch `feat/sr7-h3-crypto` |

## 8. Verification

- Count gate: `h3_count.py` over the 1h BTC/ETH model bundles.
- `python3 tools/improvement/feasibility.py` on every cell at 6 bps.
- `target/release/quantrule backtest run` per cell.
- `target/release/quantrule qualify` per base cell.
- Pooled metrics from `portfolio/metrics.json` and `portfolio/trades.csv`.
- Byte-identical rerun for a passing side.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
  `cargo doc` with warnings denied.
- Python improvement tests and the documentation/registry checkers.
- `git diff --check`.

## 9. Risks and recovery

- **Count gate fails:** stop as underpowered; do not lower the threshold.
- **ETH negative:** the premise fails; do not add a filter or drop ETH.
- **BTC control contradicts SR3:** report the contradiction; do not use ETH to
  rescue BTC.
- **Drawdown overshoot:** fail the cell; do not relax the 5% ceiling.
- **Machine restart:** runs are resumable per cell; re-run only missing cells.

## 10. Stop conditions

Stop when the count gate fails, a preflight fails, both sides fail the
development gate, a passing side fails 9 bps, any cell exceeds 5% drawdown, the
iteration cap is reached, or the human stops the programme. A stopped programme
does not open 2024 and is not retried with a new filter or instrument under the
same premise.

## 11. Outcome

| Check | Observed |
|---|---|
| Count gate | BTC 20/23, ETH 19/19, pooled 39/42; pass |
| Preflight | 20 / 20 pass at 6 bps |
| Base cells | 20 / 20 complete |
| Qualification | 20 / 20 complete |
| H3 long pooled | 27 trades, +$2,289.36, PF 2.098, max DD 5.62%, 5/5 years |
| H3 short pooled | 36 trades, +$147.02, PF 1.067, max DD 5.64%, 1/5 years |
| BTC control | Reproduces SR3 exactly; inside 5% |
| ETH long | 9 trades, +$669.86, PF 1.870, DD 5.62% |
| ETH short | 15 trades, −$576.42, PF 0.527, DD 5.64% |
| 9 bps | not run; no side passed the base gate |
| 2024 | not opened |

No side passes. The stack-fade family is closed as a candidate.
