# ADR — Three Test Tiers: Fast, Composed, and Port Parity

- Date: 2026-11-14 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`07`](../2026-11-12/07-canonical-digest-and-claim-boundary.md), [`10`](../2026-11-12/10-python-boundary.md)
- Registry: [`docs/port-parity/`](../../port-parity/README.md)
- Plan: [`DIST-000`](../../execution-plans/2026-11-12/dist-000-distributed-platform.md) (DIST-064)

## Context

The isolation rule says a test never reaches the network. The distributed system
has six services that must be tested together, so a literal reading forbids the
only evidence that the system composes. Separately, a ported computation can be
declared complete while its numbers drift from the retired implementation: the
port-parity rule existed as a sentence, with no fixture layout, no command, and
no check. The current wording names two tiers and leaves both gaps to review
memory.

## Decision

**Three tiers, one name and one command each. The fast tier and the port-parity
registry check run on every pull request. The composed tier is opt-in; the
port-parity comparison is required when a port is registered and before a
release.**

| Tier | Evidence | Command | When |
|---|---|---|---|
| Fast — unit and integration | In-process transport fake; no socket, no filesystem outside the repository, no live market, no clock or ordering dependence | `tools/gate.sh` (`make test`) | Every pull request |
| Composed | The real stack: gateway, services, Postgres, MinIO, RabbitMQ under Compose | `tools/test_tiers.sh composed` (`make test-composed`) | Opt-in; required before release |
| Port parity | Golden fixtures from the retired implementation, every numeric value matched to `1e-9` | `tools/test_tiers.sh parity` (`make test-parity`) | Registry check every pull request; comparison when a port is registered, and before release |

**Fast tier.** `tools/gate.sh` runs `cargo test` and the repository checkers. It
uses the in-process transport fake and never opens a socket. This stays the gate
for every change.

**Composed tier.** It exercises what the fast tier cannot: the real services, the
database, the queue, and the object store. It is opt-in because it needs Docker
and the Compose stack, and it is required before a release because a green fast
tier is not evidence that the services compose. The stack and its suite are
#643/#644. Until they land, `tools/test_tiers.sh composed` exits `2` and names
the prerequisite; it never reports a pass it did not run.

**Port parity tier.** A ported research computation is not complete until its
numbers match the retired implementation. `docs/port-parity/registry.json` names
each port, its Rust target, its comparison test, and its fixture; the fixture is
`docs/port-parity/<id>.golden.json` with the source, capture date, tolerance, and
cases. `tools/check_port_parity.py` runs in the gate and fails when the entry,
target marker, test marker, fixture, or tolerance is missing or wrong. The
registry is empty until #679 lands the first port; an empty registry passes
because no port means no parity claim.

**What review must reject.** A ported computation with no registry entry, no
fixture, or no comparison test. A "composed test" that ran against an in-process
fake. A fast test that opened a socket. The checker enforces the registry; review
enforces the absence of an unregistered port.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep two tiers plus the separate port-parity sentence | The rule had no layout, command, or check; a port could pass review without evidence |
| Run composed on every pull request | Docker and flaky infrastructure would slow the common change and fail for reasons unrelated to it |
| Compare a port to the current Rust code | The current code can drift from the retired behavior; the fixture is the stable reference |
| Store fixtures in Python or an ad-hoc format | The product carries no Python; JSON is checked-in data a Rust test can read |
| Leave unregistered ports to review memory | Memory is not enforceable; the marker and registry make the claim checkable |

## Consequences

- The gate stays fast and hermetic; the expensive evidence is named and required
  where it belongs.
- A release is not green until the composed tier has run and every registered
  port matches.
- The composed tier is not executable until #643/#644; the command reports that
  rather than silently skipping.
- The registry is empty until #679; no port, no parity claim.
- The policy adds one gate check and one marker convention; a port that omits
  them fails.

## Revisit trigger

Revisit when the Compose stack lands and the composed suite becomes a real
command, or when a non-Rust port needs a different fixture format.

## Verification

- `tools/test_tiers.sh fast` runs the gate, and the gate runs
  `tools/check_port_parity.py`.
- `tools/test_tiers.sh parity` prints `port parity ok` for the empty registry.
- `tools/test_tiers.sh composed` exits `2` and names #643/#644 until the stack
  exists.
- The checker self-test builds a valid port and rejects a missing fixture, a
  loose tolerance, a missing target marker, a missing fixture reference, a
  duplicate ID, and a path outside the repository.
- `tools/gate.sh` passes.
