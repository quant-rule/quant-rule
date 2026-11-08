# SR2-A BTCUSD Robustness Audit

- Date: 2026-11-08 (UTC)
- Status: **Complete — fragile BTC lead; no candidate; 2024 not opened**
- Parent run: `STRATEGY-2026-11-08-02`
- Parent record: [`STRATEGY-2026-11-08-02-sr2-dual.md`](STRATEGY-2026-11-08-02-sr2-dual.md)
- Plan: [`sr2-btc-audit.md`](../../execution-plans/2026-11-08/sr2-btc-audit.md)
- Claim type: read-only robustness audit; no edge or profitability claim

## 1. Question

SR2-A (1h, 1 ATR stop / 3 ATR target, fade the compression breakout) was the
first design with a positive pooled BTCUSD result: +$3,545 over 139 trades,
profit factor 1.42, +35.4% over five years at the 10% drawdown ceiling. Is that
a robust lead, or a few trades and a favourable regime?

## 2. Headline

| Check | Result | Verdict |
|---|---|---|
| 9 bps round trip | +$3,060.65, PF 1.376, 2 halted cells | survives in sign |
| 5% drawdown ceiling | +$3,032.40, PF 1.490, 4 halted cells, observed max 6.23% | survives in sign, but breaches 5% |
| Bootstrap mean trade, 95% CI | −$5.83 to +$60.78 | includes zero |
| t-stat, mean trade | +1.53 | not significant |
| Top 5 trade share | 89.7% of net P&L | highly concentrated |
| Without top 5 | +$364 | almost gone |
| Without top 10 | −$1,511 | negative |
| 2019 | −$692, −0.267R path | regime-dependent |
| 2024 validation | not opened | — |

The BTCUSD lead survives the 9 bps and 5% runs in sign, but it is **not
statistically significant, is concentrated in the top five trades, and is
regime-dependent**. It is a research lead, not an edge.

## 3. Results

| Metric | Base 6 bps | 9 bps | 5% drawdown |
|---|---:|---:|---:|
| Trades | 139 | 131 | 96 |
| Net P&L | +3,544.69 | +3,060.65 | +3,032.40 |
| Gross P&L | +4,138.93 | +3,896.83 | +3,441.32 |
| Commission | 594.25 | 836.18 | 408.92 |
| Profit factor | 1.420 | 1.376 | 1.490 |
| Max cell drawdown | 10.06% | 10.37% | 6.23% |
| Halted cells | 1 | 2 | 4 |

The 5% run halts four of five cells. The observed maximum drawdown is 6.23%,
not 5%: the portfolio halt is checked at bar close, so a gap can overshoot the
ceiling. A strict 5% ceiling needs a lower soft limit or an intra-bar control.

### Per year

| Year | Trades | Net P&L | Max DD | PF | Halted |
|---|---:|---:|---:|---:|---|
| 2018 | 25 | +531.47 | 5.71% | 1.294 | no |
| 2019 | 20 | −692.24 | 10.06% | 0.504 | yes |
| 2020 | 35 | +1,814.82 | 5.90% | 2.225 | no |
| 2021 | 25 | +1,025.32 | 9.94% | 1.520 | no |
| 2022 | 34 | +865.33 | 3.99% | 1.483 | no |

## 4. Concentration and bootstrap

| Measure | Value |
|---|---:|
| Mean net trade | +$25.50 |
| Standard deviation | $197.11 |
| t-stat | +1.53 |
| Bootstrap mean 95% CI | −$5.83 to +$60.78 |
| Bootstrap total 95% CI | −$810 to +$8,448 |
| Top 5 share of net P&L | 89.7% |
| Net without top 1 / 3 / 5 / 10 | +2,535 / +1,337 / +364 / −1,511 |

### Top five trades

| Date | Side | Entry | Exit | Net P&L | Hold | Exit |
|---|---|---|---:|---:|---:|---|
| 2020-03-15 | long | 5,118.71 | 5,640.68 | +1,009.64 | 0 | target |
| 2018-04-06 | long | 6,503.25 | 6,922.61 | +640.71 | 13 | target |
| 2021-02-25 | long | 48,996.50 | 51,749.49 | +557.69 | 3 | target |
| 2020-03-25 | short | 6,834.62 | 6,464.75 | +536.61 | 1 | target |
| 2021-01-31 | short | 33,960.60 | 32,437.39 | +435.62 | 4 | target |

The top trades are clustered in high-volatility events: the March 2020 COVID
crash, the 2018 April move, and the 2021 volatility. The result is not a
steady per-trade edge.

## 5. Path analysis

The path enters at the next bar, fades the event with a 1 ATR stop and a 3 ATR
target, 20-bar horizon, 6 bps cost.

| Group | Bucket | n | Mean R | t-stat | Target | Stop |
|---|---|---:|---:|---:|---:|---:|
| All | — | 171 | +0.074 | +0.56 | 25.1% | 67.3% |
| Year | 2018 | 28 | +0.013 | +0.04 | 25.0% | 71.4% |
| Year | 2019 | 43 | −0.267 | −1.12 | 18.6% | 76.7% |
| Year | 2020 | 37 | +0.183 | +0.64 | 24.3% | 62.2% |
| Year | 2021 | 25 | +0.346 | +0.94 | 32.0% | 60.0% |
| Year | 2022 | 38 | +0.221 | +0.76 | 28.9% | 63.2% |
| Side | long | 84 | +0.135 | +0.70 | 25.0% | 65.5% |
| Side | short | 87 | +0.016 | +0.09 | 25.3% | 69.0% |
| ATR ratio | high | 86 | +0.327 | +1.65 | 31.4% | 60.5% |
| ATR ratio | low | 85 | −0.182 | −1.04 | 18.8% | 74.1% |
| RVOL ratio | high | 86 | −0.127 | −0.73 | 19.8% | 70.9% |
| RVOL ratio | low | 85 | +0.278 | +1.39 | 30.6% | 63.5% |
| Session UTC | 00–06 | 20 | +1.010 | +2.32 | 50.0% | 45.0% |
| Session UTC | 06–12 | 59 | −0.332 | −1.67 | 15.3% | 79.7% |
| Session UTC | 12–18 | 64 | +0.069 | +0.32 | 25.0% | 65.6% |
| Session UTC | 18–24 | 28 | +0.273 | +0.81 | 28.6% | 60.7% |

The condition buckets are **exploratory**. The 00–06 UTC bucket has only 20
events and the table tests many buckets; its t-stat is not corrected for
multiple comparisons.

## 6. What the result says

- The BTCUSD fade has a small positive path edge in the pooled sample, but it is
  not statistically distinguishable from zero. The 95% bootstrap interval for
  the mean trade includes zero.
- The P&L is concentrated in a handful of high-volatility trades. Removing the
  top five leaves +$364; removing the top ten leaves −$1,511.
- The edge is regime-dependent: 2019 is negative, 2020–2022 carry the result.
- The 9 bps run is still positive (+$3,061), so the result is not purely a cost
  artifact. The 5% run is still positive (+$3,032), but the observed drawdown is
  6.23% and four cells halt; the 5% ceiling is not strictly met.
- EURUSD and XAUUSD remain negative on both the fade and continuation sides.
  BTC-only is the only candidate; the other instruments do not support this
  event.
- The 15m design (SR2-B) is negative; 1h is the better timeframe. 5m is not
  tested and is cost-sensitive. 1m is retired.

### Condition hypotheses for a new programme

These are hypotheses, not findings. Each needs a new charter and a one-shot
out-of-sample test.

1. **Fade quiet breakouts, avoid high-RVOL breakouts.** Low RVOL events have
   +0.278R versus −0.127R for high RVOL. A high-RVOL breakout may be informed
   continuation, not a failed breakout.
2. **Fade high-ATR events, avoid low-ATR events.** High ATR ratio has +0.327R
   versus −0.182R. The event may need enough volatility to reverse.
3. **Test the Asian session.** 00–06 UTC has +1.010R over 20 events; 06–12 UTC
   is −0.332R. The sample is small and the comparison is uncorrected.
4. **Use a signal event for direction.** `signal.compression_breakout` is a
   `context` event; a directional `signal` event (EMA stack, DI cross, level
   break) may be a better side selector, with the compression event as the
   volatility/location filter.
5. **Use 1h and 5m, not 15m.** 1h is the only positive timeframe; 15m is
   negative; 5m is untested and must clear the cost gate.

## 7. Goal gap

| Basis | 5-year total | Monthly equivalent | Gap to 10%/month |
|---|---:|---:|---:|
| Base 10% DD | +7.09% | +0.114% | ~284× total |
| 5% DD | +6.06% | +0.098% | ~287× total |

A 10%/month compounded target is 1.10^60 − 1 = 303.5× over five years. The
current lead is about 0.1%/month at the 5% ceiling. This is not a tuning gap; it
requires a fundamentally stronger or more frequent edge. The 5% run also
breaches the ceiling at 6.23%, so a strict 5% envelope needs a lower soft halt or
intra-bar risk control.

## 8. Recommendation

**Do not promote or freeze SR2-A.** The audit does not validate an edge. The
next step is a new pre-registered BTC-only hypothesis with one or two
mechanism-based conditions from Section 6, tested on `strategy_dev` and then
2024 one-shot. The 10%/month target should stay a north star, not a threshold
for accepting the next run.

## 9. Limitations

- One instrument and five years; 2024 is not opened.
- The condition buckets are exploratory and multiple-comparison-prone.
- The 5% drawdown halt is bar-close and can overshoot.
- The 9 bps and 5% runs change fills and position state; they are full runs, not
  post-hoc equity edits.
- `signal.compression_breakout` is registered `context`; the sole-entry use is
  the disclosed SR1 exception.
- The Quant Developer and Quant Validator passes are sequential same-agent
  passes, not independent agents.

## 10. Links

- [Parent SR2 record](STRATEGY-2026-11-08-02-sr2-dual.md)
- [Plan](../../execution-plans/2026-11-08/sr2-btc-audit.md)
- [AI transparency log](../../ai-transparency-log/2026-11-08/sr2-btc-audit.md)
- [Audit tool](../../../tools/improvement/sr2_btc_audit.py)
- Local artifacts: `data/improvement/2026-11-08/SR2-BTC-AUDIT/`
