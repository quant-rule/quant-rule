#!/usr/bin/env python3
"""Unit tests for the structural fitness checker.

The tests exercise the rules against minimal temporary trees so a rule change
cannot silently stop matching a violation.
"""

from __future__ import annotations

import tempfile
import unittest
from pathlib import Path

import check_structure


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def make_crate(
    root: Path,
    name: str,
    source: str = "//! test crate\n\npub fn value() -> u8 { 1 }\n",
) -> None:
    write(root / "crates" / name / "src" / "lib.rs", source + "\n#![forbid(unsafe_code)]\n")


def make_engines(root: Path) -> None:
    for crate in check_structure.ENGINE_CRATES:
        make_crate(root, crate)


class ModuleDocTests(unittest.TestCase):
    def test_missing_module_doc_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_crate(root, "quantrule-data", "pub fn value() -> u8 { 1 }\n")
            violations = check_structure.check_module_docs(root)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "module-doc")

    def test_test_files_are_skipped(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_crate(root, "quantrule-data")
            write(root / "crates" / "quantrule-data" / "src" / "tests.rs", "fn helper() {}\n")
            self.assertEqual(check_structure.check_module_docs(root), [])


class EnginePurityTests(unittest.TestCase):
    def test_filesystem_import_in_engine_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_engines(root)
            make_crate(root, "quantrule-model", "//! model\n\nuse std::fs;\n")
            violations = check_structure.check_engine_purity(root)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "engine-purity")

    def test_test_file_may_use_a_clock(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_engines(root)
            write(
                root / "crates" / "quantrule-model" / "src" / "time_tests.rs",
                "use std::time::Instant;\n",
            )
            self.assertEqual(check_structure.check_engine_purity(root), [])


class StageCommandCouplingTests(unittest.TestCase):
    def test_stage_module_importing_command_types_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-shell" / "src" / "analytics" / "run.rs",
                "use crate::command::types::AnalyticsEvaluateArgs;\n",
            )
            violations = check_structure.check_stage_command_coupling(root)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "stage-command-coupling")

    def test_stage_module_importing_command_writer_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-shell" / "src" / "backtest" / "run.rs",
                "use crate::command::model_bundle_from;\n",
            )
            violations = check_structure.check_stage_command_coupling(root)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "stage-command-coupling")

    def test_stage_module_importing_request_is_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-shell" / "src" / "analytics" / "run.rs",
                "use crate::request::AnalyticsEvaluateArgs;\n",
            )
            self.assertEqual(check_structure.check_stage_command_coupling(root), [])

    def test_request_module_importing_command_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-shell" / "src" / "request.rs",
                "use crate::command::types::Command;\n",
            )
            violations = check_structure.check_artifact_command_coupling(root)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "artifact-command-coupling")

    def test_model_bundle_module_importing_command_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-shell" / "src" / "model_bundle" / "mod.rs",
                "use crate::command::types::Command;\n",
            )
            violations = check_structure.check_artifact_command_coupling(root)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "artifact-command-coupling")

    def test_model_bundle_module_importing_render_report_is_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-shell" / "src" / "model_bundle" / "mod.rs",
                "use crate::render_report::BundleError;\n",
            )
            self.assertEqual(check_structure.check_artifact_command_coupling(root), [])


class DateModuleCouplingTests(unittest.TestCase):
    def test_old_date_path_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-shell" / "src" / "config" / "data.rs",
                "use crate::source::date::DateRange;\n",
            )
            violations = check_structure.check_date_module_coupling(root)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "date-module-coupling")

    def test_date_module_path_is_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-shell" / "src" / "config" / "data.rs",
                "use crate::date::DateRange;\n",
            )
            self.assertEqual(check_structure.check_date_module_coupling(root), [])

    def test_source_date_file_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-shell" / "src" / "source" / "date.rs",
                "//! date\n",
            )
            violations = check_structure.check_date_module_coupling(root)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "date-module-coupling")

    def test_source_date_reexport_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(
                root / "crates" / "quantrule-shell" / "src" / "source" / "mod.rs",
                "pub(crate) use date::{DateBounds, DateRange};\n",
            )
            violations = check_structure.check_date_module_coupling(root)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "date-module-coupling")


class NamingTests(unittest.TestCase):
    def test_catch_all_module_is_reported(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_crate(root, "quantrule-data")
            write(root / "crates" / "quantrule-data" / "src" / "utils.rs", "//! helpers\n")
            violations = check_structure.check_naming(root)
            self.assertTrue(any(violation.rule == "catch-all-module" for violation in violations))

    def test_refactor_document_name_must_be_kebab_case(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "docs" / "refactors" / "2026-11-06" / "Bad_Name.md", "# bad\n")
            violations = check_structure.check_naming(root)
            self.assertTrue(any(violation.rule == "refactor-doc-name" for violation in violations))


class LayoutTests(unittest.TestCase):
    def test_dated_docs_directory_is_checked(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "docs" / "audits" / "not-a-date").mkdir(parents=True)
            violations = check_structure.check_dated_docs(root)
            self.assertEqual(len(violations), 1)
            self.assertEqual(violations[0].rule, "dated-docs-directory")

    def test_workflow_document_must_be_indexed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "docs" / "README.md", "# Docs\n")
            write(root / "docs" / "workflows" / "README.md", "# Workflows\n")
            write(root / "docs" / "workflows" / "audit-workflow.md", "# Audit\n")
            violations = check_structure.workflow_index_violations(root)
            self.assertEqual(len(violations), 2)
            self.assertTrue(all(violation.rule == "workflow-index" for violation in violations))

    def test_workflow_document_indexed_in_both_places_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            write(root / "docs" / "README.md", "# Docs\n[audit](workflows/audit-workflow.md)\n")
            write(root / "docs" / "workflows" / "README.md", "# Workflows\n[audit](audit-workflow.md)\n")
            write(root / "docs" / "workflows" / "audit-workflow.md", "# Audit\n")
            self.assertEqual(check_structure.workflow_index_violations(root), [])

    def test_workflow_templates_directory_is_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "docs" / "workflows" / "templates").mkdir(parents=True)
            self.assertEqual(check_structure.check_dated_docs(root), [])

    def test_course_directories_are_allowed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "docs" / "course" / "architecture").mkdir(parents=True)
            (root / "docs" / "course" / "methodology").mkdir(parents=True)
            self.assertEqual(check_structure.check_dated_docs(root), [])

    def test_data_allowlist_accepts_readmes_and_partitions(self) -> None:
        self.assertEqual(
            check_structure.data_allowlist_violations(
                ["data/README.md", "data/sources/README.md", "data/partitions.json"]
            ),
            [],
        )

    def test_data_allowlist_rejects_a_candle_file(self) -> None:
        violations = check_structure.data_allowlist_violations(["data/sources/BTCUSD_M1.csv"])
        self.assertEqual(len(violations), 1)
        self.assertEqual(violations[0].rule, "data-allowlist")


class FullTreeTests(unittest.TestCase):
    def test_a_minimal_clean_tree_passes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            make_engines(root)
            make_crate(root, "quantrule-shell", "//! shell\n\nfn main() {}\n")
            write(root / "docs" / "README.md", "# Docs\n\n[workflow](workflows/audit-workflow.md)\n")
            write(root / "docs" / "workflows" / "README.md", "# Workflows\n\n[workflow](audit-workflow.md)\n")
            write(root / "docs" / "workflows" / "audit-workflow.md", "# Audit\n")
            self.assertEqual(check_structure.check_all(root), [])


if __name__ == "__main__":
    unittest.main()
