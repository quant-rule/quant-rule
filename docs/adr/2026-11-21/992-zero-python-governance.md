# ADR — Zero Python in the Repository

- Date: 2026-11-21 (UTC)
- Status: Accepted
- Owner: Human
- Claim type: ADR (Architecture Decision)
- Supersedes: [`2026-11-12/10-python-boundary.md`](../2026-11-12/10-python-boundary.md)
- Plan: [`992-development-speed-v1.5.0.md`](../../execution-plans/2026-11-21/992-development-speed-v1.5.0.md)
- Epic: #992

## Context

The product image has no Python, but repository governance still depends on
Python checkers, research tools, improvement tools, and historical scripts. That
keeps a second runtime, a second dependency surface, and a slower gate.

The desktop retirement removes `check_schema_prefixes.py` and its test. The
remaining live set is 26 checkers, operational tools, and current
research/improvement tools. The historical set is 66 files.

## Decision

**Remove Python from the repository.**

- Port the post-retirement live set to one Rust binary, `qr-governance`.
- Port gate checkers and their self-tests.
- Port worktree cleanup, verification building, release note resolution, label
  generation, and the Python guard.
- Port current research and improvement tools with 1e-9 parity.
- Delete the 66 historical Python files and tests.
- Delete the Python checker lists and CI Python jobs.
- Change the policy to “No Python.”
- No archive copy. Git history is the recovery.

`qr-governance` has no engine, service, gateway, or application dependency. It
has no network, no clock, explicit `--root`, atomic writes, and stable exit
codes.

## Alternatives considered

| Alternative | Why rejected |
|---|---|
| Keep Python for governance | Keeps the second runtime and the slow gate |
| Archive the historical Python | Contradicts the zero-Python policy; Git is the archive |
| Port only the gate, keep research tools | The policy would still be false |
| Rewrite Git history to remove Python | Destructive; unnecessary |

## Consequences

- The gate and CI are Rust-only.
- A port without parity is incomplete; numeric mismatch stops the port.
- Historical evidence remains in Git history and historical docs, never as live
  code.
- The governance binary becomes part of the developer toolchain, not the product
  image.

## Verification

- No `.py` file in the current tree.
- No `python3` in gate, CI, release, or current docs.
- `./tools/gate.sh` passes with Python removed from `PATH`.
- The product image carries no Python.
