#!/usr/bin/env python3
"""Unit tests for the desktop SQLite conformance guard.

Every test builds a minimal tree from the checker's own inventory, then seeds
one violation and asserts the rule that must fire. A rule that stops matching
therefore fails here instead of passing silently in the gate.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_schema_prefixes


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def baseline_text(tables: tuple[str, ...]) -> str:
    header = (
        "-- The SQLite spelling is explained in a comment: jsonb and timestamptz\n"
        "-- are Postgres, and neither CREATE SCHEMA nor FOR UPDATE exists here.\n"
    )
    body = "\n".join(
        f"CREATE TABLE IF NOT EXISTS {table} (id TEXT PRIMARY KEY) STRICT;\n"
        f"CREATE INDEX IF NOT EXISTS {table}_idx ON {table} (id);"
        for table in tables
    )
    return header + body + "\n"


def migrations_text(baseline: str) -> str:
    return (
        "use quantrule_runtime::migrations::Migration;\n\n"
        "pub fn sqlite() -> Vec<Migration> {\n"
        "    vec![Migration {\n"
        "        version: 1,\n"
        '        name: "probe",\n'
        f'        sql: include_str!("../migrations/sqlite/{Path(baseline).name}"),\n'
        "    }]\n"
        "}\n"
    )


def main_text(ledger: str) -> str:
    return (
        "async fn main() -> Result<(), RuntimeError> {\n"
        "    if config.profile == DeploymentProfile::Desktop {\n"
        "        let data_dir = config.data_dir.as_deref().ok_or(ConfigError::Missing {\n"
        '            name: "QR_DATA_DIR",\n'
        "        })?;\n"
        "        let path = data_dir.join(DATABASE_FILE);\n"
        "        let applied = apply_sqlite_migrations(\n"
        "            &database,\n"
        f'            "{ledger}",\n'
        "            &crate::migrations::sqlite(),\n"
        "        )\n"
        "        .await;\n"
        "    }\n"
        "    Ok(())\n"
        "}\n"
    )


def store_text(tables: tuple[str, ...]) -> str:
    values = ", ".join(f'"{table}"' for table in tables)
    return (
        "//! probe store\n\n"
        f"const TABLES: [&str; {len(tables)}] = [{values}];\n\n"
        "#[cfg(test)]\n"
        "mod tests {\n"
        "    #[test]\n"
        "    fn another prefix is refused in a test only() {\n"
        '        let _ = "gateway__profiles";\n'
        "    }\n"
        "}\n"
    )


def write_valid_tree(root: Path) -> None:
    """Write the files the checker's inventory declares, and nothing else."""
    grouped: dict[str, list[check_schema_prefixes.Ledger]] = {}
    for ledger in check_schema_prefixes.LEDGERS:
        grouped.setdefault(ledger.crate, []).append(ledger)
    for crate, rows in grouped.items():
        base = root / "crates" / crate
        tables = tuple(table for row in rows for table in row.tables)
        baselines = sorted({row.baseline for row in rows if row.baseline})
        for baseline in baselines:
            write(base / baseline, baseline_text(tables))
        if baselines:
            write(
                base / "src" / "migrations.rs",
                "".join(migrations_text(baseline) for baseline in baselines),
            )
        ledgers = sorted({row.name for row in rows})
        for row in rows:
            if row.main is not None:
                write(base / row.main, main_text(ledgers[0]))
            write(base / row.store, store_text(tables))


class GuardTestCase(unittest.TestCase):
    def setUp(self) -> None:
        self._temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self._temporary.cleanup)
        self.root = Path(self._temporary.name)
        write_valid_tree(self.root)

    def rules(self) -> set[str]:
        return {violation.rule for violation in check_schema_prefixes.check(self.root)}

    def replace(self, relative: str, old: str, new: str) -> None:
        path = self.root / relative
        text = path.read_text(encoding="utf-8")
        self.assertIn(old, text)
        path.write_text(text.replace(old, new), encoding="utf-8")

    def append(self, relative: str, text: str) -> None:
        path = self.root / relative
        path.write_text(path.read_text(encoding="utf-8") + text, encoding="utf-8")


class CompleteInventoryTests(GuardTestCase):
    def test_a_complete_inventory_passes(self) -> None:
        self.assertEqual(check_schema_prefixes.check(self.root), [])

    def test_a_comment_may_name_a_postgres_spelling(self) -> None:
        baseline = (
            self.root
            / "crates"
            / "quantrule-gateway"
            / "migrations"
            / "sqlite"
            / "0001_profiles.sql"
        )
        self.assertIn("jsonb", baseline.read_text(encoding="utf-8"))
        self.assertEqual(check_schema_prefixes.check(self.root), [])


class PrefixTests(GuardTestCase):
    def test_a_wrong_prefix_in_a_baseline_is_reported(self) -> None:
        self.replace(
            "crates/quantrule-gateway/migrations/sqlite/0001_profiles.sql",
            "gateway__profiles",
            "catalog__profiles",
        )
        self.assertIn("baseline-prefix", self.rules())

    def test_an_unprefixed_table_is_reported(self) -> None:
        self.append(
            "crates/quantrule-data-service/migrations/sqlite/0001_catalog.sql",
            "CREATE TABLE probe (id TEXT PRIMARY KEY) STRICT;\n",
        )
        self.assertIn("baseline-prefix", self.rules())

    def test_a_store_that_names_another_prefix_is_reported(self) -> None:
        self.append(
            "crates/quantrule-data-service/src/sqlite.rs",
            'const LEAK: &str = "gateway__profiles";\n',
        )
        self.assertIn("store-prefix", self.rules())

    def test_a_store_that_drifts_from_the_baseline_is_reported(self) -> None:
        self.replace(
            "crates/quantrule-data-service/src/sqlite.rs",
            '"catalog__configs"',
            '"catalog__config_rows"',
        )
        self.assertIn("store-table", self.rules())

    def test_a_test_module_may_name_another_prefix(self) -> None:
        path = self.root / "crates" / "quantrule-data-service" / "src" / "sqlite.rs"
        self.assertIn("gateway__profiles", path.read_text(encoding="utf-8"))
        self.assertEqual(check_schema_prefixes.check(self.root), [])


class PostgresSpellingTests(GuardTestCase):
    def append_to_gateway(self, statement: str) -> None:
        self.append(
            "crates/quantrule-gateway/migrations/sqlite/0001_profiles.sql",
            statement + "\n",
        )

    def test_jsonb_is_reported(self) -> None:
        self.append_to_gateway("CREATE TABLE probe (payload jsonb) STRICT;")
        self.assertIn("baseline-type", self.rules())

    def test_timestamptz_is_reported(self) -> None:
        self.append_to_gateway("CREATE TABLE probe (at timestamptz) STRICT;")
        self.assertIn("baseline-type", self.rules())

    def test_create_schema_is_reported(self) -> None:
        self.append_to_gateway("CREATE SCHEMA gateway;")
        self.assertIn("baseline-schema", self.rules())

    def test_for_update_is_reported(self) -> None:
        self.append_to_gateway("SELECT 1 FROM gateway__profiles FOR UPDATE;")
        self.assertIn("baseline-lock", self.rules())


class LedgerRowTests(GuardTestCase):
    def test_a_wrong_ledger_row_is_reported(self) -> None:
        self.replace(
            "crates/quantrule-data-service/src/main.rs",
            '"catalog",',
            '"gateway",',
        )
        self.assertIn("ledger-row", self.rules())

    def test_a_missing_ledger_row_is_reported(self) -> None:
        self.replace(
            "crates/quantrule-data-service/src/main.rs",
            "apply_sqlite_migrations(",
            "apply_something_else(",
        )
        self.assertIn("ledger-row", self.rules())

    def test_a_missing_baseline_wiring_is_reported(self) -> None:
        self.replace(
            "crates/quantrule-data-service/src/migrations.rs",
            "0001_catalog.sql",
            "0001_removed.sql",
        )
        self.assertIn("baseline-wired", self.rules())

    def test_an_undeclared_baseline_is_reported(self) -> None:
        write(
            self.root
            / "crates"
            / "quantrule-data-service"
            / "migrations"
            / "sqlite"
            / "0002_probe.sql",
            "CREATE TABLE catalog__probe (id TEXT) STRICT;\n",
        )
        self.assertIn("baseline-drift", self.rules())

    def test_a_crate_outside_the_inventory_is_reported(self) -> None:
        write(
            self.root / "crates" / "quantrule-model" / "migrations" / "sqlite" / "0001.sql",
            "CREATE TABLE model__probe (id TEXT) STRICT;\n",
        )
        self.assertIn("crate-discovery", self.rules())


class ProfileBranchTests(GuardTestCase):
    def test_a_missing_data_directory_branch_is_reported(self) -> None:
        self.replace(
            "crates/quantrule-gateway/src/bin/qr_gateway.rs",
            '"QR_DATA_DIR",',
            '"QR_PROFILE",',
        )
        self.assertIn("profile-branch", self.rules())

    def test_a_missing_single_file_open_is_reported(self) -> None:
        self.replace(
            "crates/quantrule-data-service/src/main.rs",
            "data_dir.join(DATABASE_FILE)",
            "data_dir.join(PROBE_FILE)",
        )
        self.assertIn("single-file", self.rules())

    def test_a_missing_store_module_is_reported(self) -> None:
        (self.root / "crates" / "quantrule-evidence-service" / "src" / "sqlite" / "mod.rs").unlink()
        self.assertIn("store-missing", self.rules())

    def test_a_store_owned_ddl_is_checked_without_a_baseline(self) -> None:
        self.replace(
            "crates/quantrule-runtime/src/adapters/inprocess.rs",
            '"queue__dead_letter"',
            '"queue__dead"',
        )
        self.assertIn("store-table", self.rules())

    def test_a_queue_baseline_would_be_reported_as_undeclared(self) -> None:
        write(
            self.root / "crates" / "quantrule-runtime" / "migrations" / "sqlite" / "0001_queue.sql",
            "CREATE TABLE queue__messages (id TEXT) STRICT;\n",
        )
        self.assertIn("baseline-drift", self.rules())


if __name__ == "__main__":
    unittest.main()
