# ADR — Canonical Digest and the Determinism Claim Boundary

- Date: 2026-11-12 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)

## Context

The determinism rule says the same input produces the same output "on every
machine". In a distributed system that sentence becomes a claim about two CPUs,
two builds, and two deployments. The operator has one machine, so the claim
cannot be fully tested locally. An untestable claim is not evidence.

## Decision

**Define exactly what is compared, and exactly what is claimed.**

**Canonical digest.** SHA-256 over the sorted list of
`(relative_path, sha256(bytes))` for the data artifacts of a run: `results.json`,
`*.csv`, `*.ndjson`. **Excluded:** `provenance.json`, logs, timestamps, host
names, and any file that records where or when the run happened.

**Claim, as recorded in the characteristics:**

> Enforced by invariant: no transcendental or fused-multiply-add operation in the
> signal path, a pinned target triple, fixed accumulation order, and
> float-roundtrip serialization. Verified locally by scale invariance (one worker
> vs four, identical digest). End-to-end confirmation across two CPUs is part of
> reviewer verification via `make verify`.

**Why this is sufficient:**

| Risk | Where it lives | How it is closed |
|---|---|---|
| Shard order, worker count | merge logic | scale-invariance test, local |
| Accumulation order | fold code | same test |
| Map iteration | serialization | reordered-input test |
| Math library differences | `sin`, `exp`, `powf`, FMA | source guard, build-time |

The CPU is the *least* dangerous variable. Concurrency is the dangerous one, and
it is testable on one machine.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Claim "verified on two machines" | False; no second machine exists |
| Compare the whole bundle | `provenance.json` differs by host and digest; every correct run would fail |
| Ignore cross-machine risk | Silent divergence is an auditability failure |
| Add an emulator to the gate | Slow, and the guard test is stronger |

## Consequences

- The claim is narrower and provable.
- The reviewer runs four commands and compares one line.
- `make verify` prints the canonical digest and nothing else that could differ.

## Revisit trigger

Revisit when a second architecture is in routine use, or when a run must be
compared across an ARM and an x86 host.

## Verification

- `make verify` on one host twice: identical digest.
- `make verify` with `--scale 1` and `--scale 4`: identical digest.
- The guard test fails on a seeded `powf` call.
