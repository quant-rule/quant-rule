# AI transparency log — methodology checkpoints 10/10

- Date: 2026-11-16 (UTC)
- Tool: OpenCode with DeepSeek V4.1 Flash 1017.
- Work item: `docs/course/methodology/src/` and
  `docs/course/methodology/README.md`.
- Claim type: Outcome Claim.

## Intent and boundary

The human asked for the five CM0639 methodology checkpoints to be rewritten as
course-ready checkpoints: a first-person spoken script, then a compact evidence
appendix. The rewrite had to keep the Claim/Evidence/Limit/Decision structure,
cut length, remove report-style labels and arrow chains, preserve the tag facts,
and stay within the page and timing limits.

The agent read the L04, L08, and L12 lecture decks, the current checkpoint
sources, the release note and map, the architecture documents, and the
pre-release kata alignment. It changed only the methodology sources, the
methodology README, the five checkpoint PDFs, and this log. It did not change
code, architecture documents, or release notes.

## Verification

- Spoken word counts: A 143, B 402, C 472, D 507, E 405.
- Timing at `espeak-ng -s 150`: A 59.2 seconds; B 177.0 seconds.
- Page counts: A 1, B 2, C 2, D 2, E 2; the overview stays 1.
- `tools/gate.sh` and `tools/checkers.sh` were not needed for a docs-only
  change; the checker gate reported 24 checkers passed.
- `python3 tools/check_links.py`: 981 Markdown files, no broken links.
- `python3 tools/check_doc_length.py`: 35 files within cap.
- `python3 tools/check_terms.py`: 602 files, no banned terms.
- `tools/course/build_pdfs.sh`: built all 11 PDFs; the unchanged architecture
  PDFs and the overview PDF were restored.
- `pdftotext` checks: every checkpoint PDF contains `v1.2.0`; A contains the new
  “client test that prints” wording; E contains `v1.1.0` and the
  server-platform baseline in the rollback row. The old A/E phrases are absent.
- `pdfinfo`: A 1 page; B, C, D, E 2 pages.

## Learning

- The report-style labels were the main source of length and stiffness. Moving
  exact tests, commands, and tables into the appendix let the spoken script stay
  in short sentences.
- The tag map belongs once in the methodology README. The checkpoints now say
  `v1.2.0` only where the spoken or evidence text needs it.
- A raw LaTeX page break is used before the appendix in B, C, and E so the
  two-page checkpoints do not leave a heading alone at the bottom of a page.
- The PDF pipeline hyphenates words at line breaks, so “printed-output” was
  replaced with “client test that prints” to keep the extracted text readable.

## Rejected or corrected suggestions

- Repeating the full tag note in every checkpoint was rejected; it would repeat
  the history five times and inflate the pages.
- Adding a coverage number to C was rejected; the evidence is the prefix
  behaviour and its limit, not a test count.
- Committing or pushing was not done because the repository rules require
  explicit authorization, and the human asked to commit only if authorized.
- No Quant Validator record is required because no model or strategy behavior
  changed.
