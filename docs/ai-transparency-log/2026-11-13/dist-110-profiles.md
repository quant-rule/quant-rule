# AI transparency log — DIST-110 profiles

- Date: 2026-11-13 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: make profiles real records — storage, endpoints, the seeded default,
  and existence checking on the dataset routes.
- Boundary: this slice does not add `profile_id` to runs, simulations, or
  hypotheses, and does not migrate their keys. Their services have no routes
  yet, and a naive column would break content-addressed sharing of one run.
- Verification: `cargo test -p quantrule-server`, clippy with `-D warnings`,
  `cargo fmt --check`, rustdoc with `-D warnings`, `check_structure`,
  `check_legacy`, and `check_doc_length`. No database was available, so the
  Postgres profile store compiles but is not exercised by a test.
- Learning: `ProfileId` deserializes without validation, so the create route
  re-parses the id before it writes. The memory profile store starts empty; the
  tests seed exactly the profiles they use, and the default profile is a
  migration responsibility rather than a constructor side effect.
- Rejected: silently accepting an unknown profile header. The data routes now
  return `404 profile.not_found`, because a row that is not scoped cannot be
  distinguished from one that is.
