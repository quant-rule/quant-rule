#!/usr/bin/env python3
"""Check the grouped release changelog.

The changelog is a release history, not a pull-request log. It has one
``## [Unreleased]`` section and any number of ``## [<version>] - YYYY-MM-DD``
sections, newest first. Each release groups entries under ``### Added``,
``### Changed``, ``### Deprecated``, ``### Removed``, ``### Fixed``, or
``### Security``. Every group has at least one bullet entry.

The checker has no dependencies beyond the Python standard library and reads no
network, clock, or external data.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RELEASE = re.compile(r"^## \[(.+)\](?: - (\d{4}-\d{2}-\d{2}))?\s*$")
TYPE = re.compile(r"^### (Added|Changed|Deprecated|Removed|Fixed|Security)\s*$")
BULLET = re.compile(r"^- (.+)$")
TYPES = ("Added", "Changed", "Deprecated", "Removed", "Fixed", "Security")


def parse(root: Path) -> tuple[list[str], list[dict[str, object]]]:
    path = root / "CHANGELOG.md"
    if not path.is_file():
        return ["CHANGELOG.md is missing"], []
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError as error:
        return [f"cannot read CHANGELOG.md: {error}"], []

    problems: list[str] = []
    if not lines or not lines[0].startswith("# Changelog"):
        problems.append("CHANGELOG.md must start with '# Changelog'")

    releases: list[dict[str, object]] = []
    current: dict[str, object] | None = None
    current_type: str | None = None
    for number, line in enumerate(lines, 1):
        release = RELEASE.match(line)
        if release is not None:
            name = release.group(1)
            date = release.group(2)
            if name == "Unreleased":
                if date is not None:
                    problems.append(f"{number}: [Unreleased] must not carry a date")
            else:
                if date is None:
                    problems.append(
                        f"{number}: release [{name}] must be '## [{name}] - YYYY-MM-DD'"
                    )
                elif not re.match(r"^\d{4}-\d{2}-\d{2}$", date):
                    problems.append(f"{number}: release date {date!r} must be YYYY-MM-DD")
            current = {"name": name, "date": date, "types": {}, "line": number}
            releases.append(current)
            current_type = None
            continue

        if line.startswith("## "):
            current = None
            current_type = None
            continue

        type_match = TYPE.match(line)
        if type_match is not None:
            if current is None:
                problems.append(f"{number}: '### {type_match.group(1)}' is outside a release")
                continue
            current_type = type_match.group(1)
            current["types"][current_type] = []  # type: ignore[index]
            continue

        if line.startswith("### "):
            if current is not None:
                problems.append(f"{number}: unknown changelog section {line!r}")
            continue

        bullet = BULLET.match(line)
        if bullet is not None:
            if current is None:
                # A bullet in the preamble or the release-rules section is not
                # a release entry.
                continue
            if current_type is None:
                problems.append(f"{number}: entry is outside a release type section")
                continue
            text = bullet.group(1).strip()
            if not text:
                problems.append(f"{number}: entry is empty")
            else:
                current["types"][current_type].append(text)  # type: ignore[index]

    if not releases:
        problems.append("CHANGELOG.md has no release section")

    names = [str(release["name"]) for release in releases]
    if "Unreleased" not in names:
        problems.append("CHANGELOG.md has no [Unreleased] section")
    elif names[0] != "Unreleased":
        problems.append("[Unreleased] must be the first release section")

    seen: set[str] = set()
    previous_date: str | None = None
    for release in releases:
        name = str(release["name"])
        if name in seen:
            problems.append(f"release [{name}] appears twice")
        seen.add(name)
        types = release["types"]
        if not types:
            problems.append(f"release [{name}] has no type section")
            continue
        release_entries = 0
        for kind, values in types.items():
            if not values:
                problems.append(f"release [{name}] has an empty '### {kind}' section")
            release_entries += len(values)
        if release_entries == 0:
            problems.append(f"release [{name}] has no entries")
        date = release["date"]
        if name != "Unreleased" and isinstance(date, str):
            if previous_date is not None and date > previous_date:
                problems.append(
                    f"release [{name}] date {date} is newer than the previous release {previous_date}"
                )
            previous_date = date

    return problems, releases


def check(root: Path) -> list[str]:
    problems, _ = parse(root)
    return problems


def counts(root: Path) -> tuple[int, int]:
    _, releases = parse(root)
    entries = sum(
        len(values)
        for release in releases
        for values in release["types"].values()  # type: ignore[union-attr]
    )
    return len(releases), entries


def main() -> int:
    problems, releases = parse(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    entries = sum(
        len(values)
        for release in releases
        for values in release["types"].values()  # type: ignore[union-attr]
    )
    print(f"changelog ok: {len(releases)} releases, {entries} entries")
    return 0


if __name__ == "__main__":
    sys.exit(main())
