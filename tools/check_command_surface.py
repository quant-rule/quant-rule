#!/usr/bin/env python3
"""Check that the documented command surface matches the implemented shell.

The source of truth is ``crates/quantrule-shell/src/command/help.rs``. The
checker reads its ``CommandHelp`` entries, the README command table, the command
registry table in ``docs/architecture/command-surface.md``, the fenced code
blocks under ``docs/tutorials``, and the command list in the ``AGENTS.md``
current-surface paragraph. It fails when:

- an implemented command is missing from ``help.rs``, the README, or a tutorial;
- a README row or an ``Implemented`` registry row names a command that
  ``help.rs`` does not have;
- a command marked ``Planned`` appears in ``help.rs`` or the README;
- a tutorial, workflow, or ``AGENTS.md`` block invokes a command that is not
  implemented;
- the ``AGENTS.md`` current-surface list differs from the implemented set;
- a ``CommandHelp`` entry has an empty usage, summary, or output contract, or a
  usage line that does not start with the command name.

The checker has no dependencies beyond the Python standard library and reads no
network, clock, or external data.
"""

from __future__ import annotations

import re
import shlex
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
HELP = ROOT / "crates/quantrule-shell/src/command/help.rs"
README = ROOT / "README.md"
SURFACE = ROOT / "docs/architecture/command-surface.md"
TUTORIALS = ROOT / "docs/tutorials"
WORKFLOWS = ROOT / "docs/workflows"
CONFIG = ROOT / "docs/config"
ARCHITECTURE = ROOT / "docs/architecture"
NORMATIVE = ROOT / "AGENTS.md"
NORMATIVE_MARKER = "The current tree implements"
COMMAND_SPAN = re.compile(r"`([a-z][a-z ]*)`")

# The retired local surface. The command-surface document is the authority; keep
# this list in step with its "Removed" section when a command is retired.
RETIRED = (
    "candles",
    "candles-tolerant",
    "timeframes",
    "fetch",
    "events",
    "report model",
    "strategy explain",
    "strategy run",
    "execution run",
    "portfolio replay",
    "backtest run",
    "qualify",
    "analytics evaluate",
    "run check",
    "memory",
)
RETIRED_SPAN = re.compile(
    "`(" + "|".join(re.escape(name) for name in RETIRED) + ")`"
)
STATUS_LINE = re.compile(r"^- Status:\s*(\S+)", re.MULTILINE)

ENTRY = re.compile(r"CommandHelp\s*\{")
FIELD = re.compile(r'^\s*(name|usage|summary|output):\s*"([^"]*)",\s*$', re.MULTILINE)


def display(path: Path) -> str:
    """Render a path relative to the repository when it lives inside it."""
    try:
        return path.relative_to(ROOT).as_posix()
    except ValueError:
        return str(path)


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


def usage_shape(usage: str, name: str) -> dict[str, object]:
    """Return the positional range and the known options from one usage line.

    The usage strings are regular enough to parse without a full grammar: a
    bracketed token is optional, an angle token is a positional, and an option
    is followed by an angle token when it takes a value. The check catches the
    error class the docs actually drifted into — a flag that the shell does not
    implement or a positional count that cannot be satisfied.
    """
    rest = usage[len(name) :].strip()
    tokens: list[tuple[str, bool]] = []
    current: list[str] = []
    depth = 0
    optional = False
    for character in rest:
        if character == "[":
            depth += 1
            current.append(character)
            optional = True
        elif character == "]":
            current.append(character)
            depth = max(0, depth - 1)
        elif character.isspace():
            if current:
                tokens.append(("".join(current), optional))
                current = []
                optional = False
        else:
            if not current and depth > 0:
                optional = True
            current.append(character)
    if current:
        tokens.append(("".join(current), optional or depth > 0))

    required_positionals = 0
    optional_positionals = 0
    required_options: set[str] = set()
    optional_options: set[str] = set()
    index = 0
    while index < len(tokens):
        raw, optional = tokens[index]
        token = raw.strip("[]")
        if token.startswith("--"):
            option = token[2:].split("=", 1)[0]
            next_token = (
                tokens[index + 1][0].strip("[]") if index + 1 < len(tokens) else ""
            )
            takes_value = bool(next_token) and not next_token.startswith("--")
            if optional:
                optional_options.add(option)
            else:
                required_options.add(option)
            if takes_value:
                index += 2
                continue
        elif token.startswith("<") or "|" in token or token in {"all", "glossary", "workflows"}:
            if optional:
                optional_positionals += 1
            else:
                required_positionals += 1
        index += 1

    max_positionals = required_positionals + optional_positionals
    # ``help`` accepts a two-word command name such as ``run submit`` even
    # though the usage line shows one optional topic token.
    if name == "help":
        max_positionals = max(max_positionals, 2)

    return {
        "required_positionals": required_positionals,
        "max_positionals": max_positionals,
        "required_options": required_options,
        "optional_options": optional_options,
    }


def invocation_errors(
    commands: dict[str, dict[str, str]], files: list[Path]
) -> list[str]:
    """Validate documented ``quantrule`` invocations against the usage lines."""
    problems: list[str] = []
    global_value_options = {"profile", "token"}
    global_flags = {"json"}
    for path in files:
        for block in fenced_blocks(path):
            logical_lines: list[str] = []
            pending = ""
            for line in block.splitlines():
                part = line.strip()
                if pending:
                    pending = f"{pending} {part}"
                else:
                    pending = part
                if pending.endswith("\\"):
                    pending = pending[:-1].rstrip()
                    continue
                logical_lines.append(pending)
                pending = ""
            if pending:
                logical_lines.append(pending)
            for stripped in logical_lines:
                if not stripped.startswith("quantrule "):
                    continue
                try:
                    words = shlex.split(stripped)
                except ValueError as error:
                    problems.append(f"{display(path)}: cannot parse {stripped!r}: {error}")
                    continue
                if len(words) < 2:
                    continue
                matched = next(
                    (
                        name
                        for name in sorted(commands, key=len, reverse=True)
                        if words[1 : 1 + len(name.split())] == name.split()
                    ),
                    None,
                )
                if matched is None:
                    continue
                shape = usage_shape(commands[matched]["usage"], matched)
                args = words[1 + len(matched.split()) :]
                positionals = 0
                found_options: set[str] = set()
                index = 0
                while index < len(args):
                    token = args[index]
                    if token.startswith("--"):
                        option = token[2:].split("=", 1)[0]
                        if option in global_flags:
                            index += 1
                            continue
                        if option in global_value_options:
                            if "=" not in token:
                                if index + 1 >= len(args):
                                    problems.append(
                                        f"{display(path)}: --{option} has no value in {stripped!r}"
                                    )
                                index += 2
                            else:
                                index += 1
                            continue
                        if option not in shape["required_options"] and option not in shape["optional_options"]:
                            problems.append(
                                f"{display(path)}: unknown option '--{option}' in {stripped!r}"
                            )
                            index += 1
                            continue
                        found_options.add(option)
                        takes_value = (
                            index + 1 < len(args)
                            and not args[index + 1].startswith("--")
                        )
                        index += 2 if takes_value else 1
                        continue
                    positionals += 1
                    index += 1
                if positionals < shape["required_positionals"] or positionals > shape["max_positionals"]:
                    problems.append(
                        f"{display(path)}: {stripped!r} has {positionals} positional(s); "
                        f"expected {shape['required_positionals']}..{shape['max_positionals']}"
                    )
                missing = shape["required_options"] - found_options
                if missing:
                    problems.append(
                        f"{display(path)}: {stripped!r} is missing required option(s) "
                        + ", ".join(f"--{name}" for name in sorted(missing))
                    )
    return problems


def current_status(path: Path) -> bool:
    """Whether a document is current rather than a historical record."""
    text = read(path)
    match = STATUS_LINE.search(text)
    if match is None:
        return True
    return not match.group(1).startswith(("Historical", "Deprecated", "Superseded"))


def retired_backticks(files: list[Path]) -> list[str]:
    """Return retired command names backticked in current documents."""
    problems: list[str] = []
    for path in files:
        if not current_status(path):
            continue
        for line_number, line in enumerate(read(path).splitlines(), 1):
            for match in RETIRED_SPAN.finditer(line):
                # A line that explicitly retires the name is a history note.
                if any(
                    word in line.lower()
                    for word in ("removed", "retired", "not current", "historical")
                ):
                    continue
                problems.append(
                    f"{display(path)}:{line_number}: retired command {match.group(1)!r} in a current contract"
                )
    return problems


def retired_config_commands() -> list[str]:
    """Return retired command names backticked in a current config contract."""
    if not CONFIG.is_dir():
        return []
    return retired_backticks(sorted(CONFIG.rglob("*.md")))


def retired_architecture_commands() -> list[str]:
    """Return retired command names backticked in a current architecture doc.

    ``command-surface.md`` is the retirement record itself and ``client.md``
    uses ``fetch`` as the browser API verb, so both are allowlisted. Every other
    current architecture document must name the gateway surface.
    """
    if not ARCHITECTURE.is_dir():
        return []
    allowed = {"command-surface.md", "client.md"}
    files = [path for path in sorted(ARCHITECTURE.glob("*.md")) if path.name not in allowed]
    return retired_backticks(files)


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


def current_documents() -> list[Path]:
    """The normative entry point, current workflows, and current config contracts."""
    files = [NORMATIVE]
    if WORKFLOWS.is_dir():
        files.extend(sorted(WORKFLOWS.glob("*.md")))
    if CONFIG.is_dir():
        files.extend(sorted(CONFIG.rglob("*.md")))
    return [path for path in files if path.is_file() and current_status(path)]


def current_tutorials() -> list[Path]:
    """The current tutorial files, excluding historical labs."""
    if not TUTORIALS.is_dir():
        return []
    return [path for path in sorted(TUTORIALS.rglob("*.md")) if current_status(path)]


def document_unimplemented(
    commands: set[str], files: list[Path] | None = None
) -> list[str]:
    """Return command invocations in the current documents that are retired.

    The tutorials are checked separately because each implemented command must
    also be demonstrated there. The workflows and ``AGENTS.md`` carry the
    normative procedure, so a command they invoke must be in the registry.
    """
    problems: list[str] = []
    pattern = re.compile(r"(?<![\w-])quantrule\s+(.+)", re.MULTILINE)
    paths = current_documents() if files is None else files
    for path in paths:
        for block in fenced_blocks(path):
            for match in pattern.finditer(block):
                rest = match.group(1).strip()
                if rest == "quit" or rest.startswith("quit "):
                    continue
                if not any(
                    rest == command or rest.startswith(command + " ")
                    for command in commands
                ):
                    problems.append(f"{display(path)}: {rest!r}")
    return problems


def normative_commands(path: Path = NORMATIVE) -> set[str]:
    """Return the command list in the ``AGENTS.md`` current-surface paragraph."""
    text = read(path)
    start = text.find(NORMATIVE_MARKER)
    if start < 0:
        fail(f"{path} has no current-surface paragraph")
        return set()
    end = text.find("\n\n", start)
    paragraph = text[start:] if end < 0 else text[start:end]
    return set(COMMAND_SPAN.findall(paragraph))


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
    ok = compare("AGENTS.md", implemented_names, normative_commands()) and ok

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

    for problem in document_unimplemented(implemented_names):
        fail(f"current document invokes an unimplemented command: {problem}")
        ok = False

    grammar_files = current_documents() + current_tutorials()
    for problem in invocation_errors(implemented, grammar_files):
        fail(f"documented invocation does not parse: {problem}")
        ok = False

    for problem in retired_config_commands():
        fail(f"current contract names a retired command: {problem}")
        ok = False

    for problem in retired_architecture_commands():
        fail(f"current architecture doc names a retired command: {problem}")
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
