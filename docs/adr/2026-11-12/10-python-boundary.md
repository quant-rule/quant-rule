# ADR — Confine Python to Repository Governance

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Related: [`09`](09-add-simulation-service.md)

## Context

The product's research tooling is Python: `tools/simulation`, `tools/charts`,
`tools/oracle`, and `tools/trades` total 37 files and 8,230 non-test lines. They
are pure standard library, which makes a faithful port realistic.

A shipped product that needs a second runtime has two toolchains, two dependency
surfaces, and two ways for a result to be computed. That is a determinism and an
operational risk.

## Decision

**The product image contains no Python interpreter and no `.py` file.**

| Tier | Contents | Rule |
|---|---|---|
| Product | Rust binaries and static assets | No Python |
| Repository governance | `tools/check_*.py`, `worktree_cleanup.py`, `improvement/cleanup.py`, `build_verification.py` | Python stays |
| Retired | `tools/simulation`, `tools/charts`, `tools/oracle`, `tools/trades` | Deleted after parity |
| Excluded from the product | `tools/improvement`, `tools/research` | Python, outside the image, post-1.0.0 programme |

**The rule:** anything that computes a market measurement, a chart, a mark, a
check, or a research artifact ships as Rust. Python never runs inside a
container.

Two assertions enforce it:

1. The Dockerfile never copies `tools/`.
2. A release check fails if `python3` exists in the image.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep the Python harness in the image | A second runtime and a second way to compute a result |
| Port every tool before v1.0.0 | 19,369 more lines of orchestration; delays the distributed stack and the course |
| Remove Python entirely | The repository gates would lose their tooling for no product gain |

## Consequences

- The port is the largest single work item after the client: 8,230 lines with
  1e-9 golden parity against captured Python outputs.
- `tools/improvement` and `tools/research` remain Python and are explicitly not
  part of the product. A reader must not confuse "not in the image" with "not
  used".
- The repository checkers stay, and `check_legacy` bans the retired paths.
- A future port of the orchestration tooling is its own programme with its own
  plan.

## Revisit trigger

Revisit if the excluded orchestration tooling starts producing artifacts that the
product consumes at runtime, or if a second runtime is accepted for a concrete
capability.

## Verification

- `docker run --rm <image> sh -c 'command -v python3'` prints nothing.
- The Dockerfile has no `COPY tools/`.
- `check_legacy` fails on a reintroduced `tools/simulation` import.
