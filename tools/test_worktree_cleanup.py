#!/usr/bin/env python3
"""Tests for the merged-worktree cleanup guard."""

from __future__ import annotations

import json
import subprocess
import tempfile
import unittest
from pathlib import Path

from worktree_cleanup import (
    CleanupError,
    archive_worktree,
    inspect_worktree,
    remove_worktree,
)


def git(repo: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(repo), *arguments],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout


def make_repo(root: Path) -> Path:
    repo = root / "repo"
    repo.mkdir()
    git(repo, "init", "-b", "main")
    git(repo, "config", "user.email", "test@example.com")
    git(repo, "config", "user.name", "Test")
    (repo / ".gitignore").write_text(
        "/data/**\n!/data/**/\n!/data/**/README.md\n", encoding="utf-8"
    )
    (repo / "data").mkdir()
    (repo / "data" / "README.md").write_text("local data\n", encoding="utf-8")
    (repo / "README.md").write_text("repo\n", encoding="utf-8")
    git(repo, "add", ".")
    git(repo, "commit", "-m", "init")
    return repo


def add_worktree(repo: Path, root: Path) -> Path:
    worktree = root / "worktree"
    git(repo, "worktree", "add", "-b", "feat/task", str(worktree), "main")
    return worktree


class WorktreeCleanupTest(unittest.TestCase):
    def test_clean_worktree(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            repo = make_repo(Path(directory))
            worktree = add_worktree(repo, Path(directory))
            self.assertEqual(inspect_worktree(worktree)["status"], "clean")

    def test_local_data_and_archive(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            worktree = add_worktree(repo, root)
            local = worktree / "data" / "local.txt"
            local.write_text("keep me\n", encoding="utf-8")
            check = inspect_worktree(worktree)
            self.assertEqual(check["status"], "local_data")
            manifest = archive_worktree(worktree, root / "archive", "task")
            self.assertEqual(manifest["file_count"], 1)
            archived = Path(manifest["destination"]) / "data" / "local.txt"
            self.assertTrue(archived.is_file())
            self.assertEqual(archived.read_text(encoding="utf-8"), "keep me\n")
            manifest_path = root / "archive" / "task" / manifest["archived_utc"] / "archive-manifest.json"
            self.assertTrue(manifest_path.is_file())
            self.assertEqual(json.loads(manifest_path.read_text())["file_count"], 1)

    def test_nested_cache_is_disposable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            worktree = add_worktree(repo, root)
            cache = worktree / "tools" / "__pycache__"
            cache.mkdir(parents=True)
            (cache / "x.pyc").write_bytes(b"cache")
            self.assertEqual(inspect_worktree(worktree)["status"], "clean")

    def test_untracked_blocker(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            worktree = add_worktree(repo, root)
            (worktree / "notes.txt").write_text("unclassified\n", encoding="utf-8")
            check = inspect_worktree(worktree)
            self.assertEqual(check["status"], "blocker")
            with self.assertRaises(CleanupError):
                archive_worktree(worktree, root / "archive", "task")

    def test_unmerged_branch_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            worktree = add_worktree(repo, root)
            (worktree / "work.txt").write_text("work\n", encoding="utf-8")
            git(worktree, "add", "work.txt")
            git(worktree, "commit", "-m", "work")
            with self.assertRaises(CleanupError):
                remove_worktree(
                    worktree,
                    "feat/task",
                    root / "archive",
                    main_branch="main",
                    yes=True,
                )
            self.assertTrue(worktree.is_dir())

    def test_merged_removal_archives_and_deletes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            repo = make_repo(root)
            worktree = add_worktree(repo, root)
            (worktree / "data" / "local.txt").write_text("keep me\n", encoding="utf-8")
            result = remove_worktree(
                worktree,
                "feat/task",
                root / "archive",
                main_branch="main",
                yes=True,
            )
            self.assertEqual(result["status"], "removed")
            self.assertFalse(worktree.exists())
            self.assertTrue((root / "archive" / "worktree" / result["archive"]["archived_utc"]).is_dir())
            branches = git(repo, "branch", "--list", "feat/task")
            self.assertEqual(branches.strip(), "")


if __name__ == "__main__":
    unittest.main()
