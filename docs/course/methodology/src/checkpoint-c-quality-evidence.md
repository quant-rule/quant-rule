# Checkpoint C - Quality and Residual Risk

## Spoken script

The risk that worries me most is look-ahead. A rule can look profitable because it read a level before the level was confirmed. I track five risks: look-ahead, non-determinism, silent artifact loss, a contract change that invalidates recorded evidence, and a wrong input accepted as valid. The one that matters most is look-ahead, because it can make a rule look profitable when it is not. The control is availability bars and a causality test per event. The other four risks have controls too: fixed accumulation order and sorted merges for non-determinism; publish only after every hash verifies; versioned contracts and immutable runs; strict config validation and cross-config checks. The non-test control is contract versioning plus the review rule.

The behaviour I test is simple: an event appears only at or after the bar on which it became knowable. I expect that appending later bars leaves every earlier published value unchanged. In the current release, the causality and prefix checks pass. The gate ran 2,118 tests with 0 failures and 14 ignored, and the 24 repository checkers pass. The exact tests are in the appendix. These are not coverage numbers. They are prefix checks that would fail if a later bar changed an earlier value.

The limit is important. These checks prove the tested readings are causal. They do not prove that a strategy built on top of them is causal. A strategy is a separate rule and can still read a level before it is confirmed. The risk table in the appendix names the residual for each risk.

Review kept the model output numeric and rejected labels at the service boundary. The browser maps codes to words from one generated table, so two clients cannot drift. That was the change the review deliberately kept. The gate is the review control: a red gate blocks the change, and a review comment blocks it too. The observed result is from the current release, not a one-off local run.

AI drafted architecture documents and work items. I checked each claim against code or the gate. I do not claim cross-machine determinism as observed.

The residual risk is that determinism is bounded to one machine until a second machine runs the verification procedure. The shard-invariance tests show the same digest with one pass and four shards, reversed order, and an unsorted-merge counterexample. That proves digest invariance, not throughput. The next step is to measure cross-machine determinism and throughput at scale. The client has shipped. The current release record names the gate result, so the claim is traceable to one command. The gate is the same control I use for every change: format, clippy with warnings denied, tests, and the documentation checkers. It is the release gate, not a separate course check. The review and the gate are both required before the evidence becomes a release record.

```{=latex}
\newpage
```

## Evidence appendix

### Five risks

| Risk | Control | Residual |
|---|---|---|
| Look-ahead leaks into a rule | availability bars; a causality test per event | a new reading can repeat a past mistake |
| Non-determinism across runs | fixed accumulation order; sorted merges; one digest | equality across two CPUs is unproven |
| A run loses an artifact silently | publish only after every hash verifies | a failure during verification is untested in production |
| A change breaks recorded evidence | versioned contracts; immutable runs | a digest definition change invalidates old bundles |
| A wrong input is accepted as valid | strict config validation; cross-config checks | a valid config can still be a bad hypothesis |

The non-test control is contract versioning plus the review rule.

### One behaviour

The risk is look-ahead. An event must appear only at or after the bar on which it became knowable. The expected result is that appending later bars leaves every earlier published value unchanged.

Observed in the 2026-11-16 gate:

- `crates/quantrule-model/tests/hardening.rs`: `every_prefix_keeps_the_output_lengths_and_ranges`
- `crates/quantrule-model/tests/progression_golden.rs`: `every_prefix_matches_the_full_series`
- `cargo test -p quantrule-model`
- `tools/gate.sh`: 2,118 passed, 0 failed, 14 ignored, 24 checkers

Limit: the tests prove the tested readings are causal, not a strategy written on top of them.

### Review, AI use, and residual risk

Review kept numeric model output; labels are a client mapping in `labels.generated.ts`, recorded in `docs/architecture/client.md` section 5.

AI use: AI drafted architecture documents and work items. Each claim was checked against code or the gate. Cross-machine determinism is not claimed as observed.

Residual risk: the determinism claim is bounded to one machine. The shard-invariance suite in `crates/quantrule-model-service/src/model/compute.rs` covers one pass and four shards, a reversed ladder, and an unsorted-merge counterexample. It proves digest invariance, not throughput. The next step is to measure cross-machine determinism and throughput at scale; the client has shipped.

Links: [current release note (v1.2.0)](../../../releases/2026-11-16/v1.2.0.md),
[scale-invariance log](../../../ai-transparency-log/2026-11-14/664-scale-invariance.md),
[client decision](../../../architecture/client.md).
