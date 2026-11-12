#!/usr/bin/env bash
#
# Build the course PDFs from the source Markdown.
#
# The build uses the pandoc/extra image so the host does not need a LaTeX
# installation. The source Markdown is authoritative; the PDFs are generated
# deliverables.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
IMAGE="${PANDOC_IMAGE:-pandoc/extra:latest}"
DATE="2026-11-12"
AUTHOR="QuantRule"

if ! command -v docker >/dev/null 2>&1; then
  echo "error: docker is required to build the course PDFs" >&2
  exit 1
fi

build_dir() {
  local source_dir="$1"
  local target_dir="$2"
  local source_root="$ROOT/docs/course/$source_dir"
  local target_root="$ROOT/docs/course/$target_dir"
  mkdir -p "$target_root"
  shopt -s nullglob
  local sources=("$source_root"/*.md)
  shopt -u nullglob
  if [ "${#sources[@]}" -eq 0 ]; then
    echo "error: no Markdown sources in docs/course/$source_dir" >&2
    exit 1
  fi
  for source in "${sources[@]}"; do
    local name
    name="$(basename "$source" .md)"
    local title
    title="$(sed -n '1s/^# //p' "$source")"
    if [ -z "$title" ]; then
      title="$name"
    fi
    local output="$target_dir/$name.pdf"
    local toc=()
    local numbers=()
    # The deliverables are short; a table of contents and section numbers add
    # noise without helping a reader find anything.
    echo "building $output"
    sed '1{/^# /d;}' "$source" | docker run --rm -i \
      --user "$(id -u):$(id -g)" \
      -v "$ROOT:/work" \
      -w /work \
      "$IMAGE" \
      --from markdown \
      --to pdf \
      --pdf-engine=xelatex \
      --variable geometry:margin=2.2cm \
      --variable fontsize=11pt \
      --metadata title="$title" \
      --metadata author="$AUTHOR" \
      --metadata date="$DATE" \
      --fail-if-warnings \
      "${toc[@]}" \
      "${numbers[@]}" \
      -o "docs/course/$target_dir/$name.pdf" \
      -
  done
}

build_dir "architecture/src" "architecture/deliverables"
build_dir "methodology/src" "methodology/deliverables"

echo "course PDFs built"
