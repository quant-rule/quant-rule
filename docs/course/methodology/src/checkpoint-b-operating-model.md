# Checkpoint B - Operating Model and Feedback

## Spoken script (3 minutes)

My main uncertainty is whether a rule-based edge exists at all. No lifecycle removes that uncertainty. Only disciplined experiments measure it. The user is one researcher who must defend a result months later. The value is a reproducible bundle.

I use an iterative, incremental lifecycle with pre-registered increments. The platform may iterate freely. A hypothesis may not. Each increment is a frozen run: hypothesis, data version, costs, and green criteria are recorded before execution.

One item starts when I write one sentence and one rule. I accept the premise. Before the run, a human reviewer checks the data version, costs, and green criteria. No run starts before that record exists. The run is unattended. The worker executes the frozen config, and the run record is the truth. The longest wait is the run. That wait is intentional.

Review is mine. I read the bundle and the digest. A Quant Validator joins when behavior changed. The decision is mine too: accept, revise, or stop. A change after the run is a new pre-registration, not an edit.

Feedback comes from the reviewer and can change that decision. It arrives before the config is frozen, while code can still be revised. After the run, a change is a new record.

The current release is v1.2.0. The pipeline, the bundle contract, the gate, and the distributed run lifecycle are observed. Two equivalent runs on the release host produced 12 canonical artifacts and 3,559,171 bytes each, with the same digest. The next hypothesis is proposed, not yet run. The unknown is whether any rule survives out-of-sample. A second-day and second-machine rerun remain reviewer-executed.

What would make me revise the method? A run that cannot be reproduced from its own provenance, or a platform change that forces a completed hypothesis to be rerun under new costs. Either stops feature work until the evidence chain is fixed.

I use incremental scope, not an MVP. A reproducible backtest is useful on its own. The value is established, so scope grows by adding services. One unit is one issue with one observable acceptance criterion. Done means the gate passes, the change is merged, and the evidence is recorded. If a unit grows past one deliverable, I split it. The review rule is set before work: review the diff and run the gate. A red gate or a review comment blocks the merge. The review happens before the next hypothesis.

```{=latex}
\newpage
```

## Evidence appendix

### One item, stage by stage

| Stage | What happens | Who checks or decides | Waits on |
|---|---|---|---|
| Hypothesis | one sentence and one rule | I write it; I accept the premise | the human |
| Pre-registration | data version, costs, green criteria | a human reviewer; no run before the record | review |
| Run | frozen config; unattended worker | worker and orchestrator; the run record is truth | the machine |
| Review | bundle and digest | me; a Quant Validator if behavior changed | the human |
| Decision | accept, revise, or stop | me; a change is a new record | the review |


Unit: one issue with one observable acceptance criterion. Done: the gate passes, the change is merged, and the evidence is recorded. If a unit grows past one deliverable, I split it. The review rule is set before work: review the diff and run the gate (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, and the documentation checks). A red gate or a review comment blocks the merge.

### Observed, proposed, unknown

| Status | What |
|---|---|
| Observed | the pipeline, the bundle contract, the gate, and the distributed run lifecycle; two equivalent runs produced 12 canonical artifacts and 3,559,171 bytes each, both `sha256:453c90b4…`; the v1.2.0 gate passed 2,118 tests, 0 failed, 14 ignored, and 24 checkers |
| Proposed | the next hypothesis will use the same frozen-run path |
| Unknown | whether any rule survives out-of-sample; a second-day and second-machine rerun remain reviewer-executed |

Links: [current release note](../../../releases/2026-11-16/v1.2.0.md), [release map](../../../releases/README.md), [run contract](../../../workflows/run-contract.md), [improvement workflow](../../../workflows/improvement-workflow.md).
