# Quant Roles: Researcher, Developer, Validator

- Date: 2026-10-29 (UTC)
- Status: **Approved — 2026-10-29.** The human authorized the full implementation
  with "proceed with the full implementation". Implementation is in progress;
  verification evidence is recorded as it is observed.
- Container: [#201](https://github.com/quant-rule/quant-rule/issues/201)
- Sub-issues: [#202](https://github.com/quant-rule/quant-rule/issues/202) (AGENTS.md),
  [#203](https://github.com/quant-rule/quant-rule/issues/203) (role reference),
  [#204](https://github.com/quant-rule/quant-rule/issues/204) (PR checklist)
- Baseline: `main` at `6f20484`
- Branch/worktree: `docs/202-quant-roles-agents` in
  `quant-rule--202-quant-roles-agents`
- Transparency log:
  [`docs/ai-transparency-log/2026-10-29/201-quant-roles.md`](../../ai-transparency-log/2026-10-29/201-quant-roles.md)

## Intent

Make research, implementation, and independent validation distinct
responsibilities in the agent model. The project remains candles-only and
rule-based: hypotheses are deterministic rules over OHLCV, and statistical,
machine-learning, and neural-network signal generation are out of scope.
Statistics is allowed only to evaluate and validate rule-based signals.

The roles are hats for agents, not new headcount. They specialize the existing
roles; they do not replace the Software Architect, Test Engineer, Code Reviewer,
Security and Privacy Reviewer, or Release / Operations Verifier. The human
remains the final authority.

Affected users: maintainers who change the model or a strategy, reviewers who
judge a strategy result, and agents that need an explicit handoff between
research, implementation, and validation.

## Current state

- `AGENTS.md` section 10 defines nine roles; none is quant-specific.
- The role assignment rules cover engineering, security, release, and
  destructive operations only.
- No `docs/roles/` kind exists. `docs/README.md` indexes document kinds.
  `README.md` has setup and commands but no contributor-role pointer.
- No open issues or pull requests existed before this program. `main` is at
  `6f20484`.
- Historical plans and transparency logs are point-in-time records and are not
  rewritten.

## Scope

Included:

1. `AGENTS.md` section 2: add the rule-based-signals invariant.
2. `AGENTS.md` section 4: add the strategy-validation rule.
3. `AGENTS.md` section 7: add quant validation to the pull request requirements.
4. `AGENTS.md` section 8: add a quant-validation line to the pull request
   format when applicable.
5. `AGENTS.md` section 10: define the Quant Researcher, Quant Developer, and
   Quant Validator; map them to the existing roles; extend the role assignment
   rules; name the human-readable reference. PR #203 adds the page and its
   links.
6. `AGENTS.md` section 11: require quant roles in the work breakdown and
   validation evidence in verification.
7. `AGENTS.md` section 13: add the researcher, developer, and validator to
   stages 2–5.
8. A plain-language role reference at
   `docs/roles/2026-10-29/quant-roles.md`.
9. Links from `docs/README.md` and `README.md`.
10. An optional pull request template with a quant-validation section.
11. This plan and its AI transparency log.

Excluded:

- No Rust, model, strategy, configuration, fixture, oracle, or CI change.
- No statistical, machine-learning, or neural-network signal generation.
- No rewrite of historical plans, logs, or release notes.
- No user-guide/developer-guide split; that remains a separate decision.
- No claim that these roles make a strategy profitable.

## Assumptions and uncertainty

- The three roles are procedural hats for agents. They do not create independent
  agents or independent model judgments.
- "No statistical systems" forbids statistical signal generation, not the
  statistics used to evaluate and audit rule-based signals. If the human meant
  to forbid all statistics, the Quant Validator cannot perform out-of-sample
  testing and the plan must stop.
- "Docs everywhere" means the normative `AGENTS.md`, the docs index, the README,
  and the role reference. It does not mean rewriting historical work items.
- The PR template is optional and is tracked as sub-issue #204.

## Decisions

| ID | Decision | Status |
|---|---|---|
| D1 | Quant roles specialize existing roles; they do not replace them. | Approved |
| D2 | Statistics is allowed for evaluation and audit, forbidden for signal generation. | Approved |
| D3 | The role reference lives at `docs/roles/2026-10-29/quant-roles.md`. | Approved |
| D4 | Add the rule-based-signals invariant to section 2. | Approved |
| D5 | One container plan and log, with per-PR evidence sections. | Approved |
| D6 | Add the optional PR template as sub-issue #204. | Approved |
| D7 | The Quant Validator records an advisory verdict; the human decides. | Approved |

## Proposed role definitions

### Quant Researcher Agent

Purpose: Turn a market question into a testable, rule-based hypothesis and a
pre-registered experiment.

Responsibilities:

- State the hypothesis as deterministic rules over OHLCV candles and the
  configured timeframe ladder.
- Define metrics, sample splits, cost assumptions, and pass/fail criteria before
  running the experiment.
- Use out-of-sample or walk-forward evaluation; record every variation
  attempted.
- Interpret performance and limitations, separating in-sample from
  out-of-sample evidence.
- Hand the approved hypothesis and protocol to the Quant Developer.

Boundaries:

- No production code, no post-hoc threshold changes, no self-approval.
- No statistical, machine-learning, or neural-network signal generation;
  statistics is limited to evaluation.
- No expansion beyond candles-only, rule-based signals.
- No profitability claim from a single backtest.

### Quant Developer Agent

Purpose: Implement approved rule-based models and trading logic correctly,
deterministically, and testably.

Responsibilities:

- Implement deterministic, causal rules over OHLCV; no future data.
- Conform to the model contract, configuration schemas, and numeric rules.
- Write unit, causality, determinism, golden, and edge-case tests.
- Ensure performance and reproducibility; record data version and configuration.
- Report deviations from the approved hypothesis.

Boundaries:

- No changing the hypothesis, thresholds, or evaluation criteria to improve
  results.
- No statistical, machine-learning, or neural-network signal generation.
- No self-approval or "validated" claim.
- No weakening tests to obtain a passing result.

### Quant Validator Agent

Purpose: Independently audit and reproduce results, and decide whether the
evidence supports the conclusion.

Responsibilities:

- Reproduce from the recorded data version and configuration.
- Check look-ahead bias, survivorship bias, data snooping, split leakage, and
  unrealistic costs.
- Verify fees, slippage, fills, risk controls, drawdown, exposure, and capacity.
- Challenge the conclusion, require out-of-sample evidence, and report residual
  risk.
- Record commands, observed results, and limitations in a validation record.

Boundaries:

- Read-only by default; does not modify the work it reviews.
- No self-validation, no waiving controls, no approving a trade-off.
- No changing evaluation criteria to turn a failure into a pass.
- No statistical, machine-learning, or neural-network signal generation;
  statistics serves audit only.

### Role assignment rules

- Quant Researcher is required for a new strategy hypothesis, a new evaluation
  metric, or a claim about model behavior.
- Quant Developer is required for any change to model or strategy
  implementation.
- Quant Validator is required before a strategy result is called promising,
  before a model or strategy behavior change is merged, and before a release
  that changes signal generation.
- Docs-only, tooling-only, and refactors that do not change model or strategy
  output do not require quant roles.
- Quant roles specialize existing roles; they do not replace them.
- If one agent performs several roles, the orchestrator discloses the sequential
  passes; no claim of independent review is made.
- The human remains the final authority; a validator verdict is evidence, not
  approval.

## Acceptance criteria

1. `AGENTS.md` section 10 defines all three roles with purpose,
   responsibilities, boundaries, and scope.
2. The role assignment rules say when each role is required, that they
   specialize the existing roles, and that the human decides.
3. Candles-only, rule-based scope is explicit; statistical, machine-learning,
   and neural-network signal generation are excluded.
4. `AGENTS.md` section 4 states the out-of-sample validation rule for strategy
   claims.
5. `docs/roles/2026-10-29/quant-roles.md` exists, is linked from
   `docs/README.md` and `README.md`, and names `AGENTS.md` as normative.
6. `AGENTS.md` sections 7, 8, 11, and 13 capture role assignment and validation
   evidence.
7. No production Rust code, configuration, fixture, oracle, or CI behavior
   changes.
8. `git diff --check`, link checks, `cargo test`, and CI pass.
9. The container and sub-issues reach Done on the board; worktrees and branches
   are cleaned up.

## Work breakdown

| Step | Work | Roles | Paths |
|---|---|---|---|
| 1 | Write this plan and the transparency log; create the issues; move #202 to In Progress | Orchestrator | plan, log, board |
| 2 | Implement the AGENTS.md changes; update the plan/log; verify; commit; open PR #202 | Quant Developer, Code Reviewer | `AGENTS.md` |
| 3 | After #202 merges, publish the role reference and links; commit; open PR #203 | Quant Developer, Code Reviewer | `docs/roles/`, `docs/README.md`, `README.md` |
| 4 | After #202 merges, add the PR template; commit; open PR #204 | Quant Developer | `.github/pull_request_template.md` |
| 5 | Verify the container criteria; move the container to Done; clean up | Orchestrator | board, worktrees |

Only one work item is In Progress at a time unless parallel work is explicitly
authorized. The default is sequential.

## Verification

| Check | Expected evidence |
|---|---|
| `git diff --check` | No whitespace errors |
| Relative links | Every link from `AGENTS.md`, `docs/README.md`, and `README.md` resolves |
| `cargo test` | 1,350 passed, 0 failed, 6 ignored; proves no accidental code change |
| CI | `sync` and `verify` pass on every PR |
| Manual read | Role boundaries, handoffs, and precedence are unambiguous |
| Board | Container and sub-issues reach Done; PRs merged |

`cargo fmt` and `cargo clippy` are not run locally for docs-only PRs because no
Rust file changes; CI still runs them. No new dependency is added for link
checking.

### Observed evidence (2026-10-29, local)

| Check | Observed result |
|---|---|
| `git diff --check` | Clean. |
| `cargo test` | 1,350 passed, 0 failed, 6 ignored. |
| Changed paths | Only `AGENTS.md`, this plan, and the transparency log; no Rust, configuration, fixture, oracle, or CI change. |
| Manual review | Role boundaries, specialization, and applicability rules read consistently; PR #203 adds the role reference and its links. |
| Remote CI | PR #205: `sync` passed (14s), `verify` passed (1m33s). |

## Risks and recovery

| Risk | Mitigation |
|---|---|
| Role overlap and ambiguity | Explicit specialization and precedence rules |
| Process bloat | Applicability matrix; docs-only changes skip quant roles |
| Drift between `AGENTS.md` and the docs page | `AGENTS.md` is normative; the page explains and links |
| False sense of independent review | Disclosure rule for sequential passes |
| Scope creep into ML or statistical signals | Explicit invariant and role boundaries |
| Historical docs become inconsistent | Do not rewrite them; they predate the roles |

Recovery is simple: these are documentation changes. A faulty PR can be reverted
without runtime impact.

## Stop conditions

Stop and request a decision if:

- The human wants the quant roles to replace existing roles rather than
  specialize them.
- The statistics boundary is meant to forbid evaluation statistics as well as
  signal generation.
- The role reference must live somewhere other than `docs/roles/`.
- Adding a PR template is out of scope.
- A sub-issue requires code, configuration, fixture, or model changes.
- The docs layout rule in `AGENTS.md` section 5 must change.
