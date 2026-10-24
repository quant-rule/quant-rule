# AI Transparency Log: The SQLite Store and Research Schema (ST1)

- Work item: [#65](https://github.com/quant-rule/quant-rule/issues/65) — arch: define the SQLite per-bar store and research schema
- Plan: [`docs/execution-plans/2026-10-24/65-store-adr.md`](../../execution-plans/2026-10-24/65-store-adr.md)
- ADR: [`docs/adr/2026-10-24/sqlite-store.md`](../../adr/2026-10-24/sqlite-store.md)
- Date: 2026-10-24 (UTC)

## Entry: the store ADR

- **Date and tool:** 2026-10-24 (UTC). opencode (DeepSeek V4.1 Flash 1017).
- **Intent and boundary:** Decide the store engine, location, schema, migrations, identity, transactions, and command surface before the implementation sub-issues. Documentation only; no code.
- **Verification and evidence:**
  - Wrote `docs/adr/2026-10-24/sqlite-store.md` and the plan.
  - Checked every relative link; both resolve.
  - Checked the column groups against the research contract: the `bar` table carries the Layer 2 columns (61 + 12) and the `layer3` table the three 67-column blocks plus the 82 aggregates.
  - The Rust checks are unchanged from the baseline: 753 passed, 0 failed, 3 ignored; fmt, clippy, and doc are clean.
- **Learning:**
  - The per-bar record is the natural table. Storing the Layer 2 columns once makes Layer 1 and Layer 2 projections; three tables would be three copies of the same 128k rows.
  - Layer 3 can avoid duplicating the M1 base. The `layer3` table holds the blocks and aggregates, and `layer3_full` joins the M1 `bar` row. That saves 73 columns on every one of the 100k rows.
  - SQLite treats NULLs as distinct in a unique index, so the event table stores `reference_code = 0` for "none" and uses a unique index over the full event key. A nullable sentinel would allow duplicate events.
  - The store is a shell module rather than a crate. A fourth crate would either move I/O out of the shell or add a layer for a single SQLite dependency; the shell already owns every read and write.
  - SQLite file bytes are not a stable reproducibility artifact. The CSV export remains the byte-identical artifact; the store's verification compares logical rows, and the ADR says so rather than claiming the database file is byte-identical.
  - The event, leg, level, and consensus records are stored, but the research CSV export still does not write them. The store verify compares the stored records with the in-memory batch, so the two paths share the model's values without a second computation rule.
  - The 100k cap is enforced twice: a `CHECK` on `dataset.m1_bars` and the full-series cap in the shell. A writer bug cannot store an over-cap dataset.
  - The schema is version 1 in one migration. The implementation sub-issues add writers and queries; they do not add tables. This keeps the schema review in one place and avoids migration churn.
  - The exact `rusqlite` version is left to #67; the ADR approves the crate and the `bundled` feature, not a version that may not exist when the code lands.

## Final verification

- ADR links: resolve.
- Schema column groups: match the research contract.
- `cargo fmt --all -- --check`: clean.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --workspace`: 753 passed, 0 failed, 3 ignored.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`: clean.
- Status: `Verified`. Commit `eb91ed2` is pushed on `arch/65-store-adr`, pull request [#99](https://github.com/quant-rule/quant-rule/pull/99) is open, and #65 is `Done` on the board.
