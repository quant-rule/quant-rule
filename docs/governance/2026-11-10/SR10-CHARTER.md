# SR10 Charter — 15m EMA/Fibonacci Entry with Exit Variants

- Date: 2026-11-10 (UTC)
- Program ID: `PROGRAM-2026-11-10-01`
- Status: **Approved — development only; 2024 closed**
- Tier: strategy
- Model: `M1.2` (pivot sequence, Fibonacci, horizontal bands, and giveback exit)
- Predecessor evidence: [`annotation-zone-analysis.md`](../../reports/2026-11-10/annotation-zone-analysis.md)
- Claim type: strategy research; no edge claim.

## 1. Premise

A 15m entry that combines an EMA-fast tap with a Fibonacci retracement test
captures the annotated swing zones better than the break-only control. The exit
variant is the primary question: a giveback exit should retain more of the
favorable excursion than a structure-flip exit or a force-loss exit.

The program is a bounded test of one entry rule and three exits. It is not a
search for a passing configuration.

## 2. Economic rationale

The 2018–2019 zone study found:

- EMA tap in the zone: 91.2% of long entries and 87.9% of short entries;
- fast-EMA tap: 86.5% long and 83.2% short;
- Fibonacci test in the zone: 85.7% of both entry groups;
- entries are below VWAP/EMA for long and near/above VWAP for short;
- exits are extended from VWAP and the fast EMA.

The entry rule uses the two strongest, registered entry-side events. The exit
rule is the part that needs testing: the human long exits are local highs but
only about 57–62% are followed by a lower price at 50 bars, so a giveback exit
may retain more of the move than a structure flip.

## 3. Model-baseline gate

`M1.2` is **not** an accepted model baseline. The new signal events have
`predictive_status: pending`. This charter requests a human-approved,
development-only research exception:

- allowed: `strategy_dev` 2018–2022 on the recorded BTCUSD 15m bundles;
- not allowed: `strategy_validation` 2024, the portfolio holdout, or a release
  claim;
- without approval, the strategy tier stays closed and this charter is not run.

The strategy does not rescue a model cell. The zone study motivates the entry;
it is not a strategy result.

Partition note: the zone evidence is Binance BTCUSDT, which is admitted for
cross-venue robustness outside the formal partitions. The formal primary cells
therefore use Dukascopy BTCUSD; Binance is a descriptive cross-venue check, not a
candidate and not part of `strategy_dev`.

## 4. Frozen inputs

| Field | Value |
|---|---|
| Tier | strategy |
| Changed | strategy evaluation only |
| Frozen | data, policy, model, execution, portfolio |
| Instrument | Dukascopy BTCUSD 15m (formal `strategy_dev`) |
| Robustness | Binance BTCUSDT 15m, descriptive only; outside formal partitions |
| Timeframe | 15m execution and 15m context |
| Event window | 3 bars |
| Development | `strategy_dev` 2018-01-01..2022-12-31 |
| Validation | 2024; closed |
| 2023 | excluded |
| Costs | 6 bps base; 9 bps sensitivity |
| Account | $10,000 per cell; fixed notional; 1x leverage; one position |
| Drawdown | 5% per cell, hard halt |
| Time stop | 50 bars |
| Stop / target | 1 ATR stop; 3 ATR target (3:1) |

## 5. Entry rule

Long entry, both conditions:

- `level.fib_test` (event type 1, subtype 1, source 56), direction +1;
- `level.test` (event type 1, subtype 1, source 1, fast EMA), direction 0.

Short entry, both conditions:

- `level.fib_test` direction −1;
- `level.test` source 1, direction 0.

The event window is 3 bars. No state filter is used in the primary rule; the
zone study did not show a single stable EMA-stack value. The entry is a
conjunction of two registered signal events; the Fibonacci event supplies the
direction.

## 6. Cells

| Cell | Side | Exit | Role |
|---|---|---|---|
| `SR10.1` | long | giveback | primary |
| `SR10.2` | short | giveback | primary |
| `SR10.3` | long | structure flip | control |
| `SR10.4` | short | structure flip | control |
| `SR10.5` | long | MACD histogram slope flip | variant |
| `SR10.6` | short | MACD histogram slope flip | variant |

Exit definitions:

- **Giveback:** `giveback_atr = 1.0`, `giveback_arm_atr = 0.5`; no event exit.
- **Structure flip:** `structure_state = -1` for long, `+1` for short.
- **Force loss:** `signal.macd_histogram_slope_flip` direction −1 for long,
  `+1` for short.

All cells keep the 1 ATR stop, 3 ATR target, and 50-bar time stop. The giveback
cell has no event-exit condition; the time stop and bracket remain backstops.

## 7. Controls and ablations

Descriptive only, not candidates:

- break-only control: `signal.compression_breakout` with the same exit and risk;
- EMA-only entry: remove `level.fib_test`;
- Fibonacci-only entry: remove `level.test`;
- no-giveback control: the structure-flip control.

The break-only control is not promoted.

## 8. Criteria

Each primary cell (`SR10.1`, `SR10.2`):

- at least 30 trades over 2018–2022;
- PF > 1.1 at 6 bps;
- PF > 1.0 at 9 bps;
- max drawdown ≤ 5%;
- no halt.

The primary giveback cell must beat its matched structure-flip control on PF at
6 bps and 9 bps. A tie is not a pass. The force-loss and control cells are
descriptive; they do not become candidates by passing.

The Binance run is a robustness check outside the formal partitions, not a
primary cell. It must not reverse the sign of the BTCUSD result at 6 bps.

## 9. Multiple testing

Six planned trials: two primary giveback cells, two structure controls, and two
force-loss variants; the 9 bps runs are sensitivity checks. Bonferroni
correction at family alpha 0.05 over the six planned cells. The break-only
control and entry ablations are not candidates and are not counted as trials.

## 10. Iteration cap and stop rule

- Iteration cap: one development pass per cell; no threshold search.
- Stop when a primary cell is underpowered, fails the criteria, loses to its
  matched control, or the data coverage is inadequate.
- A failed program is retired. It is not re-registered with a different
  threshold, stop, target, timeframe, or filter.
- No control promotion and no post-hoc filter change.

## 11. Human approval

- Premise: **Approved**
- Model-baseline exception: **Approved**
- Development access: **Approved**
- Validation access: **Closed**
- Approved by: human, 2026-11-10 (“yes continue” after the BTCUSD primary revision)
- Not authorized: 2024, portfolio holdout, control promotion, post-hoc changes.

## 12. Artifacts

| Artifact | Path |
|---|---|
| Strategy configs | `config/sr10/strategy/` |
| Program charter | this file |
| Run record | `data/improvement/2026-11-10/SR10/` |
| Zone evidence | `docs/reports/2026-11-10/annotation-zone-analysis.md` |
| Baseline / no-edge | `docs/improvement/baselines.md` |
| AI log | `docs/ai-transparency-log/2026-11-10/sr10-ema-fib.md` |
