# AI Transparency Log — Legacy Checker Speedup

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: #946
- Claim type: Behaviour Claim

## Intent and boundary

Stop `tools/check_legacy.py` from walking build output and dependency trees on
every local gate run. This is a Python scan-set fix, not a port to Rust.

The checker still reads current tracked files plus untracked non-ignored files;
history paths, size caps, binary detection, and every violation/allowlist rule
are unchanged.

## Verification and evidence

| Command | Result |
|---|---|
| `python3 tools/check_legacy.py` | 1,989 label files, 3,433 current files, 4.1 s |
| `cd tools && python3 -m unittest test_check_legacy` | 10 tests pass |
| `bash tools/checkers.sh` | 27 checkers pass |

The previous local run of the same check took 60.5 seconds on the same machine.

## Learning

- The checker already skipped `target/`, `node_modules/`, and caches, but
  `Path.rglob` still traversed those trees. `git ls-files --cached --others
  --exclude-standard` avoids the traversal and keeps untracked current files.
- The temporary-tree checker tests have no Git repository, so the old walk is
  retained as a fallback; their violation behavior is unchanged.
- The CI gate was already fast; this fix is for the local developer loop.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Port `check_legacy.py` to Rust now | Deferred with the Python retirement program |
| Scan only tracked files | Rejected; untracked non-ignored files must still be checked |
| Cache the result between runs | Rejected; a stale cache could hide a current violation |
