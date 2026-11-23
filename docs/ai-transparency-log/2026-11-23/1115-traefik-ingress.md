# AI transparency log — PLAT-002 phases 1–2 (#1115)

- Date: 2026-11-23 (UTC)
- Issue: [#1115](https://github.com/quant-rule/quant-rule/issues/1115)
- Plan: [`docs/execution-plans/2026-11-23/1115-traefik-ingress.md`](../../execution-plans/2026-11-23/1115-traefik-ingress.md)
- Tool: OpenCode / DeepSeek V4.1 Flash
- Role: Orchestrator, Architect, Implementation, Test, Code Reviewer passes by one agent
- Status: Phases 1–2 implemented and gate-verified; the live ingress-loss lab is `Unverified` (no three-VM lab available)

## Intent and boundary

Phase 1 makes the v3 ingress deployable: the VIP/interface descriptor fields,
the static and rendered Traefik configuration, cert-mode mounts, per-edge
keepalived, bootstrap config creation with drift refusal, golden/CLI tests, and
the contracts. Phase 2 adds the local/remote/probe/certificate check rows, the
shell/browser private-CA trust, and the ingress-loss lab harness; the human
approved a three-VM lab result as `Simulated HA` only, with `Unverified` when no
lab is available.

The change is limited to `crates/quantrule-deploy`, `crates/quantrule-shell`,
`deploy/v3`, `tools/`, `docs/`, `README.md`, and `CHANGELOG.md`. It does not add
a dependency, command, service, or Docker client crate. No commit, push, pull
request, board change, tag, or live Docker/Swarm mutation was performed. Every
fixture value is fake; no real secret or private key was added.

## Implemented

| Area | Change | Evidence |
|---|---|---|
| Schema | `ingress.vip` (required IPv4) and `ingress.interface` (default `eth0`); `images.keepalived` is required | `descriptor/v3.rs`, `validate/v3.rs` |
| Validation | `deploy.ingress.vip` for a missing or non-IPv4 value, loopback, multicast, unspecified, broadcast, or equality with an ingress `public_address`; `deploy.ingress.interface` for a 1–15 character Linux name; `deploy.ingress.tls` for `acme`, which is reserved and refused until the future path lands | `validate/network.rs`, `validate/v3/tests.rs` |
| Sample | VIP `203.0.113.20`, interface `eth0`, `tls: cert`, fake `/etc/quantrule/pki/ingress*` paths; edge `public_address` values remain distinct | `deploy/v3/deploy.example.json`, `deploy/v3/pki/` |
| Fixture | Fake `ingress.crt` and `ingress-key.pem`; the `.pem` name and absent PEM key marker satisfy the private-key checker | `deploy/v3/pki/README.md` |
| Static config | Checked-in `deploy/v3/traefik.yml`: one 443 `websecure`, file provider, dashboard off, `api.insecure: false`, default cert, TLS 1.2+, JSON access log dropping `Authorization`, no metrics, 900 s timeouts | `render/ingress.rs` tests; golden |
| Dynamic config | `Host(quantrule.dev)` on `websecure`, `tls: {}`, `http://<stack>_gateway:8080`, HSTS one year without preload, nosniff, no-referrer, `Server` removed, 512 MiB body cap | `render/ingress.rs` tests; golden |
| Stack | Static/dynamic cert/key mounts; configs and key secret declared at the top level; one host-mode 443, no app port | `render/swarm.rs` tests |
| Keepalived | Per-edge `keepalived-<host>.conf`; VRRP 51, priorities 101/100, unicast peers, VIP/32, `nc -z 127.0.0.1 443` fall 3/rise 2; host-network edge unit with `NET_ADMIN`/`NET_BROADCAST` and no ports/deploy | `render/ingress.rs`, `render/infra.rs` tests; golden |
| Bootstrap | Creates `quantrule_traefik` and `quantrule_traefik_dynamic` from the rendered files before the stack; drift refusal; cert/key creation remains | `render/bootstrap.rs`; `tests/swarm_bootstrap.rs` |
| Render bookkeeping | New files enter `render.json`; stale fixed/keepalived outputs are removed; operator files survive | `render/mod.rs` tests; CLI |
| Docs | New ingress contract; deploy-v3, manual 15, HA/security/deployment/operations pointers, config index, README, deploy README, changelog | `qr-governance check --all` |
| Phase-2 checks | `check/ingress.rs` adds the static/dynamic/keepalived byte rules, certificate file mode and `openssl` SAN/30-day query, HTTPS header row, port-80 closure, the four remote mounts, and the lab-only `limited` row | `check/ingress.rs`, `check/ingress/tests.rs`, `check/v3.rs`, CLI/remote CLI tests |
| Probe seam | `ProbeResponse` carries lower-case response headers; the fixture JSON accepts `headers`; a missing fixture entry is not a transport error; the plain-HTTP row passes only after every HTTPS row passed | `check/v3.rs`, `check/ingress.rs` tests |
| CA trust | Shell `QUANTRULE_CA_CERT` loads one PEM CA with `ureq::tls` and keeps the default roots when unset; no flag, no `--insecure`, no command help change. Browser install is documented, not coded | `quantrule-shell/src/client.rs`; 55 unit tests |
| Verify CA | `tools/verify.sh --cacert`, else `QUANTRULE_CA_CERT`, else `CURL_CA_BUNDLE`; every curl and the launched shell use it; `tools/test_verify.sh` proves the argument, unreadable-file refusal, and fake-curl forwarding | `tools/verify.sh`, `tools/test_verify.sh` |
| Lab harness | `deploy/v3/test.sh` preflights VIP/CA/nc, records the VIP holder and 2/2 ingress, drains it, runs a bounded 1 Hz `/health` loop, re-checks digest/stateful, runs the negative controls, restores, and prints `Simulated HA` or `UNVERIFIED`; `QR_SWARM_CACERT` reaches every ingress curl | `deploy/v3/test.sh`, `tests/ha_harness.rs` |
| Phase-2 docs | Ingress contract check/CA/lab/evidence sections; manual, deploy-v3, HA/security/deployment/operations, README, deploy README, tools README, changelog, plan, this log | `qr-governance check --all`; line caps |

## Verification actually run

- `cargo fmt --all -- --check` — clean.
- `cargo clippy -p quantrule-deploy --all-targets -- -D warnings` — clean.
- `cargo test -p quantrule-deploy` — 134 unit, 18 CLI, 6 entrypoint, 7 HA
  harness, 6 remote CLI, 1 golden, 9 bootstrap, 8 wrapper tests passed; the
  golden regeneration test is ignored by design.
- `cargo test -p quantrule-deploy --test render_golden -- --ignored` — regenerated
  the checked-in golden files deliberately.
- `cargo run -q -p quantrule-deploy -- validate deploy/v3/deploy.example.json` —
  `ok: v3 swarm-ha, 11 hosts, 7 services, … ingress quantrule.dev:443`.
- Rendered the sample with local fixture paths to
  `/tmp/opencode/qr-1115-render/out` using `--target all`; inspected
  `quantrule_traefik.yml`, `traefik-dynamic.yml`, `keepalived-edge-1.conf`,
  `keepalived-edge-2.conf`, `stack.yaml`, `infra/edge-*.yaml`, and `render.json`.
  `validate --files` printed `files checked`; a recorded-probe `check` printed
  `check ok` with all three ingress rows healthy and the digest matching.
- `docker compose -f /tmp/opencode/qr-1115-render/out/infra/edge-1.yaml config -q`
  — `edge-1 compose config ok`; `stack.yaml` also parses (Swarm-only warnings).
- `cargo run -q -p quantrule-governance -- check --all` — `governance check ok`.
- `make compose-check` — `compose config ok`.
- `tools/gate.sh` — `gate passed: full` in the worktree target.
- Phase 2 re-ran `cargo fmt --all -- --check` and
  `cargo clippy -p quantrule-deploy -p quantrule-shell --all-targets -- -D
  warnings` — clean.
- `cargo test -p quantrule-deploy` — 149 unit, 19 CLI, 6 entrypoint, 8 HA
  harness, 7 remote CLI, 1 golden, 9 bootstrap, and 8 wrapper tests passed; the
  golden regeneration test is ignored by design. `check/ingress.rs` is 759 lines
  and `check/ingress/tests.rs` 488; `check/remote.rs` stays 866.
- `cargo test -p quantrule-shell` — 55 unit tests and 8 architecture tests
  passed; the CA tests cover a missing file and a malformed PEM. The 61-command
  surface and the 49 pinned help contracts did not change; no flag was added.
- `bash tools/test_verify.sh` — `verify self-test ok`; the fake curl log proved
  `--cacert` reached the call and an unreadable bundle was refused. The stale
  checked-in self-test digest was corrected to the value `tools/verify.sh`
  already prints.
- `cargo run -q -p quantrule-governance -- check --all` — `governance check ok`
  after splitting the ingress tests under the 1,000-line cap.
- `make compose-check` — `compose config ok`.
- `tools/test_tiers.sh ha --plan` — exit 0, lists the ingress phase, contacts no
  Docker.
- Phase-2 review correction re-ran `bash -n deploy/v3/test.sh`, `cargo test -p
  quantrule-deploy --test ha_harness` (9 tests), `tools/test_tiers.sh ha --plan`,
  `cargo run -q -p quantrule-governance -- check --all`, and `tools/gate.sh`;
  all passed, and the lab remains `UNVERIFIED` on this host.
- `./deploy/v3/test.sh` — exit 1 with `ha: UNVERIFIED: Swarm is inactive; run
  this on an active Swarm manager`; this is the honest single-node result.
- `tools/gate.sh` — `gate passed: full` after all phase-2 changes.
- Phase-1 review correction: `cargo test -p quantrule-deploy
  acme_is_reserved_and_refused_until_the_future_path` passed, and a descriptor
  with `tls: acme` is refused with `deploy.ingress.tls` before rendering
  (`qr-deploy: deploy.ingress.tls at ingress.tls: v1.5.10 requires tls: cert;
  acme is reserved for a future path and is not implemented`, exit 1). The docs
  now say the variant is reserved and refused until the future path lands.

## Rejected or corrected suggestions

- A generated static Traefik config was rejected: the approved design keeps
  `deploy/v3/traefik.yml` checked in because it never depends on the stack name.
- A writable `acme.json` or an ACME resolver was rejected for v1.5.10. Cert mode
  is the deployed path. The phase-1 review found that leaving `acme` schema-valid
  rendered a stack with no certificate mounts, so validation now refuses it with
  `deploy.ingress.tls`; the enum remains reserved for the future path.
- A committed `.key` fixture was rejected: the repository private-key checker
  refuses every tracked `.key` and every PEM private-key block. The fixture is
  `ingress-key.pem` with explicit non-key text; tests copy it to a mode-600
  temporary `ingress.key`. The rendered `pki/ingress.key` is excluded from the
  golden tree for the same reason, and its mode is asserted in the CLI/golden
  tests.
- A keepalived unit with the overlay attached was rejected: Compose forbids
  `network_mode: host` together with `networks`; the unit is host-network only.
  The edge file still declares the overlay for HAProxy.
- Port 80 and an HTTP-to-HTTPS redirect were rejected by design: the static
  config has no `web` entrypoint, DNS has no port-80 service, and the manual
  treats a responding port 80 as a defect.
- Adding ingress auth was rejected: `/health`, `/ready`, and `/version` stay
  open and the gateway token remains the only human auth.
- `check/remote.rs` was not split in phase 1; the new ingress check code lives
  in `check/ingress.rs` in phase 2, as the plan states.
- A shell `--ca-cert` flag or `--insecure` switch was rejected: the trust path
  is the `QUANTRULE_CA_CERT` environment variable, and the default roots remain
  when it is unset. No command help output changed.
- Parsing the fake `deploy/v3/pki/ingress.crt` as a real certificate was
  rejected: unit tests inject a `CertificateQuery` fake and the CLI tests point
  `QR_DEPLOY_OPENSSL` at a fake executable. A missing `openssl` is a failed
  `limited` row.
- Ingress authentication was rejected again: `/health`, `/ready`, and `/version`
  stay open and the gateway token is the only human auth. The two-replica token
  rotation overlap is recorded as residual risk in the ingress contract.
- A lab command that removes a stack, volume, secret, config, node, service, or
  leaves Swarm was rejected. The harness only drains/activates nodes and stops or
  starts Docker; the test asserts the forbidden commands are absent.
- Phase-2 review correction: `negative_controls` used `host_address` (the
  private SSH/Swarm address) for the direct-edge 443 control. Corrected to
  `hosts.<host>.public_address`; a missing public address is `UNVERIFIED`, and a
  regression test asserts the boundary. `host_address` stays for SSH and the VIP
  holder.

## Limits and assumptions

- The fast tier proves schema, rendered bytes, mounts, headers, keepalived
  content, the certificate rules via a fake, and bootstrap drift/order with
  fixtures. It starts no stack, holds no VIP, and opens no live TLS handshake.
- A live VIP failover needs a three-VM lab. This host is an inactive single-node
  Swarm, so the harness printed `UNVERIFIED: Swarm is inactive`; the lab row is
  `Unverified`, not a pass, and `Real HA` remains HA-014.
- The shell CA test proves the loader rejects a missing or malformed file; it
  does not complete a live private-CA handshake. The browser trust-store install
  is an operator step.
- The pinned keepalived image must provide `keepalived` and a BusyBox-style
  `nc -z`; the image and network preconditions are documented.
- The `osixia/keepalived` digest in the sample is a fixture pin, like every other
  infrastructure digest; the release still needs real digest verification.
- The static config expects cert mode. An `acme` descriptor is refused with
  `deploy.ingress.tls`; the enum is reserved for the future path and is not a
  working v1.5.10 ingress.

## Lab status

The plan's P03 row is `Unverified` with the exact blocker. A three-VM lab run
would be `Simulated HA` only. No result in this work claims `Real HA`, and no
live Docker/Swarm mutation was performed.
