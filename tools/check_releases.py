#!/usr/bin/env python3
"""Check release notes and their alignment with the changelog.

A release note lives at ``docs/releases/<YYYY-MM-DD>/<tag>.md``. Its tag is a
``vMAJOR.MINOR.PATCH`` identity and its ``- Tag:`` line matches the file name.
A release at or after the 2026-11-07 changelog baseline has a matching
``## [<version>] - YYYY-MM-DD`` section. Historical releases before the baseline
are exempt from the changelog requirement.

The checker does not create, move, or delete a Git tag. Tag creation remains a
human-authorized Git operation.

The checker has no dependencies beyond the Python standard library and reads no
network, clock, or external data.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BASELINE = "2026-11-07"
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
TAG = re.compile(r"^v(\d+)\.(\d+)\.(\d+)$")
TAG_LINE = re.compile(r"^- Tag:\s*(.+)$", re.MULTILINE)
DATE_LINE = re.compile(r"^- Date:\s*(\d{4}-\d{2}-\d{2})", re.MULTILINE)
CHANGELOG_RELEASE = re.compile(r"^## \[([^\]]+)\] - (\d{4}-\d{2}-\d{2})\s*$")


def release_notes(root: Path) -> list[Path]:
    directory = root / "docs" / "releases"
    if not directory.is_dir():
        return []
    return sorted(
        path
        for path in directory.glob("*/*.md")
        if path.name != "README.md"
    )


def changelog_releases(root: Path) -> dict[str, str]:
    path = root / "CHANGELOG.md"
    if not path.is_file():
        return {}
    releases: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        match = CHANGELOG_RELEASE.match(line)
        if match is not None:
            releases[match.group(1)] = match.group(2)
    return releases


def check(root: Path) -> list[str]:
    problems: list[str] = []
    notes = release_notes(root)
    changelog = changelog_releases(root)

    seen_tags: set[str] = set()
    for path in notes:
        relative = path.relative_to(root).as_posix()
        if not DATE.match(path.parent.name):
            problems.append(f"{relative}: release directory must be YYYY-MM-DD")
            continue
        match = TAG.match(path.stem)
        if match is None:
            problems.append(f"{relative}: release file must be vMAJOR.MINOR.PATCH.md")
            continue
        tag = path.stem
        if tag in seen_tags:
            problems.append(f"{relative}: release {tag} appears twice")
        seen_tags.add(tag)
        text = path.read_text(encoding="utf-8")
        tag_line = TAG_LINE.search(text)
        if tag_line is None:
            problems.append(f"{relative}: missing '- Tag:' line")
        else:
            found = re.search(r"v\d+\.\d+\.\d+", tag_line.group(1))
            if found is None:
                problems.append(f"{relative}: '- Tag:' must name a vMAJOR.MINOR.PATCH tag")
            elif found.group(0) != tag:
                problems.append(
                    f"{relative}: '- Tag:' names {found.group(0)}, expected {tag}"
                )
        date_line = DATE_LINE.search(text)
        if date_line is None:
            problems.append(f"{relative}: missing '- Date: YYYY-MM-DD' line")
            continue
        note_date = date_line.group(1)
        if note_date != path.parent.name:
            problems.append(
                f"{relative}: '- Date:' {note_date} does not match the directory {path.parent.name}"
            )
        version = tag[1:]
        if note_date >= BASELINE and version not in changelog:
            problems.append(
                f"{relative}: release {tag} has no changelog section '## [{version}] - {note_date}'"
            )
        if note_date >= BASELINE and version in changelog and changelog[version] != note_date:
            problems.append(
                f"{relative}: changelog date {changelog[version]} does not match release date {note_date}"
            )

    for version, date in sorted(changelog.items()):
        if date < BASELINE:
            continue
        tag = f"v{version}"
        expected = root / "docs" / "releases" / date / f"{tag}.md"
        if not expected.is_file():
            problems.append(
                f"CHANGELOG.md: release {tag} has no release note at {expected.relative_to(root).as_posix()}"
            )

    return problems


def main() -> int:
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    notes = release_notes(ROOT)
    changelog = changelog_releases(ROOT)
    print(
        "releases ok: "
        f"{len(notes)} release notes, {len(changelog)} changelog versions, "
        f"baseline {BASELINE}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
