# SR1 Charter — Failed Compression Breakout Fade

- Date: 2026-11-08 (UTC)
- Status: **Approved** (human, 2026-11-08)
- Plan: [`sr1-programme.md`](../../execution-plans/2026-11-08/sr1-programme.md)
- Tier: strategy
- Model baseline: `MODEL-2026-11-04-04..05` (1m 4 ATR compression expansion)
- Claim type: strategy research charter; no edge or profitability claim

## 1. Premise

The accepted 1m 4 ATR compression expansion is a **direction-neutral volatility**
edge: the transition reaches the barrier more often than the coiling base, but
the event does not claim which side follows. The directional breakout strategy
lost in every fold, with a 2.64% XAUUSD win rate at a 4:1 reward/risk. The
untested side is the **fade**: enter opposite the first breakout, target 1 ATR,
stop 4 ATR, time stop 20 bars. The thesis is that the breakout side fails more
often than it follows through; the fade captures the failed-breakout reversal
after costs.

This is not a threshold change to the failed breakout. It is the opposite
thesis, using the same accepted event and the same data. It is a post-hoc mirror
of the failure, and that limitation is disclosed.

## 2. Strategy rule

| Item | Value |
|---|---|
| Engine | `event_composite` |
| Execution timeframe | 1m |
| Event | `signal.compression_breakout` `(type 3, subtype 7, source 5)` |
| Direction match | `0` (either side) |
| Entry side | Opposite the matched event (`contrarian: true`) |
| Stop | 4 ATR |
| Target | 1 ATR (`reward_to_risk: 0.25`) |
| Time stop | 20 bars |
| Additional conditions | None |

The strategy is one event and a fixed risk block. Adding a filter after a result
is a new hypothesis, not an edit.

## 3. Engine addition

Add an optional `contrarian` boolean to an event-composite event entry:

- default `false`; existing strategies are unchanged;
- only valid on an event-composite event entry; rejected for the bracket engine
  and non-event conditions;
- when `true`, the entry side is the opposite of the matched event direction;
- parser, explain, run, backtest, and tests cover default, inverted long,
  inverted short, and rejection.

This is a strategy engine capability, not a model, execution, or portfolio
change. It requires a Quant Validator record before merge.

## 4. Data and protocol

| Item | Value |
|---|---|
| Model baseline | `MODEL-2026-11-04-04..05` |
| Instruments | BTCUSD, ETHUSD, XAUUSD |
| Development | `strategy_dev`, 2018-01-01..2022-12-31 |
| Validation | `strategy_validation`, 2024-01-01..2024-12-31; one-shot, not opened now |
| Costs | 4 bps commission + 2 bps slippage; 9 bps sensitivity |
| Folds | One calendar year, 2018–2022 |
| Account | Accepted model-gate policy envelope; no policy change |
| Entry | Next execution bar; existing execution path |
| Exit | 4 ATR stop, 1 ATR target, 20-bar time stop |

## 5. Green criteria

Development: total return > 0; profit factor > 1.10; max drawdown ≤ 15%; at
least 600 trades; at least 2/3 instruments positive; at least 3/5 folds
positive; profit factor > 1.0 at 9 bps; byte-identical rerun.

Validation (one-shot 2024, only after freeze and human approval): total return
> 0; profit factor > 1.0; max drawdown ≤ 15%; at least 100 trades; at least 2/3
instruments positive.

A development fail is `no_edge`; no tuning. A validation fail is `no_edge`; the
2024 window is not reopened.

## 6. Roles

| Role | Responsibility | Boundary |
|---|---|---|
| Quant Researcher | Premise, rule, metrics, thresholds | No production code; no post-hoc change |
| Quant Developer | Engine flag, strategy config, tests, run | No hypothesis change |
| Quant Validator | Independent reproduction, look-ahead and cost checks | Read-only; no repair |
| Human | Approves premise, engine addition, thresholds, validation access, baseline | Sole final authority |

## 7. Human gates

| Gate | Action |
|---|---|
| Premise and rule | Approve the fade thesis and the 15% DD ceiling |
| Engine addition | Approve `contrarian` and the Quant Validator record |
| Development result | Review green/red; freeze or retire |
| Validation access | Approve the one-shot 2024 run |
| Strategy baseline | Accept or reject |
| Portfolio tier | Open or keep closed |

## 8. Stop rule

Stop at `no_edge` when the development qualification is not green, the 2024
validation fails, the engine addition is rejected, or a threshold/cost/partition
change would be needed after seeing a result. The rule is not re-registered as a
variant.

## 9. Not authorized

- Implementation, a run, or validation before approval.
- A model, execution, portfolio, policy, or data change.
- A different event, filter, side, stop, target, time stop, timeframe, or cost
  after a result.
- A profitability or edge claim from the development result.

## 10. Approval

- Approved by: Human
- Approved on: 2026-11-08 (UTC)
- Scope: the fade premise, the `contrarian` engine addition, the development
  criteria, the 15% drawdown ceiling, and the later one-shot 2024 request
- Not authorized: the 2024 validation run before a frozen candidate and a
  separate human approval

## 11. Result

- Outcome: **stopped early — infeasible** on 2026-11-08. Ten of fifteen cells
  completed (BTCUSD and ETHUSD 2018–2022); every completed cell halted at the
  15% drawdown ceiling.
- Structural reason: the 4 ATR stop / 1 ATR target needs an 80% win rate before
  costs and about 88% at 6 bps on BTCUSD 1m. On XAUUSD 1m the 1 ATR target is
  smaller than the 6 bps round-trip cost.
- Not run: the XAUUSD cells, the 9 bps sensitivity, and the pooled aggregate.
- Validation: `strategy_validation` 2024 was not opened.
- Gate: a future strategy hypothesis must pass the feasibility preflight
  before any backtest cell; see
  [`tools/improvement/feasibility.py`](../../../tools/improvement/feasibility.py).
