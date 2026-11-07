#!/usr/bin/env python3
"""Unit tests for the release-note and changelog alignment checker."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_releases


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def release_note(tag: str, date: str) -> str:
    return (
        f"# {tag} — Test\n\n"
        f"- Date: {date} (UTC)\n"
        f"- Tag: `{tag}`\n"
        "- Commit: `209530b`\n"
        "- Status: Released\n"
    )


def changelog(release: str = "") -> str:
    return (
        "# Changelog\n\n"
        "## [Unreleased]\n\n"
        "### Added\n\n"
        "- Pending change.\n\n"
        f"{release}"
    )


class ReleaseCheckerTests(unittest.TestCase):
    def test_post_baseline_release_with_changelog_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "2026-11-07" / "v0.6.0.md",
                release_note("v0.6.0", "2026-11-07"),
            )
            write(
                root / "CHANGELOG.md",
                changelog("## [0.6.0] - 2026-11-07\n\n### Added\n\n- Release.\n"),
            )
            self.assertEqual(check_releases.check(root), [])

    def test_historical_release_is_exempt_from_changelog(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "2026-10-28" / "v0.2.0.md",
                release_note("v0.2.0", "2026-10-28"),
            )
            write(root / "CHANGELOG.md", changelog())
            self.assertEqual(check_releases.check(root), [])

    def test_missing_changelog_section_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "2026-11-07" / "v0.6.0.md",
                release_note("v0.6.0", "2026-11-07"),
            )
            write(root / "CHANGELOG.md", changelog())
            problems = check_releases.check(root)
            self.assertTrue(any("no changelog section" in problem for problem in problems))

    def test_missing_release_note_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "CHANGELOG.md",
                changelog("## [0.6.0] - 2026-11-07\n\n### Added\n\n- Release.\n"),
            )
            problems = check_releases.check(root)
            self.assertTrue(any("no release note" in problem for problem in problems))

    def test_tag_mismatch_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "2026-11-07" / "v0.6.0.md",
                release_note("v0.6.1", "2026-11-07"),
            )
            write(root / "CHANGELOG.md", changelog())
            problems = check_releases.check(root)
            self.assertTrue(any("expected v0.6.0" in problem for problem in problems))

    def test_bad_release_file_name_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "2026-11-07" / "release.md",
                release_note("v0.6.0", "2026-11-07"),
            )
            write(root / "CHANGELOG.md", changelog())
            problems = check_releases.check(root)
            self.assertTrue(any("vMAJOR.MINOR.PATCH" in problem for problem in problems))


if __name__ == "__main__":
    unittest.main()
