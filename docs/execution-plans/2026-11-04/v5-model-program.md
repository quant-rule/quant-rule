# V5 Model Program — Execution Plan

- Date: 2026-11-04 (UTC)
- Status: **Approved** (human, 2026-11-04); Phase 0, 1, and 2 executed — **no edge**; volume family blocked; Phase 1b, 2b, and 3 not opened; program stopped
- Owner: human
- Program: `PROGRAM-2026-11-04-04` (Phase 1), `PROGRAM-2026-11-04-05` (Phase 2)
- Charter: [`docs/governance/2026-11-04/v5-program-charter.md`](../../governance/2026-11-04/v5-program-charter.md)
- Phase 2 charter: [`docs/governance/2026-11-04/v5-phase2-charter.md`](../../governance/2026-11-04/v5-phase2-charter.md)
- Pre-registration: [`docs/improvement/2026-11-04/V5-SWING-1H-PREREG.md`](../../improvement/2026-11-04/V5-SWING-1H-PREREG.md)
- Phase 2 pre-registration: [`docs/improvement/2026-11-04/V5-PHASE2-SPIKE-PREREG.md`](../../improvement/2026-11-04/V5-PHASE2-SPIKE-PREREG.md)
- Gate result: [`docs/improvement/2026-11-04/V5-SWING-1H-GATE.md`](../../improvement/2026-11-04/V5-SWING-1H-GATE.md)
- Claim type: research plan; no strategy or edge claim

## 1. Intent

The v4 event layer has no directional edge at 20 bars. The v5 program asks
whether that failure is a **horizon** problem or an **information** problem.

Phase 1 tests the horizon explanation cheaply on the existing model: the same 1h
events, a 100-bar horizon, and a 4 ATR target against a 2 ATR stop. Phase 2
builds a new-information model only if Phase 1 fails. Phase 1b and Phase 3 are
downstream of a model or strategy baseline and do not start without one.

## 2. Current state

| Item | State |
|---|---|
| v4 event layer (5m/15m/1h) | No edge at 20 bars; 12 primary cells grade C |
| Divergence, cross-timeframe, progression | No edge or underpowered |
| XAU and EURUSD session spikes | No edge |
| Accepted model baselines | 1m 4 ATR compression, 1m compression range, 5m fresh-onset range; direction-neutral |
| Strategy / portfolio baseline | None |
| Model pack | `data/model-v4/dev` and `data/model-v4/validation`, 30 cells |

## 3. Scope

Included:

- Phase 1: the 1h swing model gate on `model_dev`, with three pre-registered
  cells.
- Phase 1b: a context-ATR risk mode and a 1h swing strategy, only after an
  accepted model baseline.
- Phase 2: a new-information v5 model, only if Phase 1 fails and the human
  approves a new charter.
- Phase 3: portfolio and prop-firm adaptation, only after an accepted strategy
  baseline.

Non-goals:

- No new event, timeframe, target, stop, or filter for the tested families.
- No validation access without a passing development gate.
- No strategy run without an accepted model baseline.
- No prop-firm claim from a backtest.
- No statistical, machine-learning, or neural-network signal path.

## 4. Assumptions and uncertainty

- The v4 model pack at 1h is correct and available; the gate does not regenerate
  it.
- The 1h events have enough events for the gate. The pre-flight confirms it.
- The 100-bar horizon is long enough for a 4 ATR move but not so long that the
  path is dominated by unrelated regimes. This is the hypothesis under test.
- A 4 ATR / 2 ATR payoff has a lower base hit rate than the 1 ATR / 1 ATR
  barrier; the correction and controls still apply.
- Phase 2's new information may not produce a rule-based edge. A no-edge result
  is a valid outcome and stops the program.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Test horizon before new information | Phase 1 first | Cheap; distinguishes horizon from information |
| Barrier | 4 ATR / 2 ATR / 100 bars | Cost-aware 2R payoff; fewer, but larger, wins |
| Cells | Three 1h families | Finite; Bonferroni over 3 |
| Configs | `adx25` primary, `adx20` sensitivity | Consistent with the v4 protocol |
| Validation | One-shot, only after a dev pass | Preserves out-of-sample evidence |
| Phase 2 | New readings, new events, new charter | A new premise, not a variant |

## 6. Acceptance criteria

Phase 1 is complete when:

- the pre-flight shows `n >= 200` for every cell or the gate does not run;
- the gate runs with both configs, the declared correction, and controls;
- the result is recorded in a run record, a gate document, the index, and the
  baselines registry;
- a passing cell is frozen and a validation run is requested; a failing gate
  records `no edge` and opens nothing.

Phase 1 acceptance criteria were evaluated on 2026-11-04: the gate ran, all
three cells graded C, and the no-edge path was recorded.

Phase 2 exploration is complete when the pre-registered candidates are screened
and the volume admissibility is recorded. It was evaluated on 2026-11-04: no
candidate passed, the volume family is blocked, and no Phase 2 model charter was
written.

## 7. Work breakdown

| Phase | Work | Status |
|---|---|---|
| 0 | Charter, pre-registration, sample pre-flight | Done |
| 1 | 1h swing model gate on `model_dev` | Done — no edge |
| 1 | Validation, only if a cell passes | Not opened |
| 1b | Context-ATR risk mode (`atr_timeframe`), 1h swing strategy | Not opened — no model baseline |
| 2 | V5 new-information model: VWAP, volume profile, candle anatomy, gaps, climax, cross-asset | Exploration done — no edge; volume blocked |
| 2b | V5 strategy | Not opened |
| 3 | Portfolio, prop-firm controls, paper, small live | Not opened |

## 8. Verification

Observed on 2026-11-04:

| Command | Result |
|---|---|
| Pre-flight event count | 919 / 3,126 / 6,722 raw 1h events, all above 200 |
| `path_validation.py --timeframes 1h --horizon 100 --target-atr 4 --stop-atr 2 --back-lag 100 --primary-cells 3` | three cells grade C; selection null; no edge |
| `v5_phase2_spike.py --config adx25` | three price-only candidates no edge; volume family blocked |
| Byte-identical rerun | recorded in the run and spike reviews |
| `python3 tools/check_links.py` | no broken relative links |
| `python3 tools/check_terms.py` | no banned terms |

The full evidence is in the local run folder
`data/improvement/2026-11-04/MODEL-2026-11-04-18/`.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| Overlapping paths inflate significance | Cluster-robust bound; 100-bar cluster gap |
| Dense events chain into one cluster | Cluster floor `J >= 20`; the cell fails if it is degenerate |
| Small lift promoted as an edge | Pre-registered `+5pp` promotion threshold in addition to the harness grade |
| Back-lag explains the result | Back-lag control is a pass condition |
| Phase 2 scope creep | New charter and pre-registration before any Phase 2 code |
| Look-ahead | Availability bar entry, forward-only path, causality tests on any new event |

Recovery: the model pack and accepted baselines are read-only; the gate writes
only its own run folder and docs. No migration or rollback is required.

## 10. Stop conditions

- The 1h pre-flight sample is below 200.
- No primary cell passes the development gate.
- The validation cell fails.
- A request to change the horizon, target, stop, family, state, or costs after a
  result.
- Phase 2 has no candidate event with `n >= 200`.
- Phase 2 has no passing price-only candidate and the volume data is not
  admissible.
- The human stops the program.

Phase 1 stopped on the second condition on 2026-11-04. Phase 2 stopped on the
sixth condition on 2026-11-04: no price-only candidate passed and the volume
family is blocked by the data audit. The v5 program is stopped.
