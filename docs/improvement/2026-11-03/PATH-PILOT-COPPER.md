# PATH-PILOT-COPPER — COPPERCMD 1h

- Date: 2026-11-03 (UTC)
- Issue: [#365](https://github.com/quant-rule/quant-rule/issues/365)
- Plan: [`docs/execution-plans/2026-11-03/365-path-validation.md`](../../execution-plans/2026-11-03/365-path-validation.md)
- Protocol: [`docs/model/2026-11-03/path-validation.md`](../../model/2026-11-03/path-validation.md)
- Tool: `tools/improvement/path_pilot.py`
- Local artifacts: `data/path-pilot/`
- Verdict: **stop — the primary path lift is negative on both partitions**
- Claim type: model validation-contract evidence; no strategy or edge claim

## Question

After price tests a confirmed pivot level on COPPERCMD 1h, does the next 20-bar
path reach a 1 ATR target before a 1 ATR stop more often than the unconditional
barrier rate?

## Data

COPPERCMD 1h. Development 2018–2020, validation 2021–2022. Five one-year folds,
pooled per partition. The 1h series contains only complete M1 hours. The path
starts at the event close and uses the next 20 1h bars; gaps are part of the
path. COPPERCMD was not used by the model gate or the RSI, MACD, or structure
pilots.

## Primary result

Confirmed pivot Level test, 1.0 ATR target before 1.0 ATR stop within 20 bars,
stop-first tie-break:

| Partition | Raw | Fresh | Prepared | Valid n | Clusters | Coverage | Hit | Base | Lift | Naive SE | Cluster SE | Hit LCB | Lift LCB |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Development | 2,927 | 1,982 | 1,549 | 1,539 | 20 | 0.5258 | 0.4847 | 0.4989 | −0.0141 | 0.0127 | 0.0156 | 0.4542 | −0.0447 |
| Validation | 3,922 | 2,578 | 1,979 | 1,964 | 33 | 0.5008 | 0.4659 | 0.4962 | −0.0303 | 0.0113 | 0.0118 | 0.4429 | −0.0533 |

The primary path hit rate is below the unconditional barrier base on both
partitions. The development point lift fails the `+0.02` threshold and the lift
LCB is negative; validation is also negative. The pre-registered pass rule is not
met.

## Secondary events

| Partition | Event | Valid n | Clusters | Hit | Base | Lift | Cluster SE | Lift LCB |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| Development | Level cross | 389 | 138 | 0.5116 | 0.5008 | +0.0107 | 0.0244 | −0.0372 |
| Development | Level break | 846 | 80 | 0.5012 | 0.4979 | +0.0033 | 0.0146 | −0.0253 |
| Validation | Level cross | 527 | 169 | 0.5142 | 0.4953 | +0.0189 | 0.0216 | −0.0234 |
| Validation | Level break | 1,030 | 92 | 0.4913 | 0.4967 | −0.0054 | 0.0151 | −0.0351 |

The cross has a positive point lift on both partitions, but the lift LCB is
negative on both. It is not promoted. The break is negative on validation and its
development lift is not meaningful.

## Per-year barrier base

| Bundle | Valid bars | Long base | Short base |
|---|---:|---:|---:|
| copper-2018 | 2,864 | 0.4784 | 0.5175 |
| copper-2019 | 1,423 | 0.4933 | 0.5004 |
| copper-2020 | 2,973 | 0.5304 | 0.4652 |
| copper-2021 | 4,128 | 0.4932 | 0.4983 |
| copper-2022 | 4,808 | 0.4788 | 0.5133 |

The per-year bases differ materially, which is why the base is year- and
direction-weighted rather than pooled.

## Descriptive breakdown of the primary test

| Partition | Source/direction | n | Hit | Base | Lift |
|---|---|---:|---:|---:|---:|
| Development | Minor pivot / long | 628 | 0.4841 | 0.5032 | −0.0191 |
| Development | Minor pivot / short | 540 | 0.4944 | 0.4938 | +0.0006 |
| Development | Major pivot / long | 196 | 0.4439 | 0.5038 | −0.0599 |
| Development | Major pivot / short | 175 | 0.5029 | 0.4931 | +0.0097 |
| Validation | Minor pivot / long | 715 | 0.4559 | 0.4855 | −0.0295 |
| Validation | Minor pivot / short | 766 | 0.4634 | 0.5065 | −0.0430 |
| Validation | Major pivot / long | 246 | 0.4634 | 0.4854 | −0.0219 |
| Validation | Major pivot / short | 237 | 0.5063 | 0.5064 | −0.0001 |

The breakdown is descriptive only. The only positive cells are small and are
not a separate hypothesis.

## Decision

Stop. The primary path claim is not supported on COPPERCMD 1h. No replication
pilot, no strategy, no RSI/MACD conjunction, and no barrier or horizon tuning.
The path thesis is recorded as failed on this asset and timeframe; the fixed-
horizon sign thesis remains failed as well.

## Harness correction

The first run pooled event bar indices across one-year bundles before forming
clusters. Bar indices restart at zero in each bundle, so the pooled cluster
boundaries were wrong and produced `J=1` development and `J=4` validation. The
harness now forms clusters separately per bundle and concatenates the cluster
counts. The corrected result is the one above. The first run is not evidence.

## Verification

- An independent 2018 calculation matches the harness exactly: raw 1,163, fresh
  790, prepared 621, valid n 617, hits 318, hit 0.5154, base 0.4970, lift
  +0.0184, clusters 5, cluster SE 0.0238, lift LCB −0.0282.
- `python3 -m unittest discover -s tools/improvement`: 64 tooling tests pass.
- `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and
  `cargo doc` pass on the change.
- The harness is deterministic: the same bundles produce the same `result.json`.
- The path protocol is recorded in `docs/model/2026-11-03/path-validation.md`;
  the model contract section 14 references it and the registry level claims use
  the matched-barrier wording.

## Limitations

- One asset and one timeframe; a pilot, not a model baseline.
- The entry reference is the event close; no costs, slippage, or execution.
- Gaps are part of the 20-bar path; the horizon is a bar count, not a clock
  duration.
- The base is a matched barrier baseline, not a matched price level.
- The development cluster count is 20, the pre-registered minimum; the
  cluster-robust bound is retained, and the point estimate is negative.
- Same-session analysis, not an independent Quant Validator record.
