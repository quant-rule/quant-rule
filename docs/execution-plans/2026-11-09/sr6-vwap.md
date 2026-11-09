# SR6 VWAP Rejection — 15m Entry with 1h Stack Context

- Date: 2026-11-09 (UTC)
- Status: **Verified — no candidate; 9 bps and 2024 not opened**
- Charter: [`SR6-CHARTER.md`](../../governance/2026-11-09/SR6-CHARTER.md)
- Summary: [`SR6-VWAP.md`](../../improvement/2026-11-09/SR6-VWAP.md)
- Predecessor: [`SR5-ABSORPTION.md`](../../improvement/2026-11-09/SR5-ABSORPTION.md)
- Claim type: strategy research; no edge or profitability claim

## 1. Intent

Test a well-sampled 15m rejection event on BTCUSD: `signal.vwap_rejection` with
a 1h EMA-stack context. The count gate runs first; the development cells run
only if the V1 sample is large enough.

## 2. Current state

- SR4 found the stack-aligned compression fade BTC-only.
- SR5 found the absorption-rejection event too rare (A1 5 long / 8 short).
- The VWAP-rejection census found 4,822 long / 4,281 short A1 events.
- 2024 remains closed; no strategy baseline exists.
- The accepted model baseline is `MODEL-2026-11-04-04..05`.

## 3. Scope

Included:

- A read-only count gate for V1 on 2018–2022.
- Six pre-registered variants: V1/V2/V3 × long/short.
- 30 base cells, one calendar year per cell.
- 30 feasibility preflights at 6 bps.
- V1 9 bps sensitivity only if V1 passes the base.
- One development record and decision.

Excluded:

- 2024 validation; not opened.
- Any 1h-only or 1m model cell.
- Any post-hoc filter, threshold, target, stop, timeframe, or cost change.
- A portfolio or live/prop-firm claim.

## 4. Assumptions and uncertainty

- The VWAP rejection event has enough events on both sides; the census says yes.
- The 1h stack filter may reduce the sample; the gate checks it before backtest.
- The event may be too frequent and overlap; the one-position rule and 20-bar
  time stop determine the actual trade count.
- The 5% halt is bar-close and can overshoot; any observed cell above 5% fails.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Primary rule | V1 VWAP rejection + 1h stack | Large sample, clear rejection mechanism |
| Instrument | BTCUSD only | SR4 showed the compression fade is BTC-specific |
| Entry | 15m | The user's desired entry timeframe |
| Context | 1h EMA stack | Keeps the rejection a pullback, not a countertrend guess |
| Costs | 6 bps base, 9 bps V1 | Cost-aware gate |
| Count gate | ≥30 V1 events per side | Confirms the sample before compute |
| Validation | Closed | No 2024 without a freeze and approval |

## 6. Acceptance criteria

- The count gate runs and records V1/V2/V3 counts by side.
- If V1 has fewer than 30 events on either side, the programme stops as
  underpowered with no backtest.
- If the count gate passes, 30/30 preflight reports pass and 30/30 base cells
  complete with qualification records.
- V1 is evaluated against the charter's gate; the verdict is recorded even when
  it is `no candidate`.
- 9 bps V1 cells run only if V1 passes the base.
- 2024 is not read.
- The run record, committed summary, AI log, and registry are updated.

## 7. Work breakdown

| Step | Work | Owner | Paths |
|---|---|---|---|
| 1 | Write charter, plan, count tool, count gate | Quant Researcher / Developer | `docs/governance/2026-11-09/`, `tools/improvement/vwap_rejection_count.py` |
| 2 | Build six strategy configs and 30 runs | Quant Developer | `config/sr6/`, `config/strategy/sr6-*.json` |
| 3 | Run 30 preflights, 30 base cells, qualification | Quant Developer | `data/improvement/2026-11-09/SR6-VWAP/` |
| 4 | If V1 passes, run 10 9 bps V1 cells | Quant Developer | `artifacts-9bps/` |
| 5 | Analyze and write `results.json`, `review.md`, `decision.md` | Result Analyst | local run record |
| 6 | Write the committed summary and update the registry | Quant Researcher / Orchestrator | `docs/improvement/2026-11-09/SR6-VWAP.md` |
| 7 | Verify, commit, PR, merge, clean up | Engineering / Orchestrator | branch `feat/sr6-vwap` |

## 8. Verification

- Count gate: `vwap_rejection_count.py` over the recorded 15m/1h bundles.
- `python3 tools/improvement/feasibility.py` on every cell at 6 bps.
- `target/release/quantrule backtest run` per cell.
- `target/release/quantrule qualify` per base cell.
- Pooled metrics from `portfolio/metrics.json` and `portfolio/trades.csv`.
- Byte-identical rerun for the passing V1 side.
- `cargo fmt --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings`,
  `cargo doc` with warnings denied.
- Python improvement tests and the documentation/registry checkers.
- `git diff --check`.

## 9. Risks and recovery

- **Count gate fails:** stop as underpowered; do not lower the threshold.
- **Cost failure:** report `no edge`; do not lower costs.
- **Context failure:** report V1 vs V3; do not promote V3.
- **Drawdown overshoot:** fail the cell; do not relax the 5% ceiling.
- **Event overlap:** the one-position rule caps exposure; do not add a filter
  after seeing the result.
- **Machine restart:** runs are resumable per cell; re-run only missing cells.

## 10. Stop conditions

Stop when the count gate fails, a preflight fails, V1 fails the development or
9 bps gate, any V1 cell exceeds 5% drawdown, the iteration cap is reached, or
the human stops the programme. A stopped programme does not open 2024 and is
not retried with a new variant under the same premise.

## 11. Outcome

| Check | Observed |
|---|---|
| Count gate | 4,822 long / 4,281 short; pass |
| Preflight | 30 / 30 pass at 6 bps |
| Base cells | 30 / 30 complete |
| Qualification | 30 / 30 complete |
| V1 long | 229 trades, −$242.14, PF 0.966, max DD 6.37% |
| V1 short | 154 trades, −$719.73, PF 0.853, max DD 5.61% |
| Halts | every V1/V2/V3 cell halted in every year |
| 9 bps | not run; V1 failed the base gate |
| 2024 | not opened |

The event-composite parser was widened from subtypes `1..=7` to registered
signal subtypes; diagnostics remain rejected. V2/V3 do not rescue V1. The
programme stops with no candidate.
