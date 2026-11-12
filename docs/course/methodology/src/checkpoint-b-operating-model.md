# Checkpoint B - Operating Model and Feedback

## Lifecycle

**Iterative and incremental, with pre-registered increments.**

- Dominant uncertainty: whether a rule-based edge exists at all. No lifecycle
  choice removes that uncertainty; only disciplined experiments measure it.
- Therefore each increment is a frozen, pre-registered run: hypothesis, data
  version, costs, and green criteria recorded before execution.
- Iteration is allowed on the platform; it is not allowed on a hypothesis after
  it has been run.

Evidence: `docs/workflows/improvement-workflow.md` and the run manifest
contract. Limit: the model has no accepted baseline yet, so the loop has not yet
closed once.

## Flow of one item

```text
hypothesis -> pre-registration -> data version -> run -> review -> decision
    ↑                                                  |
    +------------ new hypothesis (new record) ---------+
```

| Stage | Waits | Depends on |
|---|---|---|
| Hypothesis | on the human | one written sentence and a rule |
| Pre-registration | on review | data version, costs, green criteria |
| Run | on the machine | frozen config; minutes to hours |
| Review | on the human | the bundle and its digest |
| Decision | on the human | review, not on the run |

The longest wait is a run, and it is unattended by design.

## Feedback

- Who: the human, reading the review and the digest.
- Which decision it can change: accept, revise, or stop the hypothesis.
- Who can revise code before commitment: the human, before the config is frozen;
  after the run, a change is a new pre-registration.

## Unit and done rule

- **Unit:** one issue with one observable acceptance criterion.
- **Done:** the gate passes, the change is merged, and the evidence is recorded.
- **Stop rule:** a unit that grows past one deliverable is split, not extended.

## Review rule

Peer-style review of the diff plus the gate (`cargo fmt`, `clippy -D warnings`,
`cargo test`, documentation checkers). A red gate blocks the merge; a review
comment blocks it too.

## MVP or incremental

Incremental. The value is established - a reproducible backtest is useful on its
own - and the scope grows by adding services, not by testing whether users want
the product.

## What would revise the plan

A run that cannot be reproduced from its own provenance, or a platform change
that forces a hypothesis to be re-run under new costs. Either would stop feature
work and fix the evidence chain first.

**Observed:** the pipeline and the gate exist. **Proposed:** the distributed
lifecycle. **Unknown:** whether any rule survives out-of-sample.
