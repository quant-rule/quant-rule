# AI transparency log — DIST-071 client parity tests

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Workflow: Requirements & Design / Execution Plan; issue #669 (DIST-071),
  claim type Behaviour Claim.
- Intent: prove that the shell and the browser present the same values for the
  same profile, from one fixture, and that a seeded divergence fails.
- Boundary: shell test files under `crates/quantrule-shell/tests/**`; the
  browser Bun test under `apps/web/src/lib/**`; the shared fixture under
  `tests/fixtures/client-parity/**`; `CHANGELOG.md`; this log. No server,
  service, runtime, simulation, desktop, Compose, Makefile, CI, or production
  client behavior change. No production export was needed.

## Implemented

- `tests/fixtures/client-parity/api.json` holds the gateway responses for
  health, dataset list and show, config list, run list and detail, and profiles
  under the `default` profile. `view.json` holds the normalized view both
  clients must produce. The shell fixture server and the Bun test read the same
  bytes; neither keeps a copy.
- `Fixture::start_parity` now loads that fixture; the existing run-detail
  parity test reads the same `run` response instead of an inline literal.
- `crates/quantrule-shell/tests/client_parity.rs` runs the real binary with
  `--json --profile default` for eight operations, asserts every request
  carries the fixed profile, and compares the resulting view with `view.json`.
- `apps/web/src/lib/clientParity.test.ts` stubs `fetch` at the API boundary,
  calls `api.ts` for the same operations (config show is a local selection from
  the list, as in the screen), and compares its view with the same `view.json`.
- Each suite seeds a divergence: the shell test changes a client value, the
  browser test changes a fixture response. Both assert the comparator rejects
  the known difference.

## Verification

| Command | Observed |
|---|---|
| `cargo test -p quantrule-shell --test client_parity` | 2 passed |
| `cargo test -p quantrule-shell --test parity` | 1 passed; still reads the shared run |
| `cargo test -p quantrule-shell` | 68 passed |
| `bun install --frozen-lockfile` | 54 packages, no lock change |
| `bun run check` | 0 errors, 0 warnings |
| `bun test` | 78 passed across 13 files |
| `python3 tools/check_command_surface.py` | 27 implemented, 0 planned |
| Seeded `view.json` queue depth 2 → 3 | shell suite 2 failed, browser suite 2 failed |
| Restore and rerun both parity suites | 2 passed and 2 passed; `view.json` byte-identical |
| `tools/gate.sh` | passed; see the PR verification table |

`bun run e2e` was not run for this change: the parity tier is a Bun test over
the API layer, not a Playwright DOM test, and no browser is needed. The existing
Playwright suite is unchanged.

## Learning and rejected alternatives

- Rejected: a live gateway or Playwright fixture route. The shared JSON plus a
  stubbed `fetch` is deterministic, network-free, and runs in the fast tier.
- Rejected: invoking Bun from the Rust test. It would make `cargo test` depend
  on the client toolchain and the web tree. Both suites compare to one checked-in
  normalized view instead, so equality is transitive and either client's drift
  fails its own gate.
- Rejected: a new dependency. `serde_json`, `bun:test`, and the existing client
  modules cover the test.
- The browser's config-show operation is a selection from `/catalog/configs`,
  because the gateway has no single-config route. The test mirrors that
  selection instead of inventing a request the client never makes.
- The simulation endpoints are excluded until #680; this fixture covers only
  the merged surface.
