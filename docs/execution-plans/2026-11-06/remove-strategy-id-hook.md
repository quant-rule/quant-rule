# Run Schema — Remove the Unused `strategy_id` Hook

- Date: 2026-11-06 (UTC)
- Status: Verified
- Issue: none (human-approved task slug)
- Branch: `fix/remove-strategy-id-hook`
- Worktree: `../quant-rule--remove-strategy-id-hook`
- Claim type: current run-schema tightening. No model or strategy behavior change.

## 1. Intent

The current `run/v2` loader accepts `strategy_id` even though no strategy file exists and no strategy command is reachable. Remove the hook from the current surface; S3.x reintroduces it with the strategy file and parser.

## 2. Current state

`RunFile`, `RunConfig`, `RunConfig::load`, and `validate_identities` all carry `strategy_id`, and the allowed-field list accepts it. The current examples do not use it.

## 3. Scope

Included: remove the field from the run file/config, reject it as an unknown field, update identity documentation, add the rejection test.

Excluded: adding a strategy file or parser; changing any other identity field; changing model or strategy behavior.

## 4. Decision

Remove now; reintroduce with S3.x. Keeping an identity for a missing file is a current-surface contradiction.

## 5. Acceptance criteria

- A `run.json` containing `strategy_id` fails with an unknown-field error at `$.strategy_id`.
- `policy check` passes on every current example.
- No current document lists `S` as a current identity prefix.
- Full Rust and Python gates pass.

## 6. Verification

Observed on 2026-11-06:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace --no-fail-fast` | pass; 0 failed; the new rejection test passes |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `policy check --config config/examples/btcusd-2024/run.json` | `policy ok` |
| `check_file_size.py`, `check_registry.py`, `check_validation.py`, `check_links.py`, `check_terms.py`, `check_legacy.py` | pass |
| `git diff --check` | pass |

## 7. Risks

This is a deliberate schema tightening. A current config that carries `strategy_id` will now fail; no current config does. The S3.x strategy addition reintroduces the field with its parser.
