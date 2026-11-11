#!/usr/bin/env python3
"""Verify the static download page against the built Linux artifacts.

The page is distribution, not a runtime. This checker fails when a download
link has no artifact, a checksum line has no file, a hash does not match, or the
page loses its Linux/no-account boundary.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

SITE = Path(__file__).resolve().parent
ROOT = SITE.parent
PACKAGE = ROOT / "apps" / "desktop" / "package.json"


def version() -> str:
    return json.loads(PACKAGE.read_text(encoding="utf-8"))["version"]


def expected_artifacts() -> list[str]:
    release = version()
    return [
        f"QuantRule_{release}_amd64.deb",
        f"QuantRule_{release}_amd64.AppImage",
    ]


def download_links(html: str) -> list[str]:
    return re.findall(r'href="downloads/([^"]+)"', html)


def checksum_lines(path: Path) -> dict[str, str]:
    entries: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        parts = line.split()
        if len(parts) != 2:
            raise ValueError(f"invalid checksum line: {line!r}")
        digest, name = parts
        if name.startswith("downloads/"):
            name = name[len("downloads/") :]
        entries[name] = digest
    return entries


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def check(require_artifacts: bool) -> list[str]:
    problems: list[str] = []
    index = SITE / "index.html"
    html = index.read_text(encoding="utf-8")

    for phrase in ["Linux", "x86_64", "No account", "checksums.txt"]:
        if phrase not in html:
            problems.append(f"index.html is missing {phrase!r}")

    expected = expected_artifacts()
    links = sorted(set(download_links(html)))
    if links != sorted(expected):
        problems.append(
            f"download links {sorted(links)} do not match {sorted(expected)}"
        )

    checksums = SITE / "checksums.txt"
    if not checksums.is_file():
        if require_artifacts:
            problems.append("checksums.txt is missing")
        return problems

    try:
        entries = checksum_lines(checksums)
    except ValueError as error:
        return problems + [str(error)]

    if sorted(entries) != sorted(expected):
        problems.append(
            f"checksum entries {sorted(entries)} do not match {sorted(expected)}"
        )

    downloads = SITE / "downloads"
    for name in expected:
        artifact = downloads / name
        if not artifact.is_file():
            if require_artifacts:
                problems.append(f"artifact is missing: downloads/{name}")
            continue
        actual = sha256(artifact)
        if entries.get(name) != actual:
            problems.append(f"checksum mismatch: downloads/{name}")

    return problems


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--require-artifacts",
        action="store_true",
        help="fail when downloads/ or checksums.txt is missing",
    )
    args = parser.parse_args()
    problems = check(args.require_artifacts)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    print(
        f"site ok: version {version()}, {len(expected_artifacts())} artifacts, "
        f"{len(expected_artifacts())} checksums"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
