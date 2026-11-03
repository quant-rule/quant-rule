# Execution Plan: Path-Based Model Validation Pilot

- Date: 2026-11-03 (UTC)
- Status: **Approved** — the human approved the path-based thesis and the exact protocol/pass rule.
- Issue: [#365](https://github.com/quant-rule/quant-rule/issues/365)
- Prior evidence: [`MODEL-GATE`](../../improvement/2026-11-03/MODEL-GATE.md),
  [`STRUCTURE-PILOT-SPX`](../../improvement/2026-11-03/STRUCTURE-PILOT-SPX.md)
- AI transparency log:
  [`docs/ai-transparency-log/2026-11-03/365-path-validation.md`](../../ai-transparency-log/2026-11-03/365-path-validation.md)
- Claim type: model validation-contract evidence; no strategy or edge claim

## 1. Intent

The v3 gate and the three single-source pilots tested a fixed-horizon sign. The
model's event claims are path claims: a level test is followed by a reaction, a
break by continuation, a divergence by reversal, and a progression by a target
hit before invalidation. The sign tests are therefore a proxy, and their failure
does not test the claims as written.

This pilot changes the model thesis from "events predict the next-bar sign" to
"events predict path asymmetry." The first test is the confirmed pivot Level
test on a fresh asset. The question is whether price reaches a pre-registered
ATR target before an ATR stop more often than the unconditional barrier rate.

The model output and signal path do not change. This is a validation-contract
change, not a new indicator.

## 2. Current state

- The v3 gate graded 48 primary cells on fixed-horizon sign; all development
  cells were C and the combined gate was no edge.
- The RSI, MACD, and structure pilots all tested next-1h sign and stopped.
- The model contract's validation boundary already names path outcomes for
  level, structure, divergence, and progression claims; the gate's sign protocol
  did not operationalize them.
- The registry's `level.test` claim already says "reaction ... more often than a
  placebo level," but no harness has measured a reaction path.
- COPPERCMD M1 2018–2024 is available and was not used by the gate or any pilot.

## 3. Scope

Included:

- COPPERCMD 1h, 2018–2022; development 2018–2020, validation 2021–2022.
- Five one-year model bundles.
- Primary event: confirmed pivot Level test/retest (`event_type=1`,
  `event_subtype=1`, `source_code` 7 or 8, `reference_code >= 1001`).
- Primary outcome: a +1.0 ATR target reached before a -1.0 ATR stop within 20
  bars, in the event direction.
- Primary comparison: the unconditional barrier rate at all valid bars, matched
  by year and direction mix.
- Secondary events: confirmed pivot Level cross (subtype 2) and break (subtype
  3), same path protocol, descriptive only.
- Descriptive breakdown by pivot scale and direction.
- The path-validation protocol, the model-contract validation boundary, and the
  registry claim wording.

Excluded:

- Strategy, costs, execution, and portfolio work.
- New indicators, new event definitions, and threshold or horizon tuning.
- Reusing BTC/ETH/XAU, EURUSD, XAGUSD, or ESANDP500 validation.
- Cross-timeframe conditioning, axis scores, and progression.
- A profitability or edge claim.

## 4. Assumptions and uncertainty

- A 1 ATR target, 1 ATR stop, and 20-bar horizon are an economically meaningful
  operationalization of a level reaction; they are fixed before the run.
- The event sample is large enough to estimate a 2pp lift. If it is not, the
  result is inconclusive, not a claim of no edge.
- The unconditional barrier rate is a fair null for the event claim. It is a
  matched barrier baseline, not a matched price level; the registry claim
  wording changes to say so.
- The event close is the entry reference, which is an event-study convention.
  A later strategy run would use the next open and costs.
- Gaps are part of the 20-bar path. The horizon is a bar count, not a clock
  duration.

## 5. Decisions

| Decision | Choice |
|---|---|
| Asset | COPPERCMD |
| Timeframe | 1h |
| Development | 2018-01-01..2020-12-31 |
| Validation | 2021-01-01..2022-12-31 |
| Primary event | Confirmed pivot Level test/retest |
| Entry reference | Event-bar close |
| Target | 1.0 ATR in the event direction |
| Stop | 1.0 ATR against the event direction |
| Horizon | 20 future 1h bars |
| Tie-break | Stop first |
| Baseline | Unconditional barrier rate, per year and direction |
| Estimator | Cluster-robust hit rate and lift |
| Primary pass | n >= 200, J >= 20, lift >= +0.02, lift LCB > 0.00 |
| Decision on pass | Replicate on a second fresh asset/timeframe; no strategy |
| Secondary events | Level cross and break, descriptive only |
| Strategy | None |

## 6. Path-validation protocol

### 6.1 Event preparation

1. Read the 1h `readings.csv` and `events.csv` from each one-year bundle.
2. Keep Level test events with `event_type_code=1`, `event_subtype_code=1`,
   `source_code` in `{7, 8}`, `reference_code >= 1001`, `direction_code` in
   `{-1, +1}`, and `data_completeness >= 1.0`.
3. Fresh test: for each `reference_code`, keep only the first event of a run of
   consecutive bars. A repeated test on the next bar is not a new test.
4. One vote per bar: group fresh events by `available_bar_index`. If the bar has
   conflicting directions, drop it as ambiguous; otherwise keep the event with
   the lowest `event_index` (tie: lowest `reference_code`).
5. Valid path: the event bar must have a non-null ATR greater than zero and at
   least 20 future 1h bars in the bundle. There is no exact-1h transition filter;
   the path is the next 20 bars and gaps are part of it.
6. Report raw, fresh, prepared, and valid-path counts, plus the reasons for
   dropped events.

### 6.2 Path outcome

Let `entry = close[event_bar]`, `atr = atr[event_bar]`, and `d = direction`.

```text
target = entry + d * 1.0 * atr
stop   = entry - d * 1.0 * atr
```

Walk bars `event_bar + 1` through `event_bar + 20`:

- Long (`d = +1`): if `low <= stop` and `high >= target` on the same bar, count
  a stop first. Otherwise, the first touch of `stop` is a miss and the first
  touch of `target` is a hit.
- Short (`d = -1`): if `high >= stop` and `low <= target` on the same bar, count
  a stop first. Otherwise, the first touch of `stop` is a miss and the first
  touch of `target` is a hit.
- If neither is touched within 20 bars, the event is a miss (timeout).

The event bar's own high and low are not used; the path starts at the next bar.
An ATR of zero is not used as a barrier; it is dropped and counted.

### 6.3 Baseline

For every bar `b` in the same bundle with a valid ATR greater than zero and at
least 20 future bars, compute the same symmetric barrier for a long and a short
entry at `close[b]` with `atr[b]`.

- `base_long_year = long_target_hits / valid_bars`
- `base_short_year = short_target_hits / valid_bars`

The directional base for the event set is the event-weighted average:

```text
base = sum_years (n_long_year * base_long_year
                + n_short_year * base_short_year) / n
```

The base rate is a fixed benchmark estimated from all valid bars. The event
sample is a subset of the base sample, so adding the full base-rate variance to
the event variance would double-count the shared bars. The primary bound uses
the cluster-robust event variance; the per-year base rates are reported as a
sensitivity.

### 6.4 Estimator

- `p = hits / n`.
- Sort valid events by event bar. Start a new cluster when the event bar is more
  than 20 bars after the previous event bar; events whose 20-bar paths can
  overlap stay in the same cluster.
- Let `J` be the number of clusters, with `n_j` events and `h_j` hits in cluster
  `j`.
- Cluster-robust variance:

```text
var = (J / (J - 1)) * sum_j (h_j - p * n_j)^2 / n^2
se  = sqrt(var)
```

- `hit_lcb = p - 1.96 * se`.
- `lift = p - base`.
- `lift_lcb = lift - 1.96 * se`.
- Report the naive `sqrt(p*(1-p)/n)` standard error and the cluster-robust
  standard error side by side.
- If `J <= 1`, the cluster-robust bound is undefined and the partition is
  inconclusive.
- The cluster-robust bound is deterministic; there is no bootstrap and no seed.

### 6.5 Decision rule

| Condition | Decision |
|---|---|
| Development `n < 200` or `J < 20` | `inconclusive` |
| Development fails `lift >= +0.02` or `lift_lcb > 0.00` | `stop` |
| Development passes, validation `n < 200` or `J < 20` | `validation_inconclusive` |
| Development passes, validation fails the same rule | `development_only` |
| Both partitions pass | `proceed_to_replication` |

`proceed_to_replication` is not a strategy go. A single asset and timeframe is a
pilot; the same protocol must replicate on a second fresh asset or timeframe
before a strategy baseline is considered. The validation partition is one-shot
for this hypothesis: no barrier, horizon, event, direction, or filter change
after seeing validation.

## 7. Contract and registry changes

- Add `docs/model/2026-11-03/path-validation.md` with the protocol above.
- Update `docs/model/2026-11-03/model-v3.md` section 14 to name the path outcome
  for Level, Structure, Divergence, and Signal claims and to state that the sign
  outcome is a screening metric, not the claim test.
- Update `docs/registry/indicators.json`:
  - `pivots` claim: a confirmed pivot test is followed by a path reaction more
    often than a matched barrier baseline.
  - `level.test` claim: a level touch is followed by a path reaction more often
    than a matched barrier baseline.
  - `level.break` claim: a close beyond a level is followed by a continuation
    path more often than a matched barrier baseline.
- Regenerate `docs/registry/indicators.md` with the repository tool.
- The model outputs, formulas, and signal path are unchanged. No Rust model
  change is required. The work is a validation-contract and tooling change.

## 8. Acceptance criteria

- The path-validation protocol and the contract/registry wording are recorded.
- Five COPPERCMD 1h model bundles exist: 2018, 2019, 2020, 2021, 2022.
- The harness filters confirmed pivot Level tests, crosses, and breaks.
- Development and validation report raw, fresh, prepared, valid n, J, base,
  hit, lift, naive LCB, and cluster-robust LCB.
- The primary decision follows the pass rule without tuning.
- The secondary events and the descriptive breakdown are recorded but not
  promoted.
- No strategy, costs, execution, or portfolio is run.
- The harness is deterministic and produces byte-identical `result.json` on a
  repeated run.

## 9. Work breakdown

| Step | Work | Role | Path |
|---|---|---|---|
| 1 | Plan, protocol, and AI log | Orchestrator, Quant Researcher | `docs/execution-plans/`, `docs/model/`, `docs/ai-transparency-log/` |
| 2 | Contract and registry wording | Quant Researcher, Architect | `docs/model/`, `docs/registry/` |
| 3 | Configs and five model bundles | Quant Developer | `config/path-pilot/`, `data/path-pilot/` |
| 4 | Path harness and tests | Quant Developer, Test | `tools/improvement/path_pilot.py` |
| 5 | Independent one-year check | Test Engineer | local check, recorded in the result |
| 6 | Development result | Quant Validator sequential pass | `data/path-pilot/PATH/` |
| 7 | Validation result | Quant Validator sequential pass | same |
| 8 | Decision, cycle, PR | Orchestrator, Code Reviewer | `docs/improvement/2026-11-03/` |

## 10. Verification

| Check | Command | Expected |
|---|---|---|
| Format | `cargo fmt --all -- --check` | pass |
| Lints | `cargo clippy --all-targets -- -D warnings` | pass |
| Tests | `cargo test --workspace` | pass |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | pass |
| Registry | `python3 tools/check_registry.py` | pass |
| Tooling | `python3 -m unittest discover -s tools/improvement` | pass |
| Links/terms | `python3 tools/check_links.py`, `python3 tools/check_terms.py` | pass |
| Determinism | run the harness twice, compare `result.json` | byte-identical |
| Independent check | manual one-year path calculation | matches the harness |

The path harness tests cover: target hit, stop hit, timeout, same-bar tie
(stop first), ATR null/zero, fewer than 20 future bars, gap path, cluster
formation at the 20-bar boundary, cluster-robust variance, and every decision
branch.

## 11. Risks and recovery

- **Barrier overfitting.** The 1 ATR / 1 ATR / 20-bar choices are fixed before
  the run. No sensitivity variant is promoted. If the result is borderline, it
  is recorded as borderline, not tuned.
- **Overlapping paths.** The cluster-robust variance groups events whose paths
  can overlap. The naive and cluster-robust errors are both reported.
- **Gaps.** A gap can jump over the target or stop. The stop-first tie-break is
  conservative. The horizon is 20 bars, not 20 clock hours.
- **Small sample.** Fewer than 200 events or 20 clusters is inconclusive.
- **Base-rate choice.** The per-year, direction-weighted base is reported with
  the per-year values; a different base weighting is a sensitivity, not a
  rescue.
- **Single asset.** A pass is a candidate for replication, not an edge claim.
- **No model change.** The model output is unchanged; the risk is in the
  validation protocol, which is why the protocol is pre-registered and tested.

## 12. Stop conditions

- The model bundle misses or corrupts the 1h series: stop and fix the pipeline.
- Development is inconclusive: record inconclusive; do not tune the barrier,
  horizon, or event to increase n.
- Development fails: stop; do not run a different barrier or a different event
  on the same asset.
- Validation contradicts development: stop; do not tune.
- The work needs a new asset, timeframe, barrier, horizon, or event definition
  outside this plan: ask the human.
