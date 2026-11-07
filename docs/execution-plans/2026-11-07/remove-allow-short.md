# Remove the `allow_short` Portfolio Rule

- Date: 2026-11-07 (UTC)
- Status: Verified; locally verified; no independent review claimed
- Issue: none (human-approved slug; no issue number is invented)
- Branch: `feat/remove-allow-short`
- Worktree: `remove-allow-short`
- Claim type: portfolio configuration and risk-rule change; no model or strategy
  formula change and no edge claim
- Approval: human instruction on 2026-11-07: “take out from the whole repo this
  rule, we always operate in both directions, we never need this rule”

## 1. Intent

QuantRule always operates in both directions. The portfolio configuration must
not carry a switch that can suppress short entries. This change removes the
`allow_short` field, the sizing rejection it produces, and every current
contract, config, test, and provenance reference to it.

## 2. Current state

- `quantrule-backtest`: `PortfolioConfig.allow_short`, a parameter on
  `SizingMode::size` and `PortfolioConfig::size_entry`, and
  `SizingRejection::ShortNotAllowed`.
- `quantrule-shell`: the `portfolio.json` parser requires `allow_short`; the
  execution stage reads it; the portfolio provenance records it.
- Current configs: `config/defaults/portfolio.json`,
  `config/examples/btcusd-2024*/portfolio.json`, and the config-run test
  fixture all carry `"allow_short": true`.
- Current docs: the policy blueprint and both README schema tables name
  `portfolio v1`.
- The S3.x stage-surface audit records `AUD-2026-11-07-03`, “allow_short: false
  does not prevent a short fill”, as a Critical finding.

## 3. Scope

### Included

- Remove `allow_short` from `PortfolioConfig`, the sizing functions, the
  `ShortNotAllowed` rejection, the shell parser, the execution stage, and the
  portfolio provenance.
- Update every current config and test fixture; the current portfolio schema
  moves to `quantrule.portfolio/v2` because a required field is removed.
- Update the current policy blueprint, README schema tables, and config index.
- Replace the short-rejection test with coverage that a short entry is sized.
- Record the human decision in the audit document: `AUD-2026-11-07-03` moves to
  `Won't fix` because the rule is removed, not because a short fill is accepted
  under the old rule.
- Add the execution plan and AI transparency log.

### Excluded

- The other audit findings. They remain `Open` and are not implemented here.
- Model, event, strategy, execution, or portfolio formulas other than the
  removal of the short gate.
- Historical work-item documents and historical research configs. They remain
  evidence of the prior contract; the config index records that they are not
  current.
- Commits, pushes, pull requests, and merges. Every Git gate is separate.

## 4. Assumptions and uncertainty

- The human owns the portfolio risk envelope and has decided that both
  directions are always available. This is a policy change, not a defect fix
  hidden inside a refactor.
- Removing a required field is a breaking configuration change. All current
  configs in the repository are updated; external configs with `allow_short` are
  rejected as unknown fields under `portfolio/v2`.
- Historical research configs under `config/research/` are evidence and are not
  loaded by the current shell. They retain the old schema and field as history.
- The sizing rule itself is side-independent; the `side` parameter was used
  only by the short gate, so it is removed from the sizing API.

## 5. Decisions

| Decision | Choice | Consequence |
|---|---|---|
| Keep a deprecated field? | No; remove it. | A config cannot suppress a side; old configs must move to v2. |
| Portfolio schema version | `quantrule.portfolio/v2` | The removed required field is an explicit contract break, not a silent v1 change. |
| Historical configs | Leave `config/research/` untouched and mark it historical. | Evidence is preserved; the current loader does not read it. |
| Audit finding 03 | `Won't fix` by human decision | The rule is removed; the finding no longer describes the product. |

## 6. Acceptance criteria

- No current production code, current config, current contract doc, or fixture
  references `allow_short` or `ShortNotAllowed`. The removal tests may name the
  removed field to prove it is rejected and absent.
- A short entry is sized by the same rule as a long entry; no side gate remains.
- Current portfolio configs use `quantrule.portfolio/v2`.
- `portfolio replay` accepts both sides and the coupled `backtest run` can size
  either bracket leg.
- The full gate passes.
- `AUD-2026-11-07-03` is `Won't fix` with the dated human decision and the plan
  link.
- Historical references are clearly historical, not current behavior.

## 7. Work breakdown

| Step | Work | Paths | Roles |
|---:|---|---|---|
| 1 | Record the plan and log | `docs/execution-plans/2026-11-07/`, `docs/ai-transparency-log/2026-11-07/` | Orchestrator |
| 2 | Remove the engine field, rejection, parameter, and tests | `crates/quantrule-backtest/src/portfolio/` | Implementation, Test |
| 3 | Remove the shell parser, execution read, provenance field, and tests | `crates/quantrule-shell/src/config/`, `crates/quantrule-shell/src/execution/`, `crates/quantrule-shell/src/portfolio/` | Implementation, Test |
| 4 | Update current configs, examples, and fixtures to v2 | `config/defaults/`, `config/examples/`, `crates/quantrule-shell/tests/fixtures/` | Implementation |
| 5 | Update current docs and the audit status | `README.md`, `docs/README.md`, `docs/config/2026-10-30/`, `docs/audits/2026-11-07/`, `config/README.md` | Documentation |
| 6 | Run the full gate and record evidence | repository | Test, Code Reviewer |

## 8. Verification

| Check | Expected |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | pass; no short-rejection test remains |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `python3 tools/check_command_surface.py` | pass |
| `python3 tools/check_registry.py` | pass |
| `python3 tools/check_validation.py` | pass |
| `python3 tools/check_verification.py` | pass |
| `python3 tools/check_verification_matrix.py` | pass |
| `python3 tools/check_event_emitters.py` | pass |
| `python3 tools/check_links.py` | pass |
| `python3 tools/check_terms.py` | pass |
| `python3 tools/check_legacy.py` | pass |
| `python3 tools/check_file_size.py` | pass |
| `python3 tools/check_structure.py` | pass |
| `git diff --check` | pass |
| `rg allow_short` over current code/config/docs | only historical records, the plan, and the audit decision |
| Short-entry unit test | a short entry sizes to the same quantity as the equivalent long entry |
| Binary `portfolio replay` with a v2 config | a short fill is accepted; no `allow_short` field exists |

## 9. Risks and recovery

- **Breaking config change.** All current configs are updated; external configs
  must move to `portfolio/v2`. The error names the unknown field.
- **Hidden short-gate test.** The only direct test is
  `short_is_rejected_when_not_allowed`; it is replaced with positive sizing
  coverage, not deleted without replacement.
- **Historical config confusion.** `config/README.md` states that the research
  configs retain the old field as history and are not loaded.
- **Behavior change.** Allowing shorts where the old config refused them is the
  intended change. The branch is the rollback; no history is rewritten.

## 10. Stop conditions

- A current config cannot be updated without changing an unrelated contract.
- A test or tool depends on the short rejection beyond the one identified test.
- The schema version decision requires a human choice not covered by this plan.
- A gate fails for a reason outside this scope.
- A Git operation would be required without explicit authorization.

## 11. Verification record

Observed in this worktree on 2026-11-07:

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | 1,312 passed, 0 failed, 8 ignored |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | pass |
| `python3 tools/check_command_surface.py` | pass; 15 implemented, 0 planned |
| `python3 tools/check_registry.py` | pass; 46 indicators, 143 events, 101 parameters |
| `python3 tools/check_validation.py` | pass; 189 claims |
| `python3 tools/check_verification.py` | pass; row=1091 family=329 tool=332 n/a=327 |
| `python3 tools/check_verification_matrix.py` | pass |
| `python3 tools/check_event_emitters.py` | pass; 143 codes |
| `python3 tools/check_links.py` | pass; 601 Markdown files |
| `python3 tools/check_terms.py` | pass; 364 files |
| `python3 tools/check_legacy.py` | pass; 703 current files |
| `python3 tools/check_file_size.py` | pass |
| `python3 tools/check_structure.py` | pass |
| `git diff --check` | pass |
| `portfolio_rejects_the_removed_allow_short_field` | pass; `allow_short` is an unknown field under `portfolio/v2` |
| `replay_accepts_a_short_fill` | pass; `trades.csv` first trade is `side=short`, and the portfolio provenance has no `allow_short` |
| `rg allow_short` over current production code, configs, and contracts | only the removal guards, historical records, the plan, and the audit decision |

The analytics and qualification golden fixtures were regenerated deliberately
because the portfolio config bytes changed the run config hash, the backtest
manifest hash, and the stage provenance hashes. The numeric results and table
rows are unchanged; only the identities moved.

The current production code and current configs no longer reference `allow_short`
or `ShortNotAllowed`. The removal tests name the field only to prove that it is
rejected and absent from the provenance. The historical research configs under
`config/research/` retain the old schema and field as evidence and are not
loaded by the current shell; `config/README.md` records that boundary.
Historical execution plans, reports, and AI logs retain the term as history and
were not rewritten.

No commit, push, pull request, or merge was performed.
