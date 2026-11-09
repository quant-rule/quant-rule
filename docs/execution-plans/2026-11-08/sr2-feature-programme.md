# SR2 Feature Programme — Top-Trade Attribution to Multi-Timeframe Strategy

- Date: 2026-11-08 (UTC)
- Status: **Verified — implemented; no candidate; 2024 not opened; human acceptance pending**
  (approved by the human on 2026-11-08: "approved proceed with all the phases")
- Parent audit: [`SR2-BTC-AUDIT.md`](../../improvement/2026-11-08/SR2-BTC-AUDIT.md)
- Phase A: [`SR2-BTC-FEATURES.md`](../../improvement/2026-11-08/SR2-BTC-FEATURES.md)
- Model change: [`rvol-regime.md`](../../model/2026-11-08/rvol-regime.md)
- Phase C: [`SR3-FEATURES.md`](../../improvement/2026-11-08/SR3-FEATURES.md)
- Claim type: research programme; no edge or profitability claim

## 1. Intent

The SR2-A BTCUSD audit found a fragile lead: the top five trades share low RVOL,
ADX below 25, and a breakout direction aligned with the EMA stack. This
programme turns that observation into model features, then tests a BTC-only
1h-context / 15m-entry strategy. The top trades are used for hypothesis
generation, not confirmation.

## 2. Phases

| Phase | Work | Gate |
|---|---|---|
| A | Charts and feature attribution for the 139 BTC trades | Review the evidence; no strategy run |
| B | Promote the surviving features into model states (`rvol_regime`) | Model Change tests; human baseline review |
| C | Pre-registered BTC-only strategy cells: 1h context, 15m entry, 1h controls | Feasibility gate, development criteria, 9 bps, 5% DD |
| D | Multi-model per timeframe only if Phase C shows the need | New Requirements & Design; not built speculatively |

## 3. Hypotheses

The programme tests a small, pre-registered set:

1. **H1 — 1h aligned low-RVOL fade.** Fade a 1h compression breakout only when
   the event direction equals the 1h EMA stack, ADX is below 25, and
   `rvol_regime` is low. Long and short are separate cells. 1 ATR stop / 3 ATR
   target.
2. **H2 — 15m entry, 1h context.** The same 1h regime conditions, with the
   compression breakout on 15m. Long and short are separate cells. 1 ATR stop /
   3 ATR target.
3. **H3 — 1h aligned low-RVOL fade without the ADX filter** as a control.
4. **H4 — 1h aligned fade with ADX but without the RVOL filter** as a control.

The control cells separate the value of each condition; they are not a search.

## 4. Data and costs

| Item | Value |
|---|---|
| Instrument | BTCUSD |
| Development | `strategy_dev`, 2018-01-01..2022-12-31 |
| Validation | `strategy_validation` 2024; not opened |
| Costs | 6 bps base; 9 bps sensitivity |
| Drawdown | 5% per cell, hard halt |
| Account | $10,000 per cell, fixed notional, 1x leverage |
| Timeframes | 1h context and 15m entry; 1h controls |
| Model ladder | `model_timeframes` = the timeframes each cell needs |

## 5. Acceptance criteria

- Phase A produces the feature table, top-trade charts, and a written
  attribution.
- Phase B adds `rvol_regime` with a model contract, registry entry, causality,
  determinism, golden, and edge-case tests; existing model outputs are unchanged.
- Phase C runs the four cells on `strategy_dev`, 6 bps, and evaluates 9 bps and
  5% DD for any cell that passes the base.
- No 2024 access before a frozen candidate and separate human approval.
- No post-hoc threshold, filter, target, or timeframe change.

## 6. Stop conditions

- Phase A does not reproduce the feature pattern across years/controls.
- Phase B cannot add the state without changing existing model outputs.
- Phase C fails the feasibility gate, the development criteria, or the 9 bps
  sensitivity.
- The evidence needs a new engine (multi-model) before Phase C is exhausted.

## 7. Roles

Quant Researcher (hypotheses and thresholds), Quant Developer (model/strategy
implementation and runs), Quant Validator (preflight, causality, costs,
reproduction). Sequential same-agent passes; not independent agents.

## 8. Outcome

| Phase | Result |
|---|---|
| A | 139 feature rows and five 1,000-candle charts; the top-five pattern is stack-aligned, weak-ADX, low-RVOL, and the 2019 loss persists in every filter subset. |
| B | `rvol_regime` is published as a model context state with a contract, registry entry, threshold unit test, fixture golden, causality, determinism, and edge-case tests; no event value changes. |
| C | 40 preflight cells, 40 base cells, and 30 9 bps cells. H1/H3/H4 are positive at 6 and 9 bps but hold 9–21 trades per side; H2 (15m entry) is negative; H4 long reaches 5.04%. No candidate. |
| D | Not triggered. The 1h single-model cells outperform the 15m multi-timeframe cells; a `ModelSet` would be speculative. |

2024 was not opened. The next hypothesis is a larger-sample stack-aligned 1h
fade (EUR/XAU controls or a different 15m entry design), pre-registered as a new
programme under the Strategy Research workflow.

### Verification

- `cargo test` — pass.
- `cargo clippy --all-targets -- -D warnings` — pass.
- `cargo fmt --check` — pass.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` — pass.
- Python improvement tests (`tools/improvement`) — 131 pass.
- Registry, links, changelog, structure, workflows, releases, partitions,
  verification matrices/evidence, event emitters, validation, command surface,
  terms, legacy, and the progression oracle — pass.
- Reproducibility: H3 long 2018 and H4 long 2021 rerun byte-identical for
  `portfolio/metrics.json`, `portfolio/trades.csv`, and
  `strategy/decisions.ndjson`.
- `git diff --check` — pass.

### Record status

The Phase C work is recorded as the research analysis `SR3-FEATURES`, not as a
`quantrule.improvement-run/v2` run: no manifest was staged before the first
artifact and the cells ran outside `run.py`. The local working record is
`data/improvement/2026-11-08/SR3-FEATURES/` with `results.json`, `review.md`, and
`decision.md`; the durable summary is
[`SR3-FEATURES.md`](../../improvement/2026-11-08/SR3-FEATURES.md). The bundle
provenance records `code_commit: unknown` (a pre-existing shell limitation), and
H4 long's bar-close halt overshoots the 5% ceiling to 5.0401%.
