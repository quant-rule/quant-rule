# Tag renumbering and v1.0.0 status correction

- Date: 2026-11-15 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Intent: Renumber the release tags to a continuous `v0.1.0`–`v1.0.0` history,
  reconcile the GitHub releases with the moved tags, and correct the repository
  release status so the current tag reads `v1.0.0`.
- Boundary: Git tags and GitHub release metadata; the release index, README
  status, changelog note, and two historical release-note tag notes. No model,
  strategy, execution, portfolio, or client behavior change.

## Verification and evidence

| Check | Observed |
|---|---|
| Local tags | Exactly `v0.1.0`–`v1.0.0`; every tag is annotated and peels to the mapped commit |
| Remote tags | Exactly the same ten tags; the old `v1.1.0`, `v0.4.0-exp-baseline`, and `audit-closure-2026-10-30` refs are gone |
| Branches | Local and remote only `main`; no task worktrees remain |
| Releases | Five release pages renamed; `v1.0.0` is Latest; assets and tag targets are unchanged |
| `python3 tools/check_links.py` | 969 Markdown files; no broken relative links |
| `python3 tools/check_doc_length.py` | 35 files within cap |
| `python3 tools/check_terms.py` | 596 files; no banned terms |
| `python3 tools/check_legacy.py` | 1,808 label files and 3,236 current files; pass |
| `python3 tools/check_releases.py` | 9 release notes, 6 changelog versions; pass |
| `git diff --check` | clean |
| `tools/gate.sh` | workspace gate passed; 21 checkers, including the 970-file link check and release/changelog checks |

## Learning

The other agent's “v1.1.0 released today” status came from the stale README and
changelog status block, not from the local tags; the local tags were already
renumbered and `git describe --tags --always` reported `v1.0.0-2-g9f9cb945`.
Rewriting the released release notes and changelog headings was rejected because
released history is immutable. The mapping table plus tag annotations and a
status note preserve the old names without inventing a new history.
