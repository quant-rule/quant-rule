# MR7 Programme — Opening-Range Sweep/Reclaim

- Date: 2026-11-08 (UTC)
- Status: **Verified** (2026-11-08) — 15 cells run sequentially; `no_edge`; validation not opened
- Charter: [`MR7-CHARTER.md`](../../governance/2026-11-08/MR7-CHARTER.md)
- Programme: `PROGRAM-2026-11-08-03`
- Model change: `M1.1` (adds the opening-range sweep/control events)
- Data change: `DATA-2026-11-08-MR7-01` (COPPERCMDUSD and TSLAUSUSD, price-only)
- Claim type: model change, data lifecycle, and pre-registered model research; no strategy or edge claim

## 1. Intent

Test a genuinely new liquidity mechanism after MR5 and MR6: a sweep of the
completed opening range that closes back inside predicts a reversal, while a
sweep that closes outside is the continuation control. The programme changes the
model measurement (two new session events), adds two fresh price-only
instruments, runs a 15-cell grid, and stops at the development result. It does
not open `model_validation` or the strategy tier.

## 2. Current evidence

- MR5 and MR6 directional event families are exhausted: the best primary lifts
  are +0.79 pp (MR5.1 failed breakout, LCB −0.42 pp) and +0.08 pp (MR6.2
  reverse control), with negative LCBs. The broad family grids produced no lift.
- The only positive direction is level/liquidity interaction: failed breakout
  and confluence were the closest; the rare MR5.7 subset was +2.0–2.4 pp but
  only 817–1,348 predictions and LCB −0.011 to −0.019.
- Every path menu is negative after 6 bps: 15m about −0.5R, 5m about −1.0R. A
  hit-rate pass without positive net R is not an edge.
- The five existing instruments and 2018–2020 data have been mined; the
  validation partition is reused; BTC/ETH volume units change within a year.

## 3. Scope

Included:

- Model Change `M1.1`: add `signal.opening_range_sweep` (144) and
  `signal.opening_range_sweep_control` (145), using the existing opening-range
  readings; add `session.sweep_atr`; update the registry, validation claims,
  verification evidence, contract, tests, and generated tables.
- Data Lifecycle: admit `COPPERCMDUSD` and `TSLAUSUSD` as price-only instruments,
  update `data/partitions.json` and the aliases, and record the quality and
  admissibility decision.
- Model Research `MR7`: 5 hypotheses × 3 configurations = 15 cells, five primary
  base cells, including a no-reclaim control; 15m and 1h; current five
  instruments plus the two fresh price-only instruments; `model_dev` 2018–2020;
  6 bps; +2/−1 ATR; 20 bars; Bonferroni over five.
- Count gate, sequential development run, aggregation, and records.

Excluded:

- `model_validation`; the strategy, execution, and portfolio tiers.
- Any post-hoc event, threshold, window, timeframe, instrument, or cost change.
- The rvol participation cell on COPPER/TSLA (volume not admissible).
- A claim of an edge from a development result.

## 4. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| New event | Opening-range sweep/reclaim and no-reclaim control | Two signal events, one source, one indicator; no new indicator |
| Event timing | Same bar; sweep beyond ORH/ORL, close back inside or outside | Distinct from MR5.1's break-then-cross at a confirmed pivot |
| Tie-break | If both sides sweep, keep the larger excursion; equal → no event | One event per bar; deterministic |
| Model version | `M1.1` | `M1.0` remains historical; registry version `2026-11-08.1` |
| Fresh data | COPPERCMDUSD and TSLAUSUSD, price-only | More cross-section; volume cells exclude them |
| Primary cells | 5 base cells | Bonferroni alpha 0.01; control is not a candidate |
| Cost gate | Mean net R > 0 on the 2/1/20 menu | A hit-rate pass on a negative-expectancy base is not a candidate |
| Execution | Sequential | The platform has restarted during long runs |

## 5. Acceptance criteria

- `cargo test`, clippy, fmt pass; the new event row contracts, causality,
  determinism, golden, and edge tests pass.
- `check_registry.py`, `check_validation.py`, `check_verification.py`, and
  `check_verification_matrix.py` pass with 145 events and 191 claims.
- `check_partitions.py` passes with 7 instruments and the new aliases.
- Coverage passes for `MR7` with the inherited M2 matrix.
- The count gate passes all five primary base cells before any outcome is read.
- All 15 cells produce a result; the report aggregates all 15 and applies the
  path gate.
- The development record names the decision, primary results, control sign,
  path evidence, limitations, and reproduction digest.
- Registries, model-family table, README, changelog, and AI log are updated.

## 6. Work breakdown

| Step | Work | Owner | Paths |
|---:|---|---|---|
| 1 | Model contract and registry entries | Quant Developer | `docs/model/2026-11-08/model-m1.1.md`, `docs/registry/indicators.json` |
| 2 | Rust events and `session.sweep_atr` | Quant Developer | `crates/quantrule-model/src/events/session.rs`, `sessions.rs`, `codes.rs`, shell spec parser |
| 3 | Tests and evidence | Test Engineer | `session/row_tests.rs`, `session.rs`, `docs/registry/validation.json`, `verification.json` |
| 4 | Data lifecycle | Data Steward | `data/partitions.json`, `data/sources/README.md`, data change record |
| 5 | Grid builder and coverage | Quant Developer | `tools/research/build_mr5_program.py`, `coverage_mr5.py`, `coverage.py`, `test_mr7.py` |
| 6 | Build the 15-cell grid | Quant Developer | `config/research/PROGRAM-2026-11-08-03/` |
| 7 | Count gate | Quant Developer | `data/research/PROGRAM-2026-11-08-03/preflight/` |
| 8 | Sequential development run | Quant Developer | `data/research/PROGRAM-2026-11-08-03/dev/` |
| 9 | Aggregation and decision | Quant Validator; Result Analyst | `program-results/`, `MR7-DEV-SWARM.md` |
| 10 | Records and delivery | Orchestrator | registries, changelog, AI log, PR |

## 7. Verification

| Check | Command | Expected |
|---|---|---|
| Rust tests | `cargo test` | pass |
| Rust lints | `cargo clippy --all-targets -- -D warnings` | clean |
| Format | `cargo fmt --check` | clean |
| Registry | `python3 tools/check_registry.py` | 46 indicators, 145 events, 102 parameters |
| Validation | `python3 tools/check_validation.py` | 191 claims |
| Verification | `python3 tools/check_verification.py`; `python3 tools/check_verification_matrix.py` | 145 events covered |
| Partitions | `python3 tools/check_partitions.py` | 7 instruments, 7 aliases |
| Manifest | `python3 tools/research/program.py --program ... --check` | 15 cells, 5 primary |
| Coverage | `python3 tools/research/coverage.py --program ... --family MR7 --inherit ...` | 99 specs, inherited M2 |
| Count gate | `model_cell.py --count-only` on five primary cells | ≥200 n and ≥20 clusters each |
| Development | `batch.py --max-parallel 1` | 15/15 results |
| Reproducibility | rerun one cell | same digest and metrics |
| Docs | link, term, legacy, structure, workflow, changelog, partition, registry, validation, verification checks | pass |

## 8. Risks and recovery

- The new event may be too rare at 1h; the count gate stops the programme as
  `underpowered` rather than relaxing the rule.
- The control may also reverse; the reclaim premise is then falsified.
- COPPER has zero volume on most rows and TSLA has a 2020 volume-unit shift;
  both are price-only, and the rvol cell excludes them.
- The opening range is London-session based; a different session is a later
  hypothesis, not a post-hoc change.
- The model change touches the registry and generated matrices; the checkers
  catch drift.
- Same-agent Quant Developer and Quant Validator passes are sequential, not
  independent.

## 9. Stop conditions

Stop and report if the count gate fails, the control also reverses, a shared
prerequisite is missing, a check fails, or a material deviation from the
approved protocol would be required. Do not change an event, window, timeframe,
instrument, or threshold to rescue a result.

## 10. Verification result

| Check | Observed |
|---|---|
| Model | `M1.1`; 145 events, 191 claims, 102 parameters |
| Data | COPPERCMDUSD and TSLAUSUSD price-only; 7 instruments |
| Manifest | 15 cells, 5 primary |
| Coverage | 15 cells, 99 specs, inherited M2 |
| Count gate | 6,621/3,997; 2,832/1,865; 5,802/3,603; 1,559/1,299; 8,314/5,382 |
| Development | 15/15 cells; `no_edge`; 0/5 primary passed |
| Reproducibility | `MR7.1-C02` digest `2f36059a0f97...`; same metrics |
| Python tests | 113 improvement, 54 research |
| Rust | `cargo test`, clippy, fmt pass |
| Docs and registry | all checkers pass |

The result record is [`MR7-DEV-SWARM.md`](../../improvement/2026-11-08/MR7-DEV-SWARM.md).
