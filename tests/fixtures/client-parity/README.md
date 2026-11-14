# Client parity fixture

One fixture set, two clients. The shell test
(`crates/quantrule-shell/tests/client_parity.rs`) serves `api.json` from the
in-process fixture server and prints each operation with `--json`; the browser
test (`apps/web/src/lib/clientParity.test.ts`) loads the same responses through
`api.ts`. Both build the normalized view and compare it with `view.json`, so a
value that one client changes or drops fails that client's test.

- `api.json` — the gateway responses for health, datasets, configs, runs, and
  profiles, under one fixed profile. The shell fixture server and the browser
  test read the same bytes; neither duplicates the values.
- `view.json` — the expected normalized view. It carries the contract fields a
  client must present, in the fixture's own order. It is updated from
  `api.json` deliberately when the contract changes, never edited to hide a
  client difference.

`view.json` is one assertion shared by both clients, not a second response
fixture. The browser E2E DOM checks remain separate; this tier compares values.
