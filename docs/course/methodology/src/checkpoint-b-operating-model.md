# Checkpoint B - Operating Model and Feedback

## Lifecycle

**Iterative and incremental, with pre-registered increments.**

- Dominant uncertainty: whether a rule-based edge exists at all. No lifecycle
  removes that; only disciplined experiments measure it.
- Each increment is a frozen run: hypothesis, data version, costs, and green
  criteria recorded before execution.
- The platform may iterate freely. A hypothesis may not: after it runs, a change
  is a new, separately recorded hypothesis.

Release evidence: [`docs/releases/2026-11-15/v1.1.0.md`](../../../releases/2026-11-15/v1.1.0.md)
(current tag `v1.0.0`).

**Observed:** the pipeline, the bundle contract, the gate, and the distributed
run lifecycle are released (server profile v0.9.0, desktop profile v1.0.0). The
release note records two equivalent runs on the release host, 12 canonical
artifacts and 3,559,171 bytes each, both printing
`sha256:453c90b4442dd049074e126bb46b400b6f61baa986443c97fa27c3edd41c1daa`.
**Unknown:** whether any rule survives out-of-sample; a second-day and
second-machine rerun remain reviewer-executed.

## Flow of one item

```text
hypothesis -> pre-registration -> data version -> run -> review -> decision
     ^                                                      |
     +----------- new hypothesis (new record) --------------+
```

| Stage | Waits on | Depends on | Who checks |
|---|---|---|---|
| Hypothesis | the human | one written sentence and one rule | Quant Researcher; human accepts the premise |
| Pre-registration | review | data version, costs, green criteria | human reviewer; no run starts before the record exists |
| Run | the machine | a frozen config; minutes to hours | orchestrator/worker; unattended, with the run record as truth |
| Review | the human | the bundle and its digest | human reviewer plus Quant Validator when behavior changed |
| Decision | the human | the review, not the run | human; accept, revise, or stop |

The longest wait is a run, and it is unattended by design. The release evidence
is the two-run comparison in `tools/verify.sh` and the v1.0.0 release note:
two verify-mini runs, 12 canonical artifacts and 3,559,171 bytes each, both
`sha256:453c90b4…`.

## Feedback

- Who supplies it: the human, reading the review and the digest.
- Which decision it can change: accept, revise, or stop.
- Who can revise code before commitment: the human, before the config is frozen.
  After the run, a change is a new pre-registration.

## Unit and done rule

- **Unit:** one issue with one observable acceptance criterion.
- **Done:** the gate passes, the change is merged, and the evidence is recorded.
- **Stop rule:** a unit that grows past one deliverable is split, not extended.

## Review rule

Defined before work starts: review of the diff plus the gate (`fmt`, `clippy -D
warnings`, tests, documentation checks). A red gate blocks the merge; a review
comment blocks it too.

## MVP or incremental

Incremental. The value is established - a reproducible backtest is useful on its
own - so scope grows by adding services, not by testing whether anyone wants the
product.

## What would revise the plan

A run that cannot be reproduced from its own provenance, or a platform change
that forces a completed hypothesis to be re-run under new costs. Either stops
feature work until the evidence chain is fixed.
