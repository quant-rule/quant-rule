# AI Transparency Log — Event Identity

- Date: 2026-11-05 (UTC)
- Tool: OpenCode / DeepSeek V4.1 Flash
- Work item: event identity remediation (no issue number)
- Branch: `fix/event-identity`

## Intent and boundary

The audit found that 143 registered event IDs collapsed into 106
`(type, subtype, source)` tuples. The spec and the event CSV therefore could not
select or count two registered events that shared a tuple. This change adds a
stable registry `code` to every event, publishes it in `EventRecord` and
`events.csv`, and makes spec filtering, conditions, prediction, and dedupe key
by that code. It also implements the missing `signal.extreme_stale`,
`signal.ema_stack_change`, and `signal.rsi_zone_change` events, and enables the
compression detectors when a v2 spec lists a compression event.

Boundary: no threshold, reading formula, or strategy rule is changed. The
registry retains 143 events and 46 indicators. The store schema is not changed in
this pass; stored datasets do not yet persist the event code.

## Verification and evidence

- `cargo test --workspace` — all tests pass after the change.
- `cargo fmt --check` — pass.
- `cargo clippy --workspace --all-targets -- -D warnings` — pass.
- `python3 tools/check_registry.py` — `registry ok: 46 indicators, 143 events, 110 parameters`.
- `python3 tools/check_links.py`, `python3 tools/check_terms.py` — pass.
- `python3 tools/research/coverage.py` — 15 cells, 46 indicators, 143 events.
- `python3 tools/research/spec_preflight.py` on the fixture — pass.
- New regression tests: same tuple/different code remains two votes; a level
  condition matches a non-EMA20 source; `signal.extreme_stale` is emitted.

The M2 development result is a historical pre-change record. This change alters
event-layer output and requires a fresh run and Quant Validator review before any
new M2 result is treated as evidence.

## Learning

The root cause was treating `(type, subtype, source)` as the event identity. The
family fields describe the detector, not the registered event. The event code is
the stable identity; the subtype remains a compact family code. The event CSV is
the raw run surface and now carries the code. The report JSON and the SQLite store
still omit it and are a follow-up.
