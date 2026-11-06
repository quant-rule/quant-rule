#!/usr/bin/env python3
"""Check the structural rules that the Refactor Workflow depends on.

The hard rules are deliberately narrow: every rule already passes on the tree
that introduced this checker. A rule that needs a cleanup is reported as
advisory instead of blocking the build.

The report is the scan input for a refactor assessment. It measures size,
surface, documentation, layout, and naming; it does not replace the assessment
or the independent verifier.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ROOT / "crates"
DOCS = ROOT / "docs"
ENGINE_CRATES = ("quantrule-data", "quantrule-model", "quantrule-backtest")
FORBIDDEN_ENGINE_IMPORTS = (
    "use std::fs",
    "use std::net",
    "use std::process",
    "use std::env",
    "use std::time::SystemTime",
    "use std::time::Instant",
    "use rand",
    "use std::thread",
    "use std::sync::Mutex",
    "static mut ",
)
FORBIDDEN_RUST_FILES = {"utils.rs", "helpers.rs", "common.rs", "misc.rs", "manager.rs"}
RUST_FILE = re.compile(r"^[a-z0-9_]+\.rs$")
DATE_DIR = re.compile(r"^\d{4}-\d{2}-\d{2}$")
REFACTOR_DOC = re.compile(r"^[a-z0-9-]+\.md$")
TEST_FILE = re.compile(r"(^tests\.rs$|_tests\.rs$)")
DATA_ALLOWED = re.compile(
    r"^(data/README\.md|data/[^/]+/README\.md|data/partitions\.json|"
    r"data/improvement/index\.(json|md))$"
)


@dataclass(frozen=True)
class Violation:
    """One structural rule failure with a reproducible location."""

    rule: str
    path: str
    message: str

    def render(self) -> str:
        return f"{self.rule}: {self.path}: {self.message}"


@dataclass(frozen=True)
class FileMetric:
    """Line count for one Rust source file."""

    path: str
    lines: int


def relative(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


def is_test_file(path: Path) -> bool:
    """Return true for a test support file or a file under a tests directory."""
    return bool(TEST_FILE.search(path.name)) or "tests" in path.parts


def rust_sources(root: Path) -> list[Path]:
    return sorted((root / "crates").glob("*/src/**/*.rs"))


def file_lines(path: Path) -> int:
    text = path.read_text(encoding="utf-8", errors="ignore")
    return text.count("\n") + (0 if text.endswith("\n") else 1)


def starts_with_module_doc(path: Path) -> bool:
    for line in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        if not line.strip():
            continue
        return line.lstrip().startswith("//!")
    return False


def check_module_docs(root: Path) -> list[Violation]:
    violations = []
    for path in rust_sources(root):
        if is_test_file(path):
            continue
        if not starts_with_module_doc(path):
            violations.append(
                Violation(
                    "module-doc",
                    relative(root, path),
                    "source module must start with a //! module doc",
                )
            )
    return violations


def check_unsafe_forbid(root: Path) -> list[Violation]:
    violations = []
    for crate in sorted((root / "crates").glob("quantrule-*")):
        roots = [crate / "src" / "lib.rs", crate / "src" / "main.rs"]
        found = [path for path in roots if path.exists()]
        if not found:
            violations.append(
                Violation("unsafe-forbid", relative(root, crate), "crate has no lib.rs or main.rs")
            )
            continue
        for path in found:
            text = path.read_text(encoding="utf-8", errors="ignore")
            if "#![forbid(unsafe_code)]" not in text:
                violations.append(
                    Violation(
                        "unsafe-forbid",
                        relative(root, path),
                        "crate root must contain #![forbid(unsafe_code)]",
                    )
                )
    return violations


def check_engine_purity(root: Path) -> list[Violation]:
    violations = []
    for crate in ENGINE_CRATES:
        directory = root / "crates" / crate / "src"
        if not directory.exists():
            violations.append(Violation("engine-purity", crate, "engine source directory is missing"))
            continue
        for path in sorted(directory.rglob("*.rs")):
            if is_test_file(path):
                continue
            text = path.read_text(encoding="utf-8", errors="ignore")
            for pattern in FORBIDDEN_ENGINE_IMPORTS:
                if pattern in text:
                    violations.append(
                        Violation(
                            "engine-purity",
                            relative(root, path),
                            f"engine source must not contain {pattern!r}",
                        )
                    )
    return violations


def check_naming(root: Path) -> list[Violation]:
    violations = []
    for path in rust_sources(root):
        if not RUST_FILE.match(path.name):
            violations.append(
                Violation(
                    "rust-file-name",
                    relative(root, path),
                    "Rust source file name must be snake_case",
                )
            )
        if path.name in FORBIDDEN_RUST_FILES:
            violations.append(
                Violation(
                    "catch-all-module",
                    relative(root, path),
                    "catch-all module names are not allowed",
                )
            )
    refactor_root = root / "docs" / "refactors"
    if refactor_root.exists():
        for path in sorted(refactor_root.rglob("*.md")):
            if path.name != "README.md" and not REFACTOR_DOC.match(path.name):
                violations.append(
                    Violation(
                        "refactor-doc-name",
                        relative(root, path),
                        "refactor document name must be kebab-case",
                    )
                )
    return violations


def check_dated_docs(root: Path) -> list[Violation]:
    docs = root / "docs"
    violations = []
    if not docs.exists():
        return violations
    for directory in sorted(docs.glob("*/*")):
        if not directory.is_dir():
            continue
        if not DATE_DIR.match(directory.name):
            violations.append(
                Violation(
                    "dated-docs-directory",
                    relative(root, directory),
                    "second-level docs directory must be a UTC YYYY-MM-DD date",
                )
            )
    return violations


def workflow_index_violations(root: Path) -> list[Violation]:
    index = root / "docs" / "README.md"
    workflows = root / "docs" / "workflows"
    if not index.exists() or not workflows.exists():
        return []
    text = index.read_text(encoding="utf-8", errors="ignore")
    violations = []
    for path in sorted(workflows.glob("*.md")):
        if path.name not in text:
            violations.append(
                Violation(
                    "workflow-index",
                    relative(root, path),
                    "workflow document is not listed in docs/README.md",
                )
            )
    return violations


def data_allowlist_violations(paths: list[str]) -> list[Violation]:
    return [
        Violation("data-allowlist", path, "committed data/ file is not a README or partition index")
        for path in paths
        if not DATA_ALLOWED.match(path)
    ]


def committed_data_paths(root: Path) -> list[str]:
    try:
        result = subprocess.run(
            ["git", "-C", str(root), "ls-files", "data"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError):
        return []
    return [line for line in result.stdout.splitlines() if line]


def check_data_allowlist(root: Path) -> list[Violation]:
    return data_allowlist_violations(committed_data_paths(root))


def check_all(root: Path) -> list[Violation]:
    violations = []
    violations.extend(check_module_docs(root))
    violations.extend(check_unsafe_forbid(root))
    violations.extend(check_engine_purity(root))
    violations.extend(check_naming(root))
    violations.extend(check_dated_docs(root))
    violations.extend(workflow_index_violations(root))
    violations.extend(check_data_allowlist(root))
    return violations


def markdown_report(root: Path) -> str:
    sources = rust_sources(root)
    metrics = [FileMetric(relative(root, path), file_lines(path)) for path in sources]
    metrics.sort(key=lambda metric: (-metric.lines, metric.path))
    crates = sorted({path.relative_to(root).parts[1] for path in sources})
    lines = ["## Structural report", "", "| Item | Observed |", "|---|---|"]
    lines.append(f"| Rust source files | {len(sources)} |")
    lines.append(f"| Rust source lines | {sum(metric.lines for metric in metrics)} |")
    lines.append(f"| Engine crates | {', '.join(ENGINE_CRATES)} |")
    lines.append(f"| Workspace crates | {len(crates)} |")
    lines.append("")
    lines.append("### Largest Rust source files")
    lines.append("")
    lines.append("| File | Lines |")
    lines.append("|---|---:|")
    for metric in metrics[:15]:
        lines.append(f"| `{metric.path}` | {metric.lines} |")
    over_600 = sum(1 for metric in metrics if metric.lines > 600)
    over_800 = sum(1 for metric in metrics if metric.lines > 800)
    over_1000 = sum(1 for metric in metrics if metric.lines > 1000)
    lines.append("")
    lines.append("### Size thresholds")
    lines.append("")
    lines.append("| Threshold | Files |")
    lines.append("|---|---:|")
    lines.append(f"| over 600 lines | {over_600} |")
    lines.append(f"| over 800 lines | {over_800} |")
    lines.append(f"| over 1,000 lines | {over_1000} |")
    lines.append("")
    lines.append("### Surface and documentation")
    lines.append("")
    lines.append("| Item | Observed |")
    lines.append("|---|---|")
    public = sum(1 for path in sources for line in path.read_text(encoding="utf-8", errors="ignore").splitlines() if line.lstrip().startswith("pub "))
    crate_public = sum(1 for path in sources for line in path.read_text(encoding="utf-8", errors="ignore").splitlines() if line.lstrip().startswith("pub(crate)"))
    unwrap = sum(
        1
        for path in sources
        if not is_test_file(path)
        for line in path.read_text(encoding="utf-8", errors="ignore").splitlines()
        if ".unwrap()" in line or ".expect(" in line
    )
    lines.append(f"| `pub` items | {public} |")
    lines.append(f"| `pub(crate)` items | {crate_public} |")
    lines.append(f"| `unwrap`/`expect` lines in non-test source | {unwrap} |")
    lines.append(f"| Workflow documents | {len(list((root / 'docs' / 'workflows').glob('*.md')))} |")
    lines.append(f"| Refactor documents | {len(list((root / 'docs' / 'refactors').rglob('*.md')))} |")
    lines.append("")
    return "\n".join(lines)


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="check_structure.py",
        description="Check the Refactor Workflow structural rules and print a scorecard.",
    )
    parser.add_argument(
        "--report",
        action="store_true",
        help="print the Markdown structural scorecard and do not fail on advisory metrics",
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=ROOT,
        help="repository root (default: the parent of tools/)",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> int:
    arguments = parse_arguments(argv)
    root = arguments.root.resolve()
    if arguments.report:
        print(markdown_report(root))
        return 0
    violations = check_all(root)
    if violations:
        for violation in violations:
            print(f"error: {violation.render()}", file=sys.stderr)
        return 1
    print("structure ok: module docs, engine purity, naming, layout, and data allowlist pass")
    return 0


if __name__ == "__main__":
    sys.exit(main())
