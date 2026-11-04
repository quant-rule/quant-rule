# AI Transparency Log — V5 Phase 2 Spike

- Date: 2026-11-04 (UTC)
- Tool: OpenCode agent session (DeepSeek V4.1 Flash)
- Work item: v5 Phase 2 price-only spike and volume audit
- Execution plan:
  [`docs/execution-plans/2026-11-04/v5-model-program.md`](../../execution-plans/2026-11-04/v5-model-program.md)
- Charter:
  [`docs/governance/2026-11-04/v5-phase2-charter.md`](../../governance/2026-11-04/v5-phase2-charter.md)
- Pre-registration:
  [`docs/improvement/2026-11-04/V5-PHASE2-SPIKE-PREREG.md`](../../improvement/2026-11-04/V5-PHASE2-SPIKE-PREREG.md)
- Result:
  [`docs/improvement/2026-11-04/V5-PHASE2-SPIKE.md`](../../improvement/2026-11-04/V5-PHASE2-SPIKE.md)
- Status: no edge — Phase 2 stopped; volume family blocked

## Intent and boundary

The human approved continuing to Phase 2 on 2026-11-04. The agent audited the
data, wrote the Phase 2 charter and pre-registration, implemented the read-only
spike and its tests, ran it on `model_dev`, and recorded the result.

The agent did not implement the new events in the model engine, open
`model_validation`, run a strategy, or accept a baseline. The candidate rules
are deterministic arithmetic on completed OHLCV bars. Statistics is used only to
evaluate the events.

## Implemented work

| Item | Status |
|---|---|
| Phase 2 charter | `docs/governance/2026-11-04/v5-phase2-charter.md` |
| Pre-registration | Written before the spike |
| Volume audit | 9 instrument-years; BTC/ETH not admissible |
| Spike tool | `tools/improvement/v5_phase2_spike.py` |
| Tests | `tools/improvement/test_v5_phase2_spike.py`, 7 tests pass |
| Candidates | Momentum bar, rejection wick, relative-strength cross |
| Validation / strategy | Not opened |
| Model implementation | Not started |

## Verification and evidence

Observed on 2026-11-04:

| Command | Result |
|---|---|
| `python3 -m unittest test_v5_phase2_spike` | 7 tests pass |
| `v5_phase2_spike.py --config adx25 --seed 0` | three candidates no edge |
| Byte-identical rerun with seed 0 | `results.json` and `results.md` matched their SHA-256 digests |
| `python3 tools/check_links.py` | no broken relative links |
| `python3 tools/check_terms.py` | no banned terms |

Results: momentum bar +0.87pp (random p 0.289, back-lag +1.89pp), rejection wick
+0.14pp (replication 1), relative-strength cross −1.64pp (random p 0.990). The
gap diagnostic is underpowered at 127 events. All three primary candidates are
below the +5pp threshold.

The volume audit is the other material result: monthly median volume changes by
600×–17,800× within a year on BTC and ETH, so VWAP, volume profile, and volume
climax cannot be tested on this pack. XAU volume is stable but is one instrument.

## Learning and rejected suggestions

- Price-derived new information does not rescue the directional program. The
  momentum bar repeats the v4 signature: a small positive point estimate that
  the prior path explains better than the event.
- Cross-asset relative strength is negative on the pooled sample, so the
  cross-sectional momentum premise is not supported.
- Testing volume anyway with local ratios was rejected: a 300×–1,000× unit jump
  inside a rolling window would generate false climax events. The data must be
  repaired at the source, not patched in the signal path.
- Repairing volume from the M1 source inside this work item was rejected: it is
  a data-engine change with its own plan, tests, and validation.
- Promoting momentum on its shuffle pass alone was rejected: the random and
  back-lag controls fail and the lift is far below the pre-registered threshold.
- Tuning the candidate thresholds after the result was rejected. The candidates
  are retired.
- Opening validation, building the model, or starting a strategy was rejected:
  no candidate passed the exploration.
