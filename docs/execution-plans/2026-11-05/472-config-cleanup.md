# P4 — Config and Command Cleanup

- Date: 2026-11-05 (UTC)
- Issue: [#472](https://github.com/quant-rule/quant-rule/issues/472)
- Status: **Implemented** 2026-11-05; PR pending. Approved by the human
  instruction "proceed" on 2026-11-05
- Claim type: config and command-surface cleanup; no model or strategy behavior
  change
- Parent: [`467-clean-current-architecture.md`](467-clean-current-architecture.md)
- Predecessor: [`471-four-axis-removal.md`](471-four-axis-removal.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-05/472-config-cleanup.md`](../../ai-transparency-log/2026-11-05/472-config-cleanup.md)

## 1. Intent

Finish the surface reduction started in P3. The shell now exposes only the
current commands; the remaining config tree still contains retired example
strategy/analytics files and the old `v5-swing` programme. Remove those, make the
current-versus-evidence boundary explicit, and lock the surviving help surface
with a golden test.

Accepted model evidence is not deleted: `config/compression*`,
`config/compression-range*`, `config/model-v4`, and `config/research` remain as
historical/model evidence. They are not current run configs and are not expected
to load through the current `run.json` loader.

## 2. Current state

- P3 merged at `84ad934`; the shell surface is data commands, `fetch`,
  `report model`, `events`, `policy check`, and `help`.
- `config/examples/btcusd-2024/` and `config/examples/btcusd-2024-breakout/`
  still carry `strategy.json` and `analytics.json` with retired schemas. Their
  `run.json` files no longer reference either file.
- `config/v5-swing/` remains and is listed as `delete` in the deletion
  manifest.
- `config/compression*`, `config/model-v4`, and `config/research` remain as
  accepted or historical evidence. Their run configs may reference retired
  `strategy.json` or `analytics.json` paths; that is evidence, not a current
  run contract.
- `tools/check_legacy.py` still lists `config/examples/`, `docs/tutorials/`, and
  `docs/model/2026-10-*` as pending even though those paths are now clean or
  deleted.
- Help output is tested by content, not byte-for-byte against a fixture.

## 3. Scope

Included:

- Delete `config/v5-swing/`.
- Delete the retired `strategy.json` and `analytics.json` example files under
  `config/examples/`.
- Add a `config/README.md` that states the current, evidence, and historical
  config boundaries.
- Add note files under `config/model-v4/` and `config/research/` where the
  directory would otherwise imply it is current.
- Tighten `tools/check_legacy.py` to scan `config/examples` and remove stale
  pending paths.
- Add golden fixtures for `help`, `help all`, and `help glossary`; assert the
  binary output byte for byte.
- Add a CLI test proving a retired `quantrule.model/v3` config fails with the
  unsupported-schema message.
- Update `README.md`, `docs/README.md`, `docs/registry/history.md`, and the
  parent plan with the P4 outcome.

Excluded:

- Deleting `config/compression*`, `config/compression-range*`,
  `config/model-v4`, or `config/research`.
- Rewriting historical research or model evidence configs.
- Changing the shell command set or any model/strategy behavior.
- P5–P8.

## 4. Decisions

| ID | Decision | Status |
|---|---|---|
| D4 | Keep `config/compression*` and `config/compression-range*` as accepted M1.0 evidence | Approved; human "proceed" 2026-11-05 |
| P4-1 | Keep `config/model-v4` as the accepted M1.0 baseline evidence | Approved for this plan |
| P4-2 | Keep `config/research` as historical M2.x evidence; it is not a current run set | Approved for this plan |
| P4-3 | Delete the unused example strategy/analytics files; the current run has five config files | Approved for this plan |
| P4-4 | Help output is a golden contract; new commands must update the fixture deliberately | Approved for this plan |

## 5. Work breakdown

1. **Config deletion.** Remove `config/v5-swing/` and the four retired example
   files.
2. **Config boundary.** Add `config/README.md`; update `config/defaults/README.md`
   and add notes under `config/model-v4/` and `config/research/`.
3. **Legacy scan.** Remove stale PENDING entries and scan `config/examples`.
4. **Help golden.** Generate `tests/fixtures/help.txt`, `help-all.txt`, and
   `help-glossary.txt` from the current binary; add byte-equality tests.
5. **Old-config CLI test.** Add a fixture with a retired model schema and assert
   the `policy check` error names `$.schema` and the unsupported schema.
6. **Docs.** Update the README tree, docs index, history boundary, parent plan,
   and this plan's execution record.
7. **Verification.** Run Rust, registry, validation, link, term, and legacy
   checks.

## 6. Acceptance criteria

- `config/v5-swing/` is gone.
- The four example `strategy.json`/`analytics.json` files are gone.
- `config/README.md` names the current, evidence, and historical directories.
- `tools/check_legacy.py` passes without a `config/examples` pending entry.
- `help`, `help all`, and `help glossary` match their golden fixtures byte for
  byte.
- A retired model schema fails cleanly through `policy check`.
- No model or strategy behavior changes.
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt
  --all -- --check`, registry, validation, links, terms, and legacy checks pass.

## 7. Verification

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
python3 tools/check_legacy.py
python3 tools/check_registry.py
python3 tools/check_validation.py
python3 tools/check_links.py
python3 tools/check_terms.py
```

## 8. Risks and recovery

| Risk | Mitigation |
|---|---|
| Deleting accepted evidence | Only `config/v5-swing` and the unused example strategy/analytics files are deleted; compression and model-v4 evidence stay |
| Historical configs reference deleted example files | Historical configs are self-contained or reference evidence paths; examples do not |
| Help golden becomes stale | The fixture update is part of the command change that alters help |
| Legacy scan becomes too strict | The remaining evidence dirs are named explicitly in PENDING |
| Old config error is unclear | The CLI test asserts the file, JSON path, and unsupported schema |

Recovery: every deleted path exists at `84ad934`.

## 9. Stop conditions

- A current run example needs a deleted file.
- An accepted evidence directory would have to be deleted.
- The help golden would lock an output that the command cannot reproduce
  deterministically.
- The legacy scan flags a current file that cannot be rephrased without changing
  model meaning.

## 10. Execution record

- Deleted `config/v5-swing/` and the four example `strategy.json` /
  `analytics.json` files.
- Added `config/README.md` and `config/model-v4/README.md`; updated
  `config/defaults/README.md` and `config/research/README.md` with the
  current/evidence/history boundary.
- Tightened `tools/check_legacy.py`: `config/examples` is scanned, and the only
  pending paths are the retired strategy contract docs and the historical
  research evidence.
- Retired `analytics.relations` in the registry (`configurable: false`,
  `status: retired`) and removed the deleted analytics example from
  `tools/check_registry.py`.
- Added `help.txt`, `help-all.txt`, and `help-glossary.txt` golden fixtures and
  byte-equality tests.
- Added the retired `quantrule.model/v3` fixture and `run-old-model.json`; the
  `policy check` CLI test asserts the clean unsupported-schema error.
- Updated the README, docs index, history boundary, and parent plan.

### Verification

| Check | Observed result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo test` | 978 passed, 0 failed, 7 ignored across 28 suites |
| `python3 tools/check_legacy.py` | 175 current files; no retired labels |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_validation.py` | 188 claims for 46 indicators and 143 events |
| `python3 tools/check_links.py` | 542 Markdown files; no broken relative links |
| `python3 tools/check_terms.py` | 325 files; no banned terms |
| `help` golden | `help`, `help all`, and `help glossary` match their fixtures byte for byte |
| Old model config | `policy check` reports `model-v3.json: $.schema: expected quantrule.model/v4, found quantrule.model/v3` |

No model, strategy, or command behavior changed. The registry count is unchanged;
`analytics.relations` is retired rather than removed, so its code and history
remain documented.
