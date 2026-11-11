# Course PDF Tooling

`build_pdfs.sh` builds the course PDFs from `docs/course/*/src` into
`docs/course/*/deliverables`.

```bash
tools/course/build_pdfs.sh
```

The script uses the `pandoc/extra:latest` Docker image and XeLaTeX with the
default Pandoc template. The host does not need Pandoc or LaTeX. The first run
pulls the image; later runs reuse it.

The source Markdown is authoritative. Rebuild and review the PDFs when a source
changes. The generated PDFs are committed because they are the submission
artifacts.

## Add a deliverable

1. Add the Markdown file under the matching `src/` folder.
2. Start it with a level-1 heading; the heading becomes the PDF title.
3. Add the file to the table in `docs/course/README.md`.
4. Run `tools/course/build_pdfs.sh`.
5. Review the PDF and commit both source and PDF.
