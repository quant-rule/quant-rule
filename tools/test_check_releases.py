#!/usr/bin/env python3
"""Unit tests for the release-note and changelog alignment checker."""

from __future__ import annotations

import json
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


def releases_readme(mapping: str, table: list[str] | None = None) -> str:
    """Build the renumbering table and the current-tag map."""
    if table is None:
        table = list(json.loads(mapping))
    rows = "\n".join(f"| `{tag}` | `abc1234` | `{tag}` |" for tag in table)
    return (
        "# Releases and Tags\n\n"
        "### Renumbering (2026-11-15)\n\n"
        "| Current tag | Commit | Was |\n"
        "|---|---|---|\n"
        f"{rows}\n\n"
        "<!-- current-tags:start -->\n"
        "```json\n"
        f"{mapping}\n"
        "```\n"
        "<!-- current-tags:end -->\n\n"
        "## 4. Historical tags\n"
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


class CurrentTagMapTests(unittest.TestCase):
    def test_a_renumbered_tag_resolves_to_its_historical_note(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            note = root / "docs" / "releases" / "2026-11-15" / "v1.1.0.md"
            write(note, release_note("v1.1.0", "2026-11-15"))
            write(
                root / "docs" / "releases" / "README.md",
                releases_readme('{"v1.0.0": "2026-11-15/v1.1.0.md"}'),
            )
            tags, problems = check_releases.current_tags(root)
            self.assertEqual(problems, [])
            self.assertEqual(tags, {"v1.0.0": "2026-11-15/v1.1.0.md"})
            self.assertEqual(check_releases.resolve(root, "v1.0.0"), note)

    def test_a_missing_map_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "README.md",
                "# Releases and Tags\n\n### Renumbering (2026-11-15)\n\n## 4. Historical tags\n",
            )
            _, problems = check_releases.current_tags(root)
            self.assertTrue(any("missing the <!-- current-tags:start -->" in p for p in problems))

    def test_invalid_map_json_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "README.md",
                releases_readme('{"v1.0.0": }', table=[]),
            )
            _, problems = check_releases.current_tags(root)
            self.assertTrue(any("not valid JSON" in problem for problem in problems))

    def test_a_duplicate_map_key_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "2026-11-15" / "v1.1.0.md",
                release_note("v1.1.0", "2026-11-15"),
            )
            write(
                root / "docs" / "releases" / "README.md",
                releases_readme(
                    '{"v1.0.0": "2026-11-15/v1.1.0.md",'
                    ' "v1.0.0": "2026-11-15/v1.1.0.md"}'
                ),
            )
            _, problems = check_releases.current_tags(root)
            self.assertTrue(any("appears twice" in problem for problem in problems))

    def test_a_map_key_must_be_a_tag(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "2026-11-15" / "v1.1.0.md",
                release_note("v1.1.0", "2026-11-15"),
            )
            write(
                root / "docs" / "releases" / "README.md",
                releases_readme('{"latest": "2026-11-15/v1.1.0.md"}', table=[]),
            )
            _, problems = check_releases.current_tags(root)
            self.assertTrue(
                any("not vMAJOR.MINOR.PATCH" in problem for problem in problems)
            )

    def test_a_map_target_must_exist(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "README.md",
                releases_readme('{"v1.0.0": "2026-11-15/v9.9.9.md"}'),
            )
            _, problems = check_releases.current_tags(root)
            self.assertTrue(
                any("points to missing note" in problem for problem in problems)
            )

    def test_a_map_target_must_stay_inside_releases(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "README.md",
                releases_readme('{"v1.0.0": "../CHANGELOG.md"}'),
            )
            _, problems = check_releases.current_tags(root)
            self.assertTrue(
                any("points outside docs/releases" in problem for problem in problems)
            )

    def test_the_map_must_name_every_renumbered_tag(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "2026-11-15" / "v1.1.0.md",
                release_note("v1.1.0", "2026-11-15"),
            )
            write(
                root / "docs" / "releases" / "README.md",
                releases_readme(
                    '{"v1.0.0": "2026-11-15/v1.1.0.md"}',
                    table=["v0.9.0", "v1.0.0"],
                ),
            )
            _, problems = check_releases.current_tags(root)
            self.assertTrue(
                any("missing v0.9.0" in problem for problem in problems)
            )

    def test_the_map_cannot_name_a_tag_the_table_does_not(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "2026-11-15" / "v1.1.0.md",
                release_note("v1.1.0", "2026-11-15"),
            )
            write(
                root / "docs" / "releases" / "README.md",
                releases_readme(
                    '{"v0.9.0": "2026-11-15/v1.1.0.md",'
                    ' "v1.0.0": "2026-11-15/v1.1.0.md"}',
                    table=["v1.0.0"],
                ),
            )
            _, problems = check_releases.current_tags(root)
            self.assertTrue(
                any("the renumbering table does not" in problem for problem in problems)
            )

    def test_an_unmapped_tag_does_not_resolve(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "docs" / "releases" / "2026-11-15" / "v1.1.0.md",
                release_note("v1.1.0", "2026-11-15"),
            )
            write(
                root / "docs" / "releases" / "README.md",
                releases_readme('{"v1.0.0": "2026-11-15/v1.1.0.md"}'),
            )
            with self.assertRaises(ValueError):
                check_releases.resolve(root, "v9.9.9")


if __name__ == "__main__":
    unittest.main()
