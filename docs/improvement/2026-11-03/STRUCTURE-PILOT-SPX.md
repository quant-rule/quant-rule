# STRUCTURE-PILOT-SPX — ESANDP500 1h

- Date: 2026-11-03 (UTC)
- Issue: [#363](https://github.com/quant-rule/quant-rule/issues/363)
- Plan: [`docs/execution-plans/2026-11-03/363-structure-pilot.md`](../../execution-plans/2026-11-03/363-structure-pilot.md)
- Tool: `tools/improvement/structure_pilot.py`
- Local artifacts: `data/structure-pilot/`
- Verdict: **stop — primary pivot-level test failed development**
- Claim type: model prediction evidence; no strategy or edge claim

## Question

After price tests a confirmed pivot level on ESANDP500 1h, is the next 1h return
more often in the direction of the event's close side than the directional base
rate?

## Data

ESANDP500 1h. Development 2018–2020, validation 2021–2022. Five one-year folds,
pooled per partition. The 1h series contains only hours with all 60 M1 candles;
a next bar counts only when it is exactly 3,600 seconds later, so a session gap
is never read as a one-hour return. ESANDP500 was not used by the model gate, the
RSI pilot, or the MACD pilot.

## Primary result

Confirmed pivot Level test/retest, next-1h return sign, directional base rate:

| Partition | Raw | Fresh | Bars | Valid n | Coverage | Hit | Base | Lift | Hit LCB | Lift LCB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Development | 3,445 | 2,338 | 1,822 | 1,423 | 0.4131 | 0.4814 | 0.4998 | −0.0184 | 0.4554 | −0.0457 |
| Validation | 4,036 | 2,627 | 1,983 | 1,741 | 0.4314 | 0.4963 | 0.4993 | −0.0030 | 0.4728 | −0.0278 |

The primary event is below the directional base rate on both partitions. The
development point lift is negative and its lift lower bound is negative; the
validation lift is essentially zero. The pre-registered pass rule is not met.

## Secondary events

| Partition | Event | n | Hit | Base | Lift | Hit LCB | Lift LCB |
|---|---|---:|---:|---:|---:|---:|---:|
| Development | Pivot Level cross | 367 | 0.5095 | 0.4995 | +0.0100 | 0.4584 | −0.0419 |
| Development | Pivot Level break | 797 | 0.4918 | 0.4983 | −0.0064 | 0.4571 | −0.0422 |
| Validation | Pivot Level cross | 442 | 0.5113 | 0.5010 | +0.0103 | 0.4647 | −0.0369 |
| Validation | Pivot Level break | 912 | 0.4945 | 0.4996 | −0.0050 | 0.4621 | −0.0384 |

The cross has a small positive point lift on both partitions, but its lift lower
bound is negative on both. It is not promoted. The break is negative.

## Descriptive breakdown of the primary test

| Partition | Source/direction | n | Hit | Base | Lift |
|---|---|---:|---:|---:|---:|
| Development | Minor pivot / long | 571 | 0.5026 | 0.5240 | −0.0214 |
| Development | Minor pivot / short | 520 | 0.4481 | 0.4734 | −0.0253 |
| Development | Major pivot / long | 176 | 0.5000 | 0.5236 | −0.0236 |
| Development | Major pivot / short | 156 | 0.4936 | 0.4720 | +0.0216 |
| Validation | Minor pivot / long | 670 | 0.5239 | 0.5183 | +0.0056 |
| Validation | Minor pivot / short | 705 | 0.4837 | 0.4811 | +0.0026 |
| Validation | Major pivot / long | 181 | 0.5138 | 0.5183 | −0.0045 |
| Validation | Major pivot / short | 185 | 0.4270 | 0.4812 | −0.0542 |

The breakdown is descriptive only. The major-pivot short validation cell is
negative with n=185; it is not a separate hypothesis and does not change the
primary decision.

## Decision

Stop. The primary confirmed-pivot Level test does not predict the next-1h return
sign on ESANDP500. No progression pilot is run, no strategy is run, and RSI or
MACD is not added to rescue the result. The event definition, timeframe, horizon,
and filters are not tuned after the result.

## Verification

- An independent 2018 calculation matches the harness exactly: raw 825, fresh
  560, bars 438, valid n 329, hits 153, hit 0.4650, base 0.4960, lift −0.0310.
- `python3 -m unittest discover -s tools/improvement`: 39 tooling tests pass.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and
  `cargo doc` pass on the change.
- The 1h series is a selection of complete hours; the valid-transition rule
  excludes gaps from the target.

## Limitations

- One asset, one timeframe, one period; a pilot, not a model baseline.
- Sign outcomes only; no costs or magnitude.
- Level tests cluster. The fresh-test and one-vote-per-bar rules reduce the
  dependence, but they do not make the events independent.
- Same-session analysis, not an independent Quant Validator record.
