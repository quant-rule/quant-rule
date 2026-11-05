# P2 — M1.0 Naming and Config Identities

- Date: 2026-11-05 (UTC)
- Issue: [#470](https://github.com/quant-rule/quant-rule/issues/470)
- Status: **Implemented** 2026-11-05; PR pending. D2 approved; strategy-parser
  removal deferred to P3/D3.
- Claim type: naming and schema identity; no model behavior change
- Parent: [`467-clean-current-architecture.md`](467-clean-current-architecture.md)
- AI transparency log: [`docs/ai-transparency-log/2026-11-05/470-m1-naming.md`](../../ai-transparency-log/2026-11-05/470-m1-naming.md)

## 1. Intent

Give the current flat model its approved `M1.0` identity, give the current
policy/execution/portfolio/run configs stable identities, and remove the old
model schema parsers. The product must read one current model config
(`quantrule.model/v4`) and one measurement spec (`quantrule.model-spec/v2`).
Old configs fail with a clear unsupported-schema message.

## 2. Current state

- The current model contract is `docs/model/2026-11-04/model-v4.md`.
- `quantrule.model/v4` is the current flat model config; `model/v1–v3` are old
  parsers.
- `quantrule.model-spec/v2` already carries `id` and `family`.
- Policy, execution, portfolio, and run configs have no identity fields.
- Current examples use `model/v2` and `strategy/v3`/`v6`; P4 rewrites the
  examples for the surviving surface.
- Strategy parsers `v2–v8` and the axis ladder remain until P3/D3. Removing them
  now would break the current shell before the cutover PR exists.

## 3. Scope

Included:

- Rename the current model contract to `docs/model/2026-11-05/model-m1.0.md`.
- Update every current link to the M1.0 contract.
- Keep `quantrule.model/v4`; delete `model/v1–v3` parsers and their tests.
- Accept an optional `id` on policy, execution, and portfolio configs.
- Add `model_id`, `policy_id`, `strategy_id`, `execution_id`, and
  `portfolio_id` to `quantrule.run/v2`; store them on the resolved run.
- Add identities to the current defaults and examples.
- Add a legacy-token scan with a documented pending allowlist for the strategy
  paths that P3/P4 remove.
- Update the docs index so no current row calls the model “v4”.

Excluded:

- Deleting the axis ladder, old shell commands, or strategy parsers (P3/D3).
- Rewriting the examples to the surviving strategy surface (P4).
- Changing a model formula, event, state, or output.
- Renaming the technical schema strings.

## 4. Decisions

| ID | Decision | Status |
|---|---|---|
| D2 | Keep `quantrule.model/v4` as the current flat model config; delete v1–v3 | Approved |
| P2-1 | Model identity lives in the run manifest and `model-spec/v2`, not in the reading config | Approved for this plan |
| P2-2 | Strategy parser removal moves to P3/D3 because the axis ladder is still live | Approved for this plan |
| P2-3 | IDs are optional on the sub-config parsers so historical evidence still parses; current defaults and examples carry them | Approved for this plan |

## 5. Work breakdown

1. **Rename the contract.** Move `model-v4.md` to `model-m1.0.md`, add the
   `M1.0` identity and status, and update every current link.
2. **Model parser.** Keep `quantrule.model/v4`; remove the v1, v2, and v3
   dispatch and parser functions. Keep the shared readings helper used by v4.
3. **Config identities.** Accept `id` on policy/execution/portfolio; add the
   five identity fields to `quantrule.run/v2` and `RunConfig`.
4. **Current configs.** Add `POL1.0`, `E1.0`, `P1.0`, and the run identity
   fields to `config/defaults` and `config/examples`.
5. **Legacy scan.** Add `tools/check_legacy.py`; it fails on old model/strategy
   labels outside the history allowlist. The pending allowlist names the
   strategy paths that P3/P4 remove.
6. **Docs index.** Update `docs/README.md`,
   `docs/model/2026-11-05/model-families.md`, and
   `docs/registry/history.md` to point at M1.0.
7. **Verification.** Run Rust, registry, validation, link, term, and legacy
   checks.

## 6. Acceptance criteria

- `docs/model/2026-11-05/model-m1.0.md` exists and every current link resolves.
- `quantrule.model/v1`, `/v2`, and `/v3` are rejected with a clear unsupported
  schema message; `/v4` parses.
- Policy, execution, portfolio, and run configs accept their identity fields.
- Current defaults and examples carry the approved identities.
- The legacy scan passes with the documented pending allowlist.
- `docs/README.md` calls the current model `M1.0`.
- No model formula, event, state, or output changes.

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
| Deleting the current model parser | D2 keeps `model/v4`; only v1–v3 go |
| Old evidence configs fail | They are historical; the error is explicit, and Git keeps them |
| Strategy parsers still present | P3/D3 removes them; the legacy scan lists them as pending |
| Identity fields ignored by engines | They are recorded on the run and the configs; no engine behavior depends on them |
| Broken links after the rename | `check_links.py` is the gate |

Recovery: every path exists at `a9110b1`; a deleted parser or contract is
recoverable from that commit.

## 9. Stop conditions

- A current config, test, or command would fail without an approved replacement.
- A model behavior change is required.
- The legacy scan cannot distinguish history from the current surface.
- The strategy removal becomes necessary before P3; stop and re-plan the split.

## 10. P2 execution record

- 2026-11-05: the M1.0 contract moved to
  `docs/model/2026-11-05/model-m1.0.md`; current and historical links updated.
- `quantrule.model/v1–v3` parsers and tests removed; `model/v4` remains.
- Policy, execution, portfolio, and run configs accept identities; the run
  validates the M/S/POL/P/E prefixes.
- The example, fixture, and tutorial model configs moved from `model/v2` to
  `model/v4` with MACD disabled to preserve the prior readings.
- The lab summary provenance fixture hash moved to `c7cbbb812daf8957` after the
  example config identities changed.
- `cargo test`: full workspace passes.
- `cargo clippy --all-targets -- -D warnings`: pass.
- `cargo fmt --all -- --check`: pass.
- `python3 tools/check_legacy.py`: 182 current files, no retired labels.
- `python3 tools/check_registry.py`: 46 indicators, 143 events, 110 parameters.
- `python3 tools/check_validation.py`: 188 claims.
- `python3 tools/check_links.py`: 549 Markdown files, no broken links.
- `python3 tools/check_terms.py`: 343 files, no banned terms.
- No model engine formula, event, state, or output changed.
