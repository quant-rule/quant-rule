# HA-011: Client Retry, Backoff and Reconnect

- Date: 2026-11-23 (UTC)
- Status: In Review (approved 2026-11-23)
- Issue: [#1109](https://github.com/quant-rule/quant-rule/issues/1109)
- Claim type: Behaviour Claim
- AI log: [`docs/ai-transparency-log/2026-11-23/1109-client-retry.md`](../../ai-transparency-log/2026-11-23/1109-client-retry.md)
- Workflow: [Execution Plan](../../workflows/execution-plan.md)
- Parent: [`1082-v1.6.0-ra-ha.md`](../2026-11-22/1082-v1.6.0-ra-ha.md)

## 1. Intent

A transient gateway restart, a draining replica, or a brief overload must not
turn a read into a failed command. The shell and the browser retry only
idempotent GET reads inside one bounded, deterministic budget; a mutation is
never repeated. The browser shows one additive reconnect banner, and the shell
prints one stable transport error naming the attempt count.

## 2. Current state

- `crates/quantrule-shell/src/client.rs` makes one attempt for every method and
  forwards the `ureq` error text, which can name a path.
- `apps/web/src/lib/api.ts` makes one `fetch` attempt; a network error reaches
  the screen raw and no reconnect state exists.
- The runtime already defines the retry vocabulary for internal calls in
  `crates/quantrule-runtime/src/retry.rs`; the shell cannot import it.
- The 61-command surface and the 49 pre-RA-009 command contracts are frozen.

## 3. Scope

Included:

- Shell GET retry: 3 attempts, 200 ms base, factor 4, 5 s cap, 15 s deadline.
- Browser GET retry: 3 attempts, 250 ms base, factor 4, 4 s cap.
- Retry on transport error, 408, 429, 502, 503 and 504 only.
- No retry on 401, 403, 404, other 4xx, an invalid response, or any
  POST/PUT/DELETE/streaming upload.
- One shell transport error naming the attempt count and no URL, token, body or
  secret.
- One browser transport store (`reconnecting`, last error) and one additive
  banner; no new section or command.
- Unit, printed-output, and E2E evidence; client, HTTP, manual, changelog, plan
  and AI-log updates.

Excluded:

- Server-side retry, drain, TLS, secrets, queue, or migration changes.
- A new command, screen, endpoint, dependency, or parity row.
- Composed/HA evidence or a claim that failover was observed.

## 4. Assumptions and uncertainty

- A GET is safe to repeat; every mutation path is rejected by method before the
  loop. The server's idempotency and content-addressed run identity are
  unchanged.
- The shell sleeps in-process; a command has no shutdown signal and the budget
  is bounded by the deadline.
- Browser delays are real in unit tests; the added wall time stays inside the
  gate budget.

## 5. Decisions

| Decision | Choice | Status |
|---|---|---|
| Shell policy | 3 attempts, 200 ms base, factor 4, 5 s cap, 15 s deadline, no jitter | Approved |
| Browser policy | 3 attempts, 250 ms base, factor 4, 4 s cap, no jitter | Approved |
| Retry scope | GET only; transport, 408, 429, 502, 503, 504 | Approved |
| Shell error | `cannot reach the gateway after N attempts` | Approved |
| Browser state | `transport.ts` store; one `reconnect-banner`; no new section | Approved |
| Identity | Retries are client-local; server idempotency and run identity unchanged | Approved |

## 6. Acceptance criteria

- A transient GET that fails twice succeeds with byte-identical output.
- A permanent GET fails after 3 requests with the stable attempt-count message.
- A POST, a 401 and a 404 are each sent once; a parse failure sends none.
- The browser retries a transient GET and a GET `fetchBytes`, never a mutation.
- The browser banner appears while retrying, clears on success, and remains with
  the last transport error after the budget.
- `clientParity.test.ts` passes unchanged; `COMMANDS.length` stays 61.
- Docs state the exact budgets, methods, statuses, no-retry list, message and
  client-local boundary.

## 7. Work breakdown

| Step | Work | Role | Paths |
|---:|---|---|---|
| 1 | Shell retry policy and transport error | Implementation | `crates/quantrule-shell/src/retry.rs`, `client.rs`, `error.rs`, `main.rs` |
| 2 | Shell printed-output tests and fixture faults | Test Engineer | `crates/quantrule-shell/tests/` |
| 3 | Browser retry and transport store | Implementation | `apps/web/src/lib/transport.ts`, `api.ts`, `App.svelte` |
| 4 | Browser unit and E2E tests | Test Engineer | `apps/web/src/lib/*.test.ts`, `apps/web/e2e/connection.e2e.ts` |
| 5 | Docs, plan, AI log | Implementation | `docs/`, `CHANGELOG.md` |

## 8. Verification

```bash
cargo fmt --all -- --check
cargo clippy -p quantrule-shell --all-targets -- -D warnings
cargo test -p quantrule-shell
cd apps/web && bun test && bun run check
cd apps/web && bunx playwright test --project=chromium
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -p quantrule-governance -- check --all
docker compose config -q
tools/gate.sh
```

## Verification table

| ID | Plan item | Acceptance criterion | Evidence / command | Expected | Observed | Status | Reviewer | Date | Notes |
|---|---|---|---|---|---|---|---|---|---|
| V01 | Shell retry | Policy schedule and status set are deterministic | `cargo test -p quantrule-shell` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | `retry::tests`; schedule 200/800 ms, cap and deadline |
| V02 | Shell output | Transient GET succeeds with byte-identical stdout | `cargo test -p quantrule-shell` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | `503` then dropped connection then success; 3 requests |
| V03 | Shell failure | Permanent GET prints the attempt-count message after 3 requests | `cargo test -p quantrule-shell` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | `cannot reach the gateway after 3 attempts` |
| V04 | Method gate | POST, 401 and 404 are sent once; parse failure sends none | `cargo test -p quantrule-shell` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | POST message names 1 attempt; parse test sees 0 requests |
| V05 | Browser retry | Transient GET and `fetchBytes` retry; mutations do not | `cd apps/web && bun test` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | 182 pass; six mutation paths and the upload stay single-attempt |
| V06 | Browser state | Banner transitions and last error are correct | `cd apps/web && bun test` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | Retry, exhausted budget, success, reset |
| V07 | Parity | 61 commands and the parity view are unchanged | `cd apps/web && bun test` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | `clientParity.test.ts` unchanged; 61 commands |
| V08 | E2E | Reconnect banner is visible and settles | `bunx playwright test --project=chromium` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | 64 pass, including the banner |
| V09 | Docs | Budgets, methods, statuses, no-retry list, message and boundary recorded | `qr-governance check --all` | Pass | Pass | Pass | Implementation agent | 2026-11-23 | `governance check ok`; docs under caps |
| V10 | Gates | Format, clippy, docs, compose and full gate | `tools/gate.sh` | Pass | Pass | Pass | Test Engineer | 2026-11-23 | Full gate passed; `docker compose config -q` exit 0; rustdoc clean |

## 9. Risks and recovery

| Risk | Control |
|---|---|
| A mutation is retried | Method gate before the loop; one-attempt tests |
| A 401/404 is hidden by retry | Non-retryable status test |
| Shell error leaks a path or token | Fixed message; test asserts the absent values |
| Banner is a new screen | One banner in `App.svelte`; parity test unchanged |
| Output changes after a retry | Byte comparison against the unretried command |
| Wall-clock time in a signal | Retry only in the client; no engine or artifact change |

Recovery: revert the client modules and their tests; no data, schema, or run
artifact changes.

## 10. Stop conditions

Stop if a parity contract or command meaning must change, if a dependency is
needed, if a mutation path cannot be excluded, or if a retry changes a printed
value, the canonical digest, or run identity.
