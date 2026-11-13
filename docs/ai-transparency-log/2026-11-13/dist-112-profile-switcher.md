# AI transparency log — DIST-112 profile switcher

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: let the browser list and create profiles, remember the active one, and
  send it on every call; switching reloads the dataset list.
- Boundary: the active id is the only value persisted; no token or secret is
  stored. Deleting a profile is not offered, because the server has no delete
  route. The switcher assumes the default profile exists, which migration 0007
  guarantees.
- Verification: `bun run check`, `bun test` (10 pass), `bun run build`,
  `cargo test --workspace`, clippy with `-D warnings`, `cargo fmt --check`,
  rustdoc with `-D warnings`, `check_structure`, `check_legacy`, and
  `check_doc_length`.
- Learning: the selection rule is a pure function (`chooseProfile`) so it can be
  tested without the Svelte runtime. A stored profile is kept even when the
  server no longer lists it; the switcher marks it `(unknown)`, shows `profile
  error`, and the next call carries the server's `profile.not_found`. A first
  run with no stored id uses `default`.
- Rejected: silently replacing an unknown stored id with `default`. It hides a
  real server disagreement; the issue asks the client to show the error.
