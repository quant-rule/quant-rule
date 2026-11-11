#!/usr/bin/env python3
"""Check, archive, and remove a merged task worktree safely.

The tool exists because local data under ``data/`` is ignored by Git and lives
only in the worktree. ``git worktree remove`` deletes it. The cleanup rule
requires a check and an archive before removal.

Commands:
    check   classify the worktree contents; no deletion
    archive copy local data to a dated archive and write a manifest; no deletion
    remove  check, archive, verify the branch is merged, then remove; --yes required

Usage:
    python3 tools/worktree_cleanup.py check ../quant-rule--task
    python3 tools/worktree_cleanup.py archive ../quant-rule--task --dest /path/archive
    python3 tools/worktree_cleanup.py remove ../quant-rule--task --branch fix/task --yes
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

DISPOSABLE_PREFIXES = (
    "target/",
    "__pycache__/",
    ".venv/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".ruff_cache/",
)
DISPOSABLE_SUFFIXES = (".pyc", ".pyo")
LOCAL_DATA_PREFIX = "data/"
CLASSIFICATION_SCHEMA = "quantrule.worktree-cleanup/v1"


class CleanupError(Exception):
    """The worktree cannot be classified, archived, or removed safely."""


def _git(worktree: Path, *arguments: str) -> str:
    try:
        result = subprocess.run(
            ["git", "-C", str(worktree), *arguments],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as error:
        raise CleanupError(f"git {' '.join(arguments)} failed in {worktree}: {error}") from error
    return result.stdout


def _status_entries(worktree: Path) -> list[tuple[str, str]]:
    """Return (status, relative path) entries, including ignored files."""
    output = subprocess.run(
        ["git", "-C", str(worktree), "status", "--porcelain=v1", "--ignored=matching", "-z"],
        check=True,
        capture_output=True,
    ).stdout
    entries: list[tuple[str, str]] = []
    for chunk in output.split(b"\0"):
        if not chunk:
            continue
        status = chunk[:2].decode("utf-8", "replace")
        path = chunk[3:].decode("utf-8", "replace")
        if path:
            entries.append((status, path))
    return entries


def _classify(path: str, status: str) -> str:
    """Return ``local_data``, ``disposable``, ``tracked``, or ``blocker``."""
    if status not in ("??", "!!"):
        return "tracked"
    if path.startswith(LOCAL_DATA_PREFIX):
        return "local_data"
    if path.startswith(DISPOSABLE_PREFIXES) or path.endswith(DISPOSABLE_SUFFIXES):
        return "disposable"
    if status == "!!":
        return "blocker"
    return "blocker"


def inspect_worktree(worktree: Path) -> dict[str, Any]:
    """Classify a worktree and return the check record."""
    if not worktree.is_dir():
        raise CleanupError(f"{worktree} is not a directory")
    if not (worktree / ".git").exists():
        raise CleanupError(f"{worktree} is not a Git worktree")
    groups: dict[str, list[str]] = {
        "local_data": [],
        "disposable": [],
        "tracked": [],
        "blocker": [],
    }
    for status, path in _status_entries(worktree):
        groups[_classify(path, status)].append(path)
    if groups["tracked"]:
        status = "blocker"
    elif groups["blocker"]:
        status = "blocker"
    elif groups["local_data"]:
        status = "local_data"
    else:
        status = "clean"
    return {
        "schema": CLASSIFICATION_SCHEMA,
        "worktree": str(worktree.resolve()),
        "status": status,
        "local_data": sorted(groups["local_data"]),
        "disposable": sorted(groups["disposable"]),
        "tracked": sorted(groups["tracked"]),
        "blockers": sorted(groups["blocker"] + groups["tracked"]),
    }


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def archive_worktree(worktree: Path, dest: Path, label: str | None = None) -> dict[str, Any]:
    """Copy local data to a dated archive and write a manifest. No deletion."""
    check = inspect_worktree(worktree)
    if check["blockers"]:
        raise CleanupError(
            f"{worktree} has untracked or modified files that are not local data: "
            f"{check['blockers']}"
        )
    if not check["local_data"]:
        return {
            "schema": CLASSIFICATION_SCHEMA,
            "worktree": check["worktree"],
            "status": "nothing_to_archive",
            "files": 0,
            "bytes": 0,
        }
    label = label or worktree.name
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    target = dest.resolve() / label / stamp
    if target.exists():
        raise CleanupError(f"archive target {target} already exists")
    files = []
    total = 0
    for relative in check["local_data"]:
        source = worktree / relative
        if source.is_dir():
            candidates = [path for path in source.rglob("*") if path.is_file()]
        else:
            candidates = [source]
        for candidate in candidates:
            relative_path = candidate.relative_to(worktree).as_posix()
            destination = target / relative_path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(candidate, destination)
            size = destination.stat().st_size
            total += size
            files.append(
                {
                    "path": relative_path,
                    "bytes": size,
                    "sha256": _sha256(destination),
                }
            )
    manifest = {
        "schema": CLASSIFICATION_SCHEMA,
        "worktree": check["worktree"],
        "destination": str(target),
        "label": label,
        "archived_utc": stamp,
        "source_commit": _git(worktree, "rev-parse", "HEAD").strip(),
        "files": files,
        "file_count": len(files),
        "bytes": total,
        "check_status": check["status"],
    }
    target.mkdir(parents=True, exist_ok=True)
    (target / "archive-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return manifest


def _main_worktree(worktree: Path) -> Path:
    output = _git(worktree, "worktree", "list", "--porcelain")
    for line in output.splitlines():
        if line.startswith("worktree "):
            return Path(line[len("worktree ") :]).resolve()
    raise CleanupError("could not find the main worktree")


def _branch_merged(main: Path, branch: str, main_branch: str) -> bool:
    result = subprocess.run(
        ["git", "-C", str(main), "merge-base", "--is-ancestor", branch, main_branch]
    )
    return result.returncode == 0


def remove_worktree(
    worktree: Path,
    branch: str,
    archive_root: Path,
    main_branch: str = "main",
    remote: str | None = None,
    yes: bool = False,
) -> dict[str, Any]:
    """Archive local data, verify the branch is merged, then remove."""
    if not yes:
        raise CleanupError("pass --yes to remove a worktree and branch")
    main = _main_worktree(worktree)
    if worktree.resolve() == main:
        raise CleanupError("refusing to remove the main worktree")
    check = inspect_worktree(worktree)
    if check["blockers"]:
        raise CleanupError(f"worktree has blockers: {check['blockers']}")
    if not _branch_merged(main, branch, main_branch):
        raise CleanupError(f"branch {branch} is not merged into {main_branch}")
    archive = None
    if check["local_data"]:
        archive = archive_worktree(worktree, archive_root, label=worktree.name)
    _git(main, "worktree", "remove", str(worktree))
    _git(main, "branch", "-d", branch)
    if remote:
        subprocess.run(
            ["git", "-C", str(main), "push", remote, "--delete", branch],
            check=True,
        )
    return {
        "schema": CLASSIFICATION_SCHEMA,
        "status": "removed",
        "worktree": str(worktree),
        "branch": branch,
        "archive": archive,
    }


def _print_json(value: Any) -> None:
    print(json.dumps(value, indent=2, sort_keys=True))


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    check_parser = sub.add_parser("check")
    check_parser.add_argument("worktree", type=Path)
    archive_parser = sub.add_parser("archive")
    archive_parser.add_argument("worktree", type=Path)
    archive_parser.add_argument("--dest", required=True, type=Path)
    archive_parser.add_argument("--label")
    remove_parser = sub.add_parser("remove")
    remove_parser.add_argument("worktree", type=Path)
    remove_parser.add_argument("--branch", required=True)
    remove_parser.add_argument("--archive-root", required=True, type=Path)
    remove_parser.add_argument("--main-branch", default="main")
    remove_parser.add_argument("--remote", default=None)
    remove_parser.add_argument("--yes", action="store_true")
    args = parser.parse_args(argv[1:])
    try:
        if args.command == "check":
            result = inspect_worktree(args.worktree.resolve())
            _print_json(result)
            return 0 if result["status"] == "clean" else 2
        if args.command == "archive":
            result = archive_worktree(args.worktree.resolve(), args.dest, args.label)
            _print_json(result)
            return 0
        result = remove_worktree(
            args.worktree.resolve(),
            args.branch,
            args.archive_root.resolve(),
            args.main_branch,
            args.remote,
            args.yes,
        )
        _print_json(result)
        return 0
    except (CleanupError, OSError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
