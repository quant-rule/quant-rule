#!/usr/bin/env python3
"""Check the user manual against the command and client registries.

The manual is both a teaching surface and a discovery gate. This checker keeps
it from drifting away from the product:

- every chapter named in ``docs/manual/README.md`` exists, is linked, and carries
  a status line;
- every implemented command in ``help.rs`` has a row in the command reference;
- every screen in ``client.md`` and every Workbench tab has a row in the GUI
  reference;
- each journey chapter carries the six required headings;
- each journey chapter names one of the four evidence labels.

It reads only files in the repository: no network, clock, or external data.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

SECTIONS = ("Goal", "Before you start", "Steps", "What you see", "What it proves", "Failure")
LABEL_PATTERN = re.compile(
    r"governed evidence|\bevidence\b|\bdiagnostic\b|\bverification\b", re.IGNORECASE
)
HEADING = re.compile(r"^##\s+(?:[0-9]+\.\s+)?(.+?)\s*$", re.MULTILINE)
STATUS = re.compile(r"^- Status:\s*\S+", re.MULTILINE)
TAB = re.compile(r'role="tab"[\s\S]*?>\s*([A-Za-z][A-Za-z ]*?)\s*</button>')


def read(path: Path) -> str:
    """Read a UTF-8 document, returning an empty string when it is absent."""
    try:
        return path.read_text(encoding="utf-8")
    except OSError:
        return ""


def table_cells(line: str) -> list[str]:
    """Split one Markdown table row, keeping escaped pipes inside a cell."""
    text = line.strip()
    if text.startswith("|"):
        text = text[1:]
    if text.endswith("|") and not text.endswith("\\|"):
        text = text[:-1]
    cells: list[str] = []
    current: list[str] = []
    escaped = False
    for character in text:
        if escaped:
            current.append(character)
            escaped = False
        elif character == "\\":
            escaped = True
        elif character == "|":
            cells.append("".join(current).strip())
            current = []
        else:
            current.append(character)
    if escaped:
        current.append("\\")
    cells.append("".join(current).strip())
    return cells


def table_rows(text: str, header: str) -> list[list[str]]:
    """Read the pipe-table rows below a header line."""
    start = text.find(header)
    if start == -1:
        return []
    lines = text[start:].splitlines()[2:]
    rows: list[list[str]] = []
    for line in lines:
        if not line.strip().startswith("|"):
            break
        rows.append(table_cells(line))
    return rows


def chapter_rows(root: Path) -> list[list[str]]:
    """Read the chapter table; columns are #, Chapter, File, Answers, Status."""
    text = read(root / "docs" / "manual" / "README.md")
    for line in text.splitlines():
        if line.strip().startswith("| # | Chapter |") and "File" in line:
            return table_rows(text, line)
    return []


def chapter_file(cell: str) -> str:
    """Return the file name from a linked or backticked table cell."""
    link = re.search(r"\[[^\]]*\]\(([^)]+)\)", cell)
    if link is not None:
        return link.group(1).strip()
    return cell.strip().strip("`")


def help_commands(root: Path) -> list[str]:
    """Return the implemented command names from the shell registry."""
    text = read(root / "crates" / "quantrule-shell" / "src" / "command" / "help.rs")
    start = text.find("const COMMANDS")
    end = text.find("];", start + 1) if start != -1 else -1
    if start == -1 or end == -1:
        return []
    return re.findall(r'^\s*name:\s*"([^"]+)",', text[start:end], re.MULTILINE)


def help_usage(root: Path) -> dict[str, str]:
    """Return command -> usage from the shell registry."""
    text = read(root / "crates" / "quantrule-shell" / "src" / "command" / "help.rs")
    start = text.find("const COMMANDS")
    end = text.find("];", start + 1) if start != -1 else -1
    if start == -1 or end == -1:
        return {}
    usage: dict[str, str] = {}
    for entry in text[start:end].split("CommandHelp {")[1:]:
        name = re.search(r'name:\s*"([^"]+)"', entry)
        line = re.search(r'usage:\s*"([^"]*)"', entry)
        if name is not None and line is not None:
            usage[name.group(1)] = line.group(1)
    return usage


def row_usage(text: str, name: str) -> str | None:
    """Return the second cell of the table row whose first cell is ``name``."""
    for line in text.splitlines():
        if not line.strip().startswith("|"):
            continue
        cells = table_cells(line)
        if len(cells) >= 2 and cells[0].strip("`") == name:
            return cells[1].strip("`")
    return None


def screens(root: Path) -> list[str]:
    """Return the screen names from the client document's screen table."""
    text = read(root / "docs" / "architecture" / "client.md")
    rows = table_rows(text, "| Screen | Component | Shows | Empty state |")
    return [row[0] for row in rows if row and row[0]]


def workbench_tabs(root: Path) -> list[str]:
    """Return the Workbench tab labels from the component."""
    text = read(root / "apps" / "web" / "src" / "lib" / "WorkbenchPanel.svelte")
    return [match.strip() for match in TAB.findall(text)]


def has_row(text: str, name: str) -> bool:
    """Whether a table has a first cell that is exactly ``name``."""
    for line in text.splitlines():
        if not line.strip().startswith("|"):
            continue
        cells = table_cells(line)
        if cells and cells[0].strip("`") == name:
            return True
    return False


def heading_body(text: str, title: str) -> str | None:
    """Return the body of a level-2 heading, or ``None`` when it is absent."""
    lines = text.splitlines()
    start = None
    for index, line in enumerate(lines):
        match = HEADING.match(line)
        if match is not None and match.group(1).lower() == title.lower():
            start = index + 1
            break
    if start is None:
        return None
    end = len(lines)
    for index in range(start, len(lines)):
        if lines[index].startswith("## "):
            end = index
            break
    return "\n".join(lines[start:end])


def check(root: Path) -> list[str]:
    """Return the manual problems for ``root``; an empty list is a pass."""
    problems: list[str] = []
    manual = root / "docs" / "manual"
    readme = read(manual / "README.md")
    if not readme:
        return [f"{manual / 'README.md'}: the manual index is missing"]

    rows = chapter_rows(root)
    if not rows:
        problems.append(f"{manual / 'README.md'}: no chapter table with a Status column")
    seen: set[str] = set()
    for row in rows:
        if len(row) < 5:
            problems.append(f"{manual / 'README.md'}: a chapter row has fewer than five columns")
            continue
        number, _chapter, file_cell, _answers, status = row[:5]
        file_name = chapter_file(file_cell)
        if not file_name:
            problems.append(f"{manual / 'README.md'}: chapter {number} names no file")
            continue
        if file_name in seen:
            problems.append(f"{manual / 'README.md'}: chapter file {file_name} is listed twice")
        seen.add(file_name)
        if f"]({file_name})" not in readme:
            problems.append(f"{manual / 'README.md'}: chapter {file_name} is not linked")
        path = manual / file_name
        if not path.is_file():
            problems.append(f"{path}: the chapter file is missing")
            continue
        text = read(path)
        if not STATUS.search(text):
            problems.append(f"{path}: missing a '- Status:' line")
        if status.strip().lower() != "current":
            problems.append(f"{manual / 'README.md'}: chapter {file_name} is not Current")
        if number.strip().isdigit() and int(number.strip()) <= 8:
            for title in SECTIONS:
                if heading_body(text, title) is None:
                    problems.append(f"{path}: missing the '{title}' section")
            proves = heading_body(text, "What it proves")
            if proves is not None:
                found = {
                    match.group(0).lower()
                    for match in LABEL_PATTERN.finditer(proves)
                }
                if not found:
                    problems.append(f"{path}: 'What it proves' names no evidence label")

    command_text = read(manual / "09-command-reference.md")
    commands = help_commands(root)
    usage = help_usage(root)
    if not commands:
        problems.append(f"{root / 'crates/quantrule-shell/src/command/help.rs'}: no commands")
    for command in commands:
        if not has_row(command_text, command):
            problems.append(f"{manual / '09-command-reference.md'}: no row for '{command}'")
            continue
        documented = row_usage(command_text, command)
        expected = usage.get(command)
        if documented is None or expected is None:
            problems.append(f"{manual / '09-command-reference.md'}: no usage for '{command}'")
        elif documented.replace("\\|", "|") != expected:
            problems.append(
                f"{manual / '09-command-reference.md'}: usage for '{command}' is "
                f"{documented!r}, registry has {expected!r}"
            )

    gui_text = read(manual / "10-gui-reference.md")
    screen_names = screens(root)
    if not screen_names:
        problems.append(f"{root / 'docs/architecture/client.md'}: no screen table rows")
    for screen in screen_names:
        if not has_row(gui_text, screen):
            problems.append(f"{manual / '10-gui-reference.md'}: no row for screen '{screen}'")
    tab_names = workbench_tabs(root)
    if not tab_names:
        problems.append(f"{root / 'apps/web/src/lib/WorkbenchPanel.svelte'}: no tabs")
    for tab in tab_names:
        label = f"Workbench — {tab}"
        if not has_row(gui_text, label):
            problems.append(f"{manual / '10-gui-reference.md'}: no row for '{label}'")
    return problems


def main() -> int:
    """Run the checker and print one stable summary line."""
    problems = check(ROOT)
    if problems:
        for problem in problems:
            print(f"error: {problem}", file=sys.stderr)
        return 1
    rows = chapter_rows(ROOT)
    print(
        "manual ok: "
        f"{len(rows)} chapters, {len(help_commands(ROOT))} commands, "
        f"{len(screens(ROOT))} screens, {len(workbench_tabs(ROOT))} tabs"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
