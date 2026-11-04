# AI Transparency Log — Session / Liquidity Spike

- Date: 2026-11-04 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: read-only session/liquidity research spike
- Pre-registration:
  [`docs/improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE-PREREG.md`](../../improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE-PREREG.md)
- Result:
  [`docs/improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE.md`](../../improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE.md)
- Status: no edge — session premise stopped

## Intent and boundary

The human approved the session/liquidity spike plan on 2026-11-04. The agent
wrote the pre-registration, added a read-only session script and tests, ran the
session profile and the barrier test on `model_dev`, and recorded the result.

The agent did not open validation, run a strategy, change the model, or accept a
baseline. The signal rules are deterministic arithmetic on OHLCV. No machine
learning, neural network, fitted model, or statistic entered the event or the
barrier.

## Implemented work

| Item | Status |
|---|---|
| Pre-registration | Written before the barrier test |
| Session profile | XAUUSD structure present |
| Session script | `tools/improvement/session_spike.py` |
| Session tests | 2 tests pass |
| Barrier test | Nine cells; no primary pass |
| Validation / strategy | Not opened |
| Baseline | Not accepted |

## Verification and evidence

Observed on 2026-11-04:

| Command | Result |
|---|---|
| `python3 -m unittest test_session_spike` | 2 tests pass |
| `session_spike.py --stage profile` | XAUUSD hourly profile written |
| `session_spike.py --stage barrier` | no edge; nine cells |
| `python3 tools/check_links.py` | no broken relative links |
| `python3 tools/check_terms.py` | no banned terms |

The closest primary cell is the XAUUSD opening range breakout: +4.19pp lift,
+1.15pp LCB, random p 0.005, but back-lag +7.78pp and shuffle p 0.418. The
back-lag control says the prior path already had the edge; the shuffle says the
direction side is not adding. The other primary cells fail their lower bound,
random, or sample.

## Learning and rejected suggestions

- The XAU session structure is real — volume and range peak in the London/New
  York overlap — but the pre-registered session events do not turn it into a
  directional edge.
- The opening range breakout was not promoted despite its positive LCB: the
  back-lag and shuffle controls fail by design.
- Tuning the 1.0 ATR setup, the session windows, the barrier, or the instrument
  mix after the result was rejected.
- A DST-aware session calendar was not added: it is a new premise, not a rescue
  of this spike.
- No validation, strategy, or baseline was opened. The session premise is
  stopped.
