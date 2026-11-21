#!/usr/bin/env python3
"""Fail when a retired model, strategy, architecture label, or tool path appears
in a current artifact.

History directories are not scanned: ADRs, audits, improvement records, AI
transparency logs, execution plans, releases, and reports keep their original
names. The naming registry is the one current document that must name the old
aliases; it is allowlisted explicitly.

The four Python research directories retired by DIST-105 are checked as paths
across every current file, not only the label-scan set. The port-parity registry,
the changelog, and the history index are allowlisted because they record the
retired source or the removal by contract.

The M2.x research evidence under ``config/research/`` predates the cutover and
still names retired schemas. It is historical evidence, not a current run
config, and is the only remaining pending path. The scan covers current Rust
source, the README, the architecture, workflow, tutorial, governance, model, and
config documents, and config files; negative tests that prove retired schemas or
commands are rejected are allowlisted explicitly.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

SCAN = [
    "AGENTS.md",
    "README.md",
    "docs/README.md",
    "docs/architecture",
    "docs/workflows",
    "docs/tutorials",
    "docs/governance",
    "docs/model/2026-11-05",
    "docs/config",
    "config/defaults",
    "config/examples",
    "config/research",
    "crates",
]

# The alias registry must name the old labels to map them.
# Negative assertions that prove retired schemas are rejected.
# The retirement and release records must name what they retired, and a release
# note may state that a retired distribution does not exist.
ALLOWLIST = {
    "docs/registry/history.md",
    "crates/quantrule-shell/tests/architecture.rs",
    "crates/quantrule-app/src/config/tests.rs",
    "crates/quantrule-shell/tests/fixtures/config-run/model-v3.json",
    # The research-ledger migration seeds the committed invalidation tombstones
    # and quotes the retired label that the tombstone recorded.
    "crates/quantrule-evidence-service/migrations/0016_research_ledger.sql",
    "crates/quantrule-evidence-service/migrations/sqlite/0004_research_ledger.sql",
    "docs/workflows/deprecation-retirement.md",
    "docs/workflows/release-operations.md",
    "crates/quantrule-governance/src/checks/legacy.rs",
}

# M2.x research evidence predates the cutover and references retired files.
# It is historical evidence, not a current run config.
PENDING = ("config/research/",)

TOKENS = [
    "model v3",
    "model v4",
    "quantrule.model/v1",
    "quantrule.model/v2",
    "quantrule.model/v3",
    "strategy v2",
    "strategy v3",
    "strategy v4",
    "strategy v5",
    "strategy v6",
    "strategy v7",
    "strategy v8",
    "strategy v9",
    "strategy v10",
    "quantrule.strategy/v2",
    "quantrule.strategy/v3",
    "quantrule.strategy/v4",
    "quantrule.strategy/v5",
    "quantrule.strategy/v6",
    "quantrule.strategy/v7",
    "quantrule.strategy/v8",
    "v5 programme",
    # Removed architecture: the analytics crate, axis modules, dashboard,
    # consensus view, market report, and their types have no current surface.
    "quantrule_analytics",
    "quantrule-analytics",
    "quantrule_model::axes",
    "crate::axes",
    "crate::dashboard",
    "crate::consensus",
    "MarketReport",
    "DashboardState",
    "TimeframeConsensus",
    "AxisContext",
    "AgreementCode",
]
PATTERN = re.compile("|".join(re.escape(token) for token in TOKENS), re.IGNORECASE)

# Retired architecture terms. The retired *shape* is banned, not the vendor word:
# a Tauri shell over the gateway URL is current (ADR-2026-11-12-12), while the
# Tauri application that embedded the app service is not. The AppImage and NSIS
# artifacts are the current desktop distribution, so the artifact name is allowed.
ARCH_TOKENS = [
    "download page",
    "static download",
    "Linux installer",
    "apps/desktop/src-tauri",
    "Tauri IPC",
    "Tauri desktop application",
    "five services",
    "eight containers",
    "local only",
    "single process",
    "shell-only fetch",
    "SQLite memory",
]
ARCH_PATTERN = re.compile(
    "|".join(re.escape(token) for token in ARCH_TOKENS), re.IGNORECASE
)

# DIST-105 retired the Python simulation harness and its companion research
# tools. A current document, script, or source file that names one is treating a
# deleted tool as live, so the four paths fail the gate when reintroduced.
RETIRED_PATHS = (
    "tools/simulation",
    "tools/charts",
    "tools/oracle",
    "tools/trades",
)
RETIRED_PATH_PATTERN = re.compile(
    r"(?<![\w-])("
    + "|".join(re.escape(path) for path in RETIRED_PATHS)
    + r")(?![\w-])"
)

# These files name the retired paths on purpose:
# - the changelog records what was removed in a released or unreleased entry;
# - the port-parity registry and fixture name the captured source by contract;
# - the history index is the retired-path record, like the alias registry above;
# - the checker and its test carry the patterns and the seeded violation.
RETIRED_PATH_ALLOWLIST = {
    "CHANGELOG.md",
    "docs/port-parity/registry.json",
    "docs/port-parity/PORT-001.golden.json",
    "docs/registry/history.md",
    "tools/check_legacy.py",
    "tools/test_check_legacy.py",
    "crates/quantrule-governance/src/checks/legacy.rs",
}

# Current documents and source, minus the history kinds and the local data root.
PATH_SKIP_ROOT = {".git", "target", "data"}
PATH_SKIP_PARTS = {"node_modules", "__pycache__", ".venv", "venv"}
PATH_HISTORY = (
    "docs/adr/",
    "docs/audits/",
    "docs/ai-transparency-log/",
    "docs/execution-plans/",
    "docs/reports/",
    "docs/releases/",
    "docs/improvement/",
    "docs/course/",
)
PATH_MAX_BYTES = 2 * 1024 * 1024


def files(root: Path) -> list[Path]:
    found: list[Path] = []
    for entry in SCAN:
        path = root / entry
        if path.is_file():
            found.append(path)
        elif path.is_dir():
            found.extend(item for item in path.rglob("*") if item.is_file())
    return sorted(set(found))


def candidate_paths(root: Path) -> list[Path]:
    """Return tracked plus untracked non-ignored files, or a full walk outside Git."""
    command = [
        "git",
        "-C",
        str(root),
        "ls-files",
        "--cached",
        "--others",
        "--exclude-standard",
        "--full-name",
        "-z",
    ]
    try:
        output = subprocess.check_output(command, stderr=subprocess.DEVNULL)
    except (OSError, subprocess.CalledProcessError):
        return sorted(root.rglob("*"))
    return [root / name for name in output.decode("utf-8", "surrogateescape").split("\0") if name]


def current_files(root: Path) -> list[Path]:
    """Every readable text file outside history, the local data root, and builds."""
    found: list[Path] = []
    for path in candidate_paths(root):
        relative = path.relative_to(root)
        name = relative.as_posix()
        if relative.parts[0] in PATH_SKIP_ROOT:
            continue
        if any(part in PATH_SKIP_PARTS for part in relative.parts):
            continue
        if name.startswith(PATH_HISTORY):
            continue
        if not path.is_file():
            continue
        try:
            if path.stat().st_size > PATH_MAX_BYTES:
                continue
            data = path.read_bytes()
        except OSError:
            continue
        if b"\x00" in data[:4096]:
            continue
        found.append(path)
    return found


def relative(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


def scan(path: Path, name: str) -> list[str]:
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except UnicodeDecodeError:
        return []
    errors: list[str] = []
    for number, line in enumerate(lines, 1):
        match = PATTERN.search(line)
        if match:
            errors.append(f"{name}:{number}: retired label {match.group(0)!r}")
            continue
        match = ARCH_PATTERN.search(line)
        if match:
            errors.append(
                f"{name}:{number}: retired architecture term {match.group(0)!r}"
            )
    return errors


def check(root: Path) -> list[str]:
    errors: list[str] = []
    label_files = files(root)
    for path in label_files:
        name = relative(root, path)
        if name in ALLOWLIST or name.startswith(PENDING):
            continue
        errors.extend(scan(path, name))
    for path in current_files(root):
        name = relative(root, path)
        if name in RETIRED_PATH_ALLOWLIST:
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue
        for number, line in enumerate(lines, 1):
            match = RETIRED_PATH_PATTERN.search(line)
            if match:
                errors.append(
                    f"{name}:{number}: retired tool path {match.group(0)!r}"
                )
    return errors


def main() -> int:
    errors = check(ROOT)
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(
        f"legacy scan ok: {len(files(ROOT))} label files, "
        f"{len(current_files(ROOT))} current files"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
