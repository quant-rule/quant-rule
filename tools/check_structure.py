#!/usr/bin/env python3
"""Check the structural rules that the Refactor Workflow depends on.

The hard rules are deliberately narrow: every rule already passes on the tree
that introduced this checker. A rule that needs a cleanup is reported as
advisory instead of blocking the build.

The report is the scan input for a refactor assessment. It measures size,
surface, module direction, documentation, layout, and naming; it does not
replace the assessment or the independent verifier.
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
SHELL_STAGE_DIRS = ("analytics", "backtest", "execution", "portfolio", "qualification", "strategy")
FORBIDDEN_ENGINE_MODULES = (
    "std::fs",
    "std::net",
    "std::process",
    "std::env",
    "std::thread",
    "std::sync::Mutex",
    "std::time::SystemTime",
    "std::time::Instant",
    "rand",
)
FORBIDDEN_ENGINE_QUALIFIED = (
    (re.compile(r"\bstd\s*::\s*fs\b"), "std::fs"),
    (re.compile(r"\bstd\s*::\s*net\b"), "std::net"),
    (re.compile(r"\bstd\s*::\s*process\b"), "std::process"),
    (re.compile(r"\bstd\s*::\s*env\b"), "std::env"),
    (re.compile(r"\bstd\s*::\s*thread\b"), "std::thread"),
    (re.compile(r"\bstd\s*::\s*sync\s*::\s*Mutex"), "std::sync::Mutex"),
    (re.compile(r"\bstd\s*::\s*time\s*::\s*SystemTime\b"), "std::time::SystemTime"),
    (re.compile(r"\bstd\s*::\s*time\s*::\s*Instant\b"), "std::time::Instant"),
    (re.compile(r"\brand\s*::"), "rand"),
)
FORBIDDEN_ENGINE_TOKENS = ("static mut ",)
USE = re.compile(r"^(?:pub(?:\([^)]*\))?\s+)?use\s+")
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


def split_use_items(body: str) -> list[str]:
    """Split a use tree's contents on the commas outside nested braces."""
    items: list[str] = []
    depth = 0
    current: list[str] = []
    for character in body:
        if character == "," and depth == 0:
            items.append("".join(current).strip())
            current = []
            continue
        if character == "{":
            depth += 1
        elif character == "}":
            depth -= 1
        current.append(character)
    items.append("".join(current).strip())
    return [item for item in items if item]


def expand_use_tree(item: str, prefix: str = "") -> list[str]:
    """Expand a Rust ``use`` tree into the full paths it imports.

    A grouped import hides the module behind a brace, so a substring test for
    ``use std::fs`` never sees ``use std::{fs, io}``. Expanding the tree to full
    paths makes the grouped, aliased, and nested forms comparable to the plain
    one.
    """
    item = item.strip()
    if not item:
        return []
    if "{" not in item:
        leaf = item.split(" as ", 1)[0].replace(" ", "")
        return [(prefix + leaf).replace("::self", "")]
    head, _, remainder = item.partition("{")
    depth = 1
    index = 0
    while index < len(remainder) and depth:
        if remainder[index] == "{":
            depth += 1
        elif remainder[index] == "}":
            depth -= 1
        index += 1
    inner = remainder[: index - 1]
    paths: list[str] = []
    for part in split_use_items(inner):
        paths.extend(expand_use_tree(part, prefix + head.strip().replace(" ", "")))
    return paths


def use_statements(text: str) -> list[str]:
    """Return each ``use`` declaration, joining a tree that spans lines."""
    statements: list[str] = []
    lines = text.splitlines()
    index = 0
    while index < len(lines):
        match = USE.match(lines[index].strip())
        if match is None:
            index += 1
            continue
        statement = lines[index].strip()
        while ";" not in statement[match.end() :] and index + 1 < len(lines):
            index += 1
            statement += " " + lines[index].strip()
        statements.append(statement[match.end() :].split(";", 1)[0])
        index += 1
    return statements


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
            modules: set[str] = set()
            for statement in use_statements(text):
                for imported in expand_use_tree(statement):
                    for module in FORBIDDEN_ENGINE_MODULES:
                        if imported == module or imported.startswith(f"{module}::"):
                            modules.add(module)
            for pattern, module in FORBIDDEN_ENGINE_QUALIFIED:
                if pattern.search(text):
                    modules.add(module)
            findings = {f"references {module}" for module in modules}
            for token in FORBIDDEN_ENGINE_TOKENS:
                if token in text:
                    findings.add(f"contains {token.strip()!r}")
            for finding in sorted(findings):
                violations.append(
                    Violation(
                        "engine-purity",
                        relative(root, path),
                        f"engine source {finding}",
                    )
                )
    return violations


def check_stage_command_coupling(root: Path) -> list[Violation]:
    """Reject any command-layer reference in the six shell stage modules.

    The command layer depends on the stage modules. The reverse edge would make
    a stage rebuild on a command change and would let a command-layer type or
    writer leak into the stage contract.
    """
    shell = root / "crates" / "quantrule-shell" / "src"
    violations = []
    for stage in SHELL_STAGE_DIRS:
        directory = shell / stage
        if not directory.exists():
            continue
        for path in sorted(directory.rglob("*.rs")):
            text = path.read_text(encoding="utf-8", errors="ignore")
            if "crate::command" in text:
                violations.append(
                    Violation(
                        "stage-command-coupling",
                        relative(root, path),
                        "stage module must not reference the command layer",
                    )
                )
    return violations


def check_artifact_command_coupling(root: Path) -> list[Violation]:
    """Reject a command-layer import in a shared shell artifact module.

    `request` and `model_bundle` are shared by the command layer and the stage
    modules; importing `crate::command` from either would recreate the reverse
    edge the stage rules remove.
    """
    shell = root / "crates" / "quantrule-shell" / "src"
    targets = [shell / "request.rs"]
    model_bundle = shell / "model_bundle"
    if model_bundle.exists():
        targets.extend(sorted(model_bundle.rglob("*.rs")))
    violations = []
    for path in targets:
        if not path.exists():
            continue
        text = path.read_text(encoding="utf-8", errors="ignore")
        if "crate::command" in text:
            violations.append(
                Violation(
                    "artifact-command-coupling",
                    relative(root, path),
                    "shared artifact module must not import the command layer",
                )
            )
    return violations


def check_date_module_coupling(root: Path) -> list[Violation]:
    """Reject the old `source` date-module path and its re-export.

    The date window is shared by configuration, source, and bundle readers; it
    is not a source protocol detail. The rule catches the declaration, the
    re-export, and every importer of the old path.
    """
    shell = root / "crates" / "quantrule-shell" / "src"
    violations = []
    source_date = shell / "source" / "date.rs"
    if source_date.exists():
        violations.append(
            Violation(
                "date-module-coupling",
                relative(root, source_date),
                "the date module must live at crate::date, not under source",
            )
        )
    source_mod = shell / "source" / "mod.rs"
    if source_mod.exists():
        text = source_mod.read_text(encoding="utf-8", errors="ignore")
        if "mod date" in text or "date::{" in text:
            violations.append(
                Violation(
                    "date-module-coupling",
                    relative(root, source_mod),
                    "source must not declare or re-export the date module",
                )
            )
    for path in sorted(shell.rglob("*.rs")):
        text = path.read_text(encoding="utf-8", errors="ignore")
        if (
            "crate::source::date" in text
            or "crate::source::DateBounds" in text
            or "crate::source::DateRange" in text
        ):
            violations.append(
                Violation(
                    "date-module-coupling",
                    relative(root, path),
                    "date types must be imported from crate::date, not crate::source",
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
    templates = docs / "workflows" / "templates"
    for directory in sorted(docs.glob("*/*")):
        if not directory.is_dir():
            continue
        if directory == templates:
            continue
        if directory.parent == docs / "course":
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
    workflows = root / "docs" / "workflows"
    if not workflows.exists():
        return []
    indexes = [
        root / "docs" / "workflows" / "README.md",
    ]
    texts = [
        (path, path.read_text(encoding="utf-8", errors="ignore"))
        for path in indexes
        if path.exists()
    ]
    violations = []
    for path in sorted(workflows.glob("*.md")):
        if path.name == "README.md":
            continue
        for index, text in texts:
            if path.name not in text:
                violations.append(
                    Violation(
                        "workflow-index",
                        relative(root, path),
                        f"workflow document is not listed in {relative(root, index)}",
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
    violations.extend(check_stage_command_coupling(root))
    violations.extend(check_artifact_command_coupling(root))
    violations.extend(check_date_module_coupling(root))
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
