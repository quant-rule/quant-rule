# SR4 Stack-Aligned 1h Fade — Multi-Instrument Development

- Date: 2026-11-09 (UTC)
- Status: **Verified — no candidate; 2024 not opened; human acceptance pending**
- Charter: [`SR4-CHARTER.md`](../../governance/2026-11-09/SR4-CHARTER.md)
- Summary: [`SR4-STACK-FADE.md`](../../improvement/2026-11-09/SR4-STACK-FADE.md)
- Predecessor: [`SR3-FEATURES.md`](../../improvement/2026-11-08/SR3-FEATURES.md)
- Claim type: strategy research; no edge or profitability claim

## 1. Intent

Test whether the SR3 stack-aligned 1h compression-breakout fade is a repeatable
behavior rather than a BTC-only small-sample artifact. The programme pools
BTCUSD, EURUSD, and XAUUSD, keeps the rule fixed, and runs the feasibility gate
before any backtest cell.

## 2. Current state

- SR3 found H1/H3/H4 positive at 6 and 9 bps, but 9–21 trades per side.
- H2 (15m entry) was negative on both sides.
- H4 long reached 5.04% drawdown, above the 5% ceiling.
- 2024 remains closed; no strategy baseline exists.
- The accepted model baseline is `MODEL-2026-11-04-04..05`.
- The `rvol_regime` state and the strategy parser are merged.

## 3. Scope

Included:

- 18 pre-registered variant-instrument combinations: 3 rules × 2 sides × 3
  instruments.
- 90 base cells, 2018–2022, one calendar year per cell.
- 90 feasibility preflight reports at 6 bps.
- 30 9 bps sensitivity cells for S1 only, if S1 passes the base.
- One development record, one no-candidate or candidate decision.
- Registry, baseline, README, and changelog updates.

Excluded:

- 2024 validation; not opened.
- Any 15m or 1m model cell.
- Any post-hoc filter, threshold, target, stop, timeframe, or cost change.
- A portfolio or live/prop-firm claim.

## 4. Assumptions and uncertainty

- The three instruments share enough market microstructure for the compression
  event to be comparable; the per-instrument results will show if they do not.
- EURUSD and XAUUSD may have different ATR/price scales; the feasibility
  preflight is computed per instrument and is the gate.
- The 5% halt is bar-close and can overshoot; any observed cell above 5% fails.
- The S1 sample may still be below 30 pooled trades per side; that is an
  underpowered result, not a reason to add filters.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Primary rule | S1 stack-only | Largest sample; S2/S3 are controls |
| Instruments | BTCUSD, EURUSD, XAUUSD | Tests instrument dependence |
| Sides | Long and short separate | A one-sided candidate is allowed |
| Costs | 6 bps base, 9 bps S1 sensitivity | Cost-aware gate |
| Multiple testing | Bonferroni, 18 trials, 6 primary | Bounded family |
| Validation | Closed | No 2024 access without a freeze and approval |

## 6. Acceptance criteria

- 90/90 preflight reports pass at 6 bps.
- 90/90 base cells complete and 90/90 qualification records exist.
- S1 is evaluated against the charter's pooled gate; the verdict is recorded
  even when it is `no candidate`.
- 9 bps S1 cells run only if S1 passes the base.
- 2024 is not read.
- The run record, committed summary, AI log, and registry are updated.

## 7. Work breakdown

| Step | Work | Owner | Paths |
|---|---|---|---|
| 1 | Write charter, plan, configs, matrix | Quant Researcher / Developer | `docs/governance/2026-11-09/`, `config/sr4/`, `config/strategy/sr4-*.json` |
| 2 | Build 90 data/run configs and qualification | Quant Developer | `config/sr4/runs/`, `config/qualification/SR4-DEV.json` |
| 3 | Run 90 feasibility preflights | Quant Validator | `data/improvement/2026-11-09/SR4-STACK-FADE/preflight/` |
| 4 | Run 90 base cells and qualification | Quant Developer | `data/improvement/2026-11-09/SR4-STACK-FADE/artifacts/` |
| 5 | If S1 passes, run 30 9 bps S1 cells | Quant Developer | `data/improvement/2026-11-09/SR4-STACK-FADE/artifacts-9bps/` |
| 6 | Analyze and write `results.json`, `review.md`, `decision.md` | Result Analyst | local run record |
| 7 | Write the committed summary and update registry | Quant Researcher / Orchestrator | `docs/improvement/2026-11-09/SR4-STACK-FADE.md` |
| 8 | Verify, commit, PR, merge, clean up | Engineering / Orchestrator | branch `feat/sr4-stack-fade` |

## 8. Verification

- `python3 tools/improvement/feasibility.py` on every cell at 6 bps.
- `target/release/quantrule backtest run` per cell.
- `target/release/quantrule qualify` per base cell.
- Pooled metrics from `portfolio/metrics.json` and `portfolio/trades.csv`.
- Byte-identical rerun for the passing S1 side.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
  `cargo doc` with warnings denied.
- Python improvement tests and the documentation/registry checkers.
- `git diff --check`.

## 9. Risks and recovery

- **Small sample:** report `underpowered`; do not add filters to rescue it.
- **Cost failure:** report `no edge`; do not lower costs.
- **Instrument dependence:** report the per-instrument split; do not drop a
  failing instrument.
- **Drawdown overshoot:** fail the cell; do not relax the 5% ceiling.
- **Machine restart:** runs are resumable per cell; re-run only missing cells.
- **Data gap:** stop the affected cell; do not interpolate.

## 10. Stop conditions

Stop when a preflight fails, S1 fails the pooled gate, any S1 cell exceeds 5%
drawdown, the 9 bps pool fails, the iteration cap is reached, or the human stops
the programme. A stopped programme does not open 2024 and is not retried with a
new variant under the same premise.

## 11. Outcome

| Check | Observed |
|---|---|
| Preflight | 90 / 90 pass at 6 bps |
| Base cells | 90 / 90 complete |
| Qualification | 90 / 90 complete |
| S1 long | 113 trades, +$1,150.76, PF 1.34, 4/5 years, 1/3 instruments |
| S1 short | 111 trades, +$411.97, PF 1.16, 3/5 years, 1/3 instruments |
| EURUSD | negative on every variant and side |
| XAUUSD | negative on every variant and side |
| S1 drawdown | BTC long 5.19% (2018), 5.0035% (2021); short 5.11% (2019) |
| 9 bps | not run; S1 failed the base gate |
| 2024 | not opened |
| Reproducibility | S1 long BTCUSD and EURUSD 2018 byte-identical |

The multi-instrument premise is falsified. The programme stops with no
candidate; S1 is not re-registered with a new filter.
