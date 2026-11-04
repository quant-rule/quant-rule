# Session / Liquidity Spike — No Edge

- Date: 2026-11-04 (UTC)
- Status: **No edge — session premise stopped**
- Partition: `model_dev` 2018–2020
- Primary: XAUUSD; secondary: BTCUSD, ETHUSD
- Validation / strategy / portfolio: not opened
- Baseline: none

## 1. Question

Does a fixed UTC session event — opening-range breakout, opening-range fade, or
New York close fade — have a directional 2 ATR / 1 ATR / 20-bar edge beyond the
same session's base rate?

The pre-registration is
[`SESSION-LIQUIDITY-SPIKE-PREREG.md`](SESSION-LIQUIDITY-SPIKE-PREREG.md).

## 2. Stage A — session profile

XAUUSD has a real session structure. The median 15m range and volume peak in the
12:00–14:00 UTC London/New York overlap, and the London open (06:00–08:00 UTC)
shows an elevated long hit rate. The full hourly profile is in the local
artifact `data/improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE/profile.md`.

The structure was present. The question was whether the pre-registered session
events could turn it into a directional edge.

## 3. Stage B — results

| Event | Instrument | n | Clusters | Hit | Base | Lift | Lift LCB | Random p | Back lag | Shuffle p | Grade |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| ORB | XAUUSD | 1,366 | 1,360 | 0.3236 | 0.2817 | +0.0419 | +0.0115 | 0.0050 | +0.0778 | 0.4179 | C |
| ORB | BTCUSD | 1,598 | 1,587 | 0.3016 | 0.3179 | −0.0163 | −0.0438 | 0.9254 | +0.0106 | 0.9851 | C |
| ORB | ETHUSD | 956 | 940 | 0.3096 | 0.3202 | −0.0106 | −0.0464 | 0.7910 | +0.0369 | 0.3682 | C |
| ORF | XAUUSD | 609 | 609 | 0.3038 | 0.2792 | +0.0246 | −0.0201 | 0.0995 | +0.0399 | 0.7612 | C |
| ORF | BTCUSD | 550 | 550 | 0.3491 | 0.3176 | +0.0314 | −0.0173 | 0.0498 | −0.0467 | 0.1990 | C |
| ORF | ETHUSD | 299 | 298 | 0.3144 | 0.3195 | −0.0051 | −0.0694 | 0.5473 | −0.0644 | 0.6816 | C |
| NCF | XAUUSD | 179 | 179 | 0.2402 | 0.2580 | −0.0178 | −0.0949 | 0.7363 | −0.1574 | 0.7164 | C |
| NCF | BTCUSD | 501 | 501 | 0.3333 | 0.3162 | +0.0171 | −0.0335 | 0.1841 | −0.1685 | 0.2886 | C |
| NCF | ETHUSD | 271 | 271 | 0.3506 | 0.3082 | +0.0424 | −0.0274 | 0.0697 | −0.1864 | 0.2537 | C |

Pass criteria: `n ≥ 200`, clusters `≥ 20`, lift `≥ +5pp`, LCB `> 0`, random
`p ≤ 0.05`, back-lag pass, shuffle `p ≤ 0.05`. No primary cell passed.

## 4. Primary analysis

- **Opening range breakout, XAUUSD** is the only primary cell with a positive
  lower bound: +4.19pp lift, +1.15pp LCB, random p 0.005. It fails the back-lag
  control (+7.78pp) and the direction shuffle (p 0.418). The same direction 20
  bars before the event did better; the breakout side itself is not adding.
- **Opening range fade, XAUUSD** has a +2.46pp lift but a −2.01pp LCB, random
  p 0.0995, and shuffle p 0.761. It is not distinguishable from the base.
- **New York close fade, XAUUSD** has only 179 events, below the 200 minimum. It
  is underpowered and its lift is negative.

The secondary crypto cells do not provide a significant replication. ORF BTCUSD
(+3.14pp, LCB −1.73pp, random 0.0498) and NCF ETHUSD (+4.24pp, LCB −2.74pp,
random 0.0697) are the closest, but neither passes.

## 5. Decision

- The session premise is **stopped**. No primary cell passed; no model
  validation, strategy run, or baseline was opened.
- Do not tune the session windows, the 1.0 ATR setup, the 2 ATR / 1 ATR / 20-bar
  barrier, or the instrument mix after this result.
- The XAU opening-range breakout is not a candidate: its back-lag and shuffle
  controls fail.
- The accepted compression model baselines are unchanged and direction-neutral.
- A new program requires a new premise and a new approved charter. A DST-aware
  session calendar, a different dataset, or a new model output would be a new
  program, not a variation of this spike.

## 6. Evidence and limitations

- Pre-registration:
  [`SESSION-LIQUIDITY-SPIKE-PREREG.md`](SESSION-LIQUIDITY-SPIKE-PREREG.md).
- Local artifacts:
  `data/improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE/profile.md`,
  `profile.json`, `results.md`, `results.json`.
- Tool: `tools/improvement/session_spike.py`; tests:
  `tools/improvement/test_session_spike.py`.
- Fixed UTC windows ignore DST. XAU is the primary instrument; crypto sessions
  are weaker. The data is bid M1. The barrier is a model-evidence barrier, not a
  cost-aware strategy result.
- Same-session review; no independent Quant Validator record. This is a
  no-edge result, so there is no promising claim to validate.
