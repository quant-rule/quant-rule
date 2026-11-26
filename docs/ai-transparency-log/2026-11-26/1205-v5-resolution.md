# AI Transparency — Run v5 Reference Resolution

- Date: 2026-11-26 (UTC)
- Work item: `REM-RA-002` ([#1205](https://github.com/quant-rule/quant-rule/issues/1205))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim; no model, strategy, signal, or edge claim.

## 1. Intent and boundary

The issue asks `run submit` to resolve a v5 reference set before a run row
exists. The implementation adds the reference carriage, the data-service
producer, and the evidence submission guard. It does not write lineage (RA-003)
or change worker consumption (RA-004).

## 2. Implemented

- `ConfigBundle` carries an ordered `references` list with `serde(default)` and
  an empty legacy shape.
- `RunConfig::load_v5_references` validates the v5 document, data and model
  files, and id/hash pairs, and returns the config hash plus references without
  reading a pack, signal, or model body.
- The data service accepts a v5 config structurally and its internal bundle
  route resolves pack and signal objects by hash, carrying pack provenance.
- Evidence normalizes the reference set at submission; a v5 model artifact must
  match the submitted model artifact id. A missing or mismatched model id is
  refused before a run row. A v5 config with no model reference refuses a
  promoted model id, so lineage cannot miss a cited model.
- A signal reference requires the artifact catalog; a missing catalog is an
  internal error, not a silent skip.
- v3/v4 bundles keep an empty reference list and their existing loader path.

## 3. Verification

Commands actually run in the RA-002 worktree:

- `cargo test -p quantrule-service-api` → 49 passed.
- `cargo test -p quantrule-app` (v5 structural test) → passed.
- `cargo test -p quantrule-data-service configs::scan` → 5 passed.
- `cargo test -p quantrule-data-service --test configs` → 9 passed.
- `cargo test -p quantrule-evidence-service --test promoted_runs a_v5_bundle` →
  3 passed, including the uncited-model regression.
- `cargo test -p quantrule-data-service -p quantrule-evidence-service` → all
  non-ignored tests passed.
- `cargo test -p quantrule-service-api -p quantrule-app -p quantrule-data-service
  -p quantrule-evidence-service` was run before the fixture-count fix; the only
  failures were the three stale fixture counts, now fixed and rerun green.

No composed, lab, soak, upgrade, or release test was run. The data-service route
has no positive HTTP test with registered packs yet; the scan, structural, and
submission paths are covered, and the worker/lineage path remains RA-003/RA-004.

## 4. Learning

- Adding a field to `ConfigBundle` touches many test literals; the compiler is
  the reliable checklist, not a blind text replace. The first insertion script
  split `config_id:` from its value and was reverted.
- The v5 config hash can be computed without pack bytes: it covers the run,
  data and model files plus the five reference hashes. That lets the data
  service accept a v5 config before any pack is resolved.
- The data service is the right owner for pack and signal byte verification; the
  evidence service validates the model-stage reference and the immutable set.
- The v5 fixture changes the mounted-config count; every count assertion had to
  move together.
- Self-review found two fail-open edges after the first push: a missing artifact
  catalog skipped signal verification, and a v5 config without a model reference
  accepted a promoted model id. Both are now closed and tested.

## 5. Next action

`REM-RA-003` writes the lineage rows in the same evidence transaction as the run
row. `REM-RA-004` resolves once in the worker and removes the legacy strategy
file requirement. Push, pull request, merge, and release remain gated.
