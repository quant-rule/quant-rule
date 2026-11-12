#!/usr/bin/env python3
"""Fail when a retired model, strategy, or architecture label appears in a
current artifact.

History directories are not scanned: ADRs, audits, improvement records, AI
transparency logs, execution plans, releases, and reports keep their original
names. The naming registry is the one current document that must name the old
aliases; it is allowlisted explicitly.

The M2.x research evidence under ``config/research/`` predates the cutover and
still names retired schemas. It is historical evidence, not a current run
config, and is the only remaining pending path. The scan covers current Rust
source, README/docs, and config files; negative tests that prove retired
schemas or commands are rejected are allowlisted explicitly.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

SCAN = [
    ROOT / "README.md",
    ROOT / "docs/README.md",
    ROOT / "docs/architecture",
    ROOT / "docs/model/2026-11-05",
    ROOT / "docs/config",
    ROOT / "config/defaults",
    ROOT / "config/examples",
    ROOT / "config/research",
    ROOT / "crates",
]

# The alias registry must name the old labels to map them.
# Negative assertions that prove retired schemas are rejected.
ALLOWLIST = {
    "docs/registry/history.md",
    "crates/quantrule-shell/tests/architecture.rs",
    "crates/quantrule-app/src/config/tests.rs",
    "crates/quantrule-shell/tests/fixtures/config-run/model-v3.json",
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
# Tauri application that embedded the app service is not.
ARCH_TOKENS = [
    "download page",
    "static download",
    "Linux installer",
    "AppImage",
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


def files() -> list[Path]:
    found: list[Path] = []
    for entry in SCAN:
        if entry.is_file():
            found.append(entry)
        elif entry.is_dir():
            found.extend(path for path in entry.rglob("*") if path.is_file())
    return sorted(set(found))


def relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def main() -> int:
    errors: list[str] = []
    for path in files():
        name = relative(path)
        if name in ALLOWLIST or name.startswith(PENDING):
            continue
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except UnicodeDecodeError:
            continue
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
    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1
    print(f"legacy scan ok: {len(files())} current files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
