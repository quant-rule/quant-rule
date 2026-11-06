#!/usr/bin/env python3
"""Check that the documented command surface matches the implemented shell.

The source of truth is ``crates/quantrule-shell/src/command/help.rs``. The
checker reads its ``CommandHelp`` entries, the README command table, the command
registry table in ``docs/architecture/command-surface.md``, and the fenced code
blocks under ``docs/tutorials``. It fails when:

- an implemented command is missing from ``help.rs``, the README, or a tutorial;
- a README row or an ``Implemented`` registry row names a command that
  ``help.rs`` does not have;
- a command marked ``Planned`` appears in ``help.rs`` or the README;
- a tutorial invokes a command that is not implemented;
- a ``CommandHelp`` entry has an empty usage, summary, or output contract, or a
  usage line that does not start with the command name.

The checker has no dependencies beyond the Python standard library and reads no
network, clock, or external data.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HELP = ROOT / "crates/quantrule-shell/src/command/help.rs"
README = ROOT / "README.md"
SURFACE = ROOT / "docs/architecture/command-surface.md"
TUTORIALS = ROOT / "docs/tutorials"

ENTRY = re.compile(r"CommandHelp\s*\{")
FIELD = re.compile(r'^\s*(name|usage|summary|output):\s*"([^"]*)",\s*$', re.MULTILINE)


def fail(message: str) -> None:
    print(f"error: {message}", file=sys.stderr)


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        fail(f"cannot read {path.relative_to(ROOT)}: {error}")
        raise SystemExit(1)


def help_commands() -> dict[str, dict[str, str]]:
    """Return the CommandHelp entries keyed by command name."""
    text = read(HELP)
    start = text.find("const COMMANDS")
    if start < 0:
        fail(f"{HELP.relative_to(ROOT)} has no COMMANDS registry")
        raise SystemExit(1)
    end = text.find("];", start)
    if end < 0:
        fail(f"{HELP.relative_to(ROOT)} has an unterminated COMMANDS registry")
        raise SystemExit(1)

    commands: dict[str, dict[str, str]] = {}
    for entry in ENTRY.split(text[start:end])[1:]:
        fields = dict(FIELD.findall(entry))
        missing = [key for key in ("name", "usage", "summary", "output") if not fields.get(key)]
        if missing:
            fail(f"a CommandHelp entry is missing {', '.join(missing)}")
            raise SystemExit(1)
        name = fields["name"]
        if name in commands:
            fail(f"help.rs documents '{name}' twice")
            raise SystemExit(1)
        tokens = name.split()
        usage_tokens = fields["usage"].split()
        if usage_tokens[: len(tokens)] != tokens:
            fail(
                f"help.rs usage for '{name}' does not start with the command name: "
                f"{fields['usage']!r}"
            )
            raise SystemExit(1)
        commands[name] = fields
    if not commands:
        fail("help.rs documents no commands")
        raise SystemExit(1)
    return commands


def heading_level(line: str) -> int:
    """Return the Markdown heading level, or zero when the line is not a heading."""
    match = re.match(r"^(#{1,6})\s", line)
    return len(match.group(1)) if match else 0


def section(text: str, title: str) -> str:
    """Return the body of the first heading whose text contains ``title``.

    The section includes its subheadings and stops at the next heading of the
    same or a higher level.
    """
    lines = text.splitlines()
    start = None
    level = 0
    for index, line in enumerate(lines):
        current = heading_level(line)
        if current and title.lower() in line.lower():
            start = index + 1
            level = current
            break
    if start is None:
        fail(f"no heading contains {title!r}")
        raise SystemExit(1)
    end = len(lines)
    for index in range(start, len(lines)):
        current = heading_level(lines[index])
        if current and current <= level:
            end = index
            break
    return "\n".join(lines[start:end])


def readme_commands() -> list[str]:
    """Return the command names in the README's Commands table."""
    body = section(read(README), "Commands")
    names = re.findall(r"^\|\s*`([^`]+)`\s*\|", body, re.MULTILINE)
    if not names:
        fail("README.md has no command table under a Commands heading")
        raise SystemExit(1)
    return names


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


def surface_commands() -> dict[str, str]:
    """Return command -> status from the command-surface registry table."""
    body = section(read(SURFACE), "Command registry")
    lines = body.splitlines()
    header = None
    for index, line in enumerate(lines):
        if line.strip().startswith("| Command |"):
            header = index
            break
    if header is None:
        fail("command-surface.md has no '| Command |' table under Command registry")
        raise SystemExit(1)

    columns = table_cells(lines[header])
    if "Command" not in columns or "Status" not in columns:
        fail("command-surface.md registry table needs Command and Status columns")
        raise SystemExit(1)

    statuses: dict[str, str] = {}
    for line in lines[header + 2 :]:
        if not line.strip().startswith("|"):
            break
        cells = table_cells(line)
        if len(cells) != len(columns):
            fail(f"command-surface.md registry row has the wrong column count: {line}")
            raise SystemExit(1)
        row = dict(zip(columns, cells))
        command = row["Command"].strip().strip("`")
        status = row["Status"]
        if command in statuses:
            fail(f"command-surface.md documents '{command}' twice")
            raise SystemExit(1)
        if status not in {"Implemented", "Planned"}:
            fail(f"command-surface.md status for '{command}' must be Implemented or Planned: {status!r}")
            raise SystemExit(1)
        statuses[command] = status
    if not statuses:
        fail("command-surface.md has an empty command registry")
        raise SystemExit(1)
    return statuses


def fenced_blocks(path: Path) -> list[str]:
    """Return the text of every fenced code block in one Markdown file."""
    blocks: list[str] = []
    current: list[str] = []
    in_fence = False
    for line in read(path).splitlines():
        if line.strip().startswith("```"):
            if in_fence:
                blocks.append("\n".join(current))
                current = []
            in_fence = not in_fence
            continue
        if in_fence:
            current.append(line)
    if in_fence:
        fail(f"{path.relative_to(ROOT)} has an unterminated code fence")
        raise SystemExit(1)
    return blocks


def invocations(commands: list[str]) -> dict[str, list[str]]:
    """Return the tutorials that invoke each command inside a code block."""
    found = {command: [] for command in commands}
    files = sorted(TUTORIALS.rglob("*.md")) if TUTORIALS.is_dir() else []
    for path in files:
        for block in fenced_blocks(path):
            for command in commands:
                tokens = command.split()
                pattern = re.compile(
                    r"(?<![\w-])quantrule\s+"
                    + r"\s+".join(re.escape(token) for token in tokens)
                    + r"(?=\s|$)",
                    re.MULTILINE,
                )
                if pattern.search(block):
                    found[command].append(path.relative_to(ROOT).as_posix())
    return found


def tutorial_unimplemented(commands: set[str]) -> list[str]:
    """Return ``quantrule`` invocations that are not an implemented command."""
    problems: list[str] = []
    files = sorted(TUTORIALS.rglob("*.md")) if TUTORIALS.is_dir() else []
    pattern = re.compile(r"(?<![\w-])quantrule\s+(.+)", re.MULTILINE)
    for path in files:
        for block in fenced_blocks(path):
            for match in pattern.finditer(block):
                rest = match.group(1).strip()
                if rest == "quit" or rest.startswith("quit "):
                    continue
                if not any(
                    rest == command or rest.startswith(command + " ")
                    for command in commands
                ):
                    problems.append(f"{path.relative_to(ROOT)}: {rest!r}")
    return problems


def compare(label: str, expected: set[str], actual: set[str]) -> bool:
    ok = True
    for name in sorted(expected - actual):
        fail(f"{label} is missing implemented command '{name}'")
        ok = False
    for name in sorted(actual - expected):
        fail(f"{label} lists '{name}', which help.rs does not implement")
        ok = False
    return ok


def main() -> int:
    implemented = help_commands()
    implemented_names = set(implemented)
    readme = readme_commands()
    registry = surface_commands()

    ok = True
    duplicate_readme = sorted({name for name in readme if readme.count(name) > 1})
    for name in duplicate_readme:
        fail(f"README.md documents '{name}' twice")
        ok = False
    ok = compare("README.md", implemented_names, set(readme)) and ok

    registry_implemented = {name for name, status in registry.items() if status == "Implemented"}
    registry_planned = {name for name, status in registry.items() if status == "Planned"}
    ok = compare("command-surface.md", implemented_names, registry_implemented) and ok

    for name in sorted(registry_planned & implemented_names):
        fail(f"command-surface.md marks implemented command '{name}' as Planned")
        ok = False
    for name in sorted(registry_planned & set(readme)):
        fail(f"README.md lists planned command '{name}' as available")
        ok = False

    found = invocations(sorted(implemented_names))
    for name in sorted(implemented_names):
        if not found[name]:
            fail(f"no tutorial code block invokes implemented command '{name}'")
            ok = False

    for problem in tutorial_unimplemented(implemented_names):
        fail(f"tutorial invokes an unimplemented command: {problem}")
        ok = False

    if not ok:
        return 1

    tutorial_files = sorted(TUTORIALS.rglob("*.md")) if TUTORIALS.is_dir() else []
    print(
        "command surface ok: "
        f"{len(implemented_names)} implemented, {len(registry_planned)} planned; "
        f"README rows {len(readme)}; tutorial files {len(tutorial_files)}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
