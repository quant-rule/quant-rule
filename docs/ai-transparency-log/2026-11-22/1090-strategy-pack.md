# AI Transparency Log — RA-002 Strategy Pack

- Date: 2026-11-22 (UTC)
- Work item: [#1090](https://github.com/quant-rule/quant-rule/issues/1090)
- Tool: OpenCode, DeepSeek V4.1 Flash 1017
- Plan: [`docs/execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md`](../../execution-plans/2026-11-22/1082-v1.6.0-ra-ha.md)
- Branch: `feat/1090-strategy-pack`

## Intent and boundary

Implement the pack envelope and strategy-pack schema, canonical content hash,
identity, lifecycle and validation. This entry covers RA-002 only. Portfolio and
execution packs, signal artifacts, the strategy service, run v5, the persistent
registry and client commands remain separate issues.

The implementation is in `quantrule-app`, which already owns strict config
parsing and depends on the strategy registry, data timeframes and API digest
types. `quantrule-api` was not used because it cannot depend on the engines.

## Work performed

- Added `quantrule_app::pack` with envelope, canonical JSON, identity, lifecycle,
  strategy payload and error modules.
- Shared the strategy engine, risk, timeframe and config construction helpers
  with the existing run strategy parser so registered events and bounds have one
  definition.
- Added a closed `quantrule.pack/v1` envelope for strategy, portfolio and
  execution kinds.
- Added canonical bytes, SHA-256 identity, duplicate `(name, version)` rejection
  and the draft/validated/frozen/retired lifecycle.
- Added signal/fold mode validation. Signal mode refuses giveback fields; fold
  mode keeps them.
- Added unit and public-API tests, including a golden canonical hash.
- Updated the architecture, config and changelog documents that the change made
  stale.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy -p quantrule-app --all-targets -- -D warnings` | Pass |
| `cargo test -p quantrule-app` | 226 unit, 1 isolation, 5 pack identity, 1 doctest — all pass |
| `RUSTDOCFLAGS="-D warnings" cargo doc -p quantrule-app --no-deps` | Pass |
| `tools/gate.sh` | Full gate passed; `governance check ok` |

No Quant Validator record exists yet. The human freeze decision and independent
reproduction remain RA-010, after the signal artifact and service exist.

## Learning and corrections

- Canonicalisation normalises the parsed numeric representation once at creation.
  Lexemes such as `1e2` are not preserved verbatim; the canonical bytes are
  stored and later hashed without re-serialisation. This is deterministic and is
  recorded as a design note for review.
- The pack envelope recognises portfolio and execution kinds, but their payload
  parsers do not exist yet. A caller cannot treat an unvalidated envelope as a
  pack.
- The in-memory `PackIndex` is the identity seam, not the persistent catalog
  registry. It enforces one content hash per `(name, version)` label.

## Residual risk

The persistent registry, signal computation, service boundary and human freeze
record are not implemented. No production claim is made by this entry.
