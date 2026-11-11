# AI Transparency Log — De-Prop-Firm the Current Surface

- Date: 2026-11-11 (UTC)
- Work item: `610-de-prop-firm`
- Issue: [#610](https://github.com/quant-rule/quant-rule/issues/610)
- Plan: [`docs/execution-plans/2026-11-11/588-tauri-platform.md`](../../execution-plans/2026-11-11/588-tauri-platform.md)
- Tool: OpenCode agent (DeepSeek V4.1 Flash)
- Branch: `arch/610-de-prop-firm`
- Claim type: ADR (Architecture Decision)

## Intent and boundary

Remove the policy/prop-firm layer from the current surface, keep portfolio
constraints, and migrate the current run to `quantrule.run/v3`. No model,
strategy, or execution signal behavior is intended to change.

## Work performed

- Changed the run schema to `quantrule.run/v3`; removed `policy` and `policy_id`
  from the loader and cross-config checks.
- Replaced `policy check` with `run check`; the command validates data, model,
  execution, and portfolio composition.
- Removed the policy config module and fixtures; updated the example and defaults
  config tree.
- Replaced the policy position cap with the portfolio sizing notional in the
  coupled and standalone execution paths.
- Removed `prop_firm_limits` from the qualification dimensions.
- Removed the policy parameters from the indicator/parameter registry and
  regenerated `docs/registry/indicators.md`.
- Updated the README, command surface, tutorials, living kata, historical policy
  contract status, and policy workflow status.
- Regenerated the affected help, analytics, and qualification golden fixtures.

## Verification

- `cargo fmt --all -- --check`: pass.
- `cargo clippy --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: pass (1,443 tests, 8 ignored in the prior release
  baseline; the run after this change passes).
- `run check` against the current example: pass.
- `python3 tools/check_command_surface.py`: pass.
- `python3 tools/check_links.py`: pass.
- `python3 tools/check_terms.py`: pass.
- `python3 tools/check_legacy.py`: pass.
- `python3 tools/check_structure.py`: pass.
- `python3 tools/check_registry.py`: 49 indicators, 150 events, 100 parameters.
- `python3 tools/check_validation.py`, verification matrix, verification, event
  emitters, partitions, file size, workflows, changelog, releases: pass.

## Learning

- The policy envelope was doing two jobs: account/objective metadata and a
  portfolio position cap. The cap now comes from the portfolio sizing mode, which
  is the actual position size; the account objective is gone.
- Removing the policy parameters from the registry changed the parameter count
  from 106 to 100; the generated registry document was rebuilt.
- The help, analytics, and qualification golden fixtures embed config hashes and
  required deliberate regeneration. The numerical strategy results were unchanged.
- Historical `run/v2` and policy records remain read-only evidence; the current
  loader rejects the removed fields.
