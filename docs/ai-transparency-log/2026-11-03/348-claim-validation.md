# AI Transparency Log — Claim Validation and Model Gate

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#348](https://github.com/quant-rule/quant-rule/issues/348)
- Plan:
  [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- Claim type: model evidence; no edge claim

## Intent and boundary

Implement the claim-specific validation harness and run the model gate on
`model_dev` and `model_validation`. The harness grades states and events by
forward outcomes, base rates, corrected lower bounds, coverage, and replication.
The model and strategy are unchanged during the run.

## Work

- Added `tools/improvement/claim_validation.py`: readings/events CSV input,
  forward horizons 1/3/5/10/20, base rates, normal lower bounds, Bonferroni
  correction, instrument replication, A/B/C/D grades, and JSON/Markdown/CSV
  outputs.
- Added `run.py --command claim-validation` so the gate is recorded by the same
  run store, manifest, results envelope, and artifact digests as any other run.
- Pre-registered the primary cells, horizons, grades, and gate rule in
  `docs/model/2026-11-03/claim-validation.md`.
- Added progression stage/direction columns to `readings.csv` for the harness.
- Created 15 one-year model cells: BTCUSD, ETHUSD, XAUUSD for 2018–2022, with
  MACD active and the 10% policy envelope.
- Created `MODEL-2026-11-03-01` (development) and `MODEL-2026-11-03-02`
  (validation) manifests.

## Verification and evidence

- The 15 model bundles are under `data/model-gate/`.
- `MODEL-2026-11-03-01` (development): 48 primary cells, all grade C.
- `MODEL-2026-11-03-02` (validation): 47 C and one B,
  `divergence.regular_bullish` 5m (hit 0.5324, base 0.5007, corrected LCB
  0.5027, replication 3).
- The same validation cell was C on development (hit 0.5197, corrected LCB
  0.4991). The combined gate is **no edge**.
- The combined gate comparison is in
  `data/improvement/2026-11-03/MODEL-GATE/gate.json`; the committed summary is
  `docs/improvement/2026-11-03/MODEL-GATE.md`.
- The run store records the commands, exit codes, and artifact digests in each
  run's `results.json`.

## Learning

- The validation partition produced exactly one B cell, and it did not replicate
  the development result. A one-shot validation is working as intended when it
  rejects that cell instead of promoting it.
- The model bundle is large: one year of 5m/15m/1h readings and events is about
  200 MB. The gate uses the CSV artifacts, not a re-run of the model for every
  analysis.
- The M1 window cap means a three-year partition must be split into one-year
  folds. Pooling folds per instrument is the correct unit for replication.
- A huge sample can make a tiny hit-rate difference statistically visible but
  economically meaningless. The grade therefore requires a minimum lift over the
  base rate, not just a lower bound above 0.50.

## Rejected suggestions

- Grading on the tuning sample only: the gate requires the one-shot validation
  partition.
- Using the old axis values as the gate: v3 grades states and events, not axes.
- Treating a single positive cell as an edge: replication and the corrected bound
  are required.
- Adding costs to the model gate: costs belong to the strategy run; the model
  gate measures predictive validity.

## Limitations

- The harness sign outcome is not a magnitude or cost model.
- Yearly folds are pooled; regimes inside a partition are not separated.
- Same-session harness and review, not an independent Quant Validator record.
