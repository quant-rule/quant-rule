#!/usr/bin/env python3
"""Fail when the desktop SQLite surface drifts from its declared inventory.

The desktop profile writes one SQLite file. SQLite has no schemas, so every
relation is a prefixed table: `catalog.datasets` is `catalog__datasets`. The
mapping is only safe while three things stay true, and this checker guards all
three:

* each service owns exactly the baseline, ledger, table set, and store module
  that `LEDGERS` below declares, and no other crate gains a SQLite baseline
  that the inventory does not name;
* no baseline uses a Postgres spelling SQLite would accept as an identifier and
  then fail to mean: `CREATE SCHEMA`, `jsonb`, `timestamptz`, `FOR UPDATE`;
* no service source outside its test modules names another service's prefix, so
  one file with six prefixes cannot silently become one shared unowned table.

It also proves each database-owning service branches once on
`QR_PROFILE=desktop` with `QR_DATA_DIR`, opens the one `quantrule.sqlite` file,
and lists its baseline in its own `migrations.rs`: a SQL file that is not listed
is inert. The runtime queue is the one row with no baseline and no service
binary: its adapter owns two idempotent `CREATE TABLE` statements because the
queue is runtime infrastructure, not a service schema, so the guard checks its
store and tables instead of a ledger.

The contract this checker enforces is `docs/architecture/desktop.md`. The rule
text is `AGENTS.md` section 3; P1e of the v1.1 plan introduced the guard.
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = "crates"
SQLITE_DIR = "migrations/sqlite"
TEST_MODULE = "#[cfg(test)]"

# Postgres spellings that have no SQLite meaning. The rule and the pattern are
# separate so a violation names the exact reason it failed.
FORBIDDEN_SQL = (
    ("baseline-schema", re.compile(r"\bCREATE\s+SCHEMA\b", re.IGNORECASE)),
    ("baseline-type", re.compile(r"\bjsonb\b", re.IGNORECASE)),
    ("baseline-type", re.compile(r"\btimestamptz\b", re.IGNORECASE)),
    ("baseline-lock", re.compile(r"\bFOR\s+UPDATE\b", re.IGNORECASE)),
)

CREATE_TABLE = re.compile(
    r"\bCREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?"
    r"([A-Za-z][A-Za-z0-9_]*)",
    re.IGNORECASE,
)
CREATE_RELATION = re.compile(
    r"\bCREATE\s+(?:UNIQUE\s+)?(?:TABLE|INDEX|TRIGGER)\s+"
    r"(?:IF\s+NOT\s+EXISTS\s+)?"
    r"([A-Za-z][A-Za-z0-9_]*)",
    re.IGNORECASE,
)
PREFIXED_TABLE = re.compile(r"(?<![A-Za-z0-9_])([a-z][a-z0-9_]*)__([a-z][a-z0-9_]*)")
SERVICE_DATABASE = re.compile(
    r"open_service_database\s*\(\s*&config\s*,\s*\"([a-z][a-z0-9_]*)\"",
    re.DOTALL,
)
RUNTIME_BACKENDS = "crates/quantrule-runtime/src/backends.rs"


@dataclass(frozen=True)
class Ledger:
    """One ledger row: a service schema, its prefix, and the files it owns."""

    name: str
    crate: str
    prefix: str
    tables: tuple[str, ...]
    baseline: str | None
    store: str
    main: str | None
    extra_baselines: tuple[str, ...] = ()


@dataclass(frozen=True)
class Violation:
    """One conformance failure with a reproducible location."""

    rule: str
    path: str
    message: str

    def render(self) -> str:
        return f"{self.rule}: {self.path}: {self.message}"


# The complete SQLite inventory. `runs`, `research`, and `stage` share one
# ledger and one forward-only history: the run and research tables start at
# version 1, the diagnostic stage tables are version 2, the run model artifact
# column is the additive version 3, and the research ledger tables are the
# additive version 4. The table set is written per prefix so it stays explicit.
# `queue` has no baseline and no binary: the runtime adapter owns its two
# tables, so `baseline` and `main` are `None` on purpose, not an omission.
LEDGERS = (
    Ledger(
        name="gateway",
        crate="quantrule-gateway",
        prefix="gateway",
        tables=("gateway__profiles",),
        baseline="migrations/sqlite/0001_profiles.sql",
        store="src/adapters/sqlite/mod.rs",
        main="src/bin/qr_gateway.rs",
    ),
    Ledger(
        name="catalog",
        crate="quantrule-data-service",
        prefix="catalog",
        tables=(
            "catalog__config_bundles",
            "catalog__configs",
            "catalog__datasets",
        ),
        baseline="migrations/sqlite/0001_catalog.sql",
        store="src/sqlite.rs",
        main="src/main.rs",
    ),
    Ledger(
        name="runs",
        crate="quantrule-evidence-service",
        prefix="runs",
        tables=(
            "runs__artifacts",
            "runs__runs",
            "runs__stages",
            "runs__submissions",
        ),
        baseline="migrations/sqlite/0001_runs_research.sql",
        extra_baselines=("migrations/sqlite/0003_run_model_artifact.sql",),
        store="src/sqlite/mod.rs",
        main="src/main.rs",
    ),
    Ledger(
        name="runs",
        crate="quantrule-evidence-service",
        prefix="research",
        tables=("research__hypotheses",),
        baseline="migrations/sqlite/0001_runs_research.sql",
        store="src/sqlite/mod.rs",
        main="src/main.rs",
    ),
    Ledger(
        name="runs",
        crate="quantrule-evidence-service",
        prefix="research",
        tables=(
            "research__improvement_runs",
            "research__baselines",
            "research__invalidations",
        ),
        baseline="migrations/sqlite/0004_research_ledger.sql",
        store="src/sqlite/mod.rs",
        main="src/main.rs",
    ),
    Ledger(
        name="runs",
        crate="quantrule-evidence-service",
        prefix="stage",
        tables=(
            "stage__artifacts",
            "stage__runs",
            "stage__submissions",
        ),
        baseline="migrations/sqlite/0002_stage.sql",
        store="src/stage/sqlite.rs",
        main="src/main.rs",
    ),
    Ledger(
        name="simulation",
        crate="quantrule-simulation-service",
        prefix="simulation",
        tables=(
            "simulation__marks",
            "simulation__retention_events",
            "simulation__runs",
        ),
        baseline="migrations/sqlite/0001_simulation.sql",
        extra_baselines=("migrations/sqlite/0002_simulation_retention.sql",),
        store="src/sqlite/mod.rs",
        main="src/main.rs",
    ),
    Ledger(
        name="queue",
        crate="quantrule-runtime",
        prefix="queue",
        tables=("queue__dead_letter", "queue__messages"),
        baseline=None,
        store="src/adapters/inprocess.rs",
        main=None,
    ),
)


def relative(root: Path, path: Path) -> str:
    return path.relative_to(root).as_posix()


def strip_sql_comments(text: str) -> str:
    """Blank `--` comments while keeping quoted literals and line numbers."""
    out: list[str] = []
    in_string = False
    index = 0
    while index < len(text):
        char = text[index]
        if char == "'":
            in_string = not in_string
            out.append(char)
            index += 1
            continue
        if not in_string and text.startswith("--", index):
            while index < len(text) and text[index] != "\n":
                out.append(" ")
                index += 1
            continue
        out.append(char)
        index += 1
    return "".join(out)


def scan_rust(text: str) -> tuple[str, list[tuple[int, int]]]:
    """Return the text with comments and literals blanked, plus literal bodies.

    Comments, string literals, raw strings, and char literals become spaces. The
    body of every string literal is reported as a half-open range into `text`,
    so a caller can read the value without reading a comment.
    """
    out = list(text)
    bodies: list[tuple[int, int]] = []
    index = 0
    length = len(text)

    def blank(start: int, end: int) -> None:
        for position in range(start, end):
            if text[position] != "\n":
                out[position] = " "

    while index < length:
        if text.startswith("//", index):
            start = index
            index = text.find("\n", index)
            if index == -1:
                index = length
            blank(start, index)
            continue
        if text.startswith("/*", index):
            start, depth = index, 1
            index += 2
            while index < length and depth:
                if text.startswith("/*", index):
                    depth += 1
                    index += 2
                elif text.startswith("*/", index):
                    depth -= 1
                    index += 2
                else:
                    index += 1
            blank(start, index)
            continue
        raw = None
        if text[index] == "r" and (index == 0 or not text[index - 1].isalnum()):
            raw = re.match(r'r(#+)"', text[index:])
        if raw is not None:
            start = index
            closing = '"' + raw.group(1)
            body_start = index + raw.end()
            body_end = text.find(closing, body_start)
            if body_end == -1:
                body_end = length
            bodies.append((body_start, body_end))
            index = min(body_end + len(closing), length)
            blank(start, index)
            continue
        if text[index] == '"':
            start = index
            body_start = index + 1
            index = body_start
            while index < length:
                if text[index] == "\\":
                    index += 2
                    continue
                if text[index] == '"':
                    break
                index += 1
            bodies.append((body_start, min(index, length)))
            index = min(index + 1, length)
            blank(start, index)
            continue
        char = re.match(r"'(?:\\.|[^\\'\n])'", text[index:])
        if char is not None:
            blank(index, index + char.end())
            index += char.end()
            continue
        index += 1
    return "".join(out), bodies


def test_module_spans(masked: str) -> list[tuple[int, int]]:
    """Return the half-open ranges of every `#[cfg(test)]` item."""
    spans: list[tuple[int, int]] = []
    search = 0
    while True:
        start = masked.find(TEST_MODULE, search)
        if start == -1:
            return spans
        brace = masked.find("{", start)
        if brace == -1:
            return spans
        depth, index = 0, brace
        while index < len(masked):
            if masked[index] == "{":
                depth += 1
            elif masked[index] == "}":
                depth -= 1
                if depth == 0:
                    index += 1
                    break
            index += 1
        spans.append((start, index))
        masked = masked[:start] + " " * (index - start) + masked[index:]
        search = index


def table_references(text: str) -> set[tuple[str, str]]:
    """Every `prefix__table` value in a Rust file outside its test modules."""
    masked, bodies = scan_rust(text)
    tests = test_module_spans(masked)
    found: set[tuple[str, str]] = set()
    for start, end in bodies:
        if any(test_start <= start < test_end for test_start, test_end in tests):
            continue
        for prefix, suffix in PREFIXED_TABLE.findall(text[start:end]):
            found.add((prefix, f"{prefix}__{suffix}"))
    return found


def check(root: Path) -> list[Violation]:
    """Return every conformance failure under `root`; empty means pass."""
    root = Path(root)
    violations: list[Violation] = []

    discovered = {
        path.parent.parent.parent.name
        for path in (root / CRATES).glob(f"*/{SQLITE_DIR}/*.sql")
    }
    inventory_crates = {ledger.crate for ledger in LEDGERS}
    for crate in sorted(discovered - inventory_crates):
        violations.append(
            Violation(
                "crate-discovery",
                f"{CRATES}/{crate}/{SQLITE_DIR}",
                "a SQLite baseline exists for a crate the inventory does not list",
            )
        )

    by_crate: dict[str, list[Ledger]] = {}
    for ledger in LEDGERS:
        by_crate.setdefault(ledger.crate, []).append(ledger)

    for crate, rows in sorted(by_crate.items()):
        violations.extend(check_crate(root, crate, rows))

    for ledger in LEDGERS:
        for table in ledger.tables:
            prefix, separator, suffix = table.partition("__")
            if not separator or not suffix or prefix != ledger.prefix:
                violations.append(
                    Violation(
                        "prefix",
                        f"inventory:{ledger.name}",
                        f"{table} is not a {ledger.prefix}__table name",
                    )
                )
    return violations


def check_crate(root: Path, crate: str, rows: list[Ledger]) -> list[Violation]:
    """Check one crate's baselines, stores, main branches, and ledger rows."""
    base = root / CRATES / crate
    violations: list[Violation] = []
    declared = {row.baseline for row in rows if row.baseline}
    declared.update(
        baseline for row in rows for baseline in row.extra_baselines if baseline
    )
    sqlite_dir = base / SQLITE_DIR
    actual = {
        relative(base, path)
        for path in sorted(sqlite_dir.glob("*.sql"))
        if path.is_file()
    } if sqlite_dir.is_dir() else set()

    if actual != declared:
        missing = sorted(declared - actual)
        extra = sorted(actual - declared)
        detail = []
        if missing:
            detail.append(f"missing {', '.join(missing)}")
        if extra:
            detail.append(f"undeclared {', '.join(extra)}")
        violations.append(
            file_violation(
                "baseline-drift",
                root,
                sqlite_dir,
                "; ".join(detail),
            )
        )

    declared_prefixes = tuple(sorted({row.prefix for row in rows}))
    declared_tables = {table for row in rows for table in row.tables}
    if declared:
        created: set[str] = set()
        for path in sorted(sqlite_dir.glob("*.sql")):
            sql = strip_sql_comments(path.read_text(encoding="utf-8"))
            for rule, pattern in FORBIDDEN_SQL:
                match = pattern.search(sql)
                if match is not None:
                    violations.append(
                        file_violation(
                            rule,
                            root,
                            path,
                            f"SQLite baseline uses the Postgres spelling {match.group(0)!r}",
                        )
                    )
            for name in CREATE_RELATION.findall(sql):
                prefix, separator, suffix = name.partition("__")
                if not separator or not suffix:
                    violations.append(
                        file_violation(
                            "baseline-prefix",
                            root,
                            path,
                            f"{name} is not a prefix__table identifier",
                        )
                    )
                elif prefix not in declared_prefixes:
                    violations.append(
                        file_violation(
                            "baseline-prefix",
                            root,
                            path,
                            f"{name} is outside {', '.join(declared_prefixes)}",
                        )
                    )
            created.update(CREATE_TABLE.findall(sql))
        if created != declared_tables:
            missing = sorted(declared_tables - created)
            extra = sorted(created - declared_tables)
            detail = []
            if missing:
                detail.append(f"missing {', '.join(missing)}")
            if extra:
                detail.append(f"undeclared {', '.join(extra)}")
            violations.append(
                file_violation(
                    "baseline-drift",
                    root,
                    sqlite_dir,
                    "the created tables do not match the inventory: " + "; ".join(detail),
                )
            )

        wiring = base / "src" / "migrations.rs"
        wiring_text = wiring.read_text(encoding="utf-8") if wiring.is_file() else ""
        for baseline in sorted(declared):
            if Path(baseline).name not in wiring_text:
                violations.append(
                    file_violation(
                        "baseline-wired",
                        root,
                        wiring,
                        f"{Path(baseline).name} is not listed by migrations.rs",
                    )
                )

    seen_main: set[str] = set()
    seen_store: set[str] = set()
    for row in rows:
        main = base / row.main if row.main else None
        if row.main is None:
            # The runtime queue is opened by the services that select it and
            # has no binary of its own; its store is checked below.
            pass
        elif not main.is_file():
            violations.append(
                file_violation(
                    "profile-branch",
                    root,
                    base,
                    f"the {row.prefix} ledger has no {row.main} to branch",
                )
            )
        elif row.main not in seen_main:
            seen_main.add(row.main)
            same_main = [other for other in rows if other.main == row.main]
            violations.extend(check_profile_branch(root, main, same_main))

        store = base / row.store
        if not store.is_file():
            violations.append(
                file_violation(
                    "store-missing",
                    root,
                    store,
                    f"the {row.prefix} ledger has no store module",
                )
            )
        elif row.store not in seen_store:
            seen_store.add(row.store)
            same_store = [other for other in rows if other.store == row.store]
            violations.extend(check_store(root, store, same_store))

    violations.extend(check_runtime_branch(root))

    store_paths = {row.store for row in rows}
    for path in sorted((base / "src").glob("**/*.rs")):
        if relative(base, path) in store_paths:
            continue
        violations.extend(
            prefix_violations(
                root, path, table_references(path.read_text(encoding="utf-8")), rows
            )
        )
    return violations


def check_profile_branch(
    root: Path, main: Path, rows: list[Ledger]
) -> list[Violation]:
    """Prove `<service> main` names its ledger and both store backends."""
    text = main.read_text(encoding="utf-8")
    violations: list[Violation] = []
    for token in (
        "open_service_database(",
        "migrations::sqlite()",
        "ServiceDatabase::Sqlite",
        "ServiceDatabase::Postgres",
    ):
        if token not in text:
            violations.append(
                file_violation(
                    "profile-branch",
                    root,
                    main,
                    f"the service database does not name {token}",
                )
            )
    expected = sorted({row.name for row in rows})
    found = sorted(set(SERVICE_DATABASE.findall(text)))
    if found != expected:
        detail = f"expected {', '.join(expected)}; found {', '.join(found) or 'none'}"
        violations.append(
            file_violation("ledger-row", root, main, f"the ledger row is wrong: {detail}")
        )
    return violations


def check_runtime_branch(root: Path) -> list[Violation]:
    """Prove the shared helper still branches on the profile and opens one file."""
    path = root / RUNTIME_BACKENDS
    if not path.is_file():
        return [
            file_violation(
                "profile-branch",
                root,
                path,
                "the shared service-database helper is missing",
            )
        ]
    text = path.read_text(encoding="utf-8")
    violations: list[Violation] = []
    for token in ("DeploymentProfile::Desktop", '"QR_DATA_DIR"'):
        if token not in text:
            violations.append(
                file_violation(
                    "profile-branch",
                    root,
                    path,
                    f"the desktop branch does not name {token}",
                )
            )
    if "DATABASE_FILE" not in text:
        violations.append(
            file_violation(
                "single-file",
                root,
                path,
                "the desktop branch does not open the shared quantrule.sqlite file",
            )
        )
    if "apply_sqlite_migrations(" not in text or "ledger," not in text:
        violations.append(
            file_violation(
                "ledger-applied",
                root,
                path,
                "the shared helper does not apply the caller's ledger",
            )
        )
    return violations


def check_store(root: Path, store: Path, rows: list[Ledger]) -> list[Violation]:
    """Prove one store names only its own prefixes and tables."""
    references = table_references(store.read_text(encoding="utf-8"))
    violations = prefix_violations(root, store, references, rows)
    declared = {table for row in rows for table in row.tables}
    named = {table for _, table in references}
    for table in sorted(declared - named):
        violations.append(
            file_violation(
                "store-table",
                root,
                store,
                f"the store never names {table}, so the baseline has no reader",
            )
        )
    for table in sorted(named - declared):
        violations.append(
            file_violation(
                "store-table",
                root,
                store,
                f"the store names {table}, which the inventory does not declare",
            )
        )
    return violations


def prefix_violations(
    root: Path, path: Path, references: set[tuple[str, str]], rows: list[Ledger]
) -> list[Violation]:
    """Report every table reference outside the prefixes the rows own."""
    allowed = {row.prefix for row in rows}
    violations = []
    for prefix, table in sorted(references):
        if prefix not in allowed:
            violations.append(
                file_violation(
                    "store-prefix",
                    root,
                    path,
                    f"{table} is outside {', '.join(sorted(allowed))}",
                )
            )
    return violations


def file_violation(rule: str, root: Path, path: Path, message: str) -> Violation:
    """Build a violation for an absolute path under `root`."""
    try:
        name = path.relative_to(root).as_posix()
    except ValueError:
        name = path.as_posix()
    return Violation(rule, name, message)


def summary() -> str:
    """The one-line pass message."""
    tables = sum(len(ledger.tables) for ledger in LEDGERS)
    baselines = len(
        {
            baseline
            for ledger in LEDGERS
            for baseline in ((ledger.baseline,) if ledger.baseline else ())
            + ledger.extra_baselines
        }
    )
    return (
        f"schema prefixes ok: {len(LEDGERS)} ledger rows, {tables} tables, "
        f"{baselines} baselines"
    )


def parse_arguments(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        prog="check_schema_prefixes.py",
        description=(
            "Check the desktop SQLite inventory: service prefixes, baseline "
            "types, ledger rows, store modules, and the desktop profile branch."
        ),
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
    violations = check(arguments.root.resolve())
    if violations:
        for violation in violations:
            print(f"error: {violation.render()}", file=sys.stderr)
        print(
            f"schema prefixes failed: {len(violations)} violation(s)",
            file=sys.stderr,
        )
        return 1
    print(summary())
    return 0


if __name__ == "__main__":
    sys.exit(main())
