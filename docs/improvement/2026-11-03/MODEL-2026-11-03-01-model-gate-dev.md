# MODEL-2026-11-03-01 — Model Gate (Development)

- Date: 2026-11-03 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_dev` (2018–2020)
- Code commit: `f5610ca`
- Manifest: [`manifests/MODEL-2026-11-03-01.json`](manifests/MODEL-2026-11-03-01.json)
- Claim-validation protocol: [`docs/model/2026-11-03/claim-validation.md`](../../model/2026-11-03/claim-validation.md)
- Local artifacts: `data/improvement/2026-11-03/MODEL-2026-11-03-01/`
- Verdict: `not green — no signal-role family passed`
- Claim type: model evidence; no edge claim

## Hypothesis

At least one signal-role model state or event has a stable forward edge on the
`model_dev` partition after a Bonferroni correction, coverage, and instrument
replication.

## Results

Nine one-year cells: BTCUSD, ETHUSD, and XAUUSD for 2018, 2019, and 2020.
Timeframes: 5m, 15m, 1h. All 48 primary cells are grade C; none is A or B.

| Family | TF | n | Hit | Base | Lift | Corrected LCB | Replication | Grade |
|---|---|---:|---:|---:|---:|---:|---:|---|
| EMA stack | 1h | 33,499 | 0.5025 | 0.5144 | −0.0119 | 0.4935 | 2 | C |
| Progression leg direction | 5m | 709,426 | 0.4940 | 0.5008 | −0.0069 | 0.4920 | 0 | C |
| Progression leg direction | 15m | 220,621 | 0.4946 | 0.5094 | −0.0148 | 0.4911 | 1 | C |
| EMA stack | 15m | 158,911 | 0.4915 | 0.5053 | −0.0138 | 0.4874 | 0 | C |
| Level test | 1h | 41,578 | 0.4939 | 0.5186 | −0.0246 | 0.4858 | 1 | C |

## Review

The best development cells are at or below the base rate. The MACD state,
histogram, zero cross, RSI zone, level events, and divergence families do not
clear the gate. The model is descriptive on this partition.

## Decision

No model baseline is accepted. The development partition does not pass; the
validation partition is run as a separate one-shot check, but a validation-only
cell cannot rescue a development failure.

## Limitations

- One-year folds are pooled per instrument.
- The gate measures sign outcomes, not magnitude or costs.
- Same-session harness and review, not an independent Quant Validator record.
