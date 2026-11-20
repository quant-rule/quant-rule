# ADR — Multi-Host Service Deployment Profile

- Date: 2026-11-20 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`01`](../2026-11-12/01-adopt-distributed-service-based-architecture.md), [`12`](../2026-11-12/12-desktop-distribution.md), [`deployment`](../../architecture/deployment.md), [`security`](../../architecture/security.md)
- Plan: [`959-multi-host`](../../execution-plans/2026-11-20/959-multi-host.md)

## Context

The six services are separate processes with separate schemas, failure units,
and scaling units, but the released deployment places all of them on one host.
The one-host Compose stack publishes only the gateway and keeps the rest on a
bridge network. That is correct for a laptop, a single server, and the desktop
installer. It is wrong when compute must be larger than one machine, when the
gateway must stay in one network zone and the workers in another, or when the
database and object store already live on dedicated hosts.

The engines already depend on URLs and environment variables, not on process
locality. The missing piece is a deployment contract: which service runs where,
which endpoints are shared, how hosts trust each other, and how one image
identity is guaranteed. The previous plan considered "split host is
configuration" sufficient; it is not. A supported profile needs a descriptor, a
validator, a network gate, tests, and documentation.

## Decision

Adopt a **multi-host server deployment profile** for the six services. It is a
deployment shape, not a second architecture.

- **Descriptor.** One JSON document (`quantrule.deploy/v2`) names the image
  digest, hosts, service placement, endpoints, replicas, and network gate.
- **Validator.** A Rust `qr-deploy` binary validates the descriptor, renders
  per-role environment files, and checks the deployed hosts.
- **Topology.** The gateway is the only public entry point. `data`, `model`,
  `evidence`, `simulation`, and `backtest` bind to a private overlay address.
  Postgres, the object store, and the queue are shared infrastructure hosts.
- **Network gate.** WireGuard (or Tailscale) is the trusted network. The
  descriptor refuses a public bind for a non-gateway service. TLS terminates at
  a reverse proxy in front of the gateway.
- **Image identity.** Every host runs the same image digest; `qr-deploy check`
  compares `/version` and fails on a mismatch.
- **Scaling.** Backtest workers scale by count across hosts through the queue.
  One model endpoint is supported in v1; multiple model hosts need a load
  balancer and are a follow-up.
- **Desktop.** The desktop profile stays embedded and single-host. Multi-host is
  a server profile.

## Non-goals

- Cloud provider integration, managed databases, or a cloud runtime.
- High availability, autoscaling, Kubernetes, or multi-region placement.
- Service-to-service mTLS. The first profile trusts a private overlay network.
- RPM and DEB packaging.
- Model or strategy behavior changes.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep one host only | Cannot use a second machine for capacity or network placement |
| Kubernetes or Nomad now | Adds a scheduler, control plane, and operational surface before the driver exists |
| Docker Swarm | A scheduler with no additional driver; Compose plus a descriptor is smaller |
| mTLS between services now | Certificates and rotation for six services before a trust-boundary need |
| Public per-service ports | Exposes internal contracts and bypasses the gateway token |
| Event-driven data plane | Makes byte-identical output depend on delivery order |

## Consequences

- The deployment contract becomes a reviewed artifact; an unsafe placement fails
  before anything starts.
- Each host needs the same image, a private overlay, and the descriptor's
  endpoint values. The operator still owns firewall and overlay configuration.
- Postgres, the object store, and the queue remain single points of failure.
  Multi-host placement is not high availability.
- The internal services have no application authentication. The trusted network
  is the security boundary, as it already is for the container network.
- `qr-deploy` is a new operator tool and a new versioned contract.
- Startup order and retries become operator-visible; the services must tolerate
  a dependency that is not ready yet.

## Revisit trigger

Revisit when a second operator needs isolation, when the platform must survive a
host failure (HA), when a managed cloud database or object store becomes a
requirement, or when a service must cross an untrusted network.

## Verification

- The descriptor validator has golden tests and refuses every unsafe case.
- A three-host LAN run completes the verification fixture.
- The multi-host canonical digest equals the single-host digest.
- One and two backtest workers produce the same digest.
- Only the gateway is reachable from the public side.
