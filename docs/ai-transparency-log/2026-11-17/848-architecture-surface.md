# AI Transparency Log — Target Architecture Surface

- Date: 2026-11-17 (UTC)
- Tool: DeepSeek V4.1 Flash 1017 in OpenCode
- Work item: Issue #848, `848-architecture-surface`
- Plan: [`docs/execution-plans/2026-11-16/842-stage-execution-surface.md`](../../execution-plans/2026-11-16/842-stage-execution-surface.md)

## Intent and boundary

Document the target stage and research architecture across the API, command
surface, services, modules, and clients, and mark the planned commands as
`Planned` without claiming they exist. The AI wrote the target architecture
document, the links from the current architecture docs, the planned command
registry rows, the README/status notes, and this log. It did not add a command,
endpoint, screen, or code path.

## Verification and evidence

- `docs/architecture/stage-surface.md` is the single target description; the
  current architecture docs link to it instead of restating it.
- The command registry marks 19 stage/research commands `Planned`; the README
  table and `help.rs` still list only the 27 implemented commands.
- `AGENTS.md` keeps the current 27-command paragraph and adds a separate target
  paragraph. The current-surface checker reads the current paragraph.
- The `client.md` screen table is unchanged; the target screens are described in
  the target document and linked from the existing paragraph.
- The checks are recorded in the pull request.

## Learning

- The architecture docs are length-capped, so the target surface belongs in one
  new reference document rather than as large additions to each capped file.
- Planned commands must be in the registry table with `Planned` status; putting
  them in the README table would make the discovery gate claim they exist.
- The service catalog now records the `stage` schema and the stage worker without
  changing the six-service boundary.
