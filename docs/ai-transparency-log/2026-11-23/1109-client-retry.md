# AI Transparency Log — HA-011 Client Retry, Backoff and Reconnect

- Date: 2026-11-23 (UTC)
- Issue: [#1109](https://github.com/quant-rule/quant-rule/issues/1109)
- Plan: [`1109-client-retry.md`](../../execution-plans/2026-11-23/1109-client-retry.md)
- Tool: OpenCode implementation subagent, DeepSeek V4.1 Flash via `opencode-go`;
  sequential Implementation, Test Engineer, and Code Reviewer passes. These were
  not independent agents.

## Intent and boundary

Implement the approved HA-011 leaf: a bounded, deterministic GET retry in the
shell and browser, one stable shell transport message, one additive browser
reconnect banner, tests, and documentation. The work excluded server retry,
drain, TLS, secrets, migrations, a new command or screen, and a dependency.
No model, strategy, execution, portfolio, or artifact behavior was touched.

## Verification and evidence

| Check | Command | Observed |
|---|---|---|
| Shell tests | `cargo test -p quantrule-shell` | Pass; retry, method-gate and output tests included |
| Shell lint | `cargo clippy -p quantrule-shell --all-targets -- -D warnings` | Pass |
| Format | `cargo fmt --all -- --check` | Pass after formatting the new modules |
| Browser tests | `cd apps/web && bun test` | 182 pass; parity count unchanged |
| Browser check | `cd apps/web && bun run check` | 0 errors, 0 warnings |
| Banner E2E | `bunx playwright test --project=chromium e2e/connection.e2e.ts` | 5 pass, including the reconnect banner |
| Repository check | `cargo run -p quantrule-governance -- check --all` | `governance check ok` |
| Docs | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | Pass; no rustdoc warnings |
| Compose | `docker compose config -q` | Exit 0 |
| Full gate | `tools/gate.sh` | `gate passed: full` |

## Learning

- The shell cannot import the runtime retry module because the crate boundary
  allows only `quantrule-api`; the policy vocabulary was mirrored locally rather
  than introducing a dependency.
- The browser store had to be plain TypeScript (`transport.ts`) rather than a
  runed `.svelte.ts` module, because `bun test` imports `api.ts` without the
  Svelte compiler; a runed store would have broken every existing unit test.
- The fixture gained scripted failures so the printed-output tests can prove the
  attempt count without a live gateway. A dropped connection and a `503` are
  both covered.
- The browser retry loop deliberately returns the response unparsed so an
  invalid body fails once, outside the loop.
- Sequential review found no need to change the 61-command surface or the frozen
  49-command contracts; no parity row was added.

## Residual risk

- The browser reconnect banner is one global presentation state. Two concurrent
  GETs can clear it briefly when one succeeds while the other is still retrying;
  it is not an authority for whether a command completed.
- The shell sleeps in-process during a retry, so a command can take the bounded
  15-second budget before it fails.
- No real network partition, failover or composed/HA run was performed; the
  retry paths are proven with in-process faults and browser tests.
