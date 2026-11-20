# AI Transparency Log — Runtime History Design

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: #938
- Claim type: ADR (Architecture Decision)

## Intent and boundary

Define the runtime-history contract before any destructive endpoint: owners,
states, tombstones, API paths, client use cases, protection rules, migrations, and
security behavior. Repository evidence stays in `qr-governance history`; the
design does not add a GUI filesystem-admin service.

## Verification and evidence

| Command | Result |
|---|---|
| `python3 tools/check_doc_length.py` | Architecture document within cap |
| `python3 tools/check_links.py` | No broken links |
| `python3 tools/check_terms.py` | No banned terms |

## Learning

- Dataset erase cannot be made safe without a cross-service reference registry.
  The design offers dataset archive only; erase stays deferred.
- Stages already have a delete path; the history command delegates to it rather
  than adding a second erase mechanism.
- The gateway already forwards the owning prefixes; only `/runs/{id}/archive`
  needs a new route.
- A tombstone per service avoids cross-schema reads while preserving an audit
  record after deletion.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Common cross-service `/history` endpoint | Rejected; it would make one service aggregate another's schema |
| Hard-delete datasets in v1 | Rejected; references cross services |
| Add a seventh filesystem-admin service | Rejected without a separate ADR |
| GUI for repository evidence | Rejected; source control stays in the governance CLI |
