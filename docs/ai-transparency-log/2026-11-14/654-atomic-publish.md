# AI transparency log — DIST-042 atomic publish plan

- Date: 2026-11-14 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash.
- Intent: write the execution plan for atomic artifact publish and the
  no-partial-publish invariant, and surface the decisions that need human
  approval.
- Boundary: planning only. No program code, test, issue, or pull request behavior
  changed. The plan reads the worker artifact path, the evidence verifier, the
  object-store trait, and the data-plane document.
- Verification: `tools/check_execution_plans.py`, `tools/check_links.py`, and
  `tools/check_doc_length.py` were run on the plan PR. The behavioral rows are
  Pending by design because this PR changes no behavior.
- Learning: the run row already hides partial writes, but the object keys do not.
  Moving the publish to one manifest object is the smallest change that makes a
  mid-write stop invisible at the storage boundary without changing the canonical
  digest.
- Rejected: a single `bundle.tar` as the publish. It is atomic, but evidence and
  the future download route would both have to untar, and content-addressed blobs
  let a retry reuse what was written. Also rejected: deleting blobs on failure,
  because a blob may be shared by another run.

## Implementation — 2026-11-14 (UTC)

- Intent: implement the approved atomic publish — content-addressed blobs, one
  manifest, evidence verification against the manifest and the run record.
- Boundary: S3 and Postgres compile behind the traits but are not live-tested;
  the real `SIGKILL` composed test remains #665. No model, strategy, execution,
  or portfolio behavior changed.
- Verification: `tools/gate.sh` passed. New tests cover a successful manifest,
  a missing manifest, a manifest that disagrees with the run record, a missing
  blob, a changed blob, a record-digest mismatch, and a stopped write that leaves
  no `runs/{run_id}/` prefix. The worker test asserts the manifest entries equal
  the run artifacts and that no per-name key exists.
- Learning: the canonical digest excludes provenance and logs, but the manifest
  includes every artifact so the completeness check covers them. The digest is
  computed over only `results.json`, `*.csv`, and `*.ndjson`, as before.
- Deviation: the plan named `evidence/artifacts.rs` for the manifest type; the
  type lives in `evidence/manifest.rs` so the canonical digest and the schema
  are one module and the verifier stays small. The plan's work breakdown allows
  either path.
- Rejected: deleting partial blobs on failure. A blob is content-addressed and
  may be shared by another run; the manifest is the visibility gate.
