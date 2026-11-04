# AI Transparency Log — V5 Model Program

- Date: 2026-11-04 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: v5 program charter and 1h swing model gate
- Execution plan:
  [`docs/execution-plans/2026-11-04/v5-model-program.md`](../../execution-plans/2026-11-04/v5-model-program.md)
- Pre-registration:
  [`docs/improvement/2026-11-04/V5-SWING-1H-PREREG.md`](../../improvement/2026-11-04/V5-SWING-1H-PREREG.md)
- Result:
  [`docs/improvement/2026-11-04/V5-SWING-1H-GATE.md`](../../improvement/2026-11-04/V5-SWING-1H-GATE.md)
- Status: no edge — Phase 1 stopped

## Intent and boundary

The human approved the v5 plan on 2026-11-04. The agent wrote the program
charter, the pre-registration, and the families file, ran the sample pre-flight,
ran the 1h swing gate on `model_dev`, and recorded the result.

The agent did not open `model_validation`, run a strategy, change the model, or
accept a baseline. The signal rules are the existing deterministic v4 events; no
machine learning, neural network, fitted model, or statistic entered the event
or the barrier. Statistics is used only to evaluate the rule-based events.

## Implemented work

| Item | Status |
|---|---|
| Program charter | `docs/governance/2026-11-04/v5-program-charter.md` |
| Pre-registration | Written before the gate |
| Families file | `config/v5-swing/1h/families.json` |
| Sample pre-flight | 919 / 3,126 / 6,722 raw 1h events; all above 200 |
| Development gate | Run `MODEL-2026-11-04-18`; three cells grade C |
| Validation / strategy | Not opened |
| Baseline | Not accepted |

## Verification and evidence

Observed on 2026-11-04:

| Command | Result |
|---|---|
| Pre-flight event count over the v4 dev pack | 919 trend, 3,126 pivot break, 6,722 EMA50; floor 200 |
| `path_validation.py --timeframes 1h --horizon 100 --target-atr 4 --stop-atr 2 --back-lag 100 --primary-cells 3 --seed 0` | three cells grade C; selection null |
| Byte-identical rerun with seed 0 | `path_validation.json` and `.md` matched the recorded SHA-256 digests |
| `python3 tools/check_links.py` | no broken relative links |
| `python3 tools/check_terms.py` | no banned terms |

The closest cell is 1h trend continuation: +3.23pp lift over the 1h base, but a
−1.72pp lower bound, a −1.63pp incremental lift against the trend state, a
+7.43pp back-lag placebo, and shuffle p 0.0896. The major pivot break (+0.76pp)
and EMA50 test (−0.44pp) are flat. No cell approaches the pre-registered +5pp
promotion threshold.

## Learning and rejected suggestions

- The horizon explanation is falsified. The 1h event lift is smaller than the
  trend state it marks, and the back-lag placebo is stronger than the event. The
  same signature appeared in the EURUSD New York opening-range breakout.
- The 100-bar cluster gap chains dense pivot-break events into about one cluster
  per bundle. The pre-registered `J >= 20` floor catches it; the pivot cell
  cannot pass. Recomputing the bound with a smaller gap after seeing the result
  was rejected as post-hoc tuning.
- Opening `model_validation` or a strategy run was rejected: no cell passed.
- Building Phase 2 from this result was rejected: Phase 2 is a new information
  premise and needs its own approved charter.
- Naming the families with a `v5_` prefix was rejected after inspection: the
  harness keys its `adx20` sensitivity rule on `h1_trend_continuation`. The IDs
  were kept as the v4 family IDs, which is also the truthful description of the
  event.
- Adding cells, changing the payoff, or relaxing the +5pp threshold after the
  result was rejected. The three cells are retired.
