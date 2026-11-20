# AI Transparency Log — Multi-Host Deployment Profile (#959)

- Date: 2026-11-20 (UTC)
- Work item: #958 ADR, #959 plan, #960–#965 profile, #966–#970 desktop install/docs
- Tool: OpenCode agent (DeepSeek V4.1 Flash 1017)

## Intent and boundary

The operator approved a multi-host server deployment profile and the desktop
install documentation. The agent drafted the ADR, the execution plan, the issue
tree, and the implementation tracks. It did not approve the ADR, merge a pull
request, publish a release, or claim a verification result that was not run.

## Work performed

- Read the existing architecture, deployment, security, service, ADR, and
  execution-plan documents.
- Created the issue tree #958–#970 with claim types and board fields.
- Drafted the ADR and this plan from the approved conversation.
- Prepared the parallel worktree and PR map.

## Verification and evidence

- Repository preflight: clean `main` at `fc2f5d97`; no other worktrees.
- GitHub CLI authenticated as `zantgo` with project scope.
- Issues created and added to the QuantRule project.
- ADR and plan are documentation only; no test result is claimed here.
- The implementation verification is recorded in the plan's verification table
  and in each pull request.

## Learning and corrections

- The first draft treated "split host" as configuration only. The review
  corrected that: a supported multi-host profile needs a descriptor, a
  validator, a network gate, tests, and an ADR.
- RPM and DEB were rejected because the operator uses Fedora and the locked
  decision is AppImage plus a user-level desktop entry.
- The packaged Tauri app starts local sidecars, so it is not a remote client;
  the manual must direct remote users to the browser or shell.
- The remote-compute tutorial omitted `QUANTRULE_API_URL`; that is recorded as a
  documentation defect to fix in #969.
