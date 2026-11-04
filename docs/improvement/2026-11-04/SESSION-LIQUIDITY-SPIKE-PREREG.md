# Session / Liquidity Spike — Pre-registration

- Date: 2026-11-04 (UTC)
- Status: **Pre-registered; frozen before the barrier test**
- Scope: `model_dev` 2018–2020 only
- Claim type: exploratory model research; no strategy, validation, or edge claim

## 1. Question

Does a fixed UTC session event have a directional barrier edge large enough to
justify a new model program?

The spike tests the session context — opening range, opening move, session
close — against the same session's base rate. It does not test another event
from the existing model layer.

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
- Deterministic: UTC, fixed windows, sorted data, fixed seeds.
- No post-hoc tuning of the windows, events, barrier, or criteria.

## 3. Data and windows

Instruments: XAUUSD primary; BTCUSD and ETHUSD secondary.

Source: the v4 model pack 15m readings for the nine `model_dev` cells:

```text
data/model-v4/dev/<instrument>-<year>-adx25/model/readings.csv
```

The 15m bars and ATR 14 come from the model pack. The session rules use only the
bar timestamp, OHLC, volume, and ATR.

Fixed UTC windows:

| Window | UTC |
|---|---|
| London | 07:00–16:00 |
| New York | 13:30–21:00 |
| NY close window | 20:30–21:00 |

The windows ignore DST. That is a disclosed limitation.

## 4. Events

### Event 1 — Opening Range Breakout (ORB)

- Opening range: the high/low of the first 30 minutes (two 15m bars).
- Trigger: the first 15m close beyond the range high or low within the next 8
  session bars.
- Direction: the breakout side.
- Entry: the next 15m open after the trigger.
- Barrier: +2 ATR target, −1 ATR stop, 20 bars, stop first.

### Event 2 — Opening Range Fade (ORF)

- Setup: the first 30 minutes close is at least 1.0 ATR from the session open.
- Direction: opposite the opening move.
- Entry: the next 15m open.
- Barrier: +2 ATR target, −1 ATR stop, 20 bars, stop first.

### Event 3 — New York Close Fade (NCF)

- Setup: the last completed 15m bar before 21:00 UTC; the session move from the
  13:30 open is at least 1.0 ATR.
- Direction: opposite the session move.
- Entry: the next 15m open (21:00 UTC).
- Barrier: +2 ATR target, −1 ATR stop, 20 bars, stop first.

## 5. Base and controls

| Item | Rule |
|---|---|
| ORB/ORF base | All 15m bars in the same session (London or New York), direction-matched |
| NCF base | All 15m bars in the New York close window, direction-matched |
| Event base | Event-weighted average of the bundle session base rates |
| Random | 200 draws from the session bars, excluding event entries |
| Back-lag | The same direction 20 bars before the event |
| Shuffle | Shuffle event directions within the session |
| Correction | Bonferroni over the three primary XAU cells; cell alpha 0.0167; z ≈ 2.394 |

## 6. Primary and secondary cells

- Primary: XAUUSD ORB, ORF, NCF.
- Secondary: BTCUSD and ETHUSD, same three events.

A primary cell passes only when all hold:

- `n ≥ 200`, clusters `≥ 20`;
- conditional lift over the session base `≥ +5pp`;
- conditional LCB `> 0`;
- random `p ≤ 0.05`;
- back-lag pass;
- shuffle `p ≤ 0.05`.

The secondary crypto cells are reported but are not required for a primary
pass. If at least one crypto cell agrees in sign with `n ≥ 50`, the XAU result
is stronger. If none agrees, a XAU-only charter is still possible with the
single-instrument limitation disclosed.

A primary cell below `n = 200` is recorded as `inconclusive — underpowered`.

## 7. Decision

| Outcome | Action |
|---|---|
| No primary XAU cell passes | Record `no edge`; stop the session premise; do not tune |
| A XAU cell passes, no crypto agreement | Write a XAU-only model program charter; disclose the single-instrument limitation |
| A XAU cell passes, crypto agrees | Write a broader session model program charter |
| Stage A shows no XAU session structure | Stop before the barrier test |
| Sample below 200 | Record `inconclusive — underpowered`; stop |

A charter is a new **model program**. It defines the session context/events in
the model and a model gate. The spike does not open validation or run a strategy.

## 8. Planned outputs

- `docs/improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE.md`
- `tools/improvement/session_spike.py`
- `tools/improvement/test_session_spike.py`
- `data/improvement/2026-11-04/SESSION-LIQUIDITY-SPIKE/`
- A program charter or a stop record
- An AI transparency log
