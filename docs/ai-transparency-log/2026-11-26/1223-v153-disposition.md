# AI Transparency — v1.5.3 Disposition and V01 Closure

- Date: 2026-11-26 (UTC)
- Work item: `REM-GOV-001` ([#1223](https://github.com/quant-rule/quant-rule/issues/1223))
- Tool: OpenCode coding agent; model identifier reported by the harness is
  `DeepSeek V4.1 Flash 1017`.
- Claim type: Behaviour Claim; governance record only, no model, strategy, edge,
  or release claim.

## 1. Intent and boundary

Close V01 of the v1.6.0 plan by recording the human-approved disposition of
`v1.5.3`. The approved remediation decision D2 selects `Superseded`: the merged
content shipped in `v1.6.0`, and no tag is created retroactively. This work
edits plan records only. It does not create, move, or delete a tag and does not
change code.

## 2. Implemented

- The old plan's V01 row now records the `Superseded` disposition in `Observed`
  and `Notes`, with the tag checks, the D2 reference, and the date.
- The acceptance criterion and closure section no longer require a `v1.5.3` tag.
- The release-train table marks `v1.5.3` `Superseded` and states that no tag was
  created and its content shipped in `v1.6.0`.
- The remediation plan's V-GOV-01 row records the observed evidence and `Pass`.

## 3. Verification and evidence

Commands actually run on 2026-11-26:

- `git tag -l 'v1.5.3'` → empty.
- `git ls-remote --tags origin 'v1.5.3'` → empty.
- `git tag -l 'v1.5.*'` → `v1.5.0`, `v1.5.1`, `v1.5.2` only.
- `docs/releases/README.md` → current release tag `v1.6.0` (2026-11-26).
- `docs/releases/2026-11-26/v1.6.0.md` → previous release `v1.5.2`.
- `cargo run --quiet -p quantrule-governance -- check --all` and
  `tools/gate.sh --changed main` → recorded in the pull request.

No tag was created or moved. No composed, HA, soak, signature, or release run
was performed.

## 4. Learning

- V01 was pending because it required a tag; the correct resolution is a
  recorded disposition, not a retroactive tag after `v1.6.0`.
- The old plan contained three places that assumed the tag would exist: the
  acceptance criterion, the release train, and closure. Leaving any of them
  would have kept a stale claim alive.
- The decision authority is the human-approved D2 in `REM-000`; this log records
  the disposition, it does not make the decision.
