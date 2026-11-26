# AI Transparency — MIT licence

- Date: 2026-11-26 (UTC)
- Tool: OpenCode / DeepSeek V4.1 Flash
- Work item: #1131 release follow-up, v1.6.0 MIT licence
- Intent: the human changed the project licence from `MIT OR Apache-2.0` to
  `MIT` after the v1.6.0 release.

## Work and boundary

- Set the workspace and every crate manifest `license` field to `MIT`.
- Replaced the root `LICENSE` pointer with a pointer to `LICENSE-MIT`, removed
  `LICENSE-APACHE` from the current tree, and kept the MIT text in
  `LICENSE-MIT`.
- Updated both application image labels and the derived Patroni image label to
  `MIT`.
- Updated the support-and-licence contract, the config index, and the
  changelog.
- The published `v1.6.0` tag and image are immutable and remain under their
  published `MIT OR Apache-2.0` dual grant; the next release carries MIT. The
  release note for v1.6.0 is not rewritten.

## Verification and evidence

- `rg 'MIT OR Apache-2.0'` in the current manifests, image labels, and current
  contract returns no remaining current declaration; the only matches are
  historical release notes, changelog history, and plan/log records.
- `cargo metadata` resolves the workspace with every crate at `license = "MIT"`.
- `qr-governance check --all`: pass.

## Learning

- A licence change after a published tag does not revoke the grant at that tag;
  the historical release tree and image labels retain the dual grant. The
  current tree and the next release are MIT.
- The manual image signature is a separate operator step and is not affected by
  the licence change.
