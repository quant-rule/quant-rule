# ADR — Profiles Are Organisational Scoping, Not a Security Boundary

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`03`](03-single-principal-no-accounts.md), [`09`](09-add-simulation-service.md)

## Context

One operator runs several research lines: instruments, timeframe sets, model
candidates, and simulation campaigns. Keeping them in one undifferentiated
catalog makes the run list and the research memory unusable after a few weeks.

The need is organisation, not isolation. There is one principal, no
authentication, and no promise that one profile cannot see another.

## Decision

Introduce **profiles** as an organisational scope over records.

| Concern | Decision |
|---|---|
| Identity | Still one principal. A profile is not an identity |
| Scope | Every catalog row, run record, simulation record, and memory note carries `profile_id` |
| Default | `default`, created by the first migration |
| Object bytes | Content-addressed and **shared**; identical candles are stored once |
| Visibility | Scoped by `profile_id`; not enforced as a security control |
| API | `GET /profiles`, `POST /profiles`, and the `X-QuantRule-Profile` header on every request |
| Client | A switcher in the top bar; the active profile is persisted locally |
| Shell | A global `--profile` and `profile list|create|use` |
| Backup | Per profile or all |
| Deletion | Removes records and references; never deletes a shared object |

**Run identity is unchanged.** `run_id` stays content-derived. Two profiles that
submit identical inputs share artifacts but keep separate run records, so
deleting one record never orphans another's.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Multi-tenancy with accounts | Out of scope by D4; adds authentication and isolation promises |
| Separate deployment per profile | Ten databases and ten object stores for one person |
| Directory-per-profile on disk | Breaks the single catalog and the shared content cache |
| Tags instead of profiles | Tags do not scope the default view; every query would filter |

## Consequences

- Every table gains `profile_id`; every query is scoped. A forgotten scope is a
  defect, not a preference.
- The migration backfills existing rows to `default`.
- Two profiles cannot collide accidentally, and they cannot be used to hide one
  research line from another. The documents say so.
- `security.md` must state that profiles are not a boundary; a reader must not
  mistake organisation for isolation.
- The desktop shell and the browser show the same switcher, so the concept is
  client-independent.

## Revisit trigger

Revisit if a second principal appears, or if a profile must be shared between
people. Both are the deferred team-cloud decision.

## Verification

- Two profiles see independent catalogs and run lists.
- Identical bytes uploaded twice are stored once.
- Deleting a run record in one profile leaves the other's record intact.
- An unknown profile header returns `404 profile.not_found`.
