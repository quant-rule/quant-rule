#!/usr/bin/env python3
"""Unit tests for the grouped release changelog checker."""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_changelog


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def changelog(release: str) -> str:
    return f"# Changelog\n\n{release}\n"


class ChangelogCheckerTests(unittest.TestCase):
    def test_valid_unreleased_changelog_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "CHANGELOG.md",
                changelog("## [Unreleased]\n\n### Added\n\n- A meaningful change.\n"),
            )
            self.assertEqual(check_changelog.check(root), [])
            self.assertEqual(check_changelog.counts(root), (1, 1))

    def test_empty_unreleased_is_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "CHANGELOG.md",
                changelog(
                    "## [Unreleased]\n\n"
                    "## [0.6.0] - 2026-11-07\n\n### Added\n\n- Release.\n"
                ),
            )
            self.assertEqual(check_changelog.check(root), [])
            self.assertEqual(check_changelog.counts(root), (2, 1))

    def test_valid_release_changelog_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "CHANGELOG.md",
                changelog(
                    "## [Unreleased]\n\n### Added\n\n- Next change.\n\n"
                    "## [0.6.0] - 2026-11-07\n\n### Changed\n\n- Previous change.\n"
                ),
            )
            self.assertEqual(check_changelog.check(root), [])
            self.assertEqual(check_changelog.counts(root), (2, 2))

    def test_missing_unreleased_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "CHANGELOG.md",
                changelog("## [0.6.0] - 2026-11-07\n\n### Added\n\n- Change.\n"),
            )
            problems = check_changelog.check(root)
            self.assertTrue(any("[Unreleased]" in problem for problem in problems))

    def test_bad_release_heading_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "CHANGELOG.md",
                changelog(
                    "## [Unreleased]\n\n### Added\n\n- Change.\n\n"
                    "## 0.6.0 - 2026-11-07\n\n### Added\n\n- Change.\n"
                ),
            )
            problems = check_changelog.check(root)
            self.assertTrue(any("release" in problem for problem in problems))

    def test_empty_type_section_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "CHANGELOG.md",
                changelog("## [Unreleased]\n\n### Added\n\n### Changed\n\n- Change.\n"),
            )
            problems = check_changelog.check(root)
            self.assertTrue(any("empty" in problem for problem in problems))

    def test_unknown_type_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "CHANGELOG.md",
                changelog("## [Unreleased]\n\n### Improved\n\n- Change.\n"),
            )
            problems = check_changelog.check(root)
            self.assertTrue(any("unknown" in problem for problem in problems))

    def test_duplicate_release_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "CHANGELOG.md",
                changelog(
                    "## [Unreleased]\n\n### Added\n\n- Change.\n\n"
                    "## [0.6.0] - 2026-11-07\n\n### Added\n\n- Change.\n\n"
                    "## [0.6.0] - 2026-11-08\n\n### Added\n\n- Change.\n"
                ),
            )
            problems = check_changelog.check(root)
            self.assertTrue(any("appears twice" in problem for problem in problems))


if __name__ == "__main__":
    unittest.main()
