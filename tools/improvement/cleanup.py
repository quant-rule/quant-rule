#!/usr/bin/env python3
"""Report retired workflow terms and legacy run paths in repository text.

The command is read-only. It scans ``docs/``, the root README, and ``data/`` by
default, prints every match with its file and line, and exits 1 when any match
exists. It never deletes or rewrites a file.
"""

from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

from common import REPO_ROOT, is_within, repo_relative

DEFAULT_PATHS = ("docs", "README.md", "data")
DEFAULT_EXCLUDES: tuple[str, ...] = ()
BANNED_TERMS = (
    "Continuous Improvement",
    "continuous improvement",
    "Governing / Experiment Control Swarm",
)
LEGACY_PATHS = (
    "data/labs",
    "data/reports",
    "data/research",
)
SKIP_DIRECTORIES = {".git", "target", "node_modules", "__pycache__", ".venv", "venv"}
MAX_BYTES = 5 * 1024 * 1024
EXCERPT_LIMIT = 160


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="cleanup.py",
        description=(
            "Check docs/, README.md, and data/ for retired workflow terms and "
            "legacy run paths (data/labs, data/reports, data/research). Prints "
            "findings and exits 1; changes nothing."
        ),
        epilog=(
            "Example: python3 tools/improvement/cleanup.py --check\n"
            "Optional --paths and --exclude narrow the scan; paths are "
            "repository-relative."
        ),
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="run the read-only scan (required)",
    )
    parser.add_argument(
        "--paths",
        nargs="+",
        default=list(DEFAULT_PATHS),
        metavar="PATH",
        help="files or directories to scan (default: docs README.md data)",
    )
    parser.add_argument(
        "--exclude",
        nargs="+",
        default=list(DEFAULT_EXCLUDES),
        metavar="PATH",
        help="files or directories to skip",
    )
    parser.add_argument(
        "--terms-only",
        action="store_true",
        help="check retired workflow terms only, not legacy data paths",
    )
    args = parser.parse_args(argv)
    if not args.check:
        parser.error("--check is required; this command only reports, it never deletes")
    return args


def resolve_repository_path(value: str) -> Path:
    path = Path(value).expanduser()
    if not path.is_absolute():
        path = REPO_ROOT / path
    return path.resolve()


def collect_files(roots: list[Path], excludes: list[Path]) -> list[Path]:
    files: set[Path] = set()
    for root in roots:
        if not root.exists():
            raise FileNotFoundError(root)
        if root.is_file():
            if not any(root == excluded or is_within(root, excluded) for excluded in excludes):
                files.add(root)
            continue
        for directory, names, filenames in os.walk(root):
            current = Path(directory)
            names[:] = sorted(
                name
                for name in names
                if name not in SKIP_DIRECTORIES
                and not any(
                    (current / name).resolve() == excluded
                    or is_within(current / name, excluded)
                    for excluded in excludes
                )
            )
            for name in sorted(filenames):
                path = current / name
                if any(
                    path.resolve() == excluded or is_within(path, excluded)
                    for excluded in excludes
                ):
                    continue
                files.add(path.resolve())
    return sorted(files, key=lambda path: repo_relative(path))


def scan_file(
    path: Path, terms_only: bool = False
) -> tuple[list[tuple[int, str, str]], str | None]:
    """Return findings and a skip reason, if the file cannot be read as text."""
    try:
        size = path.stat().st_size
    except OSError:
        return [], "unreadable"
    if size > MAX_BYTES:
        return [], "large"
    try:
        data = path.read_bytes()
    except OSError:
        return [], "unreadable"
    if b"\x00" in data[:4096]:
        return [], "binary"
    text = data.decode("utf-8", errors="replace")
    terms = BANNED_TERMS if terms_only else BANNED_TERMS + LEGACY_PATHS
    findings: list[tuple[int, str, str]] = []
    for number, line in enumerate(text.splitlines(), 1):
        if not any(term in line for term in terms):
            continue
        excerpt = " ".join(line.split())
        if len(excerpt) > EXCERPT_LIMIT:
            excerpt = excerpt[: EXCERPT_LIMIT - 3] + "..."
        for term in terms:
            if term in line:
                findings.append((number, term, excerpt))
    return findings, None


def main(argv: list[str] | None = None) -> int:
    args = parse_arguments(argv)
    roots = [resolve_repository_path(value) for value in args.paths]
    excludes = [resolve_repository_path(value) for value in args.exclude]
    try:
        files = collect_files(roots, excludes)
    except FileNotFoundError as error:
        print(f"error: scan path not found: {error.args[0]}", file=sys.stderr)
        return 1

    findings: list[tuple[str, int, str, str]] = []
    skipped = 0
    for path in files:
        matches, reason = scan_file(path, args.terms_only)
        if reason is not None:
            skipped += 1
            continue
        display = repo_relative(path)
        for number, term, excerpt in matches:
            findings.append((display, number, term, excerpt))

    if findings:
        for display, number, term, excerpt in findings:
            print(f"{display}:{number}: {term}: {excerpt}")
        print(f"{len(findings)} finding(s) in {len(files)} file(s)")
        return 1

    suffix = f"; {skipped} binary, large, or unreadable file(s) skipped" if skipped else ""
    print(f"checked {len(files)} file(s); no banned terms{suffix}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
