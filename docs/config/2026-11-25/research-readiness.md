# Research Readiness Gate

- Date: 2026-11-25 (UTC)
- Status: Current
- Issue: [#1113](https://github.com/quant-rule/quant-rule/issues/1113)
- Claim type: Behaviour Claim
- Related: [HA-014 lab evidence](../../reports/2026-11-25/ha-014/lab.md), [chaos matrix](../2026-11-24/chaos-matrix.md), [run contract](../../workflows/run-contract.md), [research artifacts](../../architecture/research-artifacts.md), [v1.6.0 plan](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)

## 1. Research-only boundary

QuantRule turns historical OHLCV candles into reproducible evidence about whether
a rule-based alpha has an edge. It **does not trade**, connect to a broker, run an
OMS, provide real-time risk, or accept live orders. The machine-readable record
carries `system.kind = research-only` and `system.trading = false`; the
`research-readiness` checker refuses a matrix that claims otherwise.

## 2. The eight rows

| ID | Claim | Evidence | Status | Boundary |
|---|---|---|---|---|
| RR-01 | One immutable data version and one declared partition per run; a later bar cannot change an earlier output. | [`data/partitions.json`](../../../data/partitions.json), [data lifecycle](../../workflows/data-lifecycle.md), [data contract](../2026-10-30/data-v3.md) | `Current` | The repository proves the contract and causality tests; it does not audit the operator's external source files. |
| RR-02 | An improvement run is pre-registered before execution; validation is one-shot and a development run cannot read the holdout. | [run contract](../../workflows/run-contract.md), [improvement workflow](../../workflows/improvement-workflow.md), [research tests](../../../crates/quantrule-shell/tests/support/research.rs) | `Current` | No tiered run has used v3 yet; the first live pre-registration remains a research action. |
| RR-03 | A significance claim carries a pre-registered correction and primary-cell count. | [engine artifacts](../2026-11-06/engine-artifacts.md), [run contract](../../workflows/run-contract.md), [qualification fixtures](../../../crates/quantrule-shell/tests/fixtures/qualification), [MR7 power record](../../improvement/2026-11-08/MR7-POWER.md) | `Current` | The correction is recorded; it is not a promise of an edge. |
| RR-04 | A worker or evidence-leader failure leaves no succeeded run without a digest and reproduces the same canonical digest after retry. | [HA-18](../../reports/2026-11-25/ha-014/HA-18.json), [HA-19](../../reports/2026-11-25/ha-014/HA-19.json), [HA-20](../../reports/2026-11-25/ha-014/HA-20.json), [HA-21](../../reports/2026-11-25/ha-014/HA-21.json) | `Simulated HA` | One physical host; no host power, AZ, or true network-partition claim. |
| RR-05 | A point-in-time restore reproduces the same canonical digest or fails cleanly. | [DR evidence](../../reports/2026-11-25/ha-014/dr.json), [backup contract](../2026-11-24/backup-restore.md) | `Simulated HA` | In-place lab restore; no production PITR, second site, or real-host RTO claim. |
| RR-06 | A referenced pack or signal is never collected; an unreferenced artifact is collected only after 30 days with a tombstone. | [research artifacts](../../architecture/research-artifacts.md), [pack contract](../2026-11-22/research-artifact-packs.md), [signal contract](../2026-11-22/signal-artifact.md) | `Current` | The 30-day window and tombstone are tested; a live MinIO sweep is not run here. |
| RR-07 | The browser and CLI reach the gateway over HTTPS with the token and no certificate bypass; a lost client reconnects. | [lab evidence](../../reports/2026-11-25/ha-014/lab.md), [client access](../../manual/13-client-access.md), [security](../../architecture/security.md), [ingress](../2026-11-23/ingress.md) | `Simulated HA` | The lab path is not a separate physical machine or a public CA. |
| RR-08 | A broker or worker loss does not lose an acknowledged run; the queue drains and a failed worker leaves one digest. | [queue evidence](../../reports/2026-11-25/ha-014/queue.json), [HA-18](../../reports/2026-11-25/ha-014/HA-18.json), [observability](../2026-11-23/observability.md), [operations](../../architecture/operations.md) | `Simulated HA` | One-broker loss and a worker kill; no 48-hour soak or production capacity claim. |

## 3. Accepted risk

`AR-RESEARCH-SIMULATED-LAB` is the one accepted risk. The four `Simulated HA`
rows are proven on the three-VM single-host lab. Host power loss, AZ loss, a true
network partition, and a real public-CA remote client remain unproven. `Real HA`
remains HA-22/HA-23; the release note must state this boundary.

The human approved the simulated evidence boundary in the v1.6.0 plan on
2026-11-25. The accepted risk is not a second approval of a Real HA claim.

## 4. What this gate proves

- The research input contract, pre-registration and holdout discipline, and
  multiple-testing record exist and are testable.
- Pack and signal retention cannot collect a referenced artifact.
- A failed worker or leader does not publish a partial succeeded run.
- A lab restore and a lab failover reproduce the canonical digest.
- The remote client path uses HTTPS and the bearer token without a certificate
  bypass.
- The machine-readable record is research-only, not a trading system.

## 5. What it does not prove

- Physical host, AZ, power, disk-controller, or true network-partition survival.
- A production PITR, a second site, or a real-host RTO.
- A 48-hour soak, production capacity, or a real public-CA remote client.
- That any alpha has an edge. Research readiness is not an edge claim.

## 6. Verification

```bash
qr-governance check --checker research-readiness
qr-governance check --all
tools/gate.sh --changed main
```

The `research-readiness` checker validates the schema, the eight rows, the
status vocabulary, the research-only system fields, the accepted-risk fields,
and the required evidence. The matrix is
[`research-readiness.json`](research-readiness.json).
