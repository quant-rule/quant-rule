# AI Transparency Log — Record Consolidation and Invalidation

- Date: 2026-11-03 (UTC)
- Tool: OpenCode coding agent (DeepSeek V4.1 Flash)
- Work item: [#350](https://github.com/quant-rule/quant-rule/issues/350)
- Plan:
  [`docs/execution-plans/2026-11-03/342-model-v3.md`](../../execution-plans/2026-11-03/342-model-v3.md)
- Claim type: documentation and record hygiene; no edge claim

## Intent and boundary

Make the committed docs the durable summary and the local data the raw evidence,
and tombstone the axis-based evidence invalidated by model v3. No model or
strategy behaviour changes.

## Work

- Added the committed-summary rule to `docs/improvement/README.md`: the docs
  summary is canonical; the local `review.md`/`decision.md` are working copies;
  heavy artifacts stay local.
- Appended tombstones for the 2026-11-01 model and strategy runs to
  `docs/improvement/INVALIDATIONS.md`. The model and strategy evidence used the
  retired axes and a 20% drawdown envelope.
- Removed the local `data/improvement/2026-11-01` run folders after verifying the
  committed summaries and the tombstone rows. The docs record remains.
- Fixed the stale “current index is empty” text in the committed index and added
  the two model-gate runs and `CYCLE-008`.
- Marked the model correctness baseline superseded and recorded the v3 gate as
  not accepted, no edge.

## Verification and evidence

- `python3 tools/check_links.py`: 370 files, 0 broken.
- `python3 tools/check_terms.py`: 229 files, 0 banned.
- `data/improvement/2026-11-01` removed; the committed summary files and
  `INVALIDATIONS.md` remain.
- The committed index now lists the two model-gate runs; the local generated
  index agrees.

## Learning

- The docs summaries were sufficient to preserve the no-edge result; the heavy
  run folders were not needed after the contract changed.
- A tombstone must name the exact run range and the superseding decision, not
  “cleanup”.
- The committed index and the generated index diverged because the committed
  copy was not refreshed; the cleanup fixes the stale text.

## Rejected suggestions

- Keeping the invalidated local runs “just in case”: they would keep two
  incompatible model contracts in the store.
- Deleting the docs summaries too: the summary is the durable evidence; only the
  local artifacts are removed.
- Rewriting the old summaries: they are history and are marked superseded, not
  edited.

## Limitations

- The old local runs are gone; the committed summaries are the only record.
- The new model-gate local bundles remain under `data/model-gate` and
  `data/improvement/2026-11-03` until the next cleanup decision.
- Same-session review, not an independent Quant Validator record.
