# Checkpoint B - Operating Model and Feedback

## Lifecycle

**Iterative and incremental, with pre-registered increments.**

- Dominant uncertainty: whether a rule-based edge exists at all. No lifecycle
  removes that; only disciplined experiments measure it.
- Each increment is a frozen run: hypothesis, data version, costs, and green
  criteria recorded before execution.
- The platform may iterate freely. A hypothesis may not: after it runs, a change
  is a new, separately recorded hypothesis.

**Observed:** the pipeline, the bundle contract, and the gate exist.
**Proposed:** the distributed run lifecycle.
**Unknown:** whether any rule survives out-of-sample.

## Flow of one item

```text
hypothesis -> pre-registration -> data version -> run -> review -> decision
     ^                                                      |
     +----------- new hypothesis (new record) --------------+
```

| Stage | Waits on | Depends on |
|---|---|---|
| Hypothesis | the human | one written sentence and one rule |
| Pre-registration | review | data version, costs, green criteria |
| Run | the machine | a frozen config; minutes to hours |
| Review | the human | the bundle and its digest |
| Decision | the human | the review, not the run |

The longest wait is a run, and it is unattended by design.

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
