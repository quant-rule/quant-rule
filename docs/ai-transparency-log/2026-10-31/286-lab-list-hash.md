# AI Transparency Log — `lab list` Config Hash

- Date: 2026-10-31 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash), sequential role passes
- Work item: #286, branch `fix/286-lab-list-hash`
- Plan: `docs/execution-plans/2026-10-31/audit-remediation-diagnosis-v1.md` (W7)
- Audit: `docs/audits/2026-10-31/remediation-and-diagnosis-v1-audit.md` (AUD-2026-10-31-40)

## Intent and boundary

W7 fixes the last finding: `lab list` sliced a `config_hash` by byte length and
could panic on a multi-byte string. The human approved D10 = B + A: validate a
present hash as 16 ASCII hex characters, and keep a bounds-safe display as a
second defense. The change is confined to the `lab list` reader, its test, and
the README contract.

## Implemented

- Added `lab_config_hash`, which returns an empty string when the provenance or
  the `config_hash` key is absent, and a `RunError::Diagnosis` naming the
  provenance path when the key is present but is not 16 ASCII hex characters.
- Replaced the byte-length slice with `config_hash.get(..12).unwrap_or(...)`.
- Updated the README `lab list` contract to state the validation.
- Added a shell test that builds a lab from a real diagnosis run and covers:
  - a 16-byte hash whose byte 12 falls inside a multi-byte character -> error,
    no panic;
  - a valid 16-hex hash -> listed;
  - an absent provenance -> still listed with an empty hash.

## Verification and evidence

| Command | Observed |
|---|---|
| `cargo test -p quantrule-shell --test diagnosis lab_list` | 2 passed, 0 failed |
| `cargo test --workspace` | 1518 passed, 0 failed, 7 ignored |
| `cargo clippy --all-targets -- -D warnings` | exit 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` | exit 0 |
| `git diff --check` | clean |

The malformed test string is `aaaaaaaaaaaééa`: 16 bytes, but byte 12 is inside
the first two-byte character, so the old slice would panic. The test asserts the
command fails with a `config_hash` message and not with exit code 101.

## Learning

- Validation is the real fix: it rejects the malformed lab before any rendering.
  The `get` is defense in depth and also protects the text column.
- A missing provenance or hash must stay listable; only a present, malformed
  value is an error. That keeps older labs readable while refusing a corrupt
  identity.
- A non-string `config_hash` is treated as present and invalid, not as absent,
  because the key exists and the lab identity is not trustworthy.
- The fail-before is derived from the old slice rather than run: the fix and the
  test were applied together. The test string is constructed to hit the exact
  non-boundary byte.

## Limitations

- No independent verifier; this is a sequential implementation pass.
- The test uses a generated lab copied into `target/report-tests`, not a
  checked-in golden.
- W7 is the last work item; findings remain `Fixed` until the independent
  Improvement Verifier pass.
