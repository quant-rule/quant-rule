# Model Structure Extensions and Position-Aware Exits

- Date: 2026-11-10 (UTC)
- Status: **Approved**
- Work item: `model-structures`
- Workflows: [Model Change](../../workflows/model-change.md), [Requirements & Design](../../workflows/requirements-design.md), [Strategy Research](../../workflows/strategy-research.md) for the analysis phase
- Claim type: model implementation and strategy/execution contract; no edge claim.
- Quant roles: Quant Researcher, Quant Developer, Test Engineer, Quant Validator.

## 1. Intent

Add the structural measurements the 2018–2019 annotation review showed are missing, and make position excursions available to the strategy so giveback/trailing exits can be tested.

The immediate user problem is:

- good entries often occur at an EMA tap or a second/third lower high or lower low;
- the model has EMA level tests and one-step swing structure, but no multi-pivot sequence count;
- volume profile exists, but Fibonacci retracements and explicit horizontal support/resistance bands do not;
- exits need to react to “the move lost force”, gains given back, and time in trade;
- the strategy cannot currently see entry price, unrealized excursion, or MFE/MAE.

The outcome is a set of registered, causal, tested model readings/events plus a position-excursion contract. The strategy analysis follows only after those measurements exist.

## 2. Current state

| Item | Observed |
|---|---|
| Model identity | `M1.1`; registry `2026-11-09.1` |
| Indicators | 46 |
| Events | 146; codes 1–146 |
| Sources | 1–54 |
| EMA taps | `level.test`, `level.cross`, `level.break`; sources 1/2/3 are EMA fast/mid/slow |
| Swing structure | `swing_structure` compares the last two confirmed major pivots and publishes `structure_state` |
| Pivots | `pivots` publishes confirmed minor/major highs/lows with `occurred_at` and `available_at` |
| Volume profile | `volume_profile`: 96 bars, 24 bins, 70% value area, POC/VAH/VAL, skew, concentration, migration |
| Fibonacci | Absent from the registry and model |
| Horizontal bands | `level_strength` counts touches within 0.2 ATR; `level.test`/`level.strength_test`/`level.break`/`level.bos`/`level.choch` exist; no explicit band object |
| Force-loss proxies | MACD histogram slope flip, ADX slope, ATR slope, range expansion, path efficiency, divergence, climax/absorption, Aroon, bars since extreme |
| Time stop | `time_stop_bars`; SR9 uses 50 |
| Trailing stop | Absent; only fixed stop/target bracket and time stop |
| Strategy position | `side`, `entry_bar`, `bars_held` only; no entry price, unrealized return, MFE, or MAE |
| Annotations | 2018: 595 marks / 11 charts; 2019: 1,025 marks / 23 charts; 1,620 total. Backed up at `2026-11-10T01:53:02Z`, SHA-256 `edec9e106fd6baa50432cfd68a637eb5e3b178b32c28d35b71714da3c26cf480` |

The model computes only from OHLCV. Volume profile is a rolling candle-volume approximation, not a tick or order-book profile.

## 3. Scope

### Included

- A new `pivot_sequence` indicator with lower-high, lower-low, higher-high, and higher-low counts.
- Two sequence events: `signal.lower_high_sequence` and `signal.lower_low_sequence`.
- A new `fibonacci` indicator with fixed retracement ratios, causal anchors, level prices, position, and nearest-level distance.
- One Fibonacci event: `level.fib_test`.
- A new `horizontal_band` indicator that clusters confirmed major pivot levels into a band with edges, touches, age, width, and position.
- One band event: `level.band_test`.
- Model artifact plumbing: readings CSV, provenance, trade-context columns, golden fixtures, registry, validation, verification, generated matrices, docs.
- Strategy position excursions: entry price, entry ATR, unrealized ATR, MFE ATR, MAE ATR, giveback ATR.
- A strategy giveback/trailing exit condition using those position fields.
- A zone-based 2018–2019 annotation analysis after the model and strategy contracts are in place.
- A pre-registered exit hypothesis after the analysis; no strategy code change before that hypothesis is approved.

### Excluded

- New data sources, tick data, order books, or volume-at-price from trades.
- Statistical, machine-learning, neural-network, or fitted signal generation.
- A search for profitable parameters.
- 2024 access; 2023 remains outside the formal partitions.
- Changing an existing command’s meaning.
- Deleting the annotator, its data, or its archives.
- An execution-engine broker-side trailing stop in this pass. The first trailing/giveback rule is strategy-managed and fills at the next open; an execution-level trailing stop is a separate Portfolio Research change.
- A profitability or edge claim.

## 4. Decisions requiring approval

| ID | Decision | Consequence | Status |
|---|---|---|---|
| D1 | Model identity becomes `M1.2` (M1.1 + structure extensions) | Contract, registry, and model-family registry update | Proposed |
| D2 | Registry version becomes `2026-11-10.1` | `REGISTRY_VERSION` and generated tables update | Proposed |
| D3 | Pivot sequence uses confirmed major pivots only; count consecutive same-kind pivots; availability is the latest pivot’s confirmation bar | Causal, no candidate or future pivot | Proposed |
| D4 | Fibonacci anchors are the latest confirmed major pivot and the most recent confirmed major pivot of the opposite kind before it | Causal leg selection; no manual anchors | Proposed |
| D5 | Band clustering uses confirmed major levels within the existing level lookback; single-linkage at `band_atr * ATR`; select the cluster with most levels, tie-break nearest close then lower center; require two levels | Deterministic bands; no fitted clustering | Proposed |
| D6 | Position view adds entry price, entry ATR, unrealized ATR, MFE ATR, MAE ATR, giveback ATR; no cash, equity, P&L, fill, or quantity | Strategy can test giveback exits without touching accounting | Proposed |
| D7 | First trailing/giveback exit is a strategy condition, not an execution trailing-stop order | Smaller, testable change; execution trailing stop deferred | Proposed |
| D8 | New indicator codes 47–49; sources 55–57; event codes 147–150 | Stable identities; no reuse | Proposed |

D1, D2, D3, D5, D6, and D7 require explicit human approval before implementation. D4 and D8 are part of the approved design if D1–D3 are approved.

## 5. Feature contracts

### 5.1 `pivot_sequence` — indicator 47

- Taxonomy: `structure`.
- Role: `signal`.
- Inputs: confirmed major pivot highs and lows from `pivots`.
- Availability: the confirmation bar of the latest pivot in the sequence.
- Outputs:
  - `lower_high_count`
  - `lower_low_count`
  - `higher_high_count`
  - `higher_low_count`
- Formula:
  - For confirmed major highs in occurrence order, `lower_high_count` is the number of consecutive pairs where the newer high is strictly lower than the previous high, ending at the latest confirmed high; zero when the latest pair is not lower.
  - Mirror for lows and for higher sequences.
  - Equal highs/lows break the sequence and produce zero, not a directional count.
- Events:
  - `signal.lower_high_sequence` — code 147, type 3, subtype 19, source 55, direction −1. Emits on the latest high’s confirmation bar when `lower_high_count >= 2`; `primary_value` is the count, `reference_value` is 2.
  - `signal.lower_low_sequence` — code 148, type 3, subtype 19, source 55, direction −1. Emits on the latest low’s confirmation bar when `lower_low_count >= 2`.
- Claim: a sequence of two or more consecutive confirmed lower highs or lower lows is followed by a directional continuation more often than the base rate. `predictive_status: pending`.

### 5.2 `fibonacci` — indicator 48

- Taxonomy: `structure`.
- Role: `signal`.
- Inputs: confirmed major pivots and ATR.
- Anchor:
  - If the latest confirmed major pivot is a high, the leg is the most recent confirmed major low before it to that high.
  - If the latest confirmed major pivot is a low, the leg is the most recent confirmed major high before it to that low.
  - Both anchors must be confirmed and available at the bar; otherwise all outputs are null.
- Fixed ratios: `0.236`, `0.382`, `0.500`, `0.618`, `0.786`.
- Outputs:
  - `fib_direction` (+1 up leg, −1 down leg)
  - `fib_low`, `fib_high`
  - `fib_236`, `fib_382`, `fib_500`, `fib_618`, `fib_786`
  - `fib_position` (0 at leg start, 1 at leg end)
  - `fib_nearest_ratio`
  - `fib_nearest_atr`
- Levels:
  - Up leg `L -> H`: `level(r) = H - r * (H - L)`.
  - Down leg `H -> L`: `level(r) = L + r * (H - L)`.
- Event:
  - `level.fib_test` — code 149, type 1, subtype 1, source 56. Emits when the bar range intersects any level within `fib_test_atr * ATR`; direction is the leg direction; `primary_value` is close, `reference_value` is the level price, `reference_code` is the ratio code (236, 382, 500, 618, 786). One event per tested level, deduplicated by code/source/level/ratio.
- Claim: a test of a Fibonacci retracement level of the last confirmed major swing leg is followed by a reaction in the leg direction more often than the base rate. `predictive_status: pending`.
- Parameter: `model.fibonacci.test_atr`, default `0.2`, unit ATR, configurable.

### 5.3 `horizontal_band` — indicator 49

- Taxonomy: `structure`.
- Role: `signal`.
- Inputs: confirmed major pivot levels and ATR.
- Availability: the bar on which at least two confirmed major levels exist in the lookback.
- Clustering:
  - Sort confirmed major levels by price.
  - Start a new cluster when the gap to the previous level exceeds `band_atr * ATR`.
  - Select the cluster with the most levels.
  - Tie-break by the smallest distance from the cluster center to close, then by the lower center.
  - Require at least two levels; otherwise all outputs are null.
- Outputs:
  - `band_upper`, `band_lower`, `band_center`
  - `band_touches`
  - `band_age` (bars since the youngest level in the cluster)
  - `band_width_atr`
  - `band_position` (`(close - lower) / (upper - lower)`; null when width is zero)
- Event:
  - `level.band_test` — code 150, type 1, subtype 1, source 57. Emits when the bar range intersects the selected band expanded by `band_atr * ATR`; direction is +1 when close is at or above `band_center` (support test) and −1 below it (resistance test); `primary_value` is close, `reference_value` is the center.
- Claim: a test of a multi-touch horizontal band is followed by a reaction away from the band more often than a single-level baseline. `predictive_status: pending`.
- Parameter: `model.band.cluster_atr`, default `0.25`, unit ATR, configurable. `level_lookback` is reused; the minimum touch count is fixed at 2.

### 5.4 Model artifacts

- `ExtendedReadings` adds the 22 new columns.
- `ExtendedConfig` adds `fib_test_atr` and `band_atr`.
- `crates/quantrule-shell/src/model_bundle/artifacts.rs` updates the readings header and every row.
- `crates/quantrule-shell/src/model_bundle/provenance.rs` records the new config values.
- `tools/trades/` context exports pick up the new columns automatically; the contract doc records the new column names.
- Old bundles without the new columns must still load.

### 5.5 Strategy position excursions

`StrategyPosition` adds:

| Field | Meaning |
|---|---|
| `entry_price` | Actual fill price of the open position; `None` when flat |
| `entry_atr` | ATR at the entry bar, fixed for the trade |
| `unrealized_atr` | Side-adjusted `(current close - entry_price) / entry_atr` |
| `mfe_atr` | Maximum favorable excursion since entry, in entry ATR |
| `mae_atr` | Maximum adverse excursion since entry, in entry ATR |
| `giveback_atr` | `mfe_atr - unrealized_atr`; zero when flat |

The runner updates these from closed bars only. The strategy still receives no cash, equity, P&L, fill, order, or quantity. A source-level boundary test enforces that.

### 5.6 Giveback/trailing exit condition

Add a position condition to the event-composite exit list:

```json
"exit": [
  {"position": "giveback_atr", "op": ">=", "value": 1.0}
]
```

Supported fields: `bars_held`, `unrealized_atr`, `mfe_atr`, `mae_atr`, `giveback_atr`. This is a strategy-managed trailing/giveback exit; the fill is at the next open. A true execution-level trailing stop order is out of scope for this pass.

## 6. Work breakdown

| # | Sub-issue | Depends on | Roles | Main paths |
|---|---|---|---|---|
| 1 | Pivot sequence reading and events | plan approval | Quant Developer, Test Engineer | `readings/extended/structure.rs`, `events/extended/structure.rs`, `events/codes.rs`, registry |
| 2 | Fibonacci reading and event | 1 (shared structure module) | Quant Developer, Test Engineer | `readings/extended/fibonacci.rs`, `events/extended/fibonacci.rs`, registry |
| 3 | Horizontal band reading and event | 1 | Quant Developer, Test Engineer | `readings/extended/band.rs`, `events/extended/band.rs`, registry |
| 4 | Model artifact plumbing and golden fixtures | 1–3 | Quant Developer, Test Engineer | `model_bundle/artifacts.rs`, `provenance.rs`, `extended_readings_golden.csv`, `extended_events_golden.csv` |
| 5 | Strategy position excursions | 4 | Quant Developer, Architect, Test Engineer | `execution/intent.rs`, runner, `strategy/*`, boundary tests |
| 6 | Giveback/trailing exit condition | 5 | Quant Developer, Test Engineer | event strategy config/parser, strategy tests, docs |
| 7 | Zone-based 2018–2019 analysis | 1–6 | Quant Researcher, Quant Developer, Quant Validator | `tools/trades/`, data annotation contexts, report |
| 8 | Verification, changelog, PRs | 1–7 | Test Engineer, Code Reviewer, Quant Validator | registry, validation, verification, docs, changelog |

Ordering: 1 → 2 and 3 (2 and 3 can be sequential to avoid shared-file conflicts) → 4 → 5 → 6 → 7 → 8. Each sub-issue gets its own branch/worktree and PR.

## 7. Acceptance criteria

- The model publishes `pivot_sequence`, `fibonacci`, and `horizontal_band` with the exact formulas above.
- Every new indicator and event has a registry entry, a claim, a validation claim, and verification evidence.
- Every new indicator and event has causality, determinism, golden, and edge-case tests.
- Equal highs/lows, missing anchors, insufficient pivots, zero ATR, and confirmation delays are covered.
- `lower_high_count`/`lower_low_count` never read a candidate or future pivot.
- Fibonacci and band outputs are null before the required confirmed anchors.
- Band tie-breaking is deterministic and documented.
- The readings CSV includes the new columns; old bundles still load.
- The model identity is `M1.2` and the registry version is `2026-11-10.1`.
- `StrategyPosition` exposes entry price, entry ATR, unrealized ATR, MFE ATR, MAE ATR, and giveback ATR.
- The boundary test proves the strategy cannot read cash, equity, P&L, fill, order, or quantity.
- The giveback exit condition is parsed, tested, causal, and deterministic.
- The 2018–2019 zone analysis is reproducible from the annotation snapshot.
- No strategy code change is made from the annotation analysis before a pre-registered exit hypothesis is approved.
- Full gate passes.

## 8. Verification

| Command | Expected |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets` | no warnings |
| `cargo test` | pass |
| `python3 tools/check_registry.py` | pass |
| `python3 tools/check_registry.py --write` | regenerates `indicators.md` |
| `python3 tools/check_validation.py` | pass |
| `python3 tools/check_verification.py` | pass |
| `python3 tools/check_verification_matrix.py` | pass |
| `python3 tools/check_event_emitters.py` | pass |
| `python3 tools/check_command_surface.py` | pass |
| `python3 tools/check_links.py` | pass |
| `python3 tools/check_changelog.py` | pass |
| `python3 tools/check_file_size.py` | pass |
| `python3 -m unittest discover -s tools/trades -p 'test_*.py'` | pass |
| `git diff --check` | pass |

Additional evidence:

- Golden fixture diff reviewed line by line; no casual regeneration.
- Quant Validator record for each model behavior change.
- Annotation zone report and reproducible command.
- No 2024 data accessed.

## 9. Risks and recovery

| Risk | Mitigation |
|---|---|
| Look-ahead in pivot sequence, Fibonacci anchors, or bands | Use only `confirmed_as_of`; availability tests; append/change/remove later bars |
| Band clustering tie ambiguity | Explicit sort and tie-break; deterministic tests |
| Registry/model drift | `check_registry.py`, `REGISTRY_VERSION`, `ECT` constants, generated Markdown |
| Golden fixture churn | Review every changed row; regenerate deliberately; document the reason |
| Position boundary creep | Source-level boundary test; no cash/equity/P&L/fill/order/quantity |
| Giveback exit fills differently than expected | Causal next-open fill tests; no intrabar assumption beyond execution contract |
| Overfitting to 2018–2019 annotations | Analysis is descriptive; pre-register before strategy changes; validation stays closed |
| Model version ambiguity | Human approves `M1.2` before implementation |
| Old bundle compatibility | New columns optional on read; explicit compatibility test |

Recovery: each sub-issue is a separate PR. If a feature fails its contract or validation, revert that PR without affecting the others. The annotator/data are untouched.

## 10. Stop conditions

- Human rejects `M1.2`, the position fields, or the strategy-managed trailing approach.
- The exact formula, anchor, availability, or tie-break cannot be stated.
- A new dependency is required.
- A test would need to be weakened or a golden fixture regenerated without review.
- The registry and code cannot be made to agree.
- The change becomes a defect fix or refactor rather than a model extension.
- The analysis requires 2024 data or a change to the partitions.
- The strategy analysis would change a rule before a pre-registered hypothesis is approved.

## 11. Records and handoff

- Plan: this file.
- AI log: `docs/ai-transparency-log/2026-11-10/model-structures.md`.
- Model contract: `docs/model/2026-11-10/model-m1.2.md` (to be written in sub-issue 1).
- Registry: `docs/registry/indicators.json`, `indicators.md`.
- Validation: `docs/registry/validation.json`.
- Verification: `docs/registry/verification.json`, generated matrices.
- Annotation snapshot: archive `2026-11-10T01:53:02Z`.
- Changelog: `[Unreleased] / Added` and `Changed` entries per sub-issue.
- Quant Validator: one record per model behavior change; one final record for the combined model version.

## 12. Progress — 2026-11-10

- Model layer implemented: `pivot_sequence`, `fibonacci`, `horizontal_band`,
  four events, registry, validation, verification, golden fixtures, shell
  readings CSV, and spec-v2 extended fields.
- `M1.2` and registry `2026-11-10.1` are in place.
- 2018 backtest regenerated with 11 trades and 121 readings columns.
- 2019 backtest regenerated with 23 trades and 121 readings columns.
- Annotation contexts re-exported: 595 contexts for 2018 and 1,025 for 2019,
  each with all 121 readings columns, including the new fields.
- `StrategyPosition` now carries entry price/ATR, unrealized ATR, MFE, MAE, and
  giveback; the event-composite strategy supports a giveback exit.
- Full gate passes: `cargo test --workspace`, `cargo clippy --all-targets`,
  `cargo fmt --all -- --check`, registry, validation, verification matrix,
  event emitters, research coverage, links, terms, and legacy scan.
- Pending: zone-based annotation analysis, Quant Validator record, and PR.

## 13. Open questions for the human

1. Approve `M1.2` and registry `2026-11-10.1`?
2. Approve the three indicator contracts and code/source/event allocations?
3. Approve the `StrategyPosition` excursion fields as position state, not P&L?
4. Approve a strategy-managed giveback/trailing exit first, with an execution-level trailing stop deferred?
5. Confirm the annotation snapshot and that 2018–2019 are the only years for this pass.
