# MODEL-2026-11-03-02 — Model Gate (Validation)

- Date: 2026-11-03 (UTC)
- Tier: model
- Changed: model
- Frozen: strategy, portfolio
- Partition: `model_validation` (2021–2022)
- Code commit: `f5610ca`
- Manifest: [`manifests/MODEL-2026-11-03-02.json`](manifests/MODEL-2026-11-03-02.json)
- Claim-validation protocol: [`docs/model/2026-11-03/claim-validation.md`](../../model/2026-11-03/claim-validation.md)
- Local artifacts: `data/improvement/2026-11-03/MODEL-2026-11-03-02/`
- Verdict: `one partition-local B cell; combined gate not green`
- Claim type: model evidence; no edge claim

## Hypothesis

The development gate result replicates on the one-shot `model_validation`
partition.

## Results

Six one-year cells: BTCUSD, ETHUSD, and XAUUSD for 2021 and 2022. Timeframes:
5m, 15m, 1h. One of 48 primary cells reaches B:

| Family | TF | n | Hit | Base | Lift | Corrected LCB | Replication | Grade |
|---|---|---:|---:|---:|---:|---:|---:|---|
| RSI regular bullish divergence | 5m | 3,052 | 0.5324 | 0.5007 | 0.0317 | 0.5027 | 3 | B |

The development partition gave the same cell C (hit 0.5197, base 0.4994, lift
0.0203, corrected LCB 0.4991). The combined gate is therefore **no edge**: the
validation-only B is not replicated on development.

## Review

The validation cell is the kind of single-sample positive that the one-shot
validation exists to expose. It does not replicate the development result, and it
is below the A threshold. Every other primary cell is C.

## Decision

No model baseline is accepted. The combined gate is no edge; the divergence B
cell is recorded as a validation-only observation, not a usable signal.

## Limitations

- One-year folds are pooled per instrument.
- The gate measures sign outcomes, not magnitude or costs.
- Same-session harness and review, not an independent Quant Validator record.
