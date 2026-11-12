# Checkpoint C - Quality and Residual Risk

## Risk map

| Risk | Control | Effort | Residual |
|---|---|---|---|
| Look-ahead leaks into a rule | availability bars + causality tests per event | high | a new reading may repeat a past mistake |
| Non-determinism across runs | fixed accumulation order, sorted merges, canonical digest | high | cross-CPU equality is unproven |
| A run silently loses an artifact | publish only after every hash verifies | medium | a store failure mid-verify is untested in production |
| A change breaks recorded evidence | versioned contracts, immutable runs | medium | a digest definition change invalidates old bundles |
| Wrong input accepted as valid | strict config validation, cross-config checks | medium | a valid config can still be a bad hypothesis |

At least one control is non-test: contract versioning and the review rule.

## One behaviour, with evidence

**Risk:** look-ahead.
**Behaviour tested:** an event appears only at or after the bar on which it
became knowable.
**Expected:** appending later bars does not change any earlier published value.
**Observed:** the prefix and causality tests pass on the current model suite.

Command: `cargo test -p quantrule-model` (causality and prefix tests).

## Limit

Passing causality tests prove the tested readings are causal. They do not prove
that a strategy written on top of them is - the strategy is a separate rule and
can still read a level before it is confirmed.

## Three decisions

- **Review.** The review kept the model's numeric output and rejected adding
  labels at the service boundary; labels are a client mapping, so two clients
  cannot drift.
- **AI use.** AI drafted the architecture documents and the issue tree. Every
  claim in them was checked against the code or the gate; cross-machine
  determinism is explicitly not claimed as observed.
- **Residual risk.** The determinism claim is bounded to one machine until a
  second one runs the verification procedure. Next: add the scale-invariance
  test before the client ships.

**Claim -> Evidence -> Limit -> Decision:** the evidence supports the causality
claim for the tested readings, not for a strategy built on them.
