#!/usr/bin/env python3
"""Resolve the persistent local data root for the simulation tools.

The simulation store is deliberately outside the repository and every Git
worktree. A task worktree may write code and run the tools, but its ignored
``data/`` directory is not a safe home for the only copy of a report or a mark.

The root is, in order:

1. ``--data-root``;
2. ``QUANTRULE_DATA_ROOT``;
3. ``$HOME/quant-rule-data``.

The simulation store is ``<root>/simulation``; archives live in
``<root>/worktree-archive``. A ``{data_root}`` placeholder in a run or check
config is replaced with the resolved root.
"""

from __future__ import annotations

import os
import subprocess
from pathlib import Path

DEFAULT_DIRECTORY = "quant-rule-data"
STORE_DIRECTORY = "simulation"
ARCHIVE_DIRECTORY = "worktree-archive"
PLACEHOLDER = "{data_root}"


class DataRootError(Exception):
    """The data root is invalid or would write into the current worktree."""


def default_root() -> Path:
    """Return the default local data root."""
    return Path.home() / DEFAULT_DIRECTORY


def resolve_root(explicit: str | Path | None = None) -> Path:
    """Resolve the root from the argument, the environment, or the default."""
    if explicit is not None and str(explicit).strip():
        return Path(explicit).expanduser().resolve()
    environment = os.environ.get("QUANTRULE_DATA_ROOT", "").strip()
    if environment:
        return Path(environment).expanduser().resolve()
    return default_root().expanduser().resolve()


def current_worktree() -> Path:
    """Return the current Git worktree root, or the working directory."""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return Path.cwd().resolve()
    return Path(result.stdout.strip()).resolve()


def main_worktree() -> Path:
    """Return the first worktree in ``git worktree list``, or the current one."""
    try:
        result = subprocess.run(
            ["git", "worktree", "list", "--porcelain"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return current_worktree()
    for line in result.stdout.splitlines():
        if line.startswith("worktree "):
            return Path(line[len("worktree ") :]).resolve()
    return current_worktree()


def is_inside(path: Path, parent: Path) -> bool:
    """Return whether ``path`` is ``parent`` or below it."""
    try:
        path.resolve().relative_to(parent.resolve())
        return True
    except ValueError:
        return False


def guard_root(root: Path, allow_worktree: bool = False) -> None:
    """Refuse a root inside the current worktree unless explicitly allowed."""
    if allow_worktree:
        return
    if is_inside(root, current_worktree()):
        raise DataRootError(
            f"the data root {root} is inside the current worktree; "
            "choose the external root or pass --allow-worktree-data"
        )


def ensure_root(root: Path) -> Path:
    """Create the root and its simulation/archive directories."""
    (root / STORE_DIRECTORY).mkdir(parents=True, exist_ok=True)
    (root / ARCHIVE_DIRECTORY).mkdir(parents=True, exist_ok=True)
    return root


def substitute(value: str | Path, root: Path) -> str:
    """Replace the ``{data_root}`` placeholder with the resolved root."""
    return str(value).replace(PLACEHOLDER, str(root))
