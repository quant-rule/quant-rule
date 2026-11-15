# AI transparency log — DIST-105 retire the Python simulation harness

- Date: 2026-11-15 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: delete the Python simulation harness and its companion research tools
  once the Rust port and the client passed parity, keep the fixtures and the run
  folders, and make the retirement enforceable.
- Boundary: `tools/simulation/**`, `tools/charts/**`, `tools/oracle/**`,
  `tools/trades/**`, `tools/check_legacy.py`, `tools/test_check_legacy.py`,
  `.github/workflows/ci.yml` (retired steps only), current docs, `CHANGELOG.md`,
  and this log. No engine semantics, no service, gateway, shell, client, image,
  compose, Makefile, release workflow, or port-parity fixture changed. No new
  dependency, no `unsafe`, and no local data folder removed.

## Inspection before deletion

- The port-parity test reads `docs/port-parity/PORT-001.golden.json` and
  `PORT-001.datasets.json` only; no Rust test reads a Python fixture.
- `tools/simulation/fixtures/mini/` and `tools/simulation/examples/` were read
  only by the deleted Python tests and the historical execution plans.
- The Progression golden fixtures already live in
  `crates/quantrule-model/tests/fixtures/`; the simulation fixture already lives
  in `docs/port-parity/`. No fixture needed to move.
- `data/simulation/` contains only the tracked README in this worktree; the run
  folders are local data and were not touched.

## Implemented

- Deleted the four directories: 79 tracked files, 37 of them Python.
- `tools/check_legacy.py` gained a retired-tool-path scan over every current text
  file, with the history kinds, the local `data/` root, builds, and the four
  allowlisted records excluded. It exposes `check(root)` so the negative test can
  build a minimal tree.
- `tools/test_check_legacy.py` seeds each retired path, the history exclusion,
  the allowlist, and a retired label. CI runs it before the checker.
- Current docs: removed the two retired tool tutorials and their index rows,
  marked the trade-context contract Historical, updated the simulation workflow
  to the service and client records, replaced the harness and oracle links with
  the port-parity record, and recorded the retirement in the history index.
- CI no longer runs the chart, trade, or oracle steps. The legacy step now runs
  the seeded test.
- `CHANGELOG.md` records the removal under `[Unreleased] / Removed`.

## Decisions and corrections

- Proposed adding the paths to `tools/improvement/cleanup.py`. Rejected:
  `check_terms.py` scans history kinds (ADRs, audits, reports) without a history
  exclusion, so the paths would fail on records that must keep them.
  `check_legacy.py` already has the history boundary and is the right gate.
- Proposed marking the two tutorials Historical. Rejected: they are procedures
  for deleted commands, no historical document links them, and a tutorial that
  cannot run is stale. The trade-context *contract* was kept as Historical
  because historical reports link it.
- Considered allowlisting the Rust doc comments. Rejected: the comments can name
  the port and `PORT-001` without the deleted path, so the gate stays tight.
- Kept `docs/port-parity/registry.json` and `CHANGELOG.md` allowlisted because
  the port registry must name its captured source and the changelog records what
  was removed.
- Fixed one historical report's Markdown link to a deleted script; the path and
  the record text remain, only the link markers changed.

## Verification

| Command | Result |
|---|---|
| `cargo test -p quantrule-simulation` | 72 unit and 6 parity tests pass |
| `tools/test_tiers.sh parity` | `port parity ok: 1 registered port(s)` |
| seeded `docs/tmp-retired-seed.md` + `python3 tools/check_legacy.py` | exit 1, names `tools/simulation` |
| `python3 tools/check_legacy.py` | exit 0 |
| `python3 tools/check_terms.py` | exit 0 |
| `python3 tools/check_links.py` | exit 0 |
| `python3 -m unittest discover -s tools -p 'test_check_legacy.py'` | 7 tests pass |
| `tools/gate.sh` | exit 0; workspace gate passed, 1956 tests across 100 test binaries |

## Learning

- The retirement gate needs a testable seam; the original script did not expose
  `check(root)`, so a seeded test was impossible without a small refactor.
- Provenance is the one place a retired path legitimately survives. The port
  registry and the changelog need an explicit, narrow allowlist; the code
  comments did not.
- The oracle's removal ends fixture regeneration. The fixtures are now read-only
  evidence compared by the Rust tests; a later intentional fixture change has no
  shipped regeneration tool and must be recorded as a new capture.

## Refresh after #801

The branch was refreshed from `bb60d394` (`origin/main` after #801) with a clean
merge; the changelog kept every entry from both sides (109 entries). The
verification was repeated on the refreshed tree:

| Command | Result |
|---|---|
| `cargo test -p quantrule-simulation` | 72 unit and 6 parity tests pass |
| `tools/test_tiers.sh parity` | `port parity ok: 1 registered port(s)` |
| `python3 tools/check_legacy.py` | exit 0; 1771 label files, 3166 current files |
| `python3 tools/check_terms.py` | exit 0; no banned terms |
| `python3 tools/check_links.py` | 946 Markdown files; no broken relative links |
| `tools/gate.sh` | exit 0; 1979 tests across 102 test binaries |
