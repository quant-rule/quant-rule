#!/usr/bin/env python3
"""Check release notes, their current-tag map, and changelog alignment.

A release note lives at ``docs/releases/<YYYY-MM-DD>/<tag>.md``. Its tag is a
``vMAJOR.MINOR.PATCH`` identity and its ``- Tag:`` line matches the file name.
A release at or after the 2026-11-07 changelog baseline has a matching
``## [<version>] - YYYY-MM-DD`` section. Historical releases before the baseline
are exempt from the changelog requirement.

The 2026-11-15 renumbering kept the historical note file names, so the current
tag cannot be resolved from a file name. ``docs/releases/README.md`` carries the
authoritative current-tag map in a fenced JSON block, and this checker validates
it. The release workflow resolves a note through ``--note <tag>`` instead of
guessing from the tag name.

The checker does not create, move, or delete a Git tag. Tag creation remains a
human-authorized Git operation.

The checker has no dependencies beyond the Python standard library and reads no
network, clock, or external data.
"""

from __future__ import annotations

import json
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
RENUMBER_ROW = re.compile(r"^\| `(v\d+\.\d+\.\d+)` \|", re.MULTILINE)
MAP_START = "<!-- current-tags:start -->"
MAP_END = "<!-- current-tags:end -->"
MAP_JSON = re.compile(r"```json\s*(\{.*?\})\s*```", re.DOTALL)


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


def current_tags(root: Path) -> tuple[dict[str, str], list[str]]:
    """Read and validate the current-tag map in ``docs/releases/README.md``.

    The map's paths are relative to ``docs/releases``. Its keys must equal the
    tags in the renumbering table, so a note added to one place and not the
    other fails the checker instead of drifting silently.
    """
    problems: list[str] = []
    path = root / "docs" / "releases" / "README.md"
    if not path.is_file():
        return {}, []
    text = path.read_text(encoding="utf-8")
    start = text.find(MAP_START)
    end = text.find(MAP_END, start + len(MAP_START)) if start != -1 else -1
    if start == -1 or end == -1:
        return {}, [f"{path.name}: missing the {MAP_START} current-tag map"]
    match = MAP_JSON.search(text[start:end])
    if match is None:
        return {}, [f"{path.name}: the current-tag map must be a fenced json block"]

    try:
        pairs = json.loads(match.group(1), object_pairs_hook=lambda items: items)
    except json.JSONDecodeError as error:
        return {}, [f"{path.name}: the current-tag map is not valid JSON: {error}"]
    if not isinstance(pairs, list):
        return {}, [f"{path.name}: the current-tag map must be a JSON object"]

    tags: dict[str, str] = {}
    for key, value in pairs:
        if not isinstance(key, str) or not isinstance(value, str):
            problems.append(f"{path.name}: the current-tag map keys and values must be strings")
            continue
        if key in tags:
            problems.append(f"{path.name}: current tag {key} appears twice")
            continue
        if TAG.match(key) is None:
            problems.append(f"{path.name}: current tag {key} is not vMAJOR.MINOR.PATCH")
            continue
        tags[key] = value

    releases = root / "docs" / "releases"
    for key, relative in sorted(tags.items()):
        note = (releases / relative).resolve()
        try:
            note.relative_to(releases.resolve())
        except ValueError:
            problems.append(f"{path.name}: current tag {key} points outside docs/releases")
            continue
        if not note.is_file():
            problems.append(
                f"{path.name}: current tag {key} points to missing note {relative}"
            )
            continue
        if TAG.match(note.stem) is None:
            problems.append(
                f"{path.name}: current tag {key} points to {relative}, not a vMAJOR.MINOR.PATCH note"
            )

    table_start = text.find("### Renumbering (2026-11-15)")
    table_end = text.find("## 4. Historical tags", table_start)
    if table_start != -1 and table_end != -1:
        table = set(RENUMBER_ROW.findall(text[table_start:table_end]))
        missing = sorted(table - set(tags))
        extra = sorted(set(tags) - table)
        if missing:
            problems.append(
                f"{path.name}: current-tag map is missing {', '.join(missing)}"
            )
        if extra:
            problems.append(
                f"{path.name}: current-tag map names {', '.join(extra)}, which the renumbering table does not"
            )
    return tags, problems


def resolve(root: Path, tag: str) -> Path:
    """Resolve a current tag to its release note.

    # Raises

    ``ValueError`` when the map is invalid or the tag is not mapped.
    """
    tags, problems = current_tags(root)
    if problems:
        raise ValueError("; ".join(problems))
    relative = tags.get(tag)
    if relative is None:
        raise ValueError(f"{tag} is not in the current-tag map")
    return root / "docs" / "releases" / relative


def check(root: Path) -> list[str]:
    problems: list[str] = []
    notes = release_notes(root)
    changelog = changelog_releases(root)
    _, map_problems = current_tags(root)
    problems.extend(map_problems)

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


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if args:
        if len(args) == 2 and args[0] == "--note":
            try:
                note = resolve(ROOT, args[1])
            except ValueError as error:
                print(f"error: {error}", file=sys.stderr)
                return 1
            if not note.is_file():
                print(f"error: {args[1]} resolves to missing note {note}", file=sys.stderr)
                return 1
            print(note.relative_to(ROOT).as_posix())
            return 0
        print("usage: check_releases.py [--note <current-tag>]", file=sys.stderr)
        return 2

    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    notes = release_notes(ROOT)
    changelog = changelog_releases(ROOT)
    tags, _ = current_tags(ROOT)
    print(
        "releases ok: "
        f"{len(notes)} release notes, {len(tags)} current tags, "
        f"{len(changelog)} changelog versions, baseline {BASELINE}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
