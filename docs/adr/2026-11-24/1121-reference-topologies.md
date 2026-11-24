# ADR-1121: Reference Topologies, Four Rack Domains, and the Managed-Stateful Boundary

- Date: 2026-11-24 (UTC)
- Status: Accepted
- Issue: [#1121](https://github.com/quant-rule/quant-rule/issues/1121)
- Decision owner: Human
- Related: [HA deployment profile](../2026-11-22/1082-ha-deployment-profile.md), [multi-host deployment](../2026-11-20/958-multi-host-deployment.md), [image distribution](1120-image-distribution.md), [reference contract](../../config/2026-11-24/reference-topologies.md)

## Context

The v3 descriptor can express hosts, failure domains, anti-affinity, three Swarm
managers, and pinned stateful units, but the only checked-in HA example is a
single illustrative eleven-host layout. An operator needs two reviewable
topologies: a three-AZ design and an honest single-AZ design. The descriptor has
one `failure_domain` label and no availability-zone field, so a rack and a zone
are both expressed through that label.

The previous work left the 16 infrastructure image pins as fixtures. Fourteen
can be resolved from registry metadata; `minio`, `minio_client`, and the
application image cannot be resolved anonymously here. The lab descriptor also
carried documentation addresses rather than the real lab LAN.

“Managed stateful” is ambiguous: it could mean an operator-managed external
database, or a public-cloud managed service. Only the first is compatible with
the single-operator, owned-host deployment model.

## Decision

1. Add two reference descriptors with the same eleven host names:
   `deploy/v3/reference/three-az.json` (`swarm-ha`) and
   `deploy/v3/reference/single-az.json` (`single-az`). Keep
   `deploy.example.json` as the sample and `deploy.lab.json` as the lab.
2. Use four rack-level `failure_domain` values. The 3-AZ file distributes them as
   two racks in AZ1, one rack in AZ2, and one rack in AZ3. PostgreSQL, etcd, and
   RabbitMQ take one member in each AZ; MinIO takes all four racks, one drive
   each, at EC:2.
3. Keep the edge/VIP pair on one L2 segment and one AZ, in two racks. VRRP needs
   that segment for the VIP and gratuitous ARP. Document the resulting
   single-AZ ingress dependency; do not pretend the edge tier is AZ-redundant.
4. Do not add an `availability_zone` field and do not create a v4 descriptor in
   PLAT-008. `failure_domain` remains the only placement label. A future field
   needs its own ADR, schema work, and validator tests.
5. Make the single-AZ reference literally single-AZ: four racks in one zone,
   profile `single-az`, no zone-failure claim. Rack-loss tolerance is the only
   reliability statement it supports.
6. Treat “managed stateful” as external endpoints on operator-controlled hosts,
   not a public cloud. PLAT-008 writes the boundary and a deferred section; the
   profile and schema implementation is a follow-up. A public-cloud managed
   service needs a new ADR and a separately qualified claim.
7. Publish the sizing table as `deploy/v3/reference/sizing.json` and mirror it in
   a bounded contract document. A Rust test keeps roles, min/recommended values,
   and reference host roles in agreement.
8. Update the lab template to the real lab addresses (`10.10.0.11..13`, VIP
   `10.10.0.20`, public `10.10.0.11/12`, one `lab` domain, three managers). Keep
   it a template: `/etc/hosts`, the private CA, the operator certificate, and the
   firewall are preconditions; the repository ships no lab certificate.
9. Resolve the 14 free infrastructure pins from registry metadata and keep
   `minio`, `minio_client`, and the app image as exact blockers. Release mode
   rejects every fixture at its field, including a reverted old pin.

## Alternatives considered

| Alternative | Consequence | Rejected because |
|---|---|---|
| Add an `availability_zone` field now | Cleaner AZ semantics | A schema and v4 change is larger than PLAT-008; `failure_domain` already expresses the contract |
| One combined reference file | Less duplication | The two topologies make different failure claims; one file would hide the single-AZ boundary |
| Edge pair stretched across two AZs | Ingress survives an AZ loss | It needs a documented stretched L2 and a second failure mode; the approved scope allows one L2/AZ with an honest dependency |
| Claim AZ tolerance for single-AZ | Simpler wording | It is false; four racks in one AZ do not survive a zone loss |
| Public-cloud managed stateful profile | Less operator work | It breaks the owned-host model and needs a new reliability, retention, and evidence contract |
| Keep all 16 pins as fixtures | No registry work | Fourteen are resolvable; leaving them stale blocks a production descriptor |
| Guess the three blocked digests | A green release check | A guessed digest is worse than a named blocker |

## Consequences

Positive:

- An operator can choose a 3-AZ or single-AZ reference without inventing hosts,
  failure domains, or placement.
- The edge/VIP L2 dependency and the single-AZ limit are explicit.
- Sizing and topology drift is caught by a test rather than by prose.
- Fourteen infrastructure units have real digests; the three blockers are
  visible and fail closed.

Negative:

- The 3-AZ reference still loses public ingress if AZ1 fails; a second ingress
  site is out of scope.
- The single-AZ reference is not zone-redundant and must not be sold as such.
- A managed-stateful profile remains unimplemented, and public-cloud managed
  services are explicitly outside the current claim.
- The pin table ages; a new tag or digest is a reviewed change, not an automatic
  update.

## Residual risks

- A fabricated digest passes the denylist because release mode is offline; only
  the registry and the signed release prove existence.
- The lab has no live run here. Remote CLI/browser TLS, the VIP, and the
  firewall remain `Unverified`; `Simulated HA` is the most a lab run can claim.
- Physical host, rack, AZ, power, and disk-controller failure remain HA-014
  evidence; no PLAT-008 document says `Real HA`.
