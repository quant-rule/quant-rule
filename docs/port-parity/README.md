# Port Parity

- Date: 2026-11-14 (UTC)
- Status: Current; one port registered (PORT-001)
- Owner: Quant Validator and Code Reviewer
- Contract: [`AGENTS.md`](../../AGENTS.md) section 4
- ADR: [`13-three-tier-testing.md`](../adr/2026-11-14/13-three-tier-testing.md)
- Checker: [`tools/check_port_parity.py`](../../tools/check_port_parity.py)

## 1. Purpose

A **port** is a computation moved from a retired implementation into Rust. The
new code can be clean, deterministic, and fully tested against itself while its
numbers drift from the retired behavior. A port is therefore not complete until
its output matches a fixture captured from the retired implementation.

The rule is one sentence: every numeric value matches to `1e-9`, and the
evidence is registered here. This directory is the registry, the fixture layout,
and the record the gate checks.

## 2. Tiers

| Tier | Evidence | Command | When |
|---|---|---|---|
| Fast — unit and integration | In-process transport fake; no socket, no live market, no clock or ordering dependence | `tools/gate.sh` | Every pull request |
| Composed | The real Compose stack: services, Postgres, MinIO, RabbitMQ | `tools/test_tiers.sh composed` | Opt-in; required before release |
| Port parity | A golden fixture from the retired implementation, matched to `1e-9` | `tools/test_tiers.sh parity` | Registry check every pull request; comparison when a port is registered, and before release |

The full policy is ADR-13. The registry check is part of every gate; the composed
suite is opt-in, and the numerical comparison is required when a port is
registered and before a release. `tools/test_tiers.sh parity` validates the
registry; the comparison itself is the Rust test named in the entry, and it runs
with the crate's test suite. The composed tier needs the stack and suite from
#643/#644; until they land the command exits non-zero and names the missing
prerequisite.

## 3. Layout

```text
docs/port-parity/
  README.md              this guide
  registry.json          every registered port
  PORT-001.golden.json   one fixture per registered port
```

The layout is flat because `docs/<kind>/<date>/` is the rule for dated work
items and a `fixtures/` subdirectory would break the second-level directory
check. A port with a large fixture uses one JSON file with many cases.

## 4. Registry entry

```json
{
  "schema": "quantrule.port-parity/v1",
  "ports": [
    {
      "id": "PORT-001",
      "source": "tools/retired.py at commit 0123456",
      "target": "crates/quantrule-example/src/ported.rs",
      "test": "crates/quantrule-example/tests/port_parity.rs",
      "fixture": "docs/port-parity/PORT-001.golden.json",
      "status": "active"
    }
  ]
}
```

| Field | Meaning |
|---|---|
| `id` | Stable `PORT-NNN` identifier, assigned once and never reused |
| `source` | The retired implementation and the commit or date it was captured from |
| `target` | The Rust source file that carries the ported computation |
| `test` | The Rust test that reads the fixture and compares every case |
| `fixture` | Must be `docs/port-parity/<id>.golden.json` |
| `status` | `active` or `retired` |
| `notes` | Optional context |

An empty `ports` list is valid: no port means no parity claim.

## 5. Fixture

```json
{
  "schema": "quantrule.port-parity-fixture/v1",
  "id": "PORT-001",
  "source": "tools/retired.py at commit 0123456",
  "captured": "2026-11-14",
  "tolerance": 1e-09,
  "cases": [
    {
      "name": "opening-range-sweep",
      "input": {"candles": [], "parameters": {}},
      "expected": {"events": []}
    }
  ]
}
```

| Field | Meaning |
|---|---|
| `id` | Must match the registry entry |
| `source` | Must match the registry entry |
| `captured` | UTC date the retired implementation produced the values |
| `tolerance` | Absolute tolerance; must be at most `1e-9` |
| `cases` | Non-empty list; each case has `name`, `input`, and `expected` |

The checker validates the envelope. The Rust test named in the registry owns the
comparison: it reads the fixture, runs the ported computation on `input`, and
asserts every numeric value in `expected` to `tolerance`.

## 6. Markers

The target file names its registry entry so an unregistered port is visible in
review:

```rust
// port-parity: PORT-001
```

The comparison test names the entry and the fixture it reads:

```rust
// port-parity test: PORT-001
const FIXTURE: &str = "docs/port-parity/PORT-001.golden.json";
```

## 7. Adding a port

1. Capture the fixture from the retired implementation and record the source and
   date.
2. Write the Rust target and the comparison test.
3. Add the marker to the target and the test marker plus fixture path to the
   test.
4. Add the registry entry.
5. Run `tools/test_tiers.sh parity` and `cargo test`.

The Quant Validator reproduces the comparison before the port is called
complete. The checker proves the evidence exists; it does not replace the
Validator's independent read.

## 8. Enforcement

`tools/check_port_parity.py` runs in `tools/gate.sh` and fails when:

- the registry is missing, malformed, or has an unknown field;
- an entry has a missing or duplicate ID, a missing target, test, or fixture;
- the target lacks `// port-parity: <id>`;
- the test lacks `// port-parity test: <id>` or does not name the fixture;
- the fixture is missing, has the wrong schema or ID, has a source that differs
  from the registry, or has no cases;
- the tolerance is looser than `1e-9`.

A port without a registry entry is a review defect. The marker is how the
reviewer finds it; the registry is how the gate checks it.

## 9. Current state and limits

- `PORT-001` registers the simulation domain from DIST-101 (#679). The meta stage
  is compared in full; the window, marks, oracle, review, check, and exit cases
  compare the reductions that the checked-in mini bundle can exercise. The
  fixture notes name each omission, and the historical SIM-01..SIM-10 runs cannot
  be replayed because their external data root is not in the repository.
- The checker proves the fixture, marker, and test exist. It does not execute
  Rust or compare values; the test does that.
- The composed tier is defined but not executable until #643/#644. This directory
  records the policy, not a composed run.
