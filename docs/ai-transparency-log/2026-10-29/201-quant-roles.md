# AI Transparency Log — Quant Roles

- Date: 2026-10-29 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash Free
- Work item: [#201](https://github.com/quant-rule/quant-rule/issues/201)
- Execution plan:
  [`201-quant-roles.md`](../../execution-plans/2026-10-29/201-quant-roles.md)
- Branch/worktree: `docs/202-quant-roles-agents` in
  `quant-rule--202-quant-roles-agents`
- Status: implementation in progress; verification evidence is added as it is
  observed.

## Entry: preflight and plan

- **Intent and boundary:** Add Quant Researcher, Quant Developer, and Quant
  Validator roles to the agent model, within the candles-only, rule-based
  scope. This entry covers the read-only preflight and the approved plan. No
  repository file was changed during preflight.
- **Verification and evidence:**
  - `main` was at `6f20484`, clean and in sync with `origin/main`, before the
    worktree was created.
  - No open issues or pull requests existed. The container and sub-issues were
    created as #201, #202, #203, and #204.
  - `AGENTS.md` section 10 defined nine roles; none was quant-specific. The
    role assignment rules covered engineering, security, release, and
    destructive operations only.
  - `docs/README.md` indexed document kinds but had no roles kind. `README.md`
    had setup and commands but no contributor-role pointer.
- **Learning:**
  - The existing roles already cover architecture, tests, code review, security,
    and release. The quant roles must specialize them rather than duplicate
    them, or the assignment rules become contradictory.
  - The dominant strategy failure mode is research bias, which code review and
    unit tests do not catch. The Quant Validator therefore audits evidence, not
    code style.
  - The project is rule-based and candles-only. Statistics is useful for
    evaluation and audit but must not become a signal-generation path.

## Entry: implementation (PR #202)

- **Intent and boundary:** Implement the approved role model in `AGENTS.md`.
  No production code, configuration, fixture, oracle, or CI change is made.
- **Implemented:**
  - Section 2 adds the rule-based-signals invariant; statistical, machine
    learning, and neural-network signal generation are excluded.
  - Section 4 adds the strategy-validation rule: in-sample and out-of-sample
    results, costs, and every variation tried.
  - Sections 7 and 8 add the `Quant validation` field to PR requirements and
    the PR format.
  - Section 10 defines the Quant Researcher, Quant Developer, and Quant
    Validator, maps them to the existing roles, and adds the applicability
    rules. The human-readable reference path is named; PR #203 adds the page
    and its links.
  - Sections 11 and 13 require quant roles in plans and validation evidence in
    handoff.
  - The execution plan and this log are added.
- **Verification and evidence:**
  - `cargo test`: 1,350 passed, 0 failed, 6 ignored.
  - `git diff --check`: clean.
  - `git diff --name-only`: `AGENTS.md` and the two new docs only; no Rust,
    configuration, fixture, oracle, or CI change.
  - The full `AGENTS.md` diff was read after editing; the role text, assignment
    rules, and stage hooks are consistent with the plan.
  - PR #205: `sync` passed (14s) and `verify` passed (1m33s).
- **Learning:**
  - The existing role assignment rules were written as short paragraphs, not a
    list. The quant rules were added as a short list inside that block so the
    applicability cases remain scannable.
  - Naming the role-reference path in `AGENTS.md` while the page is still in
    PR #203 avoids a broken link on `main`; the link is added when the file
    exists.
  - The statistics boundary needed to be stated in the invariant and in every
    role boundary. A single mention in one place would let a later change treat
    statistical signal generation as permitted.
