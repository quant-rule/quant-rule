# AI Transparency Log — Repository History CLI

- Date: 2026-11-20 (UTC)
- Tool: OpenCode, DeepSeek V4.1 Flash
- Work item: #945
- Claim type: Behaviour Claim

## Intent and boundary

Add the `qr-governance history` command for historical repository evidence:
list config/docs/records, archive with a manifest, and erase only a verified
manifest. This is developer tooling, not a product client; no service, runtime
record, or GUI is involved.

## Verification and evidence

| Command | Result |
|---|---|
| `cargo test -p quantrule-governance` | 7 tests pass (2 unit, 5 integration) |
| `cargo clippy -p quantrule-governance --all-targets -- -D warnings` | PASS |
| `python3 tools/check_structure.py` | PASS |
| Full gate | Pending CI |

## Learning

- `sha2` is already in the workspace lock; no new external crate version was
  introduced. It is used only for manifest file hashes.
- The archive id is a SHA-256 over the sorted path/bytes/hash list, so the same
  selection always yields the same identity.
- Date extraction reads the first `YYYY-MM-DD` in a path component, which covers
  both date directories and `PROGRAM-YYYY-MM-DD-NN` config directories.
- Erase verifies every byte count and hash before deleting anything; a tampered
  source refuses the whole operation.
- The temporary-tree tests run the real binary and use `--root`, so the command
  works outside the repository root without a chdir.

## Rejected or corrected suggestions

| Suggestion | Disposition |
|---|---|
| Put repository history in the product shell/browser | Rejected; repository evidence is source control, not service data |
| Delete without an archive manifest | Rejected; archive then erase |
| Add a new date/time crate for the manifest | Rejected; Unix seconds are enough for provenance |
