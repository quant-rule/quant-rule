# Execution Plan: Harder Compression-Expansion Cells

- Date: 2026-11-04 (UTC)
- Status: **Verified — 1m 4 ATR passes both partitions; accepted model baseline (human, 2026-11-04)**
- Issue: [#373](https://github.com/quant-rule/quant-rule/issues/373)
- Protocol: [`docs/model/2026-11-04/compression-harder.md`](../../model/2026-11-04/compression-harder.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-04/373-compression-harder.md`](../../ai-transparency-log/2026-11-04/373-compression-harder.md)
- Claim type: model evidence for forward range expansion; no strategy or edge
  claim.

## 1. Intent

The 2 ATR / 20-bar compression gate was inconclusive because the outcome
saturated. Test a harder, pre-registered expansion claim on the unseen 5m and
1m cells, select the stronger development cell, and validate it once.

## 2. Current state

- The compression-breakout event and range harness are merged (PR #372).
- The 2 ATR / 20-bar gate on 1h/15m is `MODEL-2026-11-04-02`; it is
  inconclusive because the coiling base is 95.8–98.9%.
- The 15m secondary showed +2.03pp lift with a +1.12pp corrected lower bound,
  but it is descriptive and cannot be promoted.
- The 5m and 1m compression cells were computed in the same model bundles but
  never evaluated.

## 3. Scope

Included:

- Pre-register two cells: 5m and 1m, forward 20-bar range ≥ 3 ATR.
- Development on the existing nine 2018–2020 compression bundles.
- Select the stronger passing cell; one-shot validation 2021–2022.
- Model evidence only.

Excluded:

- Strategy, execution, portfolio, and cost claims.
- Post-hoc threshold or timeframe changes.
- 4h and higher timeframes.
- Promotion of the 15m secondary from the prior run.

## 4. Assumptions and uncertainty

- A 3 ATR forward range is a harder, economically meaningful expansion than
  2 ATR and should have a lower coiling base, leaving room for a 5pp lift.
- The 5m/1m cells are unseen outcomes; the same M1 data underlies the prior
  1h/15m result, so residual data-snooping risk remains and is disclosed.
- The event may be too rare at 1m or too noisy at 5m; the pass rule handles it.
- The validation partition is unopened for these cells.

## 5. Decisions

| Decision | Choice |
|---|---|
| Cells | 5m and 1m, both pre-registered |
| Outcome | Forward 20-bar range ≥ 3 ATR |
| Event | Compression breakout, K=5, 0.2 ATR break, RVOL ≥ 1.2 |
| Baselines | All bars, coiling bars, placebo transition |
| Controls | Random coiling, 20-bar back-lag |
| Correction | m=2 on development; m=1 on validation |
| Selection | Highest corrected lift LCB among passing cells; 5m tie-break |
| Validation | One-shot 2021–2022 only for the selected cell |
| Strategy | None before human baseline acceptance |

## 6. Work breakdown

| Phase | Work | Paths | Role |
|---|---|---|---|
| 0 | Plan, protocol, AI log | `docs/execution-plans/`, `docs/model/`, `docs/ai-transparency-log/` | Orchestrator |
| 1 | Harness timeframe support | `tools/improvement/compression_validation.py`, `path_validation.py` | Quant Developer |
| 2 | Development runs | `data/model-v4/compression/dev/` (existing bundles) | Quant Developer |
| 3 | Selection and gate | `docs/improvement/2026-11-04/`, `data/improvement/` | Result Analyst |
| 4 | One-shot validation | validation bundles and run record, only on a dev pass | Quant Validator |

## 7. Verification

- Harness unit tests after the timeframe extension.
- Dev cells recorded with counts, baselines, controls, and selection.
- Registry/validation/link/term checks unchanged; no model code changes are
  expected.
- A deterministic rerun of the selected harness command.
- Validation run only if the dev cell passes.

## 8. Risks and recovery

- Both cells may fail: record no edge/inconclusive and stop.
- The 1m cell may be too noisy or too expensive for a later strategy; the model
  claim is not a cost claim.
- The same M1 data underlies prior cells; the 5m/1m outcomes are unseen, but
  residual snooping remains.
- A pass is a model baseline candidate, not a strategy go.

## 9. Stop conditions

- No dev pass: no validation, no baseline.
- Saturated or underpowered: inconclusive, no threshold change.
- Validation failure: development-only; stop.
- Model pass without human acceptance: stop before strategy.

## 10. Result

The family ran on the nine development bundles. Neither cell passed.

- 1m: +4.31pp lift, corrected LCB +3.92pp, random p 0.005, placebo pass,
  back-lag −4.88pp, replication 3. It misses the 5pp gate by 0.69pp.
- 5m: +7.43pp lift, corrected LCB +6.23pp, random p 0.005, replication 3, but
  the placebo transition is 86.96% vs the event's 87.70%; the breakout adds
  +0.74pp and the cell fails.

Verdict: no edge under the family rule; the 1m cell is a positive predictive
lead. No validation, baseline, or strategy. The next pre-registered cell is 1m,
4 ATR over 20 bars.

- Gate: [`HARDER-COMPRESSION-GATE`](../../improvement/2026-11-04/HARDER-COMPRESSION-GATE.md)
- Run record: [`MODEL-2026-11-04-03`](../../improvement/2026-11-04/MODEL-2026-11-04-03-compression-harder.md)

## 11. Phase 2 — 4 ATR cells

The 3 ATR family did not pass. The 1m cell is a positive lead (+4.31pp lift,
+3.92pp corrected LCB, random p 0.005, placebo pass, replication 3) but misses
the 5pp gate by 0.69pp. A harder 4 ATR target is pre-registered before its
outcome is computed, with the same event, baselines, controls, and pass rule;
two cells (1m, 5m), m = 2, one-shot validation on the selected cell. The 3 ATR
results are disclosed as prior information; the 4 ATR outcomes are unseen.

## 12. Final result

The 4 ATR phase selected 1m and passed both partitions: development +6.83pp
lift with a +6.22pp corrected LCB; validation +6.32pp lift with a +5.56pp
corrected LCB; placebo, random, back-lag, and three-instrument replication all
pass. The result is a candidate model baseline pending human acceptance. No
strategy or portfolio work starts before acceptance.
