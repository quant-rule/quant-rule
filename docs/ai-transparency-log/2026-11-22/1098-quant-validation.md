# AI Transparency Log — RA-010 Quant Validation for Packs and Signal Artifacts

- Date: 2026-11-22 (UTC)
- Work item: [#1098](https://github.com/quant-rule/quant-rule/issues/1098)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Plan: [`docs/execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md`](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)
- Branch: `feat/1098-quant-validation`
- Baseline: `3a722b59` (current `main`, merge of PR #1150)
- Report: [`docs/reports/2026-11-22/1098-ra-quant-validation.md`](../../reports/2026-11-22/1098-ra-quant-validation.md)

## Intent and boundary

Produce the RA-010 Quant Validator record for the research pack and signal
artifact contracts. The pass is read-only: it inspects the contracts, code,
tests, and fixtures, reproduces the stated evidence, and records the result.
The only repository files created are this log and the validation report. No
code, test, fixture, config, contract, changelog, or board item was changed; no
commit, push, pull request, merge, tag, or secret access was performed.

This is a sequential pass by an AI agent. It is not an independent agent, an
independent model judgment, or a second human reviewer. It makes no edge,
profitability, or strategy-validation claim.

## Work performed

- Read `AGENTS.md`, the RA/HA plan, ADR-RA, the research-artifact architecture,
  the pack and signal contracts, the RA-008 and RA-009 plans, the RA-007,
  RA-008, and RA-009 AI logs, the Quant Researcher note, and the strategy and
  model workflow boundaries.
- Inspected `quantrule-app::signal`, its closed parser, canonical writer,
  reference-position tests, and the public integration fixture.
- Inspected the pack parsers, lifecycle guards, data-service memory and Postgres
  registries, pack routes, and pack tests.
- Inspected the strategy service resolve/compute/write/register path and its
  route, boundary, and no-per-bar-call tests.
- Inspected the data-service artifact routes and retention tests, the evidence
  lineage route, the gateway signal proxy, the shell pack/signal/lineage
  commands, and the browser pack/signal/lineage transport.
- Reconstructed the canonical pack, model artifact digest, and signal document
  from the fixture and contract in a temporary script outside the repository,
  and compared the SHA-256 with the checked-in golden literal.
- Ran the requested fast suites, the web suite, the disposable-Postgres composed
  tier, the full workspace gate, and the governance check.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-app --lib signal::` | 24 passed |
| `cargo test -p quantrule-app --test signal_artifact_public` | 6 passed |
| `cargo test -p quantrule-strategy-service` | 2 unit, 3 boundary, 10 route tests passed |
| `cargo test -p quantrule-data-service --test artifacts` | 8 passed |
| `cargo test -p quantrule-data-service --test packs` | 7 passed |
| `cargo test -p quantrule-evidence-service --test lineage` | 4 passed, 1 ignored (Postgres; composed covered it) |
| `cargo test -p quantrule-shell --test command_surface --test client_parity` | 2 + 4 passed |
| `cargo test -p quantrule-gateway --test signal_proxy` | 7 passed |
| `cd apps/web && bun test` | 170 passed |
| `cd apps/web && bun run check` | 0 errors, 0 warnings |
| `tools/postgres_composed.sh` | 22 tests over 12 targets passed; four-database roles passed |
| `tools/gate.sh` | `gate passed: full`; 2,789 passed, 0 failed, 32 ignored |
| `cargo run -q -p quantrule-governance -- check --all` | `governance check ok` |

The independent reconstruction produced pack id
`sha256:b82fa966c5d06e7700e55135d541a8baab4fb17427798e9c468a94a46e798cc8`,
model artifact id
`sha256:f7cdde54c97a1a9a70f9a4e83cc9ec6de75152fd85512b1008afa3254cfc46ef`,
and signal id
`sha256:2551537003307cd1c2b324e3eb006c1a7f14abef6ede728f0cf9510ffaaf7689`,
matching the golden literal.

## Learning

- The golden id is a strong whole-document check: reconstructing the canonical
  document from the fixture and contract reproduced the exact id, so the field
  set, row values, numeric forms, and header identity agree with the contract.
- The public reuse test proves that two execution and two portfolio packs have
  different identities and that the signal bytes are unchanged, but it does not
  pass those packs into the computation. The stronger fact is the computation
  signature: execution and portfolio policies are not inputs.
- Every strategy-service fixture is frozen, so the frozen-state precondition is
  code-verified but not test-failed. This should be closed before a freeze gate
  that requires a test for every precondition.
- Event availability is enforced by the bundle input filter and the row parser,
  but the fixture event is available on its occurrence bar; a dedicated
  later-availability test would make the availability claim directly evidenced.
- The composed tier is Postgres conformance, not a service-level signal compute.
  The strategy service still runs against in-process fakes in this pass.
- Run submission and the worker do not yet write or resolve run lineage. The
  signal artifact is currently a pre-run object, not a run-reachable input.

## Residual risk

- Sequential, same-agent validation; no independent reviewer.
- No human-owned pack, production model artifact, run, or market-data partition
  was supplied or opened. No edge or profitability conclusion is possible.
- No full Compose stack, Playwright, multi-host, or HA evidence was reproduced.
- No execution/portfolio consumer applies the reference target state.
- The freeze recommendation is conditional on the non-frozen test, composed
  service evidence, run-v5 integration, and availability tests named in the
  report.

No edge or profitability claim is made.
