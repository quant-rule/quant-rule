# EURUSD Session Spike — Pre-registration

- Date: 2026-11-04 (UTC)
- Status: **Pre-registered; frozen before the barrier test**
- Scope: `model_dev` 2018–2020
- Instrument: EURUSD
- Claim type: exploratory model research; no strategy, validation, or edge claim

## 1. Question

Does a DST-aware London or New York session event on EURUSD have a directional
2 ATR / 1 ATR / 20-bar edge beyond the same session's base rate?

This is a new data program. The fixed-UTC XAU spike failed; EURUSD has real
London/New York sessions and a DST-aware calendar is a genuinely different
premise.

## 2. Fundamental constraints

- OHLCV only: open, high, low, close, volume, timestamp. No tick, order book,
  news, funding, open interest, or external data.
- No machine learning, neural network, or statistical model in the signal path.
  The session rules are deterministic arithmetic and comparison.
- Descriptive statistics are used only for the session profile and evaluation.
- No future data: opening ranges use completed bars; entry is the next bar open;
  the barrier is forward.
- `model_dev` only. No validation, strategy, or portfolio access.
- No model or strategy code changes.
- Deterministic: UTC data, explicit Europe/London and America/New_York calendars,
  fixed seeds, no ambient timezone.
- No post-hoc tuning of the windows, events, barrier, or criteria.

## 3. Data and calendar

Source: the v4 model pack 15m readings for EURUSD 2018–2020:

```text
data/session-spike/eurusd/<year>/model/readings.csv
```

The 15m bars and ATR 14 come from the model pack. The session rules use the bar
timestamp, OHLC, volume, and ATR.

DST-aware session calendar:

| Session | Local window | Timezone |
|---|---|---|
| London | 08:00–16:30 | `Europe/London` |
| New York | 09:30–16:00 | `America/New_York` |
| NY close window | 15:30–16:00 | `America/New_York` |

The calendar uses the IANA timezone database; it is not a fixed UTC offset. The
script records the timezone names and never uses the ambient system timezone.

## 4. Events

### Event 1 — London Opening Range Breakout

- Opening range: high/low of the first 30 minutes (two 15m bars).
- Trigger: first 15m close beyond the range high or low within the next 8
  session bars.
- Direction: breakout side.
- Entry: next 15m open after the trigger.
- Barrier: +2 ATR target, −1 ATR stop, 20 bars, stop first.

### Event 2 — New York Opening Range Breakout

Same rule as Event 1, for the New York 09:30 local session.

### Event 3 — New York Close Fade

- Setup: the last completed 15m bar before 16:00 New York local; the session
  move from the 09:30 open is at least 1.0 ATR.
- Direction: opposite the session move.
- Entry: the next 15m open (16:00 local).
- Barrier: +2 ATR target, −1 ATR stop, 20 bars, stop first.

## 5. Base and controls

| Item | Rule |
|---|---|
| London ORB base | All London session bars, direction-matched |
| NY ORB base | All New York session bars, direction-matched |
| NY close fade base | The last 30 minutes of the New York session, direction-matched |
| Event base | Event-weighted average of the bundle session base rates |
| Random | 200 draws from the session bars, excluding event entries |
| Back-lag | The same direction 20 bars before the event |
| Shuffle | Shuffle event directions within the session |
| Correction | Bonferroni over the three primary cells; cell alpha 0.0167; z ≈ 2.394 |

## 6. Primary cells and pass criteria

All three cells are primary. EURUSD is the only instrument; the
single-instrument limitation is disclosed. There is no secondary replication.

A cell passes only when all hold:

- `n ≥ 200`, clusters `≥ 20`;
- conditional lift over the session base `≥ +5pp`;
- conditional LCB `> 0`;
- random `p ≤ 0.05`;
- back-lag pass;
- shuffle `p ≤ 0.05`.

A cell below `n = 200` is recorded as `inconclusive — underpowered`.

## 7. Decision

| Outcome | Action |
|---|---|
| No primary cell passes | Record `no edge`; stop the EURUSD session premise; do not tune |
| A cell passes | Write a EURUSD model program charter; disclose the single-instrument limitation |
| Stage A shows no EURUSD session structure | Stop before the barrier test |
| Sample below 200 | Record `inconclusive — underpowered`; stop |

A charter is a new **model program**. It defines the DST-aware session context
in the model and a model gate. The spike does not open validation or run a
strategy.

## 8. Costs and limitations

- The model gate is pre-cost. The strategy tier would use 4 bps commission per
  operation and 2 bps slippage. For EURUSD at 15m, that is a large fraction of a
  typical ATR; the cost hurdle is disclosed and is likely to be severe.
- The data is bid M1 from the existing source; no live spread.
- Single instrument: EURUSD only.
- Same-session review; no independent Quant Validator record.

## 9. Planned outputs

- `docs/improvement/2026-11-04/EURUSD-SESSION-SPIKE.md`
- `tools/improvement/eurusd_session_spike.py`
- `tools/improvement/test_eurusd_session_spike.py`
- `data/improvement/2026-11-04/EURUSD-SESSION-SPIKE/`
- A program charter or a stop record
- An AI transparency log
