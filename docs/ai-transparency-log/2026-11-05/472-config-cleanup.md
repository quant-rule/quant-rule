# AI Transparency Log — P4 Config and Command Cleanup

- Date: 2026-11-05 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: [#472](https://github.com/quant-rule/quant-rule/issues/472)
- Plan: [`472-config-cleanup.md`](../../execution-plans/2026-11-05/472-config-cleanup.md)
- Scope: config tree cleanup, help golden tests, old-schema error test, docs

## Intent and boundary

The human approved P4 by the instruction "proceed" on 2026-11-05. P4 finishes
the surface reduction started in P3: delete the retired `v5-swing` and example
strategy/analytics files, make the current/evidence/history config boundary
explicit, tighten the legacy scan, and lock the surviving help output.

Accepted model evidence is not deleted. No model, strategy, or command behavior
changes; the only executable change is the added help golden test and the
old-schema CLI test.

## Changes

- Deleted `config/v5-swing/`.
- Deleted `strategy.json` and `analytics.json` from both `config/examples/`
  runs. The current runs already omitted them.
- Added `config/README.md` and `config/model-v4/README.md`; updated
  `config/defaults/README.md` and `config/research/README.md` with the
  current/evidence/history boundary.
- Tightened `tools/check_legacy.py` to scan `config/examples` and removed stale
  pending paths.
- Added `help.txt`, `help-all.txt`, and `help-glossary.txt` golden fixtures and
  byte-equality tests.
- Added a retired `quantrule.model/v3` fixture and a `policy check` CLI test
  asserting the clean unsupported-schema error.
- Updated `README.md`, `docs/README.md`, and `docs/registry/history.md`.

## Verification and evidence

| Check | Observed result |
|---|---|
| `cargo test` | 978 passed, 0 failed, 7 ignored across 28 suites |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo fmt --all -- --check` | Pass |
| `python3 tools/check_legacy.py` | 175 current files; no retired labels |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_validation.py` | 188 claims |
| `python3 tools/check_links.py` | 542 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 325 files; no banned terms |
| Help golden | `help`, `help all`, and `help glossary` match byte for byte |
| Old model config | `policy check` reports the file, `$.schema`, and the expected v4 schema |

## Learning

- The accepted evidence directories contain historical run configs that no
  longer load. Marking them as evidence, rather than rewriting or deleting them,
  keeps the accepted baselines reproducible without pretending they are current
  configs.
- The help output was already stable, but a golden fixture makes a future
  command addition update the contract deliberately.
- The legacy scan can now scan `config/examples`; the only remaining pending
  paths are the retired strategy contract docs and the historical research
  evidence.
