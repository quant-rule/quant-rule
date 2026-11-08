# AI Transparency Log — MR7 Programme

- Date: 2026-11-08 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: task slug `mr7-programme`
- Plan: [`mr7-programme.md`](../../execution-plans/2026-11-08/mr7-programme.md)
- Charter: [`MR7-CHARTER.md`](../../governance/2026-11-08/MR7-CHARTER.md)
- Development record: [`MR7-DEV-SWARM.md`](../../improvement/2026-11-08/MR7-DEV-SWARM.md)
- Data change: [`DATA-2026-11-08-MR7-01`](../../config/2026-11-08/data-change-MR7-01.md)
- Model contract: [`model-m1.1.md`](../../model/2026-11-08/model-m1.1.md)
- Claim type: model change + data lifecycle + pre-registered model research; no strategy or edge claim

## Intent and boundary

The human approved the MR7 scope after MR5/MR6 stopped at `no_edge`. MR7 tests a
new liquidity mechanism — an opening-range sweep that closes back inside — with
a no-reclaim control and a cost-aware path gate. The work includes a model
change (`M1.1`), a data lifecycle change (COPPERCMDUSD and TSLAUSUSD, price-only),
and a 15-cell model research grid.

It does not open `model_validation`, start the strategy tier, change an existing
model event, or alter a policy. No post-hoc event, threshold, timeframe, or
instrument change is made after seeing a result.

## Model change

- Added `signal.opening_range_sweep` (144) and
  `signal.opening_range_sweep_control` (145), using the existing opening-range
  readings.
- Added `SessionConfig.sweep_atr` and the spec session field `sweep_atr`.
- The event fires on the first bar that trades beyond the completed opening
  range; the reclaim event requires a close back inside, the control requires a
  close outside. The larger side wins a two-sided sweep; an exact tie emits no
  event.
- Updated the registry (145 events, 102 parameters, version `2026-11-08.1`), the
  validation claims (191), the verification manifest and matrices, the generated
  registry table, and the model contract `M1.1`.
- The golden session fixture was regenerated deliberately because the new events
  change the event batch; the fixture is a regression lock, not an oracle.

## Data lifecycle

- Added `COPPERCMDUSD` and `TSLAUSUSD` to all partitions and the alias map.
- COPPER volume is zero on 904,640 of 979,317 rows; TSLA volume is unit-unstable
  in 2020 (11,975× monthly median ratio). Both are admitted **price-only**.
- `MR7.4` (relative-volume) uses only XAU, BTC, S&P 500, EURUSD, and ETH.
- The quality report and the admissibility review record the evidence.

## Research

- Built the 15-cell grid at `PROGRAM-2026-11-08-03`: five primary base cells,
  two timeframes, seven instruments, five relative-volume instruments.
- Ran the count gate on the five primary cells; all passed.
- Ran the 15 cells sequentially. The 1h cell passed the count gate but is below
  the sample needed for a 2 pp MDE.
- Added a `path_mean_net_r` gate to the programme report; a hit-rate pass on a
  negative-expectancy base is not a candidate.

## Verification and evidence

| Check | Observed result |
|---|---|
| Registry | 46 indicators, 145 events, 102 parameters |
| Validation | 191 claims |
| Verification | 46 indicators, 145 events; matrix ok |
| Emitters | 145 codes referenced or family-computed |
| Partitions | 7 instruments, 6 partitions, 6 aliases |
| Manifest | 15 cells, 5 primary |
| Coverage | 15 cells, 99 specs, inherited M2 |
| Count gate | 6,621/3,997; 2,832/1,865; 5,802/3,603; 1,559/1,299; 8,314/5,382 |
| Development | 15/15 cells; `no_edge`; 0/5 primary passed |
| Reproducibility | `MR7.1-C02` digest `2f36059a0f97...`; same n, clusters, lift, LCB, controls, replication |
| Improvement tests | 113 passed |
| Research tests | 54 passed |
| Rust tests | `cargo test` passed |
| Rust lints | `cargo clippy --all-targets -- -D warnings` clean |
| Rust format | `cargo fmt --check` clean |
| Docs | link, term, legacy, structure, workflow, changelog, partition, registry, validation, verification checks pass |

## Execution incidents

- The first variant run exposed a parser bug: `sweep_atr` was looked up as
  `session.sweep_atr` inside the session object, so every variant used `0.0` and
  the C01/C02/C03 results were identical. The parser was fixed to read
  `sweep_atr`, the ten variant cells were rerun, and the report recomputed.
  This is recorded because it changed the run, not hidden.
- The environment restarted during the work. Completed cells and configs were
  kept; the remaining cells resumed.
- A `/tmp` tmpfs quota blocked one rerun; the rerun was moved under the local
  data workspace and completed.
- No completed cell was rerun to change a result. The variant rerun corrected an
  implementation bug, not an outcome.

## Learning

- The 15m two-bar sweep is the strongest MR7 primary at +0.97 pp lift, LCB
  −0.51pp, random p 0.0448, shuffle p 0.8060, and path −0.4242R. It fails the
  lift, LCB, shuffle, and path gates.
- The deeper-sweep variant reaches +1.35 pp, but LCB −0.77pp, n 3,302, shuffle
  p 0.7413, path −0.4206R. It is exploratory, not a candidate.
- The no-reclaim control is also positive (+0.89 pp, path −0.3940R). The sweep is
  only 0.08 pp better than the control, so the reclaim does not add a
  distinguishable edge; the opening-range break itself carries the small effect.
- The 1h cell is negative (−0.49 pp) and underpowered for the 2 pp economic
  threshold.
- The four-bar and relative-volume arms do not help. The rvol arm has replication
  1 and path −0.5917R.
- No primary cell has a positive path. The cost-aware gate is binding alongside
  the negative LCB.

## Rejected or corrected

- Rejected promoting `MR7.1-C03` or any exploratory variant. The primary base
  cells were frozen before the run.
- Rejected opening `model_validation`. No primary cell passed the hit-rate and
  path gates.
- Rejected a post-hoc event, threshold, timeframe, instrument, or cost change.
- Corrected the `sweep_atr` parser bug and reran the ten affected variant cells.
- Did not claim an independent Quant Validator review. The Quant Researcher,
  Quant Developer, Quant Validator, Data Steward, Statistical Methodologist,
  Test, and Code Reviewer passes are sequential role-specific passes by the same
  agent; no separate agent independently reproduced the run.

## Closeout

- Human instruction on 2026-11-08 (UTC): "proceed" with the recommended closure.
- Recorded decision: MR7 is accepted as `no_edge`; `M1.1` remains current and
  measurement-only; model research on this premise is stopped; validation and
  the strategy tier remain closed.
