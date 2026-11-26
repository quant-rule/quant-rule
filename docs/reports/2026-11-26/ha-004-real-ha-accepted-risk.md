# HA-004 — Real HA HA-22/HA-23 Accepted Risk

- Date: 2026-11-26 (UTC)
- Status: **Accepted risk** (human, D5); HA-22 and HA-23 remain `Unverified`.
- Work item: [#1214](https://github.com/quant-rule/quant-rule/issues/1214)
  (`REM-HA-004`, V-HA-04)
- Plan: [`rem-000-v1.6.1-remediation.md`](../../execution-plans/2026-11-26/rem-000-v1.6.1-remediation.md)
- Claim type: Behaviour Claim evidence record; no model, strategy, or edge claim.
- Boundary: one laptop; no separate physical hosts, no second site, no `Real HA`
  claim. HA-003 (versioned upgrade/rollback) is capacity-gated and remains
  pending; this record does not claim its evidence.

## 1. What Real HA requires

| Row | Failure | Required evidence |
|---|---|---|
| HA-22 | Power event on separate physical hosts | At least three hosts, distinct machine and product UUIDs, UTC before/after dates, a before/after canonical digest, and a human attestation |
| HA-23 | AZ loss, disk-controller failure, or true network partition | The same host/UUID/date/digest evidence for the failed domain, plus the alert and finalisation record |

`deploy/v3/real-ha.sh` is the operator-only gate. Its `--plan` output states that
it accepts only a descriptor with separate physical hosts and at least three
failure domains, that the evidence must carry the UUIDs, dates, digest and human
attestation, and that it refuses `lab-single-host` or a missing/non-human
attestation. The plan is not evidence and executes no failure.

## 2. Current evidence

- `docs/config/2026-11-24/chaos-matrix.json`:
  - HA-22: `status=Unverified`, `tier=real-host`, `physical_hosts=0`,
    `failure_domains=0`, `live_evidence=Unverified`.
  - HA-23: `status=Unverified`, `tier=real-host`, `physical_hosts=0`,
    `failure_domains=0`, `live_evidence=Unverified`.
- `deploy/v3/real-ha.sh --plan` prints the operator-only gate and the
  host/UUID/digest/attestation requirements; it reads no host or secret.
- `deploy/v3/real-ha.sh --confirm-real-ha --descriptor deploy/v3/reference/three-az.json`
  refuses with `--evidence is required`, exit 1; the gate cannot be satisfied by
  a descriptor alone.
- The only HA evidence available is `Simulated HA` on the single-host lab; it is
  not a real-host power event, AZ loss, or network partition.

## 3. Accepted risk

The human approved the single-laptop boundary in `REM-000` decision D5: record
`Accepted risk` for HA-22/HA-23, say `Simulated HA`, and make no `Real HA` claim.
The accepted risk is:

- no real power event, disk-controller failure, AZ loss, or true network
  partition was run;
- no separate physical host, machine UUID, or product UUID was collected;
- no real before/after digest or human attestation exists;
- a real-host failure could behave differently from the simulated lab.

The release boundary is explicit: the `v1.6.1` note must say `Simulated HA`, must
not say `Real HA`, and must list HA-22/HA-23 as operator-only residual risks.
The accepted risk does not waive the gate; it records that the gate was not met.

## 4. What would close it

On hardware with at least three separate physical hosts or failure domains:

1. Record `machine_id` and `product_uuid` per host before and after the event.
2. Run `tools/verify.sh` before and after on the same fixed run/config; the
   canonical digest must match.
3. Record the failure window, alerts, and one finalisation.
4. Record the human attestation and run
   `deploy/v3/real-ha.sh --confirm-real-ha --descriptor <real> --evidence <file>`.
5. Update HA-22/HA-23 from `Unverified` only after the human accepts the evidence.

## 5. Verification commands actually run

- `deploy/v3/real-ha.sh --plan` → 9-line operator-only plan; no host contacted.
- `deploy/v3/real-ha.sh --confirm-real-ha --descriptor deploy/v3/reference/three-az.json`
  → `REFUSED: --evidence is required`, exit 1.
- `cargo test -p quantrule-deploy the_ha_matrix_maps_every_row_to_a_rule` →
  1 passed; every matrix row maps to an alert/check rule.
- `qr-governance check --all` and `tools/gate.sh --changed main` → recorded in
  the pull request.

## 6. Verdict

**Accepted risk.** HA-22/HA-23 remain `Unverified`; the single laptop provides
`Simulated HA` only. The v1.6.1 release must carry the boundary above and no
`Real HA` claim. The real-host gate remains open for hardware that can satisfy it.
