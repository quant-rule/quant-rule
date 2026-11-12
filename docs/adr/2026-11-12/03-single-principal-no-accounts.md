# ADR — Single Principal, No Accounts

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`01`](01-adopt-distributed-service-based-architecture.md)

## Context

The product serves one operator. The previous increment had no accounts because
it had one process. Distribution raises the question again, and the answer must
be a recorded decision rather than an omission.

## Decision

One principal, injected by the gateway from configuration:

```text
Principal { id: "operator", workspace: "default" }
```

- **No authentication.** No passwords, no tokens, no sessions, no user table.
- **No identity service.** A service with no credentials to own has no reason to
  exist.
- **No per-user quotas.** Capacity limits are host limits.
- **`owner` column retained** on every run and artifact, written as `operator`.
- **Authorization seam retained** behind one trait in the gateway so a future
  multi-tenant increment replaces an implementation, not a call site.
- **Remote exposure is a network concern.** A bearer token or a private network
  (WireGuard, Tailscale) protects the port; it is not an account system.

**Attribution is not authentication.** A self-declared operator label may appear
in a shared deployment; the documents must call it a label, never a control.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| OIDC / Keycloak now | A 1 GB container, sessions, and recovery for one principal |
| Shared token only, no seam | Cheap now, a rewrite later |
| No `owner` column | Saves one field, forces a migration and a data backfill later |

## Consequences

- The trust boundary is the host and its container network.
- A public port without a token or a private network is a misconfiguration, and
  the security document says so.
- Multi-tenancy remains a separate ADR with a separate security model.
- The `Authorization` trait has exactly one implementation today; a test asserts
  every request resolves to a principal before reaching a use case.

## Revisit trigger

Revisit when a second operator must be prevented from reading the first
operator's runs, or when runs must be attributable to a verified identity.

## Verification

- No service stores a credential.
- Every run row carries `owner`.
- Removing the trait implementation fails the build.
