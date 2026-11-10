# Strategy Position Excursions

- Date: 2026-11-10 (UTC)
- Status: Current
- Contract: `StrategyPosition`, `EventCompositeConfig`
- Claim type: execution/strategy state contract; no edge claim.

## 1. Position fields

The execution engine supplies these fields on every `StrategyPosition` while a
position is open:

| Field | Meaning |
|---|---|
| `entry_price` | Actual fill price |
| `entry_atr` | ATR at the entry bar, fixed for the trade |
| `unrealized_atr` | Side-adjusted `(last close - entry_price) / entry_atr` |
| `mfe_atr` | Maximum favorable excursion since entry, in entry ATR |
| `mae_atr` | Maximum adverse excursion since entry, in entry ATR |
| `bars_held` | Closed bars since the entry fill |
| `giveback_atr()` | `max(0, mfe_atr - unrealized_atr)` |

The fields are `None` when the engine has no value; the flat position has `None`
for the price and excursion fields. They are position state, not cash, equity,
P&L, a fill, an order, or a quantity. The strategy input boundary test rejects
those fields.

## 2. Update rule

Excursions update on every closed bar while a position is open, including the
entry bar. A long's favorable excursion is `(high - entry) / entry_atr`; its
adverse excursion is `(entry - low) / entry_atr`. A short mirrors. Negative
values are clamped to zero; MFE and MAE only increase.

`unrealized_atr` uses the last closed bar's close. It is signed: positive for a
profitable long or short, negative for a loss. It is not a portfolio return.

## 3. Giveback exit

An event-composite strategy may set:

```json
"giveback_atr": 1.0,
"giveback_arm_atr": 0.5
```

The giveback exit fires when the position has reached at least
`giveback_arm_atr` of favorable excursion and has given back at least
`giveback_atr` from the MFE. The exit reason is `giveback`. A time stop is
evaluated first; event-condition exits are evaluated after the giveback rule.

The rule is causal: it reads only the position state produced by closed bars. It
is not a broker-side trailing stop order. A true execution-level trailing stop is
a separate Portfolio Research change.

## 4. Verification

- `execution::intent_tests::position_view_reports_the_entry_bar_and_bars_held`
- `execution::intent_tests::position_view_reports_short_excursions`
- `strategy::event::tests::a_giveback_exit_fires_after_the_arm_and_threshold`
- `strategy::event::tests::a_giveback_exit_waits_for_the_arm`
- `strategy::event::tests::an_invalid_giveback_config_is_rejected`
- `strategy::input::boundary_tests::the_strategy_input_exposes_only_identity_and_progression`
