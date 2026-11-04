# EURUSD Session Spike — No Edge

- Date: 2026-11-04 (UTC)
- Status: **No edge — EURUSD session premise stopped**
- Partition: `model_dev` 2018–2020
- Instrument: EURUSD
- Validation / strategy / portfolio: not opened
- Baseline: none

## 1. Question

Does a DST-aware London or New York session event on EURUSD have a directional
2 ATR / 1 ATR / 20-bar edge beyond the same session's base rate?

The pre-registration is
[`EURUSD-SESSION-SPIKE-PREREG.md`](EURUSD-SESSION-SPIKE-PREREG.md).

## 2. Stage A — session profile

EURUSD has a real session structure. Median 15m range and volume peak in the
13:00–15:00 UTC London/New York overlap. The late New York session
(15:00–19:00 UTC) shows a sharp drop in the unconditional 2 ATR hit rate. The
full UTC-hour profile is in the local artifact
`data/improvement/2026-11-04/EURUSD-SESSION-SPIKE/profile.md`.

The DST-aware calendar was used for the events, so the summer and winter session
shifts are handled explicitly.

## 3. Stage B — results

| Event | n | Clusters | Hit | Base | Lift | Lift LCB | Random p | Back lag | Shuffle p | Grade |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| London ORB | 712 | 712 | 0.3244 | 0.3148 | +0.0096 | −0.0325 | 0.2637 | +0.0124 | 0.6119 | C |
| New York ORB | 718 | 718 | 0.2688 | 0.2025 | +0.0663 | +0.0267 | 0.0050 | +0.0998 | 0.2985 | C |
| New York close fade | 613 | 613 | 0.1697 | 0.1623 | +0.0074 | −0.0290 | 0.3035 | −0.0905 | 0.7264 | C |

Pass criteria: `n ≥ 200`, clusters `≥ 20`, lift `≥ +5pp`, LCB `> 0`, random
`p ≤ 0.05`, back-lag pass, shuffle `p ≤ 0.05`. No primary cell passed.

## 4. Primary analysis

- **New York opening range breakout** is the only cell with a positive lower
  bound: +6.63pp lift, +2.67pp LCB, random p 0.005. It fails the back-lag control
  (+9.98pp) and the direction shuffle (p 0.2985). The same direction 20 bars
  before the event did better; the breakout side is not adding.
- **London opening range breakout** is flat: +0.96pp lift, −3.25pp LCB, random
  p 0.2637, shuffle p 0.6119.
- **New York close fade** is flat: +0.74pp lift, −2.90pp LCB, random p 0.3035,
  shuffle p 0.7264.

The DST-aware calendar did not rescue the session premise. The New York ORB has
a real pre-cost lift, but the controls say the prior path already carried it.

## 5. Decision

- The EURUSD session premise is **stopped**. No primary cell passed; no model
  validation, strategy run, or baseline was opened.
- Do not tune the session windows, the 1.0 ATR setup, the 2 ATR / 1 ATR / 20-bar
  barrier, the DST calendar, or the instrument after this result.
- The accepted compression model baselines are unchanged and direction-neutral.
- The cost hurdle is severe: at EURUSD 15m, 4 bps commission plus 2 bps slippage
  is a large fraction of a 2 ATR target. Even a passing model gate would face a
  difficult strategy tier.
- A new program requires a new dataset or a new model premise and a new approved
  charter. It cannot be another session variation on EURUSD.

## 6. Evidence and limitations

- Pre-registration:
  [`EURUSD-SESSION-SPIKE-PREREG.md`](EURUSD-SESSION-SPIKE-PREREG.md).
- Model pack: `data/session-spike/eurusd/{2018,2019,2020}/model` (local).
- Local artifacts:
  `data/improvement/2026-11-04/EURUSD-SESSION-SPIKE/profile.md`, `profile.json`,
  `results.md`, `results.json`.
- Tool: `tools/improvement/eurusd_session_spike.py`; tests:
  `tools/improvement/test_eurusd_session_spike.py`.
- The data is bid M1; no live spread. The session calendar uses
  `Europe/London` and `America/New_York` from the IANA database.
- Single instrument: EURUSD only. Same-session review; no independent Quant
  Validator record. This is a no-edge result, so there is no promising claim to
  validate.
