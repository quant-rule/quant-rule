#!/usr/bin/env python3
"""Check that every relative Markdown link resolves inside the repository.

External links, mail links, and bare anchors are outside the check. Fenced code
blocks and inline code spans are skipped so examples do not create false
positives. The script prints every broken link and exits 1 when any is found.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent
EXCLUDED_DIRECTORIES = {".git", "node_modules", "target", "data"}
FENCE = re.compile(r"^\s*(```|~~~)")
LINK = re.compile(r"!?\[[^\]]*\]\(([^)\s]+)(?:\s+['\"][^'\"]*['\"])?\)")
INLINE_CODE = re.compile(r"`[^`]*`")
EXTERNAL_PREFIXES = ("http://", "https://", "mailto:", "tel:")


def markdown_files() -> list[Path]:
    files = []
    for path in ROOT.rglob("*.md"):
        relative = path.relative_to(ROOT)
        if any(part in EXCLUDED_DIRECTORIES for part in relative.parts):
            continue
        files.append(path)
    return sorted(files)


def links_in(path: Path):
    in_fence = False
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if in_fence:
            continue
        line = INLINE_CODE.sub("", line)
        for match in LINK.finditer(line):
            yield number, match.group(1)


def resolve(source: Path, target: str) -> Path | None:
    target = target.split("#", 1)[0].split("?", 1)[0]
    if not target or target.startswith(EXTERNAL_PREFIXES):
        return None
    if target.startswith("/"):
        return ROOT / unquote(target.lstrip("/"))
    return source.parent / unquote(target)


def main() -> int:
    broken = []
    for path in markdown_files():
        for number, target in links_in(path):
            resolved = resolve(path, target)
            if resolved is None:
                continue
            if not resolved.exists():
                broken.append((path, number, target, resolved))
    if broken:
        for path, number, target, resolved in broken:
            print(
                f"{path.relative_to(ROOT)}:{number}: "
                f"broken link {target!r} -> {resolved.relative_to(ROOT)}"
            )
        print(f"{len(broken)} broken relative link(s)")
        return 1
    print(f"checked {len(markdown_files())} Markdown files; no broken relative links")
    return 0


if __name__ == "__main__":
    sys.exit(main())
