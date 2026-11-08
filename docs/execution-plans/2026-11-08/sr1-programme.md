# SR1 — Failed Compression Breakout Fade

- Date: 2026-11-08 (UTC)
- Status: **Stopped early — infeasible** (2026-11-08); 10/15 cells completed; validation not opened
- Charter: [`SR1-CHARTER.md`](../../governance/2026-11-08/SR1-CHARTER.md)
- Tier: strategy
- Model baseline: `MODEL-2026-11-04-04..05` (1m 4 ATR compression expansion, accepted)
- Claim type: strategy research proposal; no edge or profitability claim

## 1. Intent

Test the untested side of the accepted 1m compression expansion. The directional
breakout strategy (`STRATEGY-2026-11-04-01`) lost in every fold: pooled profit
factor 0.481, 0/3 positive instruments, and a 2.64% XAUUSD win rate at a 4:1
reward/risk. The accepted model baseline is a **direction-neutral volatility**
claim, not a direction claim. The proposal is a **fade** of the first breakout:
enter opposite the breakout direction, target 1 ATR, stop 4 ATR, time stop 20
bars. This is the opposite thesis, not a threshold change to the failed rule.

## 2. Current evidence

| Item | Result |
|---|---|
| Model baseline | 1m 4 ATR compression expansion; dev and validation pass; hit 0.7943 vs coiling base 0.7260; +6.83 pp lift |
| Directional breakout | `STRATEGY-2026-11-04-01`; PF 0.481; 0/3 positive; 15/15 folds halted |
| Two-sided bracket | `STRATEGY-2026-11-04-02`; PF 0.592; 0/3 positive; 15/15 folds halted |
| 5m bracket | `STRATEGY-2026-11-04-03`; average return −34.47%; 0/3 positive; 15/15 folds halted |
| 1m costs | v9 gross P&L +$2,887 vs $15,753 commission; the cost floor is the main risk |

The failed breakout direction is the only evidence for the fade. It is a
post-hoc mirror, not a pre-registered prior; the plan discloses this and uses
2024 as the one-shot out-of-sample check.

## 3. Strategy rule

```json
{
  "schema": "quantrule.strategy/v1",
  "id": "SR1-FADE-1M",
  "engine": "event_composite",
  "execution_timeframe": "1m",
  "event_window_bars": 1,
  "time_stop_bars": 20,
  "require_contiguous": true,
  "risk": {
    "stop_atr_multiple": 4.0,
    "reward_to_risk": 0.25
  },
  "entry": [
    {
      "kind": "event",
      "timeframe": "1m",
      "event_type": 3,
      "subtype": 7,
      "source": 5,
      "direction": 0,
      "contrarian": true
    }
  ],
  "exit": []
}
```

`signal.compression_breakout` is the registered base event `(type 3, subtype 7,
source 5)`. `direction: 0` matches either breakout side. `contrarian: true`
makes the entry side the opposite of the matched event. The risk block gives a
4 ATR stop and a 1 ATR target. No other condition is added; the rule complexity
budget is one event.

## 4. Engine addition

The current event-composite entry takes the matched event's side. Add an optional
`contrarian` boolean to an event entry:

- default `false`; existing rules are unchanged;
- only valid on an event-composite event entry; rejected for the bracket engine
  and for a non-event condition;
- when `true`, the entry side is the opposite of the matched event direction;
  the evidence still names the matched event;
- parser, `strategy explain`, `strategy run`, backtest, and tests cover the
  default, the inverted long, the inverted short, and the rejection cases.

This is a Requirements & Design change to the strategy engine, not a model or
execution change. It is approved or rejected with the premise.

## 5. Data, partitions, and costs

| Item | Value |
|---|---|
| Model baseline | `MODEL-2026-11-04-04..05` |
| Instruments | BTCUSD, ETHUSD, XAUUSD |
| Development | `strategy_dev`, 2018-01-01..2022-12-31 |
| Validation | `strategy_validation`, 2024-01-01..2024-12-31; one-shot, not opened now |
| Costs | 4 bps commission + 2 bps slippage; sensitivity at 9 bps total |
| Folds | One calendar year per fold, 2018–2022 |
| Account | The accepted model-gate policy envelope; no policy change |
| Entry | Next execution bar after the event, existing execution path |
| Exit | 4 ATR stop, 1 ATR target, 20-bar time stop |

The model and data versions are the accepted baseline's versions; the run
manifest records them. The strategy does not read a fill, cash, equity, or P&L.

## 6. Green criteria

Development (`strategy_dev`):

- total return > 0;
- profit factor > 1.10;
- max drawdown ≤ 15%;
- at least 600 closed trades;
- at least 2 of 3 instruments positive;
- at least 3 of 5 yearly folds positive;
- profit factor > 1.0 at 9 bps total cost;
- byte-identical rerun.

Validation (`strategy_validation` 2024, one-shot, only after freeze and human
approval):

- total return > 0;
- profit factor > 1.0;
- max drawdown ≤ 15%;
- at least 100 closed trades;
- at least 2 of 3 instruments positive.

A development fail is `no_edge`; the rule is not tuned. A validation fail is
`no_edge`; the 2024 window is not reopened.

## 7. Acceptance criteria

- The `contrarian` addition is implemented, documented, and tested; the default
  behaviour is unchanged.
- The strategy config parses and `strategy explain` shows the inverted side.
- The development run produces the decisions, backtest, portfolio, and
  qualification artifacts on `strategy_dev`.
- The qualification criteria are evaluated exactly as declared; no post-hoc
  threshold change.
- The run record, review, decision, and AI log are written.
- If the development is green, the candidate is frozen by config hash and the
  human is asked for one-shot 2024 access.
- If not green, the programme stops at `no_edge` and the strategy is retired.

## 8. Work breakdown

| Step | Work | Owner | Paths |
|---:|---|---|---|
| 1 | Approve premise, engine addition, thresholds | Human | this plan, charter |
| 2 | Add `contrarian` to strategy engine | Quant Developer | `crates/quantrule-backtest/src/strategy`, shell parser, tests |
| 3 | Strategy config and qualification criteria | Quant Developer | `config/strategy/sr1-fade-1m.json`, `config/qualification/SR1-*.json` |
| 4 | Development run on `strategy_dev` | Quant Developer | run record `data/improvement/2026-11-08/SR1-FADE-1M/` |
| 5 | Qualification and Quant Validator pass | Quant Validator | `qualification/`, validation record |
| 6 | Freeze and human gate | Coordinator | frozen config hash |
| 7 | One-shot 2024 validation if approved | Coordinator | `strategy_validation` run |
| 8 | Accept/retire and records | Human + Triage | baselines, index, changelog, AI log |

## 9. Verification

| Check | Command | Expected |
|---|---|---|
| Engine tests | `cargo test` | contrarian default, long, short, rejection |
| Strategy explain | `quantrule strategy explain ...` | inverted side in the decision |
| Strategy run | `quantrule strategy run ...` | deterministic decisions |
| Backtest | `quantrule backtest run ...` | cost-inclusive evidence |
| Qualification | `quantrule qualify ...` | declared criteria evaluated |
| Reproducibility | rerun the development bundle | byte-identical decisions and portfolio |
| Rust | `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` | clean |
| Docs | link, term, structure, changelog, partition checks | pass |

## 10. Risks and recovery

- The fade is a post-hoc mirror of a failed strategy; the evidence is one prior
  result, not a pre-registered prior. The 2024 validation is the only
  out-of-sample check.
- The 4 ATR stop can produce large losses; the 15% drawdown ceiling may halt
  folds. A halt is a fail, not a reason to loosen the ceiling.
- The 1m cost floor may dominate; the 9 bps sensitivity is a required criterion.
- The `contrarian` flag is a new engine feature; it is small and tested, but it
  is a behaviour change and needs a Quant Validator record.
- The rule is one event, no extra conditions; adding a filter after a result is
  a new hypothesis.
- Same-agent Quant Developer and Quant Validator passes are sequential, not
  independent.

## 11. Stop conditions

Stop and report if:

- the human does not approve the premise or the engine addition;
- the contrarian tests or the strategy parser fail;
- the development qualification is not green;
- the 2024 validation is not approved or fails;
- a threshold, cost, partition, or rule would need a post-hoc change;
- a material risk appears that is outside this plan.

## 12. Approval

- Status: **Approved** (human, 2026-11-08)
- Approved by: Human
- Scope: the fade premise, the `contrarian` engine addition, the development
  criteria, the 15% drawdown ceiling, and the later one-shot 2024 request.
- Not authorized: the 2024 validation run before a frozen candidate and a
  separate human approval.

## 13. Stopped early

- Result: stopped after 10 of 15 cells (BTCUSD and ETHUSD 2018–2022). Every
  completed cell halted at the 15% drawdown ceiling.
- Structural reason: the 4 ATR stop / 1 ATR target needs an 80% win rate at
  zero cost and about 88% at 6 bps on BTCUSD 1m. On XAUUSD 1m the 1 ATR target
  is smaller than the 6 bps round-trip cost. The hypothesis cannot be rescued
  by a filter or a parameter change.
- Not run: the XAUUSD cells, the 9 bps sensitivity, and the pooled aggregate.
  Running them would spend compute without changing the decision.
- New gate: a strategy hypothesis must pass the feasibility preflight
  (`tools/improvement/feasibility.py`) before any backtest cell. The gate
  requires a positive net target and a breakeven win rate at or below 50%
  after the declared costs.
- Validation: the 2024 partition was not opened. No candidate was frozen.
