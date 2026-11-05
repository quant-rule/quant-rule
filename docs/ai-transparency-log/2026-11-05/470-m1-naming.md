# AI Transparency Log — P2 M1.0 Naming

- Date: 2026-11-05 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: [#470](https://github.com/quant-rule/quant-rule/issues/470)
- Plan: [`470-m1-naming.md`](../../execution-plans/2026-11-05/470-m1-naming.md)
- Scope: M1.0 contract rename, config identities, retired model parsers, legacy scan

## Intent and boundary

The human approved D2 and the full plan. P2 gives the current flat model the
`M1.0` identity, adds identities to the current policy/execution/portfolio/run
configs, retires the `quantrule.model/v1–v3` parsers, and adds the legacy-token
gate. The axis ladder and strategy parsers remain for P3/D3.

No model formula, event, state, or output was changed.

## Changes

- Moved the current model contract to
  `docs/model/2026-11-05/model-m1.0.md` and updated current and historical links.
- Kept `quantrule.model/v4`; removed the v1, v2, and v3 parsers and their tests.
- Added optional `id` fields to policy, execution, and portfolio configs.
- Added `model_id`, `policy_id`, `strategy_id`, `execution_id`, and
  `portfolio_id` to `quantrule.run/v2`; the run validates the prefix.
- Migrated the example, test-fixture, and tutorial model configs from v2 to v4
  with MACD disabled to preserve the previous reading behaviour.
- Added `tools/check_legacy.py` and a CI step. The scan excludes history and
  lists the strategy/config paths that P3/P4/P6 still remove.
- Updated the docs index and the naming registry.

## Verification and evidence

| Check | Observed result |
|---|---|
| `cargo test` | Full workspace passes |
| `cargo clippy --all-targets -- -D warnings` | Pass |
| `cargo fmt --all -- --check` | Pass |
| `python3 tools/check_legacy.py` | 182 current files, no retired labels |
| `python3 tools/check_registry.py` | 46 indicators, 143 events, 110 parameters |
| `python3 tools/check_validation.py` | 188 claims |
| `python3 tools/check_links.py` | 549 Markdown files, no broken links |
| `python3 tools/check_terms.py` | 343 files, no banned terms |
| Lab summary fixture | `config_hash` updated to `c7cbbb812daf8957` after the example identities changed |

The model engine behavior is unchanged; the example and fixture model configs
were migrated to v4 with MACD disabled to preserve the previous readings.

## Learning

- The examples and the tutorial fixture still used `quantrule.model/v2`; the
  parser removal forced their migration to v4. MACD stays disabled in those
  configs to preserve the prior reading behaviour.
- The strategy parsers cannot be removed in P2 without breaking the live shell.
  Their removal belongs to P3/D3, and the legacy scan names them as pending.
- The naming registry is the one current document allowed to contain the old
  aliases, because it is the mapping itself.
