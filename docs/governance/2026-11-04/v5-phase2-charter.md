# V5 Phase 2 — Program Charter

- Date: 2026-11-04 (UTC)
- Program ID: `PROGRAM-2026-11-04-05`
- Status: **Approved** (human, 2026-11-04); Phase 2 exploration executed — **no edge**; volume family blocked by data quality; program stopped
- Owner: human
- Contract: [`research-mandate.md`](research-mandate.md)
- Plan: [`docs/execution-plans/2026-11-04/v5-model-program.md`](../../execution-plans/2026-11-04/v5-model-program.md)
- Pre-registration: [`docs/improvement/2026-11-04/V5-PHASE2-SPIKE-PREREG.md`](../../improvement/2026-11-04/V5-PHASE2-SPIKE-PREREG.md)

## 1. Premise

The v4 information surface is exhausted for directional claims: its events are
descriptive at 20 bars and at 100 bars, and the 1h swing gate failed. Phase 2
tests a different information surface that the v4 model does not compute:
**participation, candle anatomy, and cross-asset positioning**.

The claim is that a bar's internal structure (where the close sits in the range,
how the wick interacts with a confirmed level) and an instrument's relative
strength against its peers carry directional information that a moving-average
or oscillator event does not.

## 2. Economic rationale

- **Absorption and initiative flow.** A wide-range bar that closes at its
  extreme is a public trace of aggressive takers; a long wick at a confirmed
  level is a trace of absorbed liquidity. If either reflects real order flow
  rather than noise, the next move should continue or reverse respectively.
- **Cross-asset positioning.** Capital rotates between BTC, ETH, and gold. An
  instrument that gains relative to its peers may continue to attract flow, and
  the absolute price may follow the relative move.
- **Cost.** At a 4 ATR target over 100 bars, a 12 bps round trip is a small
  fraction of the move. The gate is pre-cost, but the payoff is sized so that a
  real edge would survive the strategy tier.

## 3. Tier and frozen inputs

| Field | Value |
|---|---|
| Tier | `model` |
| Stage | read-only exploration; no model code change |
| Changed | candidate information and events |
| Frozen | data engine, model code, policy, strategy, execution, portfolio, analytics |
| Data | the v4 model pack at 1h, `adx25` (`data/model-v4/dev`) |
| Partitions | `model_dev` 2018–2020 for exploration; `model_validation` 2021–2022 for one-shot confirmation |
| Costs | 4 bps commission, 2 bps slippage; the spike is pre-cost |

## 4. Candidates

The spike screens three price-only candidates and one diagnostic:

| ID | Candidate | Family | Status |
|---|---|---|---|
| `C1` | Momentum bar: wide range, large body, close at the extreme | candle anatomy | primary |
| `C2` | Rejection wick at a confirmed major level | candle anatomy + structure | primary |
| `C3` | Relative-strength cross against the BTC/ETH/XAU peer basket | cross-asset | primary |
| `D1` | Gap continuation, XAUUSD only | gaps | diagnostic, underpowered |

Exact rules, thresholds, and the barrier are fixed in the pre-registration.

The volume family — VWAP, volume profile, and volume climax — is **blocked** by
the data audit: the pack's volume series changes by 600×–17,800× within a year on
BTC and ETH, so a volume ratio is not comparable across the sample. XAU volume is
stable but is a single instrument. No volume claim is made or tested here.

## 5. Partitions and data budget

| Partition | Window | Use |
|---|---|---|
| `model_dev` | 2018-01-01..2020-12-31 | Exploration and candidate selection |
| `model_validation` | 2021-01-01..2022-12-31 | One-shot confirmation, only after a frozen Phase 2 model |

The spike selects on `model_dev`; therefore a survivor is **not** confirmed by a
second `model_dev` run. Confirmation runs on `model_validation` with the frozen
implementation. The validation partition has been used by the accepted
compression baselines; it has not been used for these candidates. The reuse is
disclosed.

## 6. Correction

| Field | Value |
|---|---|
| Method | `bonferroni` |
| Primary candidates | 3 |
| Family-wise alpha | 0.05 |
| Notes | `z = 2.394`; the diagnostic does not enter the correction |

## 7. Green criteria

A spike candidate passes only when all hold on `model_dev`:

- `n >= 200` events and `J >= 20` fixed 100-bar blocks;
- lift over the direction-matched 1h base `>= +0.05`;
- lift LCB `> 0` at `z = 2.394`;
- random control `p <= 0.05`;
- back-lag (100 bars) control passes;
- shuffled-direction control `p <= 0.05`;
- at least two instruments have `n >= 30` and `hit_rate > base`.

A pass is **exploratory evidence**, not a model baseline and not an edge claim.
It promotes the candidate to a Phase 2 model charter.

## 8. Concurrency and ownership

| Field | Value |
|---|---|
| Max parallel cells | 1 |
| Worktree root | `<repository>--v5-phase2-spike` |
| Ownership | one branch and one worktree |
| Shared read-only | v4 model pack, source CSVs |
| Shared write | only `config/v5-phase2/`, the spike docs, and `data/improvement/<date>/V5-PHASE2-SPIKE/` |

## 9. Stop rule

The program stops when:

- no candidate passes the spike;
- a candidate passes but the confirmatory `model_validation` run fails;
- the volume data cannot be repaired and no price-only candidate passes;
- the human stops it.

A stopped program does not open a strategy. A new premise needs a new charter.

## 10. Validation access

| Field | Value |
|---|---|
| Access | `none` — no candidate passed the exploration |
| Approved by | not applicable |
| Approved on | not applicable |
| Conditions | a candidate must pass the exploration before a frozen model is built |

## 11. Human approval

- Approved by: human
- Approved on: 2026-11-04
- Scope: the Phase 2 exploration, the three price-only candidates, and the
  volume-family block
- Not authorized: a volume claim on the current data, a different candidate
  after the result, or validation access before a frozen model

## 12. Closeout

| Field | Value |
|---|---|
| Verdict | `no_edge` |
| Accepted baseline | none |
| Registry | [`docs/improvement/2026-11-04/V5-PHASE2-SPIKE.md`](../../improvement/2026-11-04/V5-PHASE2-SPIKE.md) |
| Evidence | `data/improvement/2026-11-04/V5-PHASE2-SPIKE/` (local) |
| Cleanup | worktree and branch removed after preservation |

Phase 2 stopped on 2026-11-04. All three price-only candidates failed. The
volume family is untested because BTC/ETH volume units change by 600×–17,800×
within a year; XAU volume is stable but is one instrument. No model, validation,
or strategy work started.
