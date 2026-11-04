# AI Transparency Log — EURUSD Session Spike

- Date: 2026-11-04 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: read-only EURUSD DST-aware session spike
- Pre-registration:
  [`docs/improvement/2026-11-04/EURUSD-SESSION-SPIKE-PREREG.md`](../../improvement/2026-11-04/EURUSD-SESSION-SPIKE-PREREG.md)
- Result:
  [`docs/improvement/2026-11-04/EURUSD-SESSION-SPIKE.md`](../../improvement/2026-11-04/EURUSD-SESSION-SPIKE.md)
- Status: no edge — session premise stopped

## Intent and boundary

The human approved the EURUSD session spike plan on 2026-11-04. The agent
generated the EURUSD model pack from the accepted v4 model config, wrote the
DST-aware session script and tests, ran the profile and barrier test on
`model_dev`, and recorded the result.

The agent did not open validation, run a strategy, change the model, or accept a
baseline. The signal rules are deterministic arithmetic on OHLCV. No machine
learning, neural network, fitted model, or statistic entered the event or the
barrier.

## Implemented work

| Item | Status |
|---|---|
| EURUSD model pack 2018–2020 | Generated from the accepted v4 model config |
| Pre-registration | Written before the barrier test |
| DST-aware session script | `tools/improvement/eurusd_session_spike.py` |
| Session tests | 3 tests pass |
| Barrier test | Three cells; no primary pass |
| Validation / strategy | Not opened |
| Baseline | Not accepted |

## Verification and evidence

Observed on 2026-11-04:

| Command | Result |
|---|---|
| `python3 -m unittest test_eurusd_session_spike` | 3 tests pass |
| `eurusd_session_spike.py --stage profile` | EURUSD hourly profile written |
| `eurusd_session_spike.py --stage barrier` | no edge; three cells |
| `python3 tools/check_links.py` | no broken relative links |
| `python3 tools/check_terms.py` | no banned terms |

The New York opening range breakout is the closest cell: +6.63pp lift, +2.67pp
LCB, random p 0.005, but back-lag +9.98pp and shuffle p 0.2985. The back-lag
says the prior path already had the edge; the shuffle says the direction side is
not adding. The London ORB and NY close fade are flat.

## Learning and rejected suggestions

- The DST-aware EURUSD session structure is real — volume and range peak in the
  London/New York overlap — but the pre-registered session events do not turn it
  into a directional edge.
- The New York ORB was not promoted despite its positive LCB: the back-lag and
  shuffle controls fail by design.
- Tuning the DST windows, the 1.0 ATR setup, the barrier, or the instrument was
  rejected.
- The EURUSD cost hurdle is severe at 15m: 4 bps commission plus 2 bps slippage
  is a large fraction of a 2 ATR target. The model gate is pre-cost; the strategy
  tier would be harder.
- No validation, strategy, or baseline was opened. The EURUSD session premise is
  stopped.
