# Checkpoint C - Quality and Residual Risk

> **Tag map.** Current release [`v1.2.0`](../../../releases/2026-11-16/v1.2.0.md); server platform `v0.9.0`; desktop package `v1.0.0`; 1.1.0 alignment `v1.1.0`. Authoritative map: [`docs/releases/README.md`](../../../releases/README.md).

## Risk map

| Risk | Control | Effort | Residual |
|---|---|---|---|
| Look-ahead leaks into a rule | availability bars; a causality test per event | high | a new reading can repeat a past mistake |
| Non-determinism across runs | fixed accumulation order; sorted merges; one digest | high | equality across two CPUs is unproven |
| A run loses an artifact silently | publish only after every hash verifies | medium | a failure during verification is untested in production |
| A change breaks recorded evidence | versioned contracts; immutable runs | medium | a digest definition change invalidates old bundles |
| A wrong input is accepted as valid | strict config validation; cross-config checks | medium | a valid config can still be a bad hypothesis |

One control is not a test: contract versioning plus the review rule.

## One behaviour, with evidence

- **Risk:** look-ahead.
- **Behaviour:** an event appears only at or after the bar on which it became
  knowable.
- **Expected:** appending later bars does not change any earlier published value.
- **Observed:** `every_prefix_keeps_the_output_lengths_and_ranges`
  (`crates/quantrule-model/tests/hardening.rs`) and
  `every_prefix_matches_the_full_series`
  (`crates/quantrule-model/tests/progression_golden.rs`) pass in the
  2026-11-16 gate. The v1.2.0 gate (2026-11-16) passed 2,118 tests; the
  v1.0.0 release gate (2026-11-15) passed 2,134.

Command: `cargo test -p quantrule-model` (causality and prefix tests).

## Limit

Passing causality tests prove the tested readings are causal. They do not prove
that a strategy written on top of them is: a strategy is a separate rule and can
still read a level before it is confirmed.

## Three decisions

- **Review.** Kept numeric model output and rejected labels at the service
  boundary; labels are a client mapping in `labels.generated.ts`, so two clients
  cannot drift (`docs/architecture/client.md` §5). The review kept the
  service-side numeric contract and rejected a server-rendered label.
- **AI use.** AI drafted the architecture documents and the work items. Every
  claim was checked against code or the gate; cross-machine determinism is
  explicitly not claimed as observed.
- **Residual risk.** The determinism claim is bounded to one machine until a
  second one runs the verification procedure. The observed scale result is the
  shard-invariance suite: `one_pass_and_four_shards_share_one_digest`,
  `a_reversed_ladder_produces_the_same_digest`,
  `one_shard_merged_equals_the_full_pass`, and
  `an_unsorted_merge_would_change_the_digest` pass (2026-11-14,
  `docs/ai-transparency-log/2026-11-14/664-scale-invariance.md`). That proves
  digest invariance, not throughput. Next: measure cross-machine determinism
  and throughput at scale; the client has shipped.

**Risk -> Control -> Evidence -> Review -> Residual:** the evidence supports the
causality claim for the tested readings, not for a strategy built on them.
