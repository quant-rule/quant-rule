# AI Transparency Log — Security Baseline and Local Bind

- Date: 2026-11-25 (UTC)
- Work item: QA-005 [#1126](https://github.com/quant-rule/quant-rule/issues/1126)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017

## Intent and boundary

Review the Compose and remote security baseline, verify the live controls, and
fix any exposure that makes the unauthenticated local profile reachable without
the bearer gate. The v3 lab supply-chain path is cited, not re-run; no release
tag exists.

## Work performed

- Found the local profile bound `0.0.0.0:8080` without a token.
- Changed the base Compose bind to `${QR_GATEWAY_BIND:-127.0.0.1}:8080:8080`;
  `tools/remote.sh` sets `QR_GATEWAY_BIND=0.0.0.0` for the remote profile.
- Added a topology guard in `crates/quantrule-runtime/tests/database_topology.rs`
  and updated the README, security architecture, and remote override comment.
- Ran the live local/remote/local sequence and the role, secret, port, and TLS
  checks recorded in `docs/reports/2026-11-25/security-baseline.md`.

## Verification and evidence

- Local: `8080/tcp -> 127.0.0.1:8080`; one host listener.
- Remote: `8080/tcp -> 0.0.0.0:8080`; no token 401, wrong 401, right 200.
- `cargo test -p quantrule-runtime --test database_topology`: 9 passed.
- `make compose-check`: pass.
- `qr-governance check --all`: pass.
- Secret directories `700`, files `600`; app env only `*_FILE`; PID 1 UID
  10001; cross-database `CONNECT` denied; PostgreSQL SSL on; RabbitMQ 5671
  `amqp/ssl`; MinIO HTTPS ready.

## Learning

- The base Compose file is the local profile; it must not inherit Docker's
  all-interface default when it has no token.
- The remote script is the only place that should open an interface; a direct
  Compose run now inherits loopback, which is the safe default.

## Residual risk

- The release image signature, SBOM, and CVE report are produced only by the
  release workflow at tag time; no local scan substitutes for them.
- v2 credential sharing and the no-client-certificate stateful listeners remain
  the accepted single-operator boundary.
