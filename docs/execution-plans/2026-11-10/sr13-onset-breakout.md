# Execution Plan — SR13 5m Fresh-Onset Compression Breakout

- Date: 2026-11-10 (UTC)
- Status: **Completed — no edge; premise rejected; 2024 closed**
- Program: [`SR13-CHARTER`](../../governance/2026-11-10/SR13-CHARTER.md)
- Protocol: [`onset-breakout-5m`](../../config/2026-11-10/onset-breakout-5m.md)
- AI log: [`sr13-onset-breakout`](../../ai-transparency-log/2026-11-10/sr13-onset-breakout.md)
- Claim type: strategy research; no edge claim.

## 1. Intent

Test a new directional premise — the first confirmed close outside the accepted
5m fresh-onset compression range continues in the break direction — with a
pre-registered raw-edge screen before any strategy, execution, portfolio, or
model-event work. The problem is the repeated failure of directional strategies;
the intended outcome is a decisive development-gate verdict, not a tuned
candidate.

## 2. Current state

- The accepted 5m onset event (`signal.compression_range_onset`, type 3,
  subtype 9, source 5) is a neutral boundary-touch measurement.
- The current strategy engines cannot express a close-confirmation entry:
  `event_composite` reads events/states, not the candle close;
  `compression_bracket` enters on a touch at the boundary.
- The existing `signal.compression_breakout` is a different rule (0.2 ATR
  buffer, RVOL ≥ 1.2, pre-trigger window).
- SR11 and SR12 produced no candidate; the 2024 partition is closed.
- The ATR replacement question was analysed separately; the evidence did not
  support a global ATR replacement.

## 3. Scope

Included:

- the four protocol fixes: W=3 primary; 6-bar pooled primary with descriptive
  secondaries; 2024 one-shot validation (closed on failure); model event only
  after a passing gate;
- the SR13 charter, protocol, and execution plan;
- the 5m model bundles for BTCUSD 2018-2022;
- the `tools/research/breakout_edge.py` diagnostic and its fixture test;
- the development screen, review, and decision.

Explicitly excluded:

- the 2024 validation, because the development gate failed;
- a model event, strategy config, execution, or portfolio work;
- ETH/XAU replication, because the BTC gate failed;
- ATR bracket, sizing, or parameter search;
- any change to the model engine or strategy engine.

## 4. Assumptions and uncertainty

- The 5m series derived from the 1m Dukascopy source is the accepted data
  version; the 2018 coverage is 67.17%, a known limitation.
- The onset event is neutral and its range is published causally at the last
  coiling bar.
- The non-overlapping sample and the 20-bar cluster gap make the forward
  windows approximately independent.
- The main uncertainty is whether the first close direction carries any
  information at all; the fade and placebo controls test that directly.

## 5. Decisions

| # | Decision | Choice | Consequence |
|---|---|---|---|
| 1 | Breakout window | W=3 primary; W=1/W=6 descriptive | Keeps the breakout fresh; the sensitivity cannot promote |
| 2 | Primary horizon and sides | 6 bars pooled; sides/horizons descriptive | One promotion test; no 8-test correction burden |
| 3 | Validation partition | 2024 one-shot; 2021-2022 descriptive only | Avoids the repeatedly used model-validation partition |
| 4 | Production path | New model event after a passing gate | The strategy boundary forbids the strategy from computing the close comparison |
| 5 | Costs | 6 bps base; 9 bps stress | The gross LCB must clear the stress cost |
| 6 | Iteration cap | 1 | A failure rejects the premise; no re-registration |

## 6. Acceptance criteria

- The protocol has no open parameter; a reviewer can run it without asking a
  question.
- The diagnostic's rule and the eventual model event use the same written state
  machine.
- The fixture test covers long, short, boundary-equal, window expiry,
  next-onset supersession, gap-open, end-of-series, non-overlap, and stats.
- The 5m-only ladder is byte-identical to the full ladder on a common sample.
- The development screen applies G1–G9 and records pass/fail.
- The 2024 partition is untouched.
- No ATR bracket, sizing, or parameter matrix is run.

## 7. Work breakdown

| Step | Work | Output | Status |
|---|---|---|---|
| 1 | Write charter, protocol, plan | `docs/governance/2026-11-10/SR13-CHARTER.md`, `docs/config/2026-11-10/onset-breakout-5m.md`, this file | done |
| 2 | Build the 5m model config and run configs | `config/sr13/` | done |
| 3 | Compute BTCUSD 5m bundles 2018-2022 | `data/improvement/2026-11-10/SR13/artifacts/model/` | done |
| 4 | Build the diagnostic and fixture test | `tools/research/breakout_edge.py`, `test_breakout_edge.py` | done |
| 5 | Run W=3 primary and W=1/W=6 descriptive | `data/improvement/2026-11-10/SR13/raw-edge/w1/w3/w6` | done |
| 6 | Apply the gates and write review/decision | `review.md`, `decision.md` | done — fail |
| 7 | 2024 validation | — | not opened |
| 8 | Model event and strategy backtest | — | not built |

## 8. Verification

| Command | Observed |
|---|---|
| `cargo build --release -p quantrule-shell` | success |
| `quantrule policy check --config config/sr13/runs/BTCUSD/2018-5m.json` | `policy ok`, model ladder 1m,5m |
| `report model` for 2018-2022 | five bundles, each with readings/events/provenance |
| `python3 -m unittest tools/research/test_breakout_edge.py -v` | 9 tests pass |
| `breakout_edge.py --window 3 ...` | 6,379 non-overlapping signals; mean −2.1790 bps; LCB −3.5608; gates fail |
| `breakout_edge.py --window 1 ...` | 3,500 signals; mean −1.5597; LCB −3.4140; gates fail |
| `breakout_edge.py --window 6 ...` | 7,985 signals; mean −2.1536; LCB −3.4587; gates fail |
| Ladder check (2018 Q1) | 18,267 common 5m rows, 0 differences; 201,431 5m events identical |

The 2018 model run's 5m coverage is 67.17%; 2019–2022 are 90.42%, 93.68%,
99.87%, and 93.27%.

## 9. Result

The primary failed every economic, baseline, placebo, random, cost, and
stability gate; only the back-lag and sample gates passed. The full record is
[`SR13`](../../improvement/2026-11-10/SR13.md). The premise is rejected; the
2024 validation remains closed; no model event, strategy, execution, or
portfolio work follows.

## 10. Risks and recovery

| Risk | Outcome / handling |
|---|---|
| 5m cost floor | The primary gross mean is negative, far below the 6/9 bps cost; no recovery |
| Window choice | W=1 and W=6 are also negative; not a window artifact |
| BTC-only | The replication was not run because the primary failed; the result is BTC-only and is recorded as such |
| 2018 coverage | 67.17%; recorded; the negative result is not driven by the missing bars |
| Production boundary | The close comparison is a model measurement; the model event is the only correct production path and is not built |
| Validation leakage | 2024 remains closed; 2021-2022 is descriptive only |

## 11. Stop conditions

- Development gate fails: stop; do not open 2024, do not build the model event.
- Sample below 200 or clusters below 20: stop as underpowered.
- Data coverage inadequate: stop.
- Diagnostic and production event diverge: stop and reconcile.
- Human stops the program.

## 12. Completion

The plan is complete with a negative result. The premise is not re-registered
with a different window, horizon, side, cost, or filter. The task worktree and
branch are retained until the human decides whether to commit, merge, or stop.
