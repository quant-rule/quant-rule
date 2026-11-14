# AI transparency log — DIST-064 three-tier testing policy

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: resolve the conflict between the no-network rule and a distributed
  system, and give port parity a registry, fixture layout, and gate check.
- Boundary: policy, documentation, and repository tooling. No engine, model,
  strategy, service, or pipeline behavior changed. The composed tier is defined
  but not executed: there is no Compose stack until #643/#644. The port-parity
  registry is empty until #679 lands the first port.
- Verification: the checker self-test has 13 passing cases covering the valid
  port, the empty registry, a missing fixture, a loose tolerance, a missing
  target marker, a missing fixture reference, a duplicate ID, a fixture ID
  mismatch, path traversal, malformed JSON, a non-string status, a non-string
  fixture, and a NaN tolerance. `tools/check_port_parity.py`
  prints `port parity ok: 0 registered port(s)`. `tools/test_tiers.sh fast`
  runs the gate; `tools/test_tiers.sh parity` runs the checker;
  `tools/test_tiers.sh composed` exits 2 and names #643/#644.
- Learning: the first draft placed fixtures under `docs/port-parity/fixtures/`,
  which the structure gate rejects because every second-level docs directory
  must be a UTC date. The flat `<id>.golden.json` layout keeps the registry
  indexable and the structure rule intact.
- Rejected: running the checker over arbitrary Rust files to find undeclared
  ports. It cannot tell a port from a new computation without a marker, and a
  false positive on every file would make the gate unusable. The marker plus
  registry is the enforceable half; review rejects an unregistered port.
- Rejected: a composed-tier stub that returns success. A pass it did not run
  would be false evidence; the tier exits non-zero until the stack exists.
